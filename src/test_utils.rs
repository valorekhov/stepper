#[cfg(feature = "async")]
use alloc::boxed::Box;
#[cfg(feature = "async")]
use embedded_hal_async::delay::DelayUs;

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;

use tokio::time::{sleep, Sleep};

pub(crate) struct SleepWrapper {
    pub sleep: Pin<Box<Sleep>>,
}

impl Future for SleepWrapper {
    type Output = Result<(), std::convert::Infallible>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        match self.sleep.as_mut().poll(cx) {
            Poll::Ready(()) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(feature = "async")]
pub(crate) struct DelayUsPosix {}

#[cfg(feature = "async")]
impl DelayUs for DelayUsPosix {
    type Error = std::convert::Infallible;
    type DelayUsFuture<'a> = SleepWrapper where Self: 'a;

    fn delay_us(&mut self, us: u32) -> SleepWrapper {
        SleepWrapper {
            sleep: Box::pin(sleep(Duration::from_micros(us as u64))),
        }
    }

    type DelayMsFuture<'a> = SleepWrapper where Self: 'a;

    fn delay_ms(&mut self, ms: u32) -> SleepWrapper {
        SleepWrapper {
            sleep: Box::pin(sleep(Duration::from_millis(ms as u64))),
        }
    }
}
