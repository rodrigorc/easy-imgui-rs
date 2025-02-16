use futures::FutureExt;
use std::{
    cell::Cell,
    future::Future,
    mem::ManuallyDrop,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};
use winit::event_loop::EventLoopProxy;

use crate::{AppEvent, Application, Args};

trait IdleRunner {
    fn idle_run(&self, f: Box<dyn FnOnce() + Send + Sync>) -> Result<(), ()>;
}

impl<A: Application> IdleRunner for EventLoopProxy<AppEvent<A>> {
    fn idle_run(&self, f: Box<dyn FnOnce() + Send + Sync>) -> Result<(), ()> {
        self.send_event(AppEvent::RunIdleSimple(f)).map_err(drop)
    }
}

//The raw pointer in the RawWaker will be a pointer to an Arc-allocated IdleTask
struct IdleTask {
    runner: Box<dyn IdleRunner>,
    future: Cell<Option<Pin<Box<dyn Future<Output = ()> + 'static>>>>,
}

impl Drop for IdleTask {
    fn drop(&mut self) {}
}

#[inline]
unsafe fn increment_arc_count(ptr: *const ()) {
    let rc = Arc::from_raw(ptr as *const IdleTask);
    std::mem::forget(rc.clone());
    std::mem::forget(rc);
}
#[inline]
unsafe fn decrement_arc_count(ptr: *const ()) {
    Arc::from_raw(ptr as *const IdleTask);
}

unsafe fn gwaker_clone(ptr: *const ()) -> RawWaker {
    increment_arc_count(ptr);
    RawWaker::new(ptr, &GWAKER_VTABLE)
}
unsafe fn gwaker_wake(ptr: *const ()) {
    //poll_idle consumes one reference count, as wake requires, so nothing to do
    poll_idle(ptr);
}
unsafe fn gwaker_wake_by_ref(ptr: *const ()) {
    //poll_idle consumes one reference count, so we have to increment it here one
    increment_arc_count(ptr);
    poll_idle(ptr);
}
unsafe fn gwaker_drop(ptr: *const ()) {
    decrement_arc_count(ptr);
}

static GWAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(gwaker_clone, gwaker_wake, gwaker_wake_by_ref, gwaker_drop);

//Actually the inner future is not Send, but IdleTask is private to this module, so if
//we are careful we can move it between threads, as long as we only use the future in the
//main thread.
unsafe impl Send for IdleTask {}
unsafe impl Sync for IdleTask {}

//poll_idle() can be called from an arbitrary thread, because Waker is Send,
//but once we are in the idle callback we are in the main loop.
//When it ends, Waker::drop decrements the counter for the Arc<IdleTask>.
fn poll_idle(ptr: *const ()) {
    let task = unsafe { &*(ptr as *const IdleTask) };

    let res = task.runner.idle_run(Box::new(move || {
        let raw = RawWaker::new(task as *const IdleTask as *const (), &GWAKER_VTABLE);
        let waker = unsafe { Waker::from_raw(raw) };

        // It is unlikely but the call to poll could reenter the this idle call.
        // We avoid reentering the future by taking it from the
        // IdleTask and storing it later if it returns pending.
        // This has the additional advantage that the future is automatically fused.
        let mut op_future = task.future.take();

        if let Some(future) = op_future.as_mut() {
            let mut ctx = Context::from_waker(&waker);
            match future.as_mut().poll(&mut ctx) {
                Poll::Ready(()) => {}
                Poll::Pending => {
                    task.future.set(op_future);
                }
            }
        }
    }));

    if res.is_err() {
        // If running the idle task fails the future and the IdleTask will leak.
        // Try to release everything here, first the future because it usually contains a
        // hidden referece to the waker, then the waker reference that would have consumed
        // the idle task.
        task.future.take();
        unsafe { gwaker_drop(ptr) };
    }
}

/// Spawns an idle future.
///
/// This function registers the given future to be run in the idle time of the main loop.
///
/// It must be called from the main loop or it will panic. Since the future will be run in the
/// same thread, it does not need to be `Send`.
///
/// The future can return any value, but it is discarded.
///
/// Currently there is no way to cancel a future. If you need that you can use `futures::future::AbortHandle`.
///
/// SAFETY: It must be called from the main UI thread, the one that owns the event_loop.
pub unsafe fn spawn_idle<T, F, A>(
    event_proxy: &EventLoopProxy<AppEvent<A>>,
    future: F,
) -> FutureHandle<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
    A: Application,
{
    let runner = Box::new(event_proxy.clone());
    //Check that we are in the main loop
    //assert!(glib::MainContext::default().is_owner(), "idle_spawn can only be called from the main loop");

    let res: Rc<Cell<Option<T>>> = Rc::new(Cell::new(None));
    let task = Arc::new(IdleTask {
        runner,
        future: Cell::new(Some(Box::pin(future.map({
            let res = res.clone();
            move |t| res.set(Some(t))
        })))),
    });
    let wtask = Arc::downgrade(&task);
    let ptr = Arc::into_raw(task) as *const ();
    poll_idle(ptr);
    FutureHandle { task: wtask, res }
}

/// A handle to a running idle future.
///
/// You can use this handle to cancel the future and to retrieve the return
/// value of a future that has finished.
///
/// If the Handle is dropped, the future will keep on running, and there will
/// be no way to cancel it or get the return value.
pub struct FutureHandle<T> {
    task: std::sync::Weak<IdleTask>,
    res: Rc<Cell<Option<T>>>,
}

impl<T> FutureHandle<T> {
    /// If the future has finished, it returns `Some(t)` the first time it is called.
    /// If it has not finished, the future is dropped and it returns `None`.
    pub fn cancel(self) -> Option<T> {
        if let Some(task) = self.task.upgrade() {
            task.future.take();
        }
        self.res.take()
    }
    /// If the future has finished, it returns `Some(t)`.
    /// If it has not finished, it does nothing and return `None`.
    pub fn result(&self) -> Option<T> {
        self.res.take()
    }
    /// Returns `true` if the future has finished, `false` otherwise.
    /// If it returns `true` then you can be sure that [`cancel`](#method.cancel) will return `Some(t)`.
    pub fn has_finished(&self) -> bool {
        match self.res.take() {
            None => false,
            Some(x) => {
                self.res.set(Some(x));
                true
            }
        }
    }
    /// Converts this handle into a future that is satisfied when the original job finishes.
    /// It returns the value returned by the original future.
    pub async fn future(self) -> T {
        if let Some(task) = self.task.upgrade() {
            if let Some(fut) = task.future.take() {
                fut.await;
            }
        }
        //This unwrap() cannot fail: if the future has finished, then self.res must be Some,
        //because the last action of the future is assigning to it.
        //And the future cannot be cancelled, because both Handle::cancel() and Handle::future()
        //consume self, thus only one of them can be used.
        self.res.take().unwrap()
    }

    pub fn guard(self) -> FutureHandleGuard<T> {
        FutureHandleGuard(ManuallyDrop::new(self))
    }
}

pub struct FutureHandleGuard<T>(ManuallyDrop<FutureHandle<T>>);

impl<T> Drop for FutureHandleGuard<T> {
    fn drop(&mut self) {
        let h = unsafe { ManuallyDrop::take(&mut self.0) };
        h.cancel();
    }
}

/////////////////////////////////////

pub struct FutureBackCaller<A: Application> {
    pd: std::marker::PhantomData<*const A>,
}

impl<A: Application> FutureBackCaller<A> {
    pub fn run<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut A, Args<'_, A>) -> R,
    {
        // Take the thread-local pointer to avoid nested calls
        let the_app = THE_APP.take();
        let guard = TheAppGuard(the_app);

        let Some(the_app) = &guard.0 else {
            return None;
        };

        // Sanity check for the inner type
        let type_id = std::any::TypeId::of::<A>();
        if type_id != the_app.type_id {
            return None;
        }
        let mut ptr = the_app.ptr.cast::<TheAppType<'_, A>>();
        let (app, args) = unsafe { ptr.as_mut() };
        // Reborrow Args
        let args = Args {
            window: args.window,
            event_loop: args.event_loop,
            event_proxy: args.event_proxy,
            data: args.data,
        };
        let res = f(app, args);
        Some(res)
    }
}

pub fn future_back_caller_new<A: Application>() -> FutureBackCaller<A> {
    FutureBackCaller {
        pd: std::marker::PhantomData,
    }
}

pub fn future_back_caller_prepare<A: Application>(aa: TheAppType<'_, A>, f: impl FnOnce()) {
    let the_app = TheAppThreadLocal {
        type_id: std::any::TypeId::of::<A>(),
        ptr: std::ptr::NonNull::from(&aa).cast(),
    };
    let old = THE_APP.replace(Some(the_app));
    let _guard = TheAppGuard(old);
    f();
}

struct TheAppThreadLocal {
    // The type of Application
    type_id: std::any::TypeId,
    // A type-deleted pointer to `TheAppType<A>`
    ptr: std::ptr::NonNull<()>,
}

type TheAppType<'x, A> = (&'x mut A, Args<'x, A>);

thread_local! {
    static THE_APP: std::cell::Cell<Option<TheAppThreadLocal>> = const { std::cell::Cell::new(None) };
}

struct TheAppGuard(Option<TheAppThreadLocal>);

impl Drop for TheAppGuard {
    fn drop(&mut self) {
        THE_APP.set(self.0.take());
    }
}
