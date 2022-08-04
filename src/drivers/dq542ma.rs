//! DQ542MA Driver
//!
//! Platform-agnostic driver API for the DQ542MA stepper motor driver. Can be
//! used on any platform for which implementations of the required
//! [embedded-hal] traits are available.
//!
//! For the most part, users are not expected to use this API directly. Please
//! check out [`Stepper`](crate::Stepper) instead.
//!
//! [embedded-hal]: https://crates.io/crates/embedded-hal

use core::convert::Infallible;

use embedded_hal::digital::blocking::OutputPin;
use embedded_hal::digital::PinState;
use embedded_hal::digital::PinState::{High, Low};
use fugit::NanosDurationU32 as Nanoseconds;
use crate::Direction;

use crate::traits::{EnableDirectionControl, EnableStepControl, OutputPinAction, SetDirection, Step as StepTrait};

const STEP_PIN_BUS_WIDTH: usize = 1;

/// The DQ542MA driver API
///
/// Users are not expected to use this API directly, except to create an
/// instance using [`DQ542MA::new`]. Please check out
/// [`Stepper`](crate::Stepper) instead.
pub struct DQ542MA<Enable, Step, Dir> {
    enable: Enable,
    step: Step,
    dir: Dir,
}

impl DQ542MA<(), (), ()> {
    /// Create a new instance of `DQ542MA`
    pub fn new() -> Self {
        Self {
            enable: (),
            step: (),
            dir: (),
        }
    }
}

impl<Step, Dir, OutputPinError> EnableDirectionControl<Dir>
    for DQ542MA<(), Step, ()>
where
    Dir: OutputPin<Error = OutputPinError>,
{
    type WithDirectionControl = DQ542MA<(), Step, Dir>;

    fn enable_direction_control(self, dir: Dir) -> Self::WithDirectionControl {
        DQ542MA {
            enable: self.enable,
            step: self.step,
            dir,
        }
    }
}

impl<Step, Dir, OutputPinError> SetDirection for DQ542MA<(), Step, Dir>
where
    Dir: OutputPin<Error = OutputPinError>,
{
    // https://wiki.linuxcnc.org/cgi-bin/wiki.pl?Stepper_Drive_Timing
    const SETUP_TIME: Nanoseconds = Nanoseconds::from_ticks(500);

    type Dir = Dir;
    type Error = Infallible;

    fn dir(&mut self, direction: Direction) -> Result<OutputPinAction<&mut Self::Dir>, Self::Error> {
        Ok(OutputPinAction::Set(&mut self.dir, match direction {
            Direction::Forward => High,
            Direction::Backward => Low,
        }))
    }
}

impl<Step, Dir, OutputPinError> EnableStepControl<Step, STEP_PIN_BUS_WIDTH> for DQ542MA<(), (), Dir>
where
    Step: OutputPin<Error = OutputPinError>,
{
    type WithStepControl = DQ542MA<(), Step, Dir>;

    fn enable_step_control(self, step: Step) -> Self::WithStepControl {
        DQ542MA {
            enable: self.enable,
            step,
            dir: self.dir,
        }
    }
}

impl<Step, Dir, OutputPinError> StepTrait<STEP_PIN_BUS_WIDTH> for DQ542MA<(), Step, Dir>
where
    Step: OutputPin<Error = OutputPinError>,
{
    // https://wiki.linuxcnc.org/cgi-bin/wiki.pl?Stepper_Drive_Timing
    const PULSE_LENGTH: Nanoseconds = Nanoseconds::from_ticks(5050);

    type StepPin = Step;
    type Error = Infallible;

    fn step_leading(&mut self) -> Result<[OutputPinAction<&mut Self::StepPin>; STEP_PIN_BUS_WIDTH ], Self::Error> {
        Ok([OutputPinAction::Set(&mut self.step, High)])
    }

    fn step_trailing(&mut self) -> Result<[OutputPinAction<&mut Self::StepPin>; STEP_PIN_BUS_WIDTH ], Self::Error> {
        Ok([OutputPinAction::Set(&mut self.step, Low)])
    }
}
