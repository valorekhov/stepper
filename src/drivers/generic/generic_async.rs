use crate::drivers::generic::{Generic, GenericStepError};
use crate::traits::{OutputStepFutureItem, Step as StepAsync};
use crate::SignalError;
use core::convert::Infallible;
use core::fmt::Debug;
use core::future::Future;
use embedded_hal::digital::blocking::OutputPin;
use embedded_hal_async::delay::DelayUs;

/// Experimental Async implementations for the Generic driver
///
/// Users are not expected to use this API directly, except to create an
/// instance using [`Generic::new`]. Please check out
/// [`Stepper`](crate::Stepper) instead.

impl<Pins, OutputPinError, const NUM_STEPS: usize, Delay> OutputStepFutureItem
    for Generic<Pins, NUM_STEPS, Delay>
where
    Pins: OutputPin<Error = OutputPinError>,
    OutputPinError: Debug,
{
    type OutputStepFutureResult = ();
    type OutputStepFutureError = GenericStepError;
}

impl<
        LinePin,
        Delay,
        OutputPinError,
        const STEP_BUS_WIDTH: usize,
        const NUM_STEPS: usize,
    > StepAsync for Generic<[LinePin; STEP_BUS_WIDTH], NUM_STEPS, Delay>
where
    LinePin: OutputPin<Error = OutputPinError>,
    OutputPinError: Debug,
    Delay: DelayUs,
{
    type Delay = Delay;
    type OutputStepFutureResult = ();
    type OutputStepFutureError =
        SignalError<Infallible, OutputPinError, Delay::Error>;

    type OutputStepFuture<'r>
    = impl Future<Output = Result<Self::OutputStepFutureResult, Self::OutputStepFutureError>> where Self: 'r;

    fn step<'r>(
        self: &'r mut Self,
        delay: &'r mut Delay,
    ) -> Self::OutputStepFuture<'r> {
        // unable to express the same as an inline async{} block due to https://github.com/rust-lang/rust/issues/65442
        // moving into a call of a an async function instead
        self.step_async_int(delay)
    }
}

impl<
        LinePin,
        OutputPinError,
        Delay,
        const STEP_BUS_WIDTH: usize,
        const NUM_STEPS: usize,
    > Generic<[LinePin; STEP_BUS_WIDTH], NUM_STEPS, Delay>
where
    LinePin: OutputPin<Error = OutputPinError>,
    OutputPinError: Debug,
    Delay: DelayUs,
{
    async fn step_async_int<'d>(
        &mut self,
        delay: &'d mut Delay,
    ) -> Result<(), SignalError<Infallible, OutputPinError, Delay::Error>> {
        let mut current_step = self.step.unwrap_or_else(|| 0) as usize;

        let firing_sequence =
            self.steps.get(current_step).expect("Within index");

        for i in 0..STEP_BUS_WIDTH {
            let pin_idx = STEP_BUS_WIDTH - 1 - i;
            let pin = &mut self.pins[pin_idx];
            {
                (if (firing_sequence >> i) & 0x01 == 0x01 {
                    pin.set_high()
                } else {
                    pin.set_low()
                })
                .expect("it to work");
            };
        }

        current_step += 1;
        if current_step >= NUM_STEPS {
            current_step = 0
        }
        self.step = Some(current_step as u8);

        delay.delay_us(100).await.expect("sleep finished");
        Ok(())
    }
}

// impl<LinePin, Src  OutputPinError, const STEP_BUS_WIDTH: usize, const NUM_STEPS: usize>
// EnableStepControlAsync<(), Delay>
// for Generic<LinePin, Src STEP_BUS_WIDTH, NUM_STEPS>
//     where
//         LinePin: OutputPin<Error = OutputPinError>,
//         Delay: DelayUs,
//         OutputPinError: Debug
// {
//     type WithAsyncStepControl = Generic<LinePin, STEP_BUS_WIDTH, NUM_STEPS>;
//
//     fn enable_step_control_async(self, _: (), delay: Delay) -> Self::WithAsyncStepControl {
//         Generic {
//             pins: self.pins,
//             steps: self.steps,
//             step: self.step,
//             direction: self.direction,
//             delay
//         }
//     }
// }
//

// impl<
//         LinePin,
//         T
//         OutputPinError,
//         const STEP_BUS_WIDTH: usize,
//         const NUM_STEPS: usize,
//     > SetDelayAsync for Generic<LinePin, T STEP_BUS_WIDTH, NUM_STEPS>
// where
//     Self: DelayAsyncEnabled<TDelay>,
//     LinePin: OutputPin<Error = OutputPinError>,
//     OutputPinError: Debug,
//     TDelay: DelayUs,
// {
//     type AsyncEnabled<Delay: DelayUs> = Self;
//
//     fn set_delay<Delay: DelayUs>(
//         self,
//         delay: T
//     ) -> Self::AsyncEnabled<Delay> {
//         Self {
//             pins: self.pins,
//             steps: self.steps,
//             step: self.step,
//             direction: self.direction,
//
//         }
//     }
// }

#[cfg(test)]
mod test {
    use crate::drivers::generic::Generic;
    use crate::stepper::Stepper;
    use crate::test_utils::MockPin;
    use crate::test_utils::SysClockTimer;
    use crate::util::delay::AsyncDelay;
    use crate::Direction;

    #[tokio::test]
    pub async fn test_stepping_async() {
        let mut pin1 = MockPin::new();
        let mut pin2 = MockPin::new();

        pin1.expect_set_low().return_once(|| Ok(()));
        pin2.expect_set_high().return_once(|| Ok(()));

        let mut dir_control_timer = SysClockTimer::<1000_u32>::new();
        let mut delay =
            AsyncDelay::from_timer(SysClockTimer::<1000_u32>::new());

        let pins = [&mut pin1, &mut pin2];
        let dir_control_stepper =
            Stepper::from_driver(Generic::new([0b01_u8, 0b10_u8]))
                .enable_direction_control(
                    (),
                    Direction::Forward,
                    &mut dir_control_timer,
                )
                .expect("direction setting failed");
        let mut stepper = dir_control_stepper.enable_step_control(pins);

        let res = stepper.step(&mut delay).await;
        res.unwrap();
        assert_eq!(stepper.driver().step, Some(1));
    }
}
