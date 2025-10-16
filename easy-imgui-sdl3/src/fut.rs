use easy_imgui::future::{Action, FutureBackCallerImpl, IdleRunner};

use super::*;

pub struct MyIdleRunner<A: Application>(pub Proxy<A>);

impl<A: Application> IdleRunner for MyIdleRunner<A> {
    fn idle_run(&self, f: Action) -> Result<(), Action> {
        self.0
            .send_priv(AppEvent::<A>::RunIdleSimple(f))
            .map_err(|SendError(e)| match e {
                AppEvent::RunIdleSimple(a) => a,
                _ => unreachable!(),
            })
    }
}

type MyFutureBackCallerImpl<'a, A> = FutureBackCallerImpl<(&'a mut A, Args<'a, A>)>;

// Fake static lifetimes, should be raw pointers, but the Args<> doesn't play nice with those.
pub struct FutureBackCaller<A: Application>(MyFutureBackCallerImpl<'static, A>);

impl<A: Application> Default for FutureBackCaller<A> {
    fn default() -> Self {
        Self::new()
    }
}

struct IdType<A>(A);

impl<A: Application> FutureBackCaller<A> {
    pub fn new() -> Self {
        Self(MyFutureBackCallerImpl::default())
    }

    pub fn prepare(app: &mut A, args: Args<'_, A>, f: impl FnOnce()) {
        MyFutureBackCallerImpl::<'_, A>::prepare::<IdType<A>>((app, args), f);
    }

    pub fn run<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut A, Args<'_, A>) -> R,
    {
        unsafe {
            self.0.run::<IdType<A>, _, _>(move |t| {
                let (app, args) = t;
                f(app, args.reborrow())
            })
        }
    }
}
