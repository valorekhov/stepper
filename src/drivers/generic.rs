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
use core::mem::MaybeUninit;
use core::mem;

use embedded_hal::digital::blocking::OutputPin;
use embedded_hal::digital::PinState::{High, Low};
use fugit::NanosDurationU32 as Nanoseconds;

use crate::{Direction, traits::{
    EnableDirectionControl, SetDirection, EnableStepControl, OutputPinAction, Step
}};

/// Quad Line Motor driver API
///
/// Users are not expected to use this API directly, except to create an
/// instance using [`QuadLine::new`]. Please check out
/// [`Stepper`](crate::Stepper) instead.
pub struct Generic<LinePin, Delay, const STEP_BUS_WIDTH: usize, const NUM_STEPS: usize>
{
    pins: [LinePin; STEP_BUS_WIDTH],
    steps: [u8; NUM_STEPS],
    step: Option<u8>,
    direction: Option<Direction>,
    delay: Delay
}

impl<LinePin, const STEP_BUS_WIDTH: usize, const NUM_STEPS: usize> Generic<LinePin, (), STEP_BUS_WIDTH, NUM_STEPS> {
    /// Create a new instance of `QuadLine`
    pub fn new(pins: [LinePin; STEP_BUS_WIDTH], steps: [u8; NUM_STEPS]) -> Self {
        Self {
            pins,
            steps,
            step: None,
            direction: None,
            delay: ()
        }
    }
}

impl<LinePin, Delay, OutputPinError, const STEP_BUS_WIDTH: usize, const NUM_STEPS: usize>
EnableDirectionControl<()>
for Generic<LinePin, Delay, STEP_BUS_WIDTH, NUM_STEPS>
    where
        LinePin: OutputPin<Error = OutputPinError>,
{
    type WithDirectionControl = Generic<LinePin, Delay, STEP_BUS_WIDTH, NUM_STEPS>;

    fn enable_direction_control(self, _: ()) -> Self::WithDirectionControl {
        Generic {
            pins: self.pins,
            steps: self.steps,
            step: self.step,
            direction: self.direction,
            delay: self.delay
        }
    }
}

impl<LinePin, Delay, OutputPinError, const STEP_BUS_WIDTH: usize, const NUM_STEPS: usize> SetDirection
for Generic<LinePin, Delay, STEP_BUS_WIDTH, NUM_STEPS>
    where
        LinePin: OutputPin<Error = OutputPinError>,
{
    const SETUP_TIME: Nanoseconds = Nanoseconds::from_ticks(0);

    // The Dir pin will not really be driven. Setting it to a real type instead of ()
    // to satisfy generic arg constraints
    type Dir = LinePin;
    type Error = Infallible;

    fn dir(&mut self, direction: Direction) -> Result<OutputPinAction<&mut Self::Dir>, Self::Error> {
        self.direction = Some(direction);
        Ok(OutputPinAction::None)
    }
}

impl<LinePin, SrcDelay, OutputPinError, const STEP_BUS_WIDTH: usize, const NUM_STEPS: usize>
EnableStepControl<(), STEP_BUS_WIDTH>
for Generic<LinePin, SrcDelay, STEP_BUS_WIDTH, NUM_STEPS>
    where
        LinePin: OutputPin<Error = OutputPinError>,
        OutputPinError: Debug
{
    type WithStepControl = Generic<LinePin, SrcDelay, STEP_BUS_WIDTH, NUM_STEPS>;

    fn enable_step_control(self, _: ()) -> Self::WithStepControl {
        Generic {
            pins: self.pins,
            steps: self.steps,
            step: self.step,
            direction: self.direction,
            delay: self.delay
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
/// Specifies errors which may occur during the step operation on the Generic driver
pub enum GenericStepError{
    /// Call enable_direction_control on the ['Stepper'] instance first
    MustCallEnableDirection
}

impl<LinePin, Delay, OutputPinError, const STEP_BUS_WIDTH: usize, const NUM_STEPS: usize> Step<STEP_BUS_WIDTH>
for Generic<LinePin, Delay, STEP_BUS_WIDTH, NUM_STEPS>
    where
        LinePin: OutputPin<Error = OutputPinError>,
        OutputPinError: Debug
{
    /// NOT USED
    const PULSE_LENGTH: Nanoseconds = Nanoseconds::from_ticks(0);

    /// NOT USED
    type StepPin = LinePin;
    type Error = GenericStepError;

    fn step_leading(&mut self) -> Result<[OutputPinAction<&mut Self::StepPin>; STEP_BUS_WIDTH ], Self::Error> {

        if self.direction.is_none() {
            return Err(GenericStepError::MustCallEnableDirection);
        }

        let direction = self.direction.unwrap();

        let mut current_step = self.step.unwrap_or_else(|| 0) as usize;

        // Retain current firing_seq here before current step is incremented
        let firing_sequence = *self.steps.get(current_step).expect("Within index");

        current_step = match current_step.checked_add_signed( match direction {
            Direction::Forward => 1 as isize,
            Direction::Backward => -1 as isize,
        }) {
            Some(step) => if direction == Direction::Forward && step >= NUM_STEPS { 0 } else { step },
            // Subtraction (more like) / addition (less like) overflow occurred
            None => match direction {
                // Prior step was 0 counting down / moving backward, set Step to max of the range
                Direction::Backward => NUM_STEPS -1,
                // Unlikely to get here unless someone supplied `usize::Max` steps? :)
                Direction::Forward => 0,
            }
        };

        self.step = Some(current_step as u8);

        let mut data: [MaybeUninit<OutputPinAction<&mut Self::StepPin>>; STEP_BUS_WIDTH] = unsafe { MaybeUninit::uninit().assume_init() };

        for (i, pin) in self.pins.iter_mut().enumerate(){
            let bit_idx = STEP_BUS_WIDTH - 1 - i;

            data[i] = MaybeUninit::new(if (firing_sequence >> bit_idx) & 0x01  == 0x01
                { OutputPinAction::Set( pin, High) } else { OutputPinAction::Set(pin, Low) }
            )
        }

        let ptr = &mut data as *mut _ as *mut [OutputPinAction<&mut Self::StepPin>; STEP_BUS_WIDTH];
        let res = unsafe { ptr.read() };
        mem::forget(data);

        Ok( res )
    }

    fn step_trailing(&mut self) -> Result<[OutputPinAction<&mut Self::StepPin>; STEP_BUS_WIDTH ], Self::Error> {
        // Could've done `return [OutputPinAction::None; STEP_BUS_WIDTH]` but don't want to take an assumption on LinePin implementing Copy

        let mut data: [MaybeUninit<OutputPinAction<&mut Self::StepPin>>; STEP_BUS_WIDTH] = unsafe { MaybeUninit::uninit().assume_init() };

        for (i, _) in self.pins.iter_mut().enumerate(){
            data[i] = MaybeUninit::new(OutputPinAction::None )
        }

        let ptr = &mut data as *mut _ as *mut [OutputPinAction<&mut Self::StepPin>; STEP_BUS_WIDTH];
        let res = unsafe { ptr.read() };
        mem::forget(data);

        Ok( res )
    }
}

// impl<LinePin, SrcDelay, Delay, OutputPinError, const STEP_BUS_WIDTH: usize, const NUM_STEPS: usize>
// EnableStepControlAsync<(), Delay>
// for Generic<LinePin, SrcDelay, STEP_BUS_WIDTH, NUM_STEPS>
//     where
//         LinePin: OutputPin<Error = OutputPinError>,
//         Delay: DelayUs,
//         OutputPinError: Debug
// {
//     type WithAsyncStepControl = Generic<LinePin, Delay, STEP_BUS_WIDTH, NUM_STEPS>;
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
// impl<LinePin, Delay, OutputPinError, const STEP_BUS_WIDTH: usize, const NUM_STEPS: usize> StepAsync
// for Generic<LinePin, Delay, STEP_BUS_WIDTH, NUM_STEPS>
//     where
//         LinePin: OutputPin<Error = OutputPinError>,
//         Delay: DelayUs,
//         OutputPinError: Debug
// {
//     type OutputFut<'r> = impl Future<Output = Result<(), Self::Error>> where Self: 'r;
//     type Error = OutputPinError;
//
//     fn step_async<'r>(self: &'r mut Self) -> Self::OutputFut<'r> {
//         // unable to express the same as an inline async{} block due to https://github.com/rust-lang/rust/issues/65442
//         // moving into a call of a an async function instead
//         self.step()
//     }
// }

impl<LinePin, Delay, const STEP_BUS_WIDTH: usize, const NUM_STEPS: usize> Generic<LinePin, Delay, STEP_BUS_WIDTH, NUM_STEPS>
    where
        Self: Step<STEP_BUS_WIDTH>
{
    /// Sets the current step in the provided step sequence. Has to be less than the "total number of steps"
    pub fn set_step(&mut self, step: u8) -> Result<(), ()>{
        if step < NUM_STEPS as u8 {
            self.step = Some(step);
            Ok(())
        } else {
            Err(())
        }
    }
}

// impl<LinePin, Delay, OutputPinError, const STEP_BUS_WIDTH: usize, const NUM_STEPS: usize>  Generic<LinePin, Delay, STEP_BUS_WIDTH, NUM_STEPS>
// where LinePin: OutputPin<Error = OutputPinError>, Delay: DelayUs, OutputPinError: Debug {
//     async fn step<Error>(self: &mut Self) -> Result<(), Error> {
//         let mut current_step = self.step.unwrap_or_else(|| 0) as usize;
//
//         let firing_sequence = self.steps.get(current_step).expect("Within index");
//
//         for i in 0..STEP_BUS_WIDTH {
//             let pin_idx = STEP_BUS_WIDTH - 1 - i;
//             // println!("i = {:?}; pin_idx = {}", i, pin_idx);
//             match self.pins[pin_idx] {
//                 ref mut pin => {
//                     // println!("firing_sequence = {:?}; cond = {:?}", firing_sequence, (firing_sequence >> i) & 0x01  );
//                     (if (firing_sequence >> i) & 0x01 == 0x01
//                         { pin.set_high() } else { pin.set_low() }).expect("it to work");
//                 }
//             };
//         }
//
//         current_step += 1;
//         if current_step >= NUM_STEPS {
//             current_step = 0
//         }
//         self.step = Some(current_step as u8);
//
//         self.delay.delay_us(100).await.expect("sleep finished");
//         Ok(())
//     }
// }

#[cfg(test)]
mod test{
    use core::convert::Infallible;
    use core::task::Poll;
    use embedded_hal::digital::blocking::OutputPin;
    use embedded_hal::digital::ErrorType;
    use mockall::mock;
    use nb::Error::WouldBlock;
    use crate::drivers::generic::{Generic, GenericStepError};
    use crate::{Direction, Stepper};
    use crate::traits::Step;

    mock!{
        Pin{}
        impl ErrorType for Pin {
            type Error = Infallible;
        }

        impl OutputPin for Pin {
            fn set_low(&mut self) -> Result<(), <Self as ErrorType>::Error>;
            fn set_high(&mut self) -> Result<(), <Self as ErrorType>::Error>;
       }
    }

    mock!{
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

    #[test]
    pub fn test_stepping() {
        let steps = [0b01 as u8, 0b10 as u8];

        {
            let mut pin1 = MockPin::new();
            let mut pin2 = MockPin::new();

            pin1.expect_set_low().return_once(|| Ok(()));
            pin2.expect_set_high().return_once(|| Ok(()));

            let mut dir_timer = MockTimer::new();
            dir_timer.expect_start().return_once(|_| Ok(()));
            dir_timer.expect_wait().return_once(|| Ok(()));

            let mut stepper = Stepper::from_driver(Generic::new([&mut pin1, &mut pin2], steps))
                .enable_direction_control((), Direction::Backward, &mut dir_timer).expect("setting dit control to work")
                .enable_step_control(());
            //// Enable motion control using the software fallback
            //.enable_motion_control((timer, profile, crate::motion_control::TimeConversionError::DelayToTicks));

            let mut timer = MockTimer::new();
            timer.expect_start().return_once(|_| Ok(()));
            timer.expect_wait().times(2).returning(|| Err(WouldBlock));
            timer.expect_wait().return_once(|| Ok(()));

            let mut fut = stepper.step(&mut timer);

            // First poll should kick off the timer calling Timer::start(duration)
            assert_eq!(fut.poll(), Poll::Pending);
            // Second & 3rd poll should result in the timer returning WouldBlock
            assert_eq!(fut.poll(), Poll::Pending);
            assert_eq!(fut.poll(), Poll::Pending);

            match fut.poll() {
                Poll::Ready(_) => {
                    assert!(true)
                }
                Poll::Pending => assert!(false)
            };

            assert_eq!(stepper.driver().step, Some(1));
        }
    }

    #[test]
    pub fn test_reverse_stepping() {
        let steps = [0b01 as u8, 0b10 as u8];
        {
            {
                let mut pin1 = MockPin::new();
                let mut pin2 = MockPin::new();

                pin1.expect_set_low().return_once(|| Ok(()));
                pin2.expect_set_high().return_once(|| Ok(()));

                let mut dir_timer = MockTimer::new();
                dir_timer.expect_start().return_once(|_| Ok(()));
                dir_timer.expect_wait().return_once(|| Ok(()));

                let mut stepper = Stepper::from_driver(Generic::new([&mut pin1, &mut pin2], steps))
                    .enable_direction_control((), Direction::Backward, &mut dir_timer).expect("setting dit control to work")
                    .enable_step_control(());

                let mut timer = MockTimer::new();
                timer.expect_start().return_once(|_| Ok(()));
                timer.expect_wait().return_once(|| Ok(()));

                assert_eq!(stepper.driver().step, None);

                let mut fut = stepper.step(&mut timer);

                assert_eq!(fut.poll(), Poll::Pending);
                match fut.poll() {
                    Poll::Ready(res) => {
                        assert!(!res.is_err())
                    }
                    Poll::Pending => assert!(false)
                };

                assert_eq!(stepper.driver().step, Some(1));
            }

            {
                let mut pin1 = MockPin::new();
                let mut pin2 = MockPin::new();

                pin1.expect_set_high().return_once(|| Ok(()));
                pin2.expect_set_low().return_once(|| Ok(()));

                let mut dir_timer = MockTimer::new();
                dir_timer.expect_start().return_once(|_| Ok(()));
                dir_timer.expect_wait().return_once(|| Ok(()));

                let mut stepper = Stepper::from_driver(Generic::new([&mut pin1, &mut pin2], steps))
                    .enable_direction_control((), Direction::Backward, &mut dir_timer).expect("setting dit control to work")
                    .enable_step_control(());

                let mut timer = MockTimer::new();
                timer.expect_start().return_once(|_| Ok(()));
                timer.expect_wait().return_once(|| Ok(()));

                stepper.driver().set_step(1).expect("step be set");
                assert_eq!(stepper.driver().step, Some(1));

                let mut fut = stepper.step(&mut timer);

                assert_eq!(fut.poll(), Poll::Pending);
                match fut.poll() {
                    Poll::Ready(res) => {
                        assert!(!res.is_err())
                    }
                    Poll::Pending => assert!(false)
                };

                assert_eq!(stepper.driver().step, Some(0));
            }
        }
    }

    #[test]
    pub fn require_enabling_dir_control_before_stepping() {
        let mut pin = MockPin::new();
        let mut driver = Generic::new([&mut pin], [0, 1]);
        match driver.step_leading() {
            Err(e) => assert_eq!(e, GenericStepError::MustCallEnableDirection),
            _ => assert!(false)
        }
    }


    #[tokio::test]
    pub async fn test_stepping_async(){
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

    #[test]
    pub fn test_mut_array() {
        struct A<T>{
            pins: [T;2]
        }

        impl<T : OutputPin> A<T>{
            pub fn new(pins: [T;2]) -> Self {
                Self {
                    pins
                }
            }

            pub fn set_high(&mut self){
                for i in self.pins.iter_mut(){
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