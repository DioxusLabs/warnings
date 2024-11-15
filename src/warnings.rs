use std::{fmt::Debug, future::Future};

use pin_project::pin_project;

pub trait Warning: 'static {
    const ID: WarningId;

    fn enabled() -> bool {
        Self::ID.enabled()
    }

    fn if_enabled(item: impl FnOnce()) {
        Self::ID.if_enabled(item)
    }

    fn allow<O>(item: impl FnOnce() -> O) -> O {
        allow::<Self, _>(item)
    }

    fn allow_async<F: Future>(future: F) -> AllowFuture<F> {
        AllowFuture::new(future, Self::ID)
    }
}

#[derive(Clone, Copy)]
pub struct WarningId {
    #[cfg(debug_assertions)]
    type_id: fn() -> std::any::TypeId,
    #[cfg(debug_assertions)]
    name: fn() -> &'static str,
}

impl Debug for WarningId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("WarningId");
        #[cfg(debug_assertions)]
        {
            dbg.field("type_id", &self.type_id)
                .field("name", &(self.name)());
        }
        dbg.finish()?;
        Ok(())
    }
}

impl WarningId {
    /// Get the ID of a warning
    #[allow(unused)]
    pub const fn of<W: Warning + ?Sized>() -> Self {
        Self {
            #[cfg(debug_assertions)]
            type_id: std::any::TypeId::of::<W>,
            #[cfg(debug_assertions)]
            name: std::any::type_name::<W>,
        }
    }

    #[allow(unreachable_code)]
    pub fn enabled(&self) -> bool {
        #[cfg(debug_assertions)]
        return !ALLOW_STACK.with(|stack| {
            let stack = stack.borrow();
            tracing::trace!("Checking if warning {self:?} is enabled, stack: {stack:?}");
            stack.iter().any(|w| (w.type_id)() == (self.type_id)())
        });
        false
    }

    pub fn if_enabled(&self, f: impl FnOnce()) {
        if self.enabled() {
            f();
        }
    }
}

#[cfg(debug_assertions)]
thread_local! {
    static ALLOW_STACK: std::cell::RefCell<Vec<WarningId>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub struct Allow {
    _private: (),
}

impl Allow {
    #[allow(unused)]
    pub fn new(warning: WarningId) -> Self {
        #[cfg(debug_assertions)]
        ALLOW_STACK.with(|stack| {
            stack.borrow_mut().push(warning);
        });
        Self { _private: () }
    }
}

impl Drop for Allow {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        ALLOW_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

#[test]
fn warning_guard() {
    struct Lint {}

    impl Warning for Lint {
        const ID: WarningId = WarningId::of::<Lint>();
    }

    let warning = WarningId::of::<Lint>();
    {
        let _guard = Allow::new(warning);
        assert!(!warning.enabled());
    }
    assert!(warning.enabled());
}

#[pin_project]
pub struct AllowFuture<F> {
    #[pin]
    future: F,
    #[allow(unused)]
    warning: WarningId,
}

impl<F> AllowFuture<F> {
    pub fn new(future: F, warning: WarningId) -> Self {
        Self { future, warning }
    }
}

impl<F: Future> Future for AllowFuture<F> {
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        #[cfg(debug_assertions)]
        let _guard = Allow::new(self.warning);
        let this = self.project();
        this.future.poll(cx)
    }
}

pub fn allow<W: Warning + ?Sized, O>(item: impl FnOnce() -> O) -> O {
    #[cfg(debug_assertions)]
    let _gaurd = Allow::new(W::ID);
    item()
}

pub trait AllowFutureExt: Future {
    /// Allow a lint while a future is running
    fn allow<W: Warning + ?Sized>(self) -> AllowFuture<Self>
    where
        Self: Sized,
    {
        AllowFuture::new(self, W::ID)
    }
}

impl<F: Future> AllowFutureExt for F {}

#[cfg(test)]
#[tokio::test]
async fn allow_future() {
    struct Lint {}

    impl Warning for Lint {
        const ID: WarningId = WarningId::of::<Lint>();
    }

    let warning = WarningId::of::<Lint>();
    let assert_future_enabled = async {
        let mut poll_count = 0;
        std::future::poll_fn(|cx| {
            assert!(!warning.enabled());
            match poll_count {
                ..=5 => {
                    poll_count += 1;
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                }
                6.. => std::task::Poll::Ready(()),
            }
        })
        .await;
    };
    let future = AllowFuture::new(assert_future_enabled, warning);
    future.await;
    assert!(warning.enabled());
}
