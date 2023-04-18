//! Generic Driver
//!
//! Platform-agnostic generic driver API for software-controlled motors driven signal lines, such as the ULN200xx stepper motor drivers, unidirectional On-Off actuators, etc.
//! Upon initialization, takes in a signal bus width and a set of firing sequences for the supplied lines. Loops through the firing sequence in forward or
//! backward direction upon each step
//!
//! For the most part, users are not expected to use this API directly. Please
//! check out [`Stepper`](crate::Stepper) instead.
//!
//! [embedded-hal]: https://crates.io/crates/embedded-hal

use core::convert::Infallible;
use core::fmt::Debug;
use core::mem;
use core::mem::MaybeUninit;

use embedded_hal::digital::OutputPin;
use embedded_hal::digital::ErrorType;
use embedded_hal::digital::PinState::Low;
use embedded_hal_async::delay::DelayUs;
use fugit::NanosDurationU32 as Nanoseconds;

use crate::traits::ReleaseCoils;
use crate::{
    traits::{
        EnableDirectionControl, EnableStepControl, OutputPinAction,
        SetDirection,
    },
    Direction,
};

// #[cfg(feature = "async")]
/// Async extensions for the Generic driver
pub mod generic_async;

/// Quad Line Motor driver API
///
/// Users are not expected to use this API directly, except to create an
/// instance using [`QuadLine::new`]. Please check out
/// [`Stepper`](crate::Stepper) instead.
///

//TODO: Rename to `GenericDriver`
pub struct Generic<Pins, const NUM_STEPS: usize, Delay> {
    pins: Pins,
    steps: [u8; NUM_STEPS],
    step: Option<u8>,
    direction: Option<Direction>,
    delay: Delay,
}

impl<const NUM_STEPS: usize> Generic<(), NUM_STEPS, ()> {
    /// Create a new instance of `Generic`
    pub fn new(steps: [u8; NUM_STEPS]) -> Self {
        Self {
            pins: (),
            steps,
            step: None,
            direction: None,
            delay: (),
        }
    }
}

impl<Pins, const NUM_STEPS: usize, Delay> EnableDirectionControl<()>
    for Generic<Pins, NUM_STEPS, Delay>
{
    type WithDirectionControl = Generic<Pins, NUM_STEPS, Delay>;

    fn enable_direction_control(self, _: ()) -> Self::WithDirectionControl {
        Generic {
            pins: self.pins,
            steps: self.steps,
            step: self.step,
            direction: self.direction,
            delay: self.delay,
        }
    }
}

struct FooPin {}

impl ErrorType for FooPin {
    type Error = Infallible;
}

impl OutputPin for FooPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        todo!()
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        todo!()
    }
}

impl<Pins, const NUM_STEPS: usize, Delay> SetDirection
    for Generic<Pins, NUM_STEPS, Delay>
{
    const SETUP_TIME: Nanoseconds = Nanoseconds::from_ticks(0);

    // TODO: Remove `FooPin` after `SetDirection` is refactored to return a Future
    type Dir = FooPin;
    type Error = Infallible;

    fn dir(
        &mut self,
        direction: Direction,
    ) -> Result<OutputPinAction<&mut Self::Dir>, Self::Error> {
        self.direction = Some(direction);
        Ok(OutputPinAction::None)
    }
}

impl<
        LinePin: OutputPin,
        Delay: DelayUs,
        const STEP_BUS_WIDTH: usize,
        const NUM_STEPS: usize,
    > EnableStepControl<[LinePin; STEP_BUS_WIDTH], Delay>
    for Generic<(), NUM_STEPS, Delay>
{
    type WithStepControl = Generic<[LinePin; STEP_BUS_WIDTH], NUM_STEPS, Delay>;

    fn enable_step_control(
        self,
        pins: [LinePin; STEP_BUS_WIDTH],
    ) -> Self::WithStepControl {
        Generic {
            pins,
            steps: self.steps,
            step: self.step,
            direction: self.direction,
            delay: self.delay,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
/// Specifies errors which may occur during the step operation on the Generic driver
pub enum GenericStepError {
    /// Call enable_direction_control on the ['Stepper'] instance first
    MustCallEnableDirection,
}
//
// impl<
//     Pins,
//         OutputPinError,
//         const STEP_BUS_WIDTH: usize,
//         const NUM_STEPS: usize,
//     > StepLegacy<STEP_BUS_WIDTH> for Generic<Pins, NUM_STEPS>
// where
//     OutputPinError: Debug,
// {
//     /// NOT USED
//     const PULSE_LENGTH: Nanoseconds = Nanoseconds::from_ticks(0);
//
//     /// Type of the step pin(s)
//     type StepPin = LinePin;
//     type Error = GenericStepError;
//
//     fn step_leading(
//         &mut self,
//     ) -> Result<
//         [OutputPinAction<&mut Self::StepPin>; STEP_BUS_WIDTH],
//         Self::Error,
//     > {
//         let direction = self.direction.unwrap_or(Direction::Forward);
//
//         let mut current_step = self.step.unwrap_or(0) as usize;
//
//         // Retain current firing_seq here before current step is incremented
//         let firing_sequence =
//             *self.steps.get(current_step).expect("Within index");
//
//         current_step = match current_step.checked_add_signed(match direction {
//             Direction::Forward => 1_isize,
//             Direction::Backward => -1_isize,
//         }) {
//             Some(step) => {
//                 if direction == Direction::Forward && step >= NUM_STEPS {
//                     0
//                 } else {
//                     step
//                 }
//             }
//             // Subtraction (more like) / addition (less like) overflow occurred
//             None => match direction {
//                 // Prior step was 0 counting down / moving backward, set Step to max of the range
//                 Direction::Backward => NUM_STEPS - 1,
//                 // Unlikely to get here unless someone supplied `usize::Max` steps? :)
//                 Direction::Forward => 0,
//             },
//         };
//
//         self.step = Some(current_step as u8);
//
//         let x = move |i, pin| {
//             if firing_sequence >> (STEP_BUS_WIDTH - 1 - i) & 0x01 == 0x01 {
//                 OutputPinAction::Set(pin, High)
//             } else {
//                 OutputPinAction::Set(pin, Low)
//             }
//         };
//
//         let res = self.create_step_actions(x);
//
//         Ok(res)
//     }
//
//     fn step_trailing(
//         &mut self,
//     ) -> Result<
//         [OutputPinAction<&mut Self::StepPin>; STEP_BUS_WIDTH],
//         Self::Error,
//     > {
//         let res = self.create_step_actions(|_, _| OutputPinAction::None);
//         Ok(res)
//     }
// }

impl<LinePin, const STEP_BUS_WIDTH: usize, const NUM_STEPS: usize, Delay>
    Generic<[LinePin; STEP_BUS_WIDTH], NUM_STEPS, Delay>
{
    fn create_step_actions<'a, F>(
        &'a mut self,
        val_func: F,
    ) -> [OutputPinAction<&'a mut LinePin>; STEP_BUS_WIDTH]
    where
        F: Fn(usize, &'a mut LinePin) -> OutputPinAction<&'a mut LinePin>,
    {
        // Could've done `return [OutputPinAction::None; STEP_BUS_WIDTH]` but don't want to take an assumption on LinePin implementing Copy
        let mut data: [MaybeUninit<OutputPinAction<&mut LinePin>>;
            STEP_BUS_WIDTH] = unsafe { MaybeUninit::uninit().assume_init() };

        for (i, pin) in self.pins.iter_mut().enumerate() {
            data[i] = MaybeUninit::new(val_func(i, pin))
        }

        let ptr = &mut data as *mut _
            as *mut [OutputPinAction<&mut LinePin>; STEP_BUS_WIDTH];
        let res = unsafe { ptr.read() };
        mem::forget(data);
        res
    }
}

impl<
        LinePin,
        OutputPinError,
        Delay,
        const STEP_BUS_WIDTH: usize,
        const NUM_STEPS: usize,
    > ReleaseCoils
    for Generic<[LinePin; STEP_BUS_WIDTH], NUM_STEPS, Delay>
where
    LinePin: OutputPin<Error = OutputPinError>,
    OutputPinError: Debug,
{
    type Error = LinePin::Error;

    async fn release_coils<Delay2: DelayUs>(
        &mut self,
        _: &mut Delay2
    ) -> Result<(), Self::Error> {
        for pin in self.pins.iter_mut() {
            pin.set_state(Low)?
        }
        Ok(())
    }
}

impl<LinePin, Delay, const STEP_BUS_WIDTH: usize, const NUM_STEPS: usize>
    Generic<[LinePin; STEP_BUS_WIDTH], NUM_STEPS, Delay>
{
    /// Sets the current step in the provided step sequence. Has to be less than the "total number of steps"
    pub fn set_step(&mut self, step: u8) -> Result<(), ()> {
        if step < NUM_STEPS as u8 {
            self.step = Some(step);
            Ok(())
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::drivers::generic::Generic;
    use crate::test_utils::{NoDelay, OkTimer};
    use crate::{motion_control, test_utils::MockPin, Direction, Stepper};
    use core::convert::Infallible;
    use embedded_hal::digital::OutputPin;
    use fixed::FixedI64;
    use fixed::traits::Fixed;
    use fugit::TimerDurationU32;
    use mockall::mock;
    use ramp_maker::Trapezoidal;
    use typenum::U32;

    type FixedI64U32 = fixed::FixedI64<typenum::U32>;
    mock! {
        Timer{}
        impl fugit_timer::Timer<100> for Timer {
            /// An error that might happen during waiting
            type Error = Infallible;

            /// Return current time `Instant`
            fn now(&mut self) -> fugit::TimerInstantU32<100>;

            /// Start timer with a `duration`
            fn start(&mut self, duration: fugit::TimerDurationU32<100>) -> Result<(), <MockTimer as fugit_timer::Timer<100>>::Error>;

            /// Tries to stop this timer.
            /// An error will be returned if the timer has already been canceled or was never started.
            /// An error is also returned if the timer is not `Periodic` and has already expired.
            fn cancel(&mut self) -> Result<(), <MockTimer as fugit_timer::Timer<100>>::Error>;

            /// Wait until timer `duration` has expired.
            /// Must return `nb::Error::WouldBlock` if timer `duration` is not yet over.
            /// Must return `OK(())` as soon as timer `duration` has expired.
            fn wait(&mut self) -> nb::Result<(), <MockTimer as fugit_timer::Timer<100>>::Error>;
        }
    }

    pub struct DelayToTicks;
    impl<Delay: Fixed, const TIMER_HZ: u32>
        motion_control::DelayToTicks<Delay, TIMER_HZ> for DelayToTicks
    {
        type Error = Infallible;

        fn delay_to_ticks(
            &self,
            delay: Delay,
        ) -> Result<TimerDurationU32<TIMER_HZ>, Self::Error> {
            // TODO: This needs to be reviewed again.
            // Compiling but am unsure about the logic
            Ok(TimerDurationU32::<TIMER_HZ>::from_ticks(
                delay.to_u32().unwrap(),
            ))
        }
    }

    #[tokio::test]
    pub async fn test_stepping() {
        let steps = [0b01_u8, 0b10_u8];

        {
            let mut pin1 = MockPin::new();
            let mut pin2 = MockPin::new();

            pin1.expect_set_low().return_once(|| Ok(()));
            pin2.expect_set_high().return_once(|| Ok(()));

            let mut dir_timer = OkTimer::<1>::new();

            let mut delay = NoDelay {};

            let pins = [&mut pin1, &mut pin2];
            let mut stepper = Stepper::from_driver(Generic::new(steps))
                .set_delay(delay)
                .enable_direction_control(
                    (),
                    Direction::Backward
                )
                .expect("setting dit control to work")
                .enable_step_control(pins);

            let mut delay = NoDelay {};
            stepper
                .step(&mut delay)
                .await
                .expect("Stepping did not work");
            assert_eq!(stepper.driver().step, Some(1));
        }
    }

    #[tokio::test]
    pub async fn test_reverse_stepping() {
        let steps = [0b01 as u8, 0b10 as u8];
        {
            {
                let mut pin1 = MockPin::new();
                let mut pin2 = MockPin::new();

                pin1.expect_set_low().return_once(|| Ok(()));
                pin2.expect_set_high().return_once(|| Ok(()));

                let pins = [&mut pin1, &mut pin2];
                let mut delay = NoDelay;

                let mut stepper = Stepper::from_driver(Generic::new(steps))
                    .set_delay(delay)
                    .enable_direction_control(
                        (),
                        Direction::Backward
                    )
                    .expect("setting dit control to work")
                    .enable_step_control(pins);

                assert_eq!(stepper.driver().step, None);

                stepper
                    .step(&mut delay)
                    .await
                    .expect("Stepping did not work");
                assert_eq!(stepper.driver().step, Some(1));
            }

            {
                let mut pin1 = MockPin::new();
                let mut pin2 = MockPin::new();

                pin1.expect_set_high().return_once(|| Ok(()));
                pin2.expect_set_low().return_once(|| Ok(()));

                let mut dir_timer = OkTimer::<1>::new();

                let pins = [&mut pin1, &mut pin2];
                let mut delay = NoDelay;
                let mut stepper = Stepper::from_driver(Generic::new(steps))
                    .set_delay(delay)
                    .enable_direction_control(
                        (),
                        Direction::Backward
                    )
                    .expect("setting dit control to work")
                    .enable_step_control(pins);

                stepper.driver().set_step(1).expect("step be set");
                assert_eq!(stepper.driver().step, Some(1));

                stepper
                    .step(&mut delay)
                    .await
                    .expect("Stepping did not work");

                assert_eq!(stepper.driver().step, Some(0));
            }
        }
    }

    #[tokio::test]
    pub async fn test_software_motion_control() {
        let steps = [0_u8, 1_u8];

        let mut pin1 = MockPin::new();

        // The stepper will drive a single pin high and low, once per executed step.
        // For the first 10 steps movement we are expecting 5 high and 5 low pulses
        // For the second movement of 4 steps, we are adding 2 pulses each
        pin1.expect_set_low().times(5 + 2).returning(|| Ok(()));
        pin1.expect_set_high().times(5 + 2).returning(|| Ok(()));

        let mut dir_timer = OkTimer::<1>::new();

        let max_velocity = FixedI64U32::from_num(0.001);
        let profile: Trapezoidal<FixedI64<U32>> = Trapezoidal::new(max_velocity);

        let pins = [&mut pin1];
        let mut delay = NoDelay;
        let stepper = Stepper::from_driver(Generic::new(steps))
            .set_delay(delay)
            .enable_direction_control((), Direction::Backward)
            .expect("setting dir control to work")
            .enable_step_control(pins);
        let mut stepper =
            stepper.enable_motion_control((dir_timer, profile));

        let num_steps = 10;

        {
            // Assuming motion range of 0..=10 with the starting step at 0
            // Moving to position 10
            let res = stepper.move_to_position(max_velocity, num_steps).await;
            // Expecting the correct count of High/Low pulses on the pin should be the correct approach here.
            assert_eq!(res, Ok(()));
        }

        // Moving to position 6 from position 10, 4 steps in total
        {
            let going_back_steps = 4;
            let res = stepper
                .move_to_position(max_velocity, num_steps - going_back_steps).await;

            assert_eq!(res, Ok(()));
        }
    }

    // #[test]
    // pub fn require_enabling_dir_control_before_stepping() {
    //     let mut pin = MockPin::new();
    //     let mut driver = Generic::new([&mut pin], [0, 1]);
    //     match driver.step_leading() {
    //         Err(e) => assert_eq!(e, GenericStepError::MustCallEnableDirection),
    //         _ => assert!(false),
    //     }
    // }

    #[tokio::test]
    pub async fn release_coils() {
        let mut pin = MockPin::new();

        // Steps have been programmed to only send HIGH
        pin.expect_set_high().times(1).returning(|| Ok(()));

        // Low signal is expected out of the release calls
        pin.expect_set_low().times(2).returning(|| Ok(()));

        let pins = [&mut pin];
        let driver = Generic::new([1, 1]);

        let mut delay = NoDelay;
        let mut stepper =
            Stepper::from_driver(driver)
                .set_delay(delay)
                .enable_step_control(pins);

        stepper.step(&mut delay).await.expect("stepping failed");
        assert_eq!(stepper.driver().step, Some(1));
        stepper
            .release_coils(&mut delay)
            .await
            .expect("Coil release did not work");
        // Expecting step position not to change
        assert_eq!(stepper.driver().step, Some(1));
        stepper
            .release_coils(&mut delay)
            .await
            .expect("Coil release did not work");
        assert_eq!(stepper.driver().step, Some(1));
    }

    #[test]
    pub fn test_mut_array() {
        struct A<T> {
            pins: [T; 2],
        }

        impl<T: OutputPin> A<T> {
            pub fn new(pins: [T; 2]) -> Self {
                Self { pins }
            }

            pub fn set_high(&mut self) {
                for i in self.pins.iter_mut() {
                    i.set_high().unwrap()
                }
            }
        }

        let mut pin1 = MockPin::new();
        let mut pin2 = MockPin::new();

        pin1.expect_set_high().return_once(|| Ok(()));
        pin2.expect_set_high().return_once(|| Ok(()));
        let mut struct_a = A::new([pin1, pin2]);
        struct_a.set_high()
    }
}
