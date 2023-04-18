use core::future::Future;
use core::pin::Pin;
use core::task::Poll;

use embedded_hal::digital::OutputPin;
use embedded_hal::digital::ErrorType;
use embedded_hal_async::delay::DelayUs;
use futures::pin_mut;
use crate::legacy_future::LegacyFuture;
use crate::traits::OutputPinAction;
use crate::{traits::SetDirection, Direction};

use super::SignalError;

/// The [`core::future::Future`] returned by [`Stepper::set_direction`]
///
/// [`Stepper::set_direction`]: crate::Stepper::set_direction
#[must_use]
pub struct SetDirectionFuture<'r, Driver, Delay: DelayUs + 'r> {
    direction: Direction,
    driver: Driver,
    delay: Delay,
    state: State<Pin<&'r mut dyn Future<Output= Result<(), Delay::Error>>>>
}

impl<'r, Driver, Delay>
    SetDirectionFuture<'r, Driver, Delay>
where
    Driver: SetDirection,
    Delay: DelayUs,
{
    /// Create new instance of `SetDirectionFuture`
    ///
    /// This constructor is public to provide maximum flexibility for
    /// non-standard use cases. Most users can ignore this and just use
    /// [`Stepper::set_direction`] instead.
    ///
    /// [`Stepper::set_direction`]: crate::Stepper::set_direction
    pub fn new(direction: Direction, driver: Driver, delay: Delay) -> Self {
        Self {
            direction,
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
    for SetDirectionFuture<'r, Driver, Delay>
where
    Driver: SetDirection,
    Delay: DelayUs,
{
    type DriverError = <Driver::Dir as ErrorType>::Error;
    type TimerError = Delay::Error;

    type FutureOutput = Result<
        (),
        SignalError<
            Driver::Error,
            <Driver::Dir as ErrorType>::Error,
            Delay::Error,
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
    /// completes, or set up an interrupt that fires once the delay finishes
    /// counting down, and call this method again once it does.
    fn poll(&mut self) -> Poll<Self::FutureOutput> {
        todo!()
    }
}

impl<'r, Driver, Delay> Future
for SetDirectionFuture<'r, Driver, Delay>
    where
        Driver: SetDirection,
        Delay: DelayUs,
{
    type Output = Result<
        (),
        SignalError<
            Driver::Error,
            <Driver::Dir as ErrorType>::Error,
            Delay::Error,
        >,
    >;

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        match &self.state {
            State::Initial => {
                let action = self
                    .driver
                    .dir(self.direction)
                    .map_err(|err| SignalError::PinUnavailable(err))?;

                match action {
                    OutputPinAction::Set(pin, state) => pin
                        .set_state(state)
                        .map_err(|err| SignalError::Pin(err))?,
                    OutputPinAction::None => {}
                }

                let fut = self.delay
                    .delay_us(Driver::SETUP_TIME.to_micros());
                pin_mut!(fut);

                self.state = State::DirectionSet(fut);
                Poll::Pending
            }
            State::DirectionSet(pinned_fut) => match pinned_fut.poll(cx) {
                Poll::Ready(Ok(())) => {
                    self.state = State::Finished;
                    Poll::Ready(Ok(()))
                }
                Poll::Ready(Err(err)) => {
                    self.state = State::Finished;
                    Poll::Ready(Err(SignalError::Timer(err)))
                }
                Poll::Pending => Poll::Pending,
                _ => unreachable!()
            },
            State::Finished => Poll::Ready(Ok(())),
        }
    }
}

enum State<Fut> {
    Initial,
    DirectionSet(Fut),
    Finished,
}

// pub async fn set_direction_async<'r, Driver, Delay>(
//     direction: Direction,
//     driver: &mut Driver,
//     delay: &mut Delay,
// ) -> Result<
//     (),
//     SignalError<
//         Driver::Error,
//         <Driver::Dir as ErrorType>::Error,
//         Delay::Error,
//     >,
// >
// where
//     Driver: SetDirection,
//     Delay: DelayUs,
// {
//     let action = driver
//         .dir(direction)
//         .map_err(|err| SignalError::PinUnavailable(err))?;
//
//     match action {
//         OutputPinAction::Set(pin, state) => pin
//             .set_state(state)
//             .map_err(|err| SignalError::Pin(err))?,
//         OutputPinAction::None => {}
//     }
//
//     delay.delay_us(Driver::SETUP_TIME.to_micros()).await?;
//
//     Ok(())
// }