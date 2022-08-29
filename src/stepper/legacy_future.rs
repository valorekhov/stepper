use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

pub trait LegacyFuture {
    type DriverError;
    type TimerError;
    type FutureOutput;

    fn poll(&mut self) -> Poll<Self::FutureOutput>;

    /// Wait until the operation completes
    ///
    /// This method will call [`Self::poll`] in a busy loop until the operation
    /// has finished.
    fn wait(&mut self) -> Self::FutureOutput {
        loop {
            if let Poll::Ready(result) = self.poll() {
                return result;
            }
        }
    }
}

#[pin_project::pin_project]
pub struct WrappedLegacyFuture<Driver, Timer, Fut> {
    future: Fut,
    _marker_timer: PhantomData<Timer>,
    _marker_driver: PhantomData<Driver>,
}

impl<Driver, Timer, Fut: LegacyFuture> WrappedLegacyFuture<Driver, Timer, Fut> {
    pub fn new(future: Fut) -> Self {
        Self {
            future,
            _marker_timer: PhantomData::default(),
            _marker_driver: PhantomData::default(),
        }
    }

    pub fn wrapped_future(self) -> Fut {
        self.future
    }
}

impl<Driver, Timer, Fut: LegacyFuture> Future
    for WrappedLegacyFuture<Driver, Timer, Fut>
{
    type Output = Fut::FutureOutput;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self;
        let res = this.future.poll();
        if res.is_pending() {
            cx.waker().wake_by_ref();
        }
        res
    }
}
