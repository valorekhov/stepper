//! [`embedded_hal_async::delay::DelayUs`] implementation

use crate::util::ref_mut::RefMut;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use embedded_hal_async::delay::DelayUs;
use fugit::TimerDurationU32;

/// Wraps an instance of `fugit_timer::Timer` to provide `embedded_hal_async::delay::DelayUs` functionality
#[pin_project::pin_project]
pub struct AsyncDelay<Timer, const TIMER_HZ: u32> {
    #[pin]
    _timer: Timer,
}

impl<Timer: fugit_timer::Timer<TIMER_HZ> + Unpin, const TIMER_HZ: u32> Future
    for AsyncDelay<Timer, TIMER_HZ>
where
    Timer: fugit_timer::Timer<TIMER_HZ>,
{
    type Output = Result<(), ()>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        let mut timer: Pin<&mut Timer> = self.project()._timer;

        match timer.as_mut().wait() {
            Ok(_) => Poll::Ready(Ok(())),
            Err(nb::Error::Other(_)) => Poll::Ready(Err(())),
            Err(nb::Error::WouldBlock) => Poll::Pending,
        }
    }
}

impl<Timer: fugit_timer::Timer<TIMER_HZ>, const TIMER_HZ: u32>
    AsyncDelay<Timer, TIMER_HZ>
{
    /// Assumes a timer instance while creating a new instance of the `DelayFromTimer` struct
    pub fn from_timer(timer: Timer) -> AsyncDelay<Timer, TIMER_HZ> {
        AsyncDelay::<Timer, TIMER_HZ> { _timer: timer }
    }

    /// Creates a new instance of the timer and starts countdown for the specified duration value
    pub fn start(
        timer: &mut Timer,
        duration: TimerDurationU32<TIMER_HZ>,
    ) -> AsyncDelay<RefMut<Timer>, TIMER_HZ> {
        timer.start(duration).expect("timer started");
        AsyncDelay::<RefMut<Timer>, TIMER_HZ> {
            _timer: RefMut(timer),
        }
    }
}

impl<Timer, const TIMER_HZ: u32> DelayUs for AsyncDelay<Timer, TIMER_HZ>
where
    Timer: fugit_timer::Timer<TIMER_HZ>,
{
    type Error = ();
    type DelayUsFuture<'a> = AsyncDelay<RefMut<'a, Timer>, TIMER_HZ> where Self: 'a;

    fn delay_us(&mut self, us: u32) -> Self::DelayUsFuture<'_> {
        AsyncDelay::start(
            &mut self._timer,
            TimerDurationU32::<TIMER_HZ>::micros(us),
        )
    }

    type DelayMsFuture<'a> = AsyncDelay<RefMut<'a, Timer>, TIMER_HZ> where Self: 'a;

    fn delay_ms(&mut self, ms: u32) -> Self::DelayMsFuture<'_> {
        AsyncDelay::start(
            &mut self._timer,
            TimerDurationU32::<TIMER_HZ>::millis(ms),
        )
    }
}
