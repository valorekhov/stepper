use crate::stepper::legacy_future::LegacyFuture;
use core::{convert::Infallible, task::Poll};
use fugit::TimerDurationU32 as TimerDuration;
use fugit_timer::Timer as TimerTrait;

use crate::traits::SetStepMode;

use super::SignalError;

enum State {
    Initial,
    ApplyingConfig,
    EnablingDriver,
    Finished,
}

/// The "future" returned by [`Stepper::set_step_mode`]
///
/// Please note that this type provides a custom API and does not implement
/// [`core::future::Future`]. This might change, when using futures for embedded
/// development becomes more practical.
///
/// [`Stepper::set_step_mode`]: crate::Stepper::set_step_mode
#[must_use]
pub struct SetStepModeFuture<Driver: SetStepMode, Timer, const TIMER_HZ: u32> {
    step_mode: Driver::StepMode,
    driver: Driver,
    timer: Timer,
    state: State,
}

impl<Driver, Timer, const TIMER_HZ: u32>
    SetStepModeFuture<Driver, Timer, TIMER_HZ>
where
    Driver: SetStepMode,
    Timer: TimerTrait<TIMER_HZ>,
{
    /// Create new instance of `SetStepModeFuture`
    ///
    /// This constructor is public to provide maximum flexibility for
    /// non-standard use cases. Most users can ignore this and just use
    /// [`Stepper::set_step_mode`] instead.
    ///
    /// [`Stepper::set_step_mode`]: crate::Stepper::set_step_mode
    pub fn new(
        step_mode: Driver::StepMode,
        driver: Driver,
        timer: Timer,
    ) -> Self {
        Self {
            step_mode,
            driver,
            timer,
            state: State::Initial,
        }
    }

    /// Drop the future and release the resources that were moved into it
    pub fn release(self) -> (Driver, Timer) {
        (self.driver, self.timer)
    }
}

impl<Driver, Timer, const TIMER_HZ: u32> LegacyFuture
    for SetStepModeFuture<Driver, Timer, TIMER_HZ>
where
    Driver: SetStepMode,
    Timer: TimerTrait<TIMER_HZ>,
{
    type DriverError = Driver::Error;
    type TimerError = Timer::Error;

    type FutureOutput = Result<
        (),
        SignalError<
            Infallible, // only applies to `SetDirection`, `Step`
            Self::DriverError,
            Self::TimerError,
        >,
    >;

    /// Poll the future
    ///
    /// The future must be polled for the operation to make progress. The
    /// operation won't start, until this method has been called once. Returns
    /// [`Poll::Pending`], if the operation is not finished yet, or
    /// [`Poll::Ready`], once it is.
    ///
    /// If this method returns [`Poll::Pending`], the user can opt to keep
    /// calling it at a high frequency (see [`Self::wait`]) until the operation
    /// completes, or set up an interrupt that fires once the timer finishes
    /// counting down, and call this method again once it does.
    fn poll(&mut self) -> Poll<Self::FutureOutput> {
        match self.state {
            State::Initial => {
                self.driver
                    .apply_mode_config(self.step_mode)
                    .map_err(|err| SignalError::Pin(err))?;

                let ticks: TimerDuration<TIMER_HZ> =
                    Driver::SETUP_TIME.convert();

                self.timer
                    .start(ticks)
                    .map_err(|err| SignalError::Timer(err))?;

                self.state = State::ApplyingConfig;
                Poll::Pending
            }
            State::ApplyingConfig => match self.timer.wait() {
                Ok(()) => {
                    self.driver
                        .enable_driver()
                        .map_err(|err| SignalError::Pin(err))?;

                    let ticks: TimerDuration<TIMER_HZ> =
                        Driver::HOLD_TIME.convert();

                    self.timer
                        .start(ticks)
                        .map_err(|err| SignalError::Timer(err))?;

                    self.state = State::EnablingDriver;
                    Poll::Ready(Ok(()))
                }
                Err(nb::Error::Other(err)) => {
                    self.state = State::Finished;
                    Poll::Ready(Err(SignalError::Timer(err)))
                }
                Err(nb::Error::WouldBlock) => Poll::Pending,
            },
            State::EnablingDriver => match self.timer.wait() {
                Ok(()) => {
                    self.state = State::Finished;
                    Poll::Ready(Ok(()))
                }
                Err(nb::Error::Other(err)) => {
                    self.state = State::Finished;
                    Poll::Ready(Err(SignalError::Timer(err)))
                }
                Err(nb::Error::WouldBlock) => Poll::Pending,
            },
            // block!(timer.try_wait()).map_err(|err| Error::Timer(err))?;
            State::Finished => Poll::Ready(Ok(())),
        }
    }
}

// impl<Driver, Timer, const TIMER_HZ: u32> Future
//     for SetStepModeFuture<Driver, Timer, TIMER_HZ>
// where
//     Driver: SetStepMode,
//     Timer: TimerTrait<TIMER_HZ>,
// {
//     type Output = Result<
//         (),
//         SignalError<
//             Infallible, // only applies to `SetDirection`, `Step`
//             Driver::Error,
//             Timer::Error,
//         >,
//     >;
//
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         let this = self.project().as_mut();
//         this.poll()
//     }
// }
