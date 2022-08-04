use core::task::Poll;

use embedded_hal::digital::blocking::OutputPin;
use embedded_hal::digital::ErrorType;
use fugit::TimerDurationU32 as TimerDuration;
use fugit_timer::Timer as TimerTrait;

use crate::traits::{OutputPinAction, Step};

use super::SignalError;

/// The "future" returned by [`Stepper::step`]
///
/// Please note that this type provides a custom API and does not implement
/// [`core::future::Future`]. This might change, when using futures for embedded
/// development becomes more practical.
///
/// [`Stepper::step`]: crate::Stepper::step
#[must_use]
pub struct StepFuture<Driver, Timer, const TIMER_HZ: u32, const STEP_BUS_WIDTH: usize> {
    driver: Driver,
    timer: Timer,
    state: State,
}

impl<Driver, Timer, const TIMER_HZ: u32, const STEP_BUS_WIDTH: usize> StepFuture<Driver, Timer, TIMER_HZ, STEP_BUS_WIDTH>
where
    Driver: Step<STEP_BUS_WIDTH>,
    Timer: TimerTrait<TIMER_HZ>,
{
    /// Create new instance of `StepFuture`
    ///
    /// This constructor is public to provide maximum flexibility for
    /// non-standard use cases. Most users can ignore this and just use
    /// [`Stepper::step`] instead.
    ///
    /// [`Stepper::step`]: crate::Stepper::step
    pub fn new(driver: Driver, timer: Timer) -> Self {
        Self {
            driver,
            timer,
            state: State::Initial,
        }
    }

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
    pub fn poll(
        &mut self,
    ) -> Poll<
        Result<
            (),
            SignalError<
                Driver::Error,
                <Driver::StepPin as ErrorType>::Error,
                Timer::Error,
            >,
        >,
    > {
        match self.state {
            State::Initial => {
                // Start step action
                let mut pin_actions = self.driver
                    .step_leading()
                    .map_err(|err| SignalError::PinUnavailable(err))?;

                for pin_action in pin_actions.iter_mut(){
                    let action = match pin_action {
                        // OutputPinAction::Pulse(pin) => Some((pin, SetPin::High)),
                        // OutputPinAction::Toggle(pin, state, _) => Some((pin, *state)),
                        OutputPinAction::Set(pin, state) => Some((pin, *state)),
                        OutputPinAction::None => None
                    };
                    match action {
                        Some((pin, state)) =>
                            pin.set_state(state).map_err(|err| SignalError::Pin(err))?,
                        _ => {}
                    }
                }

                let ticks: TimerDuration<TIMER_HZ> =
                    Driver::PULSE_LENGTH.convert();

                self.timer
                    .start(ticks)
                    .map_err(|err| SignalError::Timer(err))?;

                self.state = State::PulseStarted;
                Poll::Pending
            }
            State::PulseStarted => {
                match self.timer.wait() {
                    Ok(()) => {
                        // End step action
                        let mut pin_actions = self.driver
                            .step_trailing()
                            .map_err(|err| SignalError::PinUnavailable(err))?;

                        for pin_action in pin_actions.iter_mut(){
                            let action = match pin_action {
                                // OutputPinAction::Pulse(pin) => Some((pin, SetPin::Low)),
                                // OutputPinAction::Toggle(pin, _, state) => Some((pin, *state)),
                                OutputPinAction::Set(pin, state) => Some((pin, *state)),
                                OutputPinAction::None => None
                            };
                            match action {
                                Some((pin, state)) =>
                                    pin.set_state(state).map_err(|err| SignalError::Pin(err))?,
                                _ => {}
                            }
                        }

                        self.state = State::Finished;
                        Poll::Ready(Ok(()))
                    }
                    Err(nb::Error::Other(err)) => {
                        self.state = State::Finished;
                        Poll::Ready(Err(SignalError::Timer(err)))
                    }
                    Err(nb::Error::WouldBlock) => Poll::Pending,
                }
            }
            State::Finished => Poll::Ready(Ok(())),
        }
    }

    /// Wait until the operation completes
    ///
    /// This method will call [`Self::poll`] in a busy loop until the operation
    /// has finished.
    pub fn wait(
        &mut self,
    ) -> Result<
        (),
        SignalError<
            Driver::Error,
            <Driver::StepPin as ErrorType>::Error,
            Timer::Error,
        >,
    > {
        loop {
            if let Poll::Ready(result) = self.poll() {
                return result;
            }
        }
    }

    /// Drop the future and release the resources that were moved into it
    pub fn release(self) -> (Driver, Timer) {
        (self.driver, self.timer)
    }
}

enum State {
    Initial,
    PulseStarted,
    Finished,
}

// #[cfg(feature = "async")]
// use embedded_hal_async::delay::DelayUs;
//
// /// Rotates the motor one step in the given direction
// ///
// /// Steps the motor one step in the direction that was previously set,
// /// according to the current entry in the pin firing configuration. To achieve a specific
// /// speed, the user must call this method at an appropriate frequency.
// ///
// /// You might need to call [`Stepper::enable_step_control`] to make this
// /// method available.
// #[cfg(feature = "async")]
// pub async fn step_async<Driver, Delay, const TIMER_HZ: u32, const BUS_WIDTH: usize>(
//     driver: &mut Driver,
//     delay: &mut Delay,
// ) -> Result<
//     (),
//     SignalError<
//         <Driver::StepPin as Step<BUS_WIDTH>>::Error,
//         <Driver::StepPin as ErrorType>::Error,
//         Delay::Error,
//     >,
// >
//     where
//         Driver: Step<BUS_WIDTH> + OutputPin,
//         Delay: DelayUs,
//         <Driver as Step<BUS_WIDTH>>::StepPin: Step<BUS_WIDTH>,
//         SignalError<
//             <Driver::StepPin as Step<BUS_WIDTH>>::Error,
//             <Driver::StepPin as ErrorType>::Error,
//             Delay::Error,
//         >: From< SignalError<
//             <Driver as Step<BUS_WIDTH>>::Error,
//             <Driver::StepPin as ErrorType>::Error,
//             Delay::Error,
//         >>
//
// {
//     driver
//         .step()
//         .map_err(|err| SignalError::PinUnavailable(err))?
//         .set_high()
//         .map_err(|err| SignalError::Pin(err))?;
//
//     delay.delay_us(Driver::PULSE_LENGTH.to_micros())
//         .await
//         .map_err(|err| SignalError::Timer(err))?;
//
//     driver
//         .step()
//         .map_err(|err| SignalError::PinUnavailable(err))?
//         .set_low()
//         .map_err(|err| SignalError::Pin(err))?;
//
//     Ok(())
// }
