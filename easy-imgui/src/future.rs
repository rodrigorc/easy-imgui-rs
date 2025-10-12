use futures_util::{
    FutureExt,
    task::{ArcWake, waker},
};
use send_wrapper::SendWrapper;
/// Helper module to integrate futures in a backend.
///
/// Technically maybe it should be its own crate, without any actual dependency to ImGui,
/// but I don't think it is worth it, for now.
use std::{
    cell::Cell,
    mem::ManuallyDrop,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll},
};

/// A simple function to be enqueued in a callback list.
pub type Action = Box<dyn FnOnce() + Send + Sync>;

/// An IdleRunner provides a mechanism to run a callback during the UI idle time.
pub trait IdleRunner: Send + Sync {
    /// Enqeue the given action to be run during the idle time of the UI thread.
    ///
    /// It should callable from any thread.
    /// If for some reason the action can't be sent, return it back as error.
    fn idle_run(&self, f: Action) -> Result<(), Action>;
}

/// A task that is being run during the idle time.
pub struct IdleTask {
    runner: Box<dyn IdleRunner>,
    // Actually the inner future is not Send, but IdleTask is private to this module, so if
    // we are careful we can move it between threads, as long as we only use the future in the
    // main thread.
    #[allow(clippy::type_complexity)]
    future: Cell<Option<SendWrapper<Pin<Box<dyn Future<Output = ()> + 'static>>>>>,
}

impl IdleTask {
    pub fn new<RUN, FUT>(runner: RUN, f: FUT) -> IdleTask
    where
        RUN: IdleRunner + 'static,
        FUT: Future<Output = ()> + 'static,
    {
        IdleTask {
            runner: Box::new(runner),
            future: Cell::new(Some(SendWrapper::new(Box::pin(f)))),
        }
    }
    pub fn into_future(&self) -> Option<impl Future<Output = ()>> {
        self.future.take().map(|s| s.take())
    }
}

impl Drop for IdleTask {
    fn drop(&mut self) {
        // A future task is usually dropped in one of two ways:
        // * The future is cancelled (`FutureHandle::cancel): then self.future has already been
        //   taken and it is `None` now.
        // * The future has completed: then self.future has been taken in the `poll` call and it is
        //   also `None` now.
        // The only way `self.future` can be `Some` here is if the future has been cancelled by
        // surprise, such as by shutting down the whole async runtime. This usually happens at the
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
            task.into_future();
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
            && let Some(fut) = task.into_future()
        {
            fut.await;
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

/// Spawns an idle future.
///
/// This function registers the given future to be run in the idle time of the main loop.
///
/// It must be called from the main loop or it will panic. Since the future will be run in the
/// same thread, it does not need to be `Send`.
///
/// The future can return any value, but it is discarded.
///
/// You can use the returned handle to get the returned value or cancel the future.
///
/// SAFETY: It must be called from the main UI thread, the one that owns the event_loop.
pub unsafe fn spawn_idle<T, F, R>(runner: R, future: F) -> FutureHandle<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
    R: IdleRunner + 'static,
{
    let res: Rc<Cell<Option<T>>> = Rc::new(Cell::new(None));
    let task = Arc::new(IdleTask::new(
        runner,
        future.map({
            let res = res.clone();
            move |t| res.set(Some(t))
        }),
    ));
    let wtask = Arc::downgrade(&task);
    ArcWake::wake_by_ref(&task);
    FutureHandle { task: wtask, res }
}

/////////////////////////////////////

/// Helper type to get the application arguments during an idle future.
///
/// Idle futures don't have access to the main application, because they must be `'static`.
/// But calling `FutureBackCaller::run` a future can use the application data freely.
/// The main limitation is that you can't await inside `run`, obviously.
/// Another limitation is that you can't call `run` recursively, because it returns a mutable
/// (exclusive) reference to the application.
pub struct FutureBackCallerImpl<T> {
    pd: std::marker::PhantomData<*const T>,
}

impl<T> Default for FutureBackCallerImpl<T> {
    fn default() -> Self {
        FutureBackCallerImpl {
            pd: std::marker::PhantomData,
        }
    }
}

impl<T> FutureBackCallerImpl<T> {
    /// Runs the given function, while making the `t` available.
    ///
    /// If the given function calls `run`, it will receive the `t` as a
    /// parameter.
    pub fn prepare<ID: 'static>(t: T, f: impl FnOnce()) {
        let the_app = TheAppThreadLocal {
            type_id: std::any::TypeId::of::<ID>(),
            ptr: std::ptr::NonNull::from(&t).cast(),
        };
        let old = THE_APP.replace(Some(the_app));
        let _guard = TheAppGuard(old);
        f();
    }

    /// Runs the given function with the current application as argument.
    ///
    /// It returns `Some(x)` being `x` the return value of your function.
    /// If you call it recursively, or from a function outside of an idle future it will return
    /// `None`.
    /// The `ID` is to check that the `prepare` and the `run` use compatible types; we can't use
    /// `T` because it may not be 'static.
    ///
    /// SAFETY: If you use a fake ID you could run into trouble. To be extra safe use a private new-type,
    /// so that nobody can spoof you.
    pub unsafe fn run<ID, F, R>(&self, f: F) -> Option<R>
    where
        ID: 'static,
        F: FnOnce(&mut T) -> R,
    {
        // Take the thread-local pointer to avoid nested calls
        let the_app = THE_APP.take();
        let guard = TheAppGuard(the_app);

        let Some(the_app) = &guard.0 else {
            return None;
        };

        // Sanity check for the inner type
        let type_id = std::any::TypeId::of::<ID>();
        if type_id != the_app.type_id {
            return None;
        }

        let mut ptr = the_app.ptr.cast::<T>();
        let t = unsafe { ptr.as_mut() };
        let res = f(t);
        Some(res)
    }
}

struct TheAppThreadLocal {
    // The type of the stored object
    type_id: std::any::TypeId,
    // A type-deleted pointer to `T`
    ptr: std::ptr::NonNull<()>,
}

thread_local! {
    static THE_APP: std::cell::Cell<Option<TheAppThreadLocal>> = const { std::cell::Cell::new(None) };
}

struct TheAppGuard(Option<TheAppThreadLocal>);

impl Drop for TheAppGuard {
    fn drop(&mut self) {
        THE_APP.set(self.0.take());
    }
}
