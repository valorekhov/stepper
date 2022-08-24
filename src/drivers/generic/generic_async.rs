use crate::drivers::generic::{Generic, GenericStepError};
use crate::traits::EnableStepControl;
use crate::traits_async::{OutputFutureItem, StepAsync};
use core::fmt::Debug;
use core::future::Future;
use embedded_hal::digital::blocking::OutputPin;
use embedded_hal_async::delay::DelayUs;

/// Async extensions for the Generic driver
///
/// Users are not expected to use this API directly, except to create an
/// instance using [`QuadLine::new`]. Please check out
/// [`Stepper`](crate::Stepper) instead.

impl<
        LinePin,
        OutputPinError,
        const STEP_BUS_WIDTH: usize,
        const NUM_STEPS: usize,
    > OutputFutureItem for Generic<LinePin, STEP_BUS_WIDTH, NUM_STEPS>
where
    LinePin: OutputPin<Error = OutputPinError>,
    OutputPinError: Debug,
{
    type OutputFutResult = ();
    type Error = GenericStepError;
}

impl<
        LinePin,
        Delay,
        OutputPinError,
        Resources,
        const STEP_BUS_WIDTH: usize,
        const NUM_STEPS: usize,
    > StepAsync<Resources, Delay, STEP_BUS_WIDTH>
    for Generic<LinePin, STEP_BUS_WIDTH, NUM_STEPS>
where
    Self: EnableStepControl<Resources, STEP_BUS_WIDTH>,
    LinePin: OutputPin<Error = OutputPinError>,
    // TODO: Revisit static?
    Delay: DelayUs + 'static,
    OutputPinError: Debug,
{
    type OutputFut<'r>
    = impl Future<Output = Result<Self::OutputFutResult, Self::Error>> where Self: 'r;

    fn step_async<'r>(
        self: &'r mut Self,
        delay: &'r mut Delay,
    ) -> Self::OutputFut<'r> {
        // unable to express the same as an inline async{} block due to https://github.com/rust-lang/rust/issues/65442
        // moving into a call of a an async function instead
        self.step_async_int(delay)
    }
}

impl<
        LinePin,
        OutputPinError,
        const STEP_BUS_WIDTH: usize,
        const NUM_STEPS: usize,
    > Generic<LinePin, STEP_BUS_WIDTH, NUM_STEPS>
where
    LinePin: OutputPin<Error = OutputPinError>,
    OutputPinError: Debug,
{
    async fn step_async_int<'d, Delay: DelayUs>(
        &mut self,
        delay: &'d mut Delay,
    ) -> Result<(), GenericStepError> {
        let mut current_step = self.step.unwrap_or_else(|| 0) as usize;

        let firing_sequence =
            self.steps.get(current_step).expect("Within index");

        for i in 0..STEP_BUS_WIDTH {
            let pin_idx = STEP_BUS_WIDTH - 1 - i;
            // println!("i = {:?}; pin_idx = {}", i, pin_idx);
            match self.pins[pin_idx] {
                ref mut pin => {
                    // println!("firing_sequence = {:?}; cond = {:?}", firing_sequence, (firing_sequence >> i) & 0x01  );
                    (if (firing_sequence >> i) & 0x01 == 0x01 {
                        pin.set_high()
                    } else {
                        pin.set_low()
                    })
                    .expect("it to work");
                }
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

    #[tokio::test]
    pub async fn test_stepping_async() {
        // let _dir = Pin::<u16>;
        //
        // let steps = [0b01 as u8, 0b10 as u8];
        //
        // {
        //     let mut pin1 = MockPin::new();
        //     let mut pin2 = MockPin::new();
        //
        //     pin1.expect_set_low().return_once(|| Ok(()));
        //     pin2.expect_set_high().return_once(|| Ok(()));
        //
        //     // Enable step control
        //     let mut stepper = Stepper::from_driver(Generic::new([&mut pin1, &mut pin2], steps)).enable_step_control_async((), DelayUsPosix{});
        //     //// Enable motion control using the software fallback
        //     //.enable_motion_control((timer, profile, crate::motion_control::TimeConversionError::DelayToTicks));
        //
        //     stepper.step_async().await.unwrap();
        //     assert_eq!(stepper.driver().step, Some(1));
        //     pin1.checkpoint();
        //     pin2.checkpoint();
        // }
        //
        // {
        //     let mut pin1 = MockPin::new();
        //     let mut pin2 = MockPin::new();
        //
        //     pin1.expect_set_high().return_once(|| Ok(()));
        //     pin2.expect_set_low().return_once(|| Ok(()));
        //
        //     let mut stepper = Stepper::from_driver(Generic::new([&mut pin1, &mut pin2], steps)).enable_step_control_async((), DelayUsPosix{});
        //
        //     stepper.driver().set_step(1).expect("correct number of steps");
        //     stepper.step_async().await.unwrap();
        //     assert_eq!(stepper.driver().step, Some(0));
        //     pin1.checkpoint();
        //     pin2.checkpoint();
        // }
    }
}
