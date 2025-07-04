use futures_util::{
    FutureExt,
    task::{ArcWake, waker},
};
use send_wrapper::SendWrapper;
use std::{
    cell::Cell,
    future::Future,
    mem::ManuallyDrop,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll},
};
use winit::event_loop::EventLoopProxy;

use crate::{AppEvent, Application, Args};

type Action = Box<dyn FnOnce() + Send + Sync>;
trait IdleRunner: Send + Sync {
    fn idle_run(&self, f: Action) -> Result<(), Action>;
}

impl<A: Application> IdleRunner for EventLoopProxy<AppEvent<A>> {
    fn idle_run(&self, f: Action) -> Result<(), Action> {
        self.send_event(AppEvent::RunIdleSimple(f))
            .map_err(|e| match e.0 {
                AppEvent::RunIdleSimple(a) => a,
                _ => unreachable!(),
            })
    }
}

struct IdleTask {
    runner: Box<dyn IdleRunner>,
    //Actually the inner future is not Send, but IdleTask is private to this module, so if
    //we are careful we can move it between threads, as long as we only use the future in the
    //main thread.
    #[allow(clippy::type_complexity)]
    future: Cell<Option<SendWrapper<Pin<Box<dyn Future<Output = ()> + 'static>>>>>,
}

impl Drop for IdleTask {
    fn drop(&mut self) {
        // A future task is usually dropped in one of two ways:
        // * The future is cancelled (`FutureHandle::cancel): then self.future has already been
        //   taken and it is `None` now.
        // * The future has completed: then self.future has been taken in the `poll` call and it is
        //   also `None` now.
        // The only way `self.future` can be `Some` here is if the future has been cancelled by
        // surprise, such as by shutting down the whole async runtime.This usually happens at the
        // end of the program...
        // Now, if this drop happens to be run from the main loop, all is well. But if it runs from
        // a random thread, we can't drop the future. We'll first try to send it to the main loop,
        // but it this fails, it'll just leak.
        if let Some(future) = self.future.take() {
            // Checks the SendWrapper invariant
            if future.valid() {
                // If we are in the correct thread, drop it.
                drop(future);
            } else {
                // If not, we have to try and send it to the main loop
                let err = self.runner.idle_run(Box::new(move || {
                    let _ = future;
                }));
                // But if idle_run fails, the future can't be dropped, forget the whole thing.
                // The future now lives inside the callback that is returned by the failed
                // `idle_run`.
                if let Err(e) = err {
                    log::warn!(target: "easy_imgui", "future leaked in drop");
                    std::mem::forget(e);
                }
            }
        }
    }
}

// SAFETY: Idle Task is already Send by the `SendWrapper` magic.
// It is not auto-sync because of the `Cell`. but that cell is only touched from the main loop or the
// drop implementation, and those can't break the `Sync` invariant.
// We could add the `Cell` inside the `SendWrapper`, but then `IdleTask::drop` could not reliably get to the
// inner `Option` without calling `idle_run`. If only `SendWrapper` had a `fn unsafe_take()`
// function...
unsafe impl Sync for IdleTask {}

//wake_by_ref() can be called from an arbitrary thread, because Waker is Send,
//but once we are in the idle callback we are in the main loop.
impl ArcWake for IdleTask {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let task = arc_self.clone();
        let err = arc_self.runner.idle_run(Box::new(move || {
            // It is unlikely but the call to poll could reenter the this idle call.
            // We avoid reentering the future by taking it from the
            // IdleTask and storing it later if it returns pending.
            // This has the additional advantage that the future is automatically fused.
            let waker = waker(task.clone());
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

        // If idle_run fails, the future can't be dropped because it is not actually Send, forget the whole thing.
        // It will leak, but there is nothing we can do about it.
        if let Err(e) = err {
            log::warn!(target: "easy_imgui", "future leaked in poll");
            std::mem::forget(e);
        }
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

    let res: Rc<Cell<Option<T>>> = Rc::new(Cell::new(None));
    let task = Arc::new(IdleTask {
        runner,
        future: Cell::new(Some(SendWrapper::new(Box::pin(future.map({
            let res = res.clone();
            move |t| res.set(Some(t))
        }))))),
    });
    let wtask = Arc::downgrade(&task);
    ArcWake::wake_by_ref(&task);
    FutureHandle { task: wtask, res }
}

/// A handle to a running idle future.
///
/// You can use this handle to cancel the future and to retrieve the return
/// value of a future that has finished.
///
/// If the Handle is dropped, the future will keep on running, and there will
/// be no way to cancel it or get the return value.
///
/// This type is notably `!Send`.
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
        if let Some(task) = self.task.upgrade()
            && let Some(fut) = task.future.take()
        {
            fut.take().await;
        }
        //This unwrap() cannot fail: if the future has finished, then self.res must be Some,
        //because the last action of the future is assigning to it.
        //And the future cannot be cancelled, because both Handle::cancel() and Handle::future()
        //consume self, thus only one of them can be used.
        self.res.take().unwrap()
    }

    /// Creates a `FutureHandleGuard` that cancels the future on drop.
    pub fn guard(self) -> FutureHandleGuard<T> {
        FutureHandleGuard(ManuallyDrop::new(self))
    }
}

/// Helper newtype that cancels a future on drop.
pub struct FutureHandleGuard<T>(ManuallyDrop<FutureHandle<T>>);

impl<T> Drop for FutureHandleGuard<T> {
    fn drop(&mut self) {
        let h = unsafe { ManuallyDrop::take(&mut self.0) };
        h.cancel();
    }
}

impl<T> FutureHandleGuard<T> {
    /// Gets the inner handle. It is no longer cancelled automatically.
    pub fn into_inner(mut self) -> FutureHandle<T> {
        let res = unsafe { ManuallyDrop::take(&mut self.0) };
        std::mem::forget(self);
        res
    }
}

/////////////////////////////////////

/// Helper type to get the application arguments during an idle future.
///
/// Idle futures don't have access to the main application, because they must be `'static`.
/// But calling `FutureBackCaller::run` a future can use the application data freely.
/// The main limitation is that you can't await inside `run`, obviously.
/// Another limitation is that you can't call `run` recursively, because it returns a mutable
/// (exclusive) reference to the application.
pub struct FutureBackCaller<A: Application> {
    pd: std::marker::PhantomData<*const A>,
}

impl<A: Application> FutureBackCaller<A> {
    /// Runs the given function with the current application as argument.
    ///
    /// It returns `Some(x)` being `x` the return value of your function.
    /// If you call it recursively, or from a function outside of an idle future it will return
    /// `None`.
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
