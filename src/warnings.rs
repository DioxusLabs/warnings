use std::{
    any::{Any, TypeId},
    cell::RefCell,
    future::Future,
};

use pin_project::pin_project;
// #[lint]
// fn my_lint() {
//     // tracing::warn!("warning");
// }

// fn main() {
//     #[lints::allow(my_lint)]
//     syntax.await;
// }

#[test]
fn warning_guard() {
    fn lint() {}
    let warning = Warning::new(&lint);
    {
        let _guard = Allow::new(warning);
        assert!(!warning.enabled());
    }
    assert!(warning.enabled());
}

#[cfg(test)]
#[tokio::test]
async fn allow_future() {
    fn lint() {}
    let warning = Warning::new(&lint);
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

#[derive(Clone, Copy)]
pub struct Warning {
    #[cfg(debug_assertions)]
    type_id: fn() -> TypeId,
}

impl Warning {
    #[allow(unused)]
    pub const fn new<T: 'static>(lint: &T) -> Self {
        Self {
            #[cfg(debug_assertions)]
            type_id: std::any::TypeId::of::<T>,
        }
    }

    #[allow(unreachable_code)]
    pub fn enabled(&self) -> bool {
        #[cfg(debug_assertions)]
        return !ALLOW_STACK.with(|stack| stack.borrow().iter().any(|w| w.type_id == self.type_id));
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
    static ALLOW_STACK: RefCell<Vec<Warning>> = const { RefCell::new(Vec::new()) };
}

pub struct Allow {
    warning: Warning,
}

impl Allow {
    pub fn new(warning: Warning) -> Self {
        #[cfg(debug_assertions)]
        ALLOW_STACK.with(|stack| {
            stack.borrow_mut().push(warning);
        });
        Self { warning }
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

#[pin_project]
pub struct AllowFuture<F> {
    #[pin]
    future: F,
    #[allow(unused)]
    warning: Warning,
}

impl<F> AllowFuture<F> {
    pub fn new(future: F, warning: Warning) -> Self {
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

pub fn allow<O>(lint: &impl Any, item: impl FnOnce() -> O) -> O {
    #[cfg(debug_assertions)]
    let _gaurd = Allow::new(Warning::new(lint));
    item()
}

pub trait AllowFutureExt: Future {
    /// Allow a lint while a future is running
    fn allow(self, lint: &impl Any) -> AllowFuture<Self>
    where
        Self: Sized,
    {
        AllowFuture::new(self, Warning::new(lint))
    }
}

impl<F: Future> AllowFutureExt for F {}
