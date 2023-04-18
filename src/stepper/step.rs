use core::pin::Pin;

use core::convert::Infallible;
use core::future::Future;
use core::task::Poll::{Pending, Ready};
use core::task::{Context, Poll};

use embedded_hal::digital::{OutputPin, PinState};
use embedded_hal_async::delay::DelayUs;
use fugit::NanosDurationU32;
use futures::pin_mut;

use crate::traits::OutputPinAction;

use super::SignalError;

/// The "future" returned by [`Stepper::step`]
///
/// Please note that this type provides a custom API and does not implement
/// [`core::future::Future`]. This might change, when using futures for embedded
/// development becomes more practical.
///
/// [`Stepper::step`]: crate::Stepper::step
#[must_use]
#[pin_project::pin_project]
pub struct StepFuture<
    'r,
    Delay: DelayUs + 'r,
    OutputPin,
    const STEP_BUS_WIDTH: usize,
> {
    leading: [OutputPinAction<OutputPin>; STEP_BUS_WIDTH],
    duration: NanosDurationU32,
    trailing: [OutputPinAction<OutputPin>; STEP_BUS_WIDTH],
    delay: Delay,
    state: State<Pin<&'r mut dyn Future<Output = Result<(), Delay::Error>>>>,
}

impl<'r, Delay: DelayUs, OutputPin, const STEP_BUS_WIDTH: usize>
    StepFuture<'r, Delay, OutputPin, STEP_BUS_WIDTH>
{
    // /// Create new instance of `StepFuture`
    // ///
    // /// This constructor is public to provide maximum flexibility for
    // /// non-standard use cases. Most users can ignore this and just use
    // /// [`Stepper::step`] instead.
    // ///
    // /// [`Stepper::step`]: crate::Stepper::step
    // pub fn new_from_timer<Timer, const TIMER_HZ: u32>(
    //     leading: [OutputPinAction<OutputPin>; STEP_BUS_WIDTH],
    //     duration: NanosDurationU32,
    //     trailing: [OutputPinAction<OutputPin>; STEP_BUS_WIDTH],
    //     timer: Timer,
    // ) -> Self
    // where
    //     Timer: TimerTrait<TIMER_HZ>,
    // {
    //     Self {
    //         leading,
    //         duration,
    //         trailing,
    //         delay: AsyncDelay::from_timer(timer),
    //         state: State::Initial,
    //     }
    // }

    /// Create new instance of `StepFuture`
    ///
    /// This constructor is public to provide maximum flexibility for
    /// non-standard use cases. Most users can ignore this and just use
    /// [`Stepper::step`] instead.
    ///
    /// [`Stepper::step`]: crate::Stepper::step
    pub fn new(
        leading: [OutputPinAction<&'r mut OutputPin>; STEP_BUS_WIDTH],
        duration: NanosDurationU32,
        trailing: [OutputPinAction<&'r mut OutputPin>; STEP_BUS_WIDTH],
        delay: &'r mut Delay,
    ) -> StepFuture<'r, &'r mut Delay, &'r mut OutputPin, STEP_BUS_WIDTH> {
        StepFuture::<'r, &'r mut Delay, &'r mut OutputPin, STEP_BUS_WIDTH> {
            leading,
            duration,
            trailing,
            delay,
            state: State::Initial,
        }
    }

    /// Drop the future and release the resources that were moved into it
    pub fn release(
        self,
    ) -> (
        [OutputPinAction<OutputPin>; STEP_BUS_WIDTH],
        [OutputPinAction<OutputPin>; STEP_BUS_WIDTH],
        Delay,
    ) {
        (self.leading, self.trailing, self.delay)
    }
}

// impl<OutputPin, Delay, const STEP_BUS_WIDTH: usize> IntoFuture
//     for StepFuture<OutputPin, Delay, STEP_BUS_WIDTH>
// where
//     Delay: DelayUs,
// {
//     type Output = <Self as LegacyFuture>::FutureOutput;
//     type IntoFuture =
//         WrappedLegacyFuture<StepFuture<OutputPin, Delay, STEP_BUS_WIDTH>>;
//
//     fn into_future(self) -> Self::IntoFuture {
//         WrappedLegacyFuture::new(self)
//     }
// }

impl<'r, Delay: DelayUs, StepPin, const STEP_BUS_WIDTH: usize> Future
    for StepFuture<'r, Delay, StepPin, STEP_BUS_WIDTH>
where
    StepPin: OutputPin,
    Delay: DelayUs,
{
    type Output =
        Result<(), SignalError<Infallible, StepPin::Error, Delay::Error>>;

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
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &self.state {
            State::Initial => {
                // Start step action
                let mut pin_actions = &self.leading;

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

                let fut = self.delay.delay_us(self.duration.to_micros());
                pin_mut!(fut);
                self.state = State::PulseStarted(fut);
                Pending
            }
            State::PulseStarted(pinned_fut) => {
                match pinned_fut.poll(cx) {
                    Ready(Ok(())) => {
                        // End step action
                        let mut pin_actions = &self.trailing;

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
                        Ready(Ok(()))
                    }
                    Ready(Err(err)) => {
                        self.state = State::Finished;
                        Ready(Err(SignalError::Timer(err)))
                    }
                    Pending => Pending,
                    _ => unreachable!(),
                }
            }
            State::Finished => Ready(Ok(())),
        }
    }
}

enum State<Fut> {
    Initial,
    PulseStarted(Fut),
    Finished,
}

pub async fn toggle_pin<Pin: OutputPin, Delay: DelayUs>(
    pin: &mut Pin,
    duration: NanosDurationU32,
    delay: &mut Delay,
) -> Result<(), SignalError<Infallible, Pin::Error, Delay::Error>> {
    pin.set_state(PinState::High).map_err(SignalError::Pin)?;
    delay.delay_us(duration.to_micros()).await.map_err(SignalError::Timer)?;
    pin.set_state(PinState::Low).map_err(SignalError::Pin)?;
    Ok(())
}

pub async fn toggle_pins<Pin: OutputPin, Delay: DelayUs>(
    pins: &mut [Pin],
    duration: NanosDurationU32,
    delay: &mut Delay,
) -> Result<(), SignalError<Infallible, Pin::Error, Delay::Error>> {
    for pin in pins.iter_mut() {
        pin.set_state(PinState::High).map_err(SignalError::Pin)?;
    }
    delay.delay_us(duration.to_micros()).await.map_err(SignalError::Timer)?;
    for pin in pins.iter_mut() {
        pin.set_state(PinState::Low).map_err(SignalError::Pin)?;
    }
    Ok(())
}