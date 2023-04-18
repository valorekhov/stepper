use crate::stepper::legacy_future::LegacyFuture;
use core::{convert::Infallible, task::Poll};
use core::future::Future;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll::Ready;
use Poll::Pending;
use embedded_hal_async::delay::DelayUs;
use futures::pin_mut;

use crate::traits::SetStepMode;

use super::SignalError;

enum State<Fut> {
    Initial,
    ApplyingConfig(Fut),
    EnablingDriver(Fut),
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
pub struct SetStepModeFuture<'r, Driver: SetStepMode, Delay: DelayUs + 'r> {
    step_mode: Driver::StepMode,
    driver: Driver,
    delay: Delay,
    state: State<Pin<&'r mut dyn Future<Output = Result<(), Delay::Error>>>>,
}

impl<'r, Driver, Delay>
    SetStepModeFuture<'r, Driver, Delay>
where
    Driver: SetStepMode,
    Delay: DelayUs,
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
        delay: Delay,
    ) -> Self {
        Self {
            step_mode,
            driver,
            delay,
            state: State::Initial,
        }
    }

    /// Drop the future and release the resources that were moved into it
    pub fn release(self) -> (Driver, Delay) {
        (self.driver, self.delay)
    }
}

impl<'r, Driver, Delay> LegacyFuture
    for SetStepModeFuture<'r, Driver, Delay>
where
    Driver: SetStepMode,
    Delay: DelayUs,
{
    type DriverError = Driver::Error;
    type TimerError = Delay::Error;

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
        todo!("implement `SetStepModeFuture::poll`")
    }
}

impl<'r, Driver, Delay> Future
    for SetStepModeFuture<'r, Driver, Delay>
where
    Driver: SetStepMode,
    Delay: DelayUs,
{
    type Output = Result<
        (),
        SignalError<
            Infallible, // only applies to `SetDirection`, `Step`
            Driver::Error,
            Delay::Error,
        >,
    >;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &self.state {
            State::Initial => {
                self.driver
                    .apply_mode_config(self.step_mode)
                    .map_err(|err| SignalError::Pin(err))?;

                let fut = self.delay
                    .delay_us(Driver::SETUP_TIME.to_micros());
                pin_mut!(fut);

                self.state = State::ApplyingConfig(fut);
                Pending
            }
            State::ApplyingConfig(pinned_fut) => match pinned_fut.poll(cx) {
                Ready(Ok(())) => {
                    self.driver
                        .enable_driver()
                        .map_err(|err| SignalError::Pin(err))?;

                    drop(pinned_fut);

                    let fut = self.delay
                        .delay_us(Driver::HOLD_TIME.to_micros());
                    pin_mut!(fut);

                    self.state = State::EnablingDriver(fut);
                    Ready(Ok(()))
                }
                Ready(Err(err)) => {
                    self.state = State::Finished;
                    Ready(Err(SignalError::Timer(err)))
                }
                Pending => Pending,
                _ => unreachable!(),
            },
            State::EnablingDriver(pinned_fut) => match pinned_fut.poll(cx) {
                Ready(Ok(())) => {
                    drop(pinned_fut);
                    self.state = State::Finished;
                    Ready(Ok(()))
                }
                Ready(Err(err)) => {
                    self.state = State::Finished;
                    Ready(Err(SignalError::Timer(err)))
                }
                Pending => Pending,
                _ => unreachable!(),
            },
            State::Finished => Ready(Ok(())),
        }
    }
}
