use core::task::Poll;

use crate::stepper::legacy_future::LegacyFuture;
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
pub struct StepFuture<
    Driver,
    Timer,
    const TIMER_HZ: u32,
    const STEP_BUS_WIDTH: usize,
> {
    driver: Driver,
    timer: Timer,
    state: State,
}

impl<Driver, Timer, const TIMER_HZ: u32, const STEP_BUS_WIDTH: usize>
    StepFuture<Driver, Timer, TIMER_HZ, STEP_BUS_WIDTH>
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

    /// Drop the future and release the resources that were moved into it
    pub fn release(self) -> (Driver, Timer) {
        (self.driver, self.timer)
    }
}

impl<Driver, Timer, const TIMER_HZ: u32, const STEP_BUS_WIDTH: usize>
    LegacyFuture for StepFuture<Driver, Timer, TIMER_HZ, STEP_BUS_WIDTH>
where
    Driver: Step<STEP_BUS_WIDTH>,
    Timer: TimerTrait<TIMER_HZ>,
{
    type DriverError = Driver::Error;
    type TimerError = Timer::Error;

    type FutureOutput = Result<
        (),
        SignalError<
            Driver::Error,
            <Driver::StepPin as ErrorType>::Error,
            Timer::Error,
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
                // Start step action
                let mut pin_actions = self
                    .driver
                    .step_leading()
                    .map_err(SignalError::PinUnavailable)?;

                for pin_action in pin_actions.iter_mut() {
                    let action = match pin_action {
                        // OutputPinAction::Pulse(pin) => Some((pin, SetPin::High)),
                        // OutputPinAction::Toggle(pin, state, _) => Some((pin, *state)),
                        OutputPinAction::Set(pin, state) => Some((pin, *state)),
                        OutputPinAction::None => None,
                    };
                    if let Some((pin, state)) = action {
                        pin.set_state(state).map_err(SignalError::Pin)?
                    }
                }

                let ticks: TimerDuration<TIMER_HZ> =
                    Driver::PULSE_LENGTH.convert();

                self.timer.start(ticks).map_err(SignalError::Timer)?;

                self.state = State::PulseStarted;
                Poll::Pending
            }
            State::PulseStarted => {
                match self.timer.wait() {
                    Ok(()) => {
                        // End step action
                        let mut pin_actions = self
                            .driver
                            .step_trailing()
                            .map_err(SignalError::PinUnavailable)?;

                        for pin_action in pin_actions.iter_mut() {
                            let action = match pin_action {
                                // OutputPinAction::Pulse(pin) => Some((pin, SetPin::Low)),
                                // OutputPinAction::Toggle(pin, _, state) => Some((pin, *state)),
                                OutputPinAction::Set(pin, state) => {
                                    Some((pin, *state))
                                }
                                OutputPinAction::None => None,
                            };
                            if let Some((pin, state)) = action {
                                pin.set_state(state)
                                    .map_err(SignalError::Pin)?
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
}

enum State {
    Initial,
    PulseStarted,
    Finished,
}
