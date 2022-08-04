//! STSPIN220 Driver
//!
//! Platform-agnostic driver API for the STSPIN220 stepper motor driver. Can be
//! used on any platform for which implementations of the required
//! [embedded-hal] traits are available.
//!
//! For the most part, users are not expected to use this API directly. Please
//! check out [`Stepper`](crate::Stepper) instead.
//!
//! [embedded-hal]: https://crates.io/crates/embedded-hal

use core::convert::Infallible;

use embedded_hal::digital::{blocking::OutputPin, PinState};
use embedded_hal::digital::PinState::{High, Low};
use fugit::NanosDurationU32 as Nanoseconds;

use crate::{Direction, step_mode::StepMode256, traits::{
    EnableDirectionControl, EnableStepControl, EnableStepModeControl,
    SetDirection, SetStepMode, Step,
}};
use crate::traits::OutputPinAction;

const STEP_PIN_BUS_WIDTH: usize = 1;

/// The STSPIN220 driver API
///
/// Users are not expected to use this API directly, except to create an
/// instance using [`STSPIN220::new`]. Please check out
/// [`Stepper`](crate::Stepper) instead.
pub struct STSPIN220<
    EnableFault,
    StandbyReset,
    Mode1,
    Mode2,
    StepMode3,
    DirMode4,
> {
    enable_fault: EnableFault,
    standby_reset: StandbyReset,
    mode1: Mode1,
    mode2: Mode2,
    step_mode3: StepMode3,
    dir_mode4: DirMode4,
}

impl STSPIN220<(), (), (), (), (), ()> {
    /// Create a new instance of `STSPIN220`
    pub fn new() -> Self {
        Self {
            enable_fault: (),
            standby_reset: (),
            mode1: (),
            mode2: (),
            step_mode3: (),
            dir_mode4: (),
        }
    }
}

impl<
        EnableFault,
        StandbyReset,
        Mode1,
        Mode2,
        StepMode3,
        DirMode4,
        OutputPinError,
    > EnableStepModeControl<(StandbyReset, Mode1, Mode2)>
    for STSPIN220<EnableFault, (), (), (), StepMode3, DirMode4>
where
    StandbyReset: OutputPin<Error = OutputPinError>,
    Mode1: OutputPin<Error = OutputPinError>,
    Mode2: OutputPin<Error = OutputPinError>,
    StepMode3: OutputPin<Error = OutputPinError>,
    DirMode4: OutputPin<Error = OutputPinError>,
{
    type WithStepModeControl =
        STSPIN220<EnableFault, StandbyReset, Mode1, Mode2, StepMode3, DirMode4>;

    fn enable_step_mode_control(
        self,
        (standby_reset, mode1, mode2): (StandbyReset, Mode1, Mode2),
    ) -> Self::WithStepModeControl {
        STSPIN220 {
            enable_fault: self.enable_fault,
            standby_reset,
            mode1,
            mode2,
            step_mode3: self.step_mode3,
            dir_mode4: self.dir_mode4,
        }
    }
}

impl<
        EnableFault,
        StandbyReset,
        Mode1,
        Mode2,
        StepMode3,
        DirMode4,
        OutputPinError,
    > SetStepMode
    for STSPIN220<EnableFault, StandbyReset, Mode1, Mode2, StepMode3, DirMode4>
where
    StandbyReset: OutputPin<Error = OutputPinError>,
    Mode1: OutputPin<Error = OutputPinError>,
    Mode2: OutputPin<Error = OutputPinError>,
    StepMode3: OutputPin<Error = OutputPinError>,
    DirMode4: OutputPin<Error = OutputPinError>,
{
    const SETUP_TIME: Nanoseconds = Nanoseconds::from_ticks(1_000);
    const HOLD_TIME: Nanoseconds = Nanoseconds::from_ticks(100_000);

    type Error = OutputPinError;
    type StepMode = StepMode256;

    fn apply_mode_config(
        &mut self,
        step_mode: Self::StepMode,
    ) -> Result<(), Self::Error> {
        // Force driver into standby mode.
        self.standby_reset.set_low()?;

        use PinState::*;
        use StepMode256::*;
        let (mode1, mode2, mode3, mode4) = match step_mode {
            Full => (Low, Low, Low, Low),
            M2 => (High, Low, High, Low),
            M4 => (Low, High, Low, High),
            M8 => (High, High, High, Low),
            M16 => (High, High, High, High),
            M32 => (Low, High, Low, Low),
            M64 => (High, High, Low, High),
            M128 => (High, Low, Low, Low),
            M256 => (High, High, Low, Low),
        };

        // Set mode signals.
        self.mode1.set_state(mode1)?;
        self.mode2.set_state(mode2)?;
        self.step_mode3.set_state(mode3)?;
        self.dir_mode4.set_state(mode4)?;

        Ok(())
    }

    fn enable_driver(&mut self) -> Result<(), Self::Error> {
        // Leave standby mode.
        self.standby_reset.set_high()
    }
}

impl<
        EnableFault,
        StandbyReset,
        Mode1,
        Mode2,
        StepMode3,
        DirMode4,
        OutputPinError,
    > EnableDirectionControl<DirMode4>
    for STSPIN220<EnableFault, StandbyReset, Mode1, Mode2, StepMode3, ()>
where
    DirMode4: OutputPin<Error = OutputPinError>,
{
    type WithDirectionControl =
        STSPIN220<EnableFault, StandbyReset, Mode1, Mode2, StepMode3, DirMode4>;

    fn enable_direction_control(
        self,
        dir_mode4: DirMode4,
    ) -> Self::WithDirectionControl {
        STSPIN220 {
            enable_fault: self.enable_fault,
            standby_reset: self.standby_reset,
            mode1: self.mode1,
            mode2: self.mode2,
            step_mode3: self.step_mode3,
            dir_mode4,
        }
    }
}

impl<
        EnableFault,
        StandbyReset,
        Mode1,
        Mode2,
        StepMode3,
        DirMode4,
        OutputPinError,
    > SetDirection
    for STSPIN220<EnableFault, StandbyReset, Mode1, Mode2, StepMode3, DirMode4>
where
    DirMode4: OutputPin<Error = OutputPinError>,
{
    const SETUP_TIME: Nanoseconds = Nanoseconds::from_ticks(100);

    type Dir = DirMode4;
    type Error = Infallible;

    fn dir(&mut self, direction: Direction) -> Result<OutputPinAction<&mut Self::Dir>, Self::Error> {
        Ok(OutputPinAction::Set(&mut self.dir_mode4, match direction {
            Direction::Forward => High,
            Direction::Backward => Low,
        }))
    }
}

impl<
        EnableFault,
        StandbyReset,
        Mode1,
        Mode2,
        StepMode3,
        DirMode4,
        OutputPinError,
    > EnableStepControl<StepMode3, 1>
    for STSPIN220<EnableFault, StandbyReset, Mode1, Mode2, (), DirMode4>
where
    StepMode3: OutputPin<Error = OutputPinError>,
{
    type WithStepControl =
        STSPIN220<EnableFault, StandbyReset, Mode1, Mode2, StepMode3, DirMode4>;

    fn enable_step_control(
        self,
        step_mode3: StepMode3,
    ) -> Self::WithStepControl {
        STSPIN220 {
            enable_fault: self.enable_fault,
            standby_reset: self.standby_reset,
            mode1: self.mode1,
            mode2: self.mode2,
            step_mode3,
            dir_mode4: self.dir_mode4,
        }
    }
}

impl<
        EnableFault,
        StandbyReset,
        Mode1,
        Mode2,
        StepMode3,
        DirMode4,
        OutputPinError,
    > Step<1>
    for STSPIN220<EnableFault, StandbyReset, Mode1, Mode2, StepMode3, DirMode4>
where
    StepMode3: OutputPin<Error = OutputPinError>,
{
    const PULSE_LENGTH: Nanoseconds = Nanoseconds::from_ticks(100);

    type StepPin = StepMode3;
    type Error = Infallible;

    fn step_leading(&mut self) -> Result<[OutputPinAction<&mut Self::StepPin>; STEP_PIN_BUS_WIDTH ], Self::Error> {
        Ok([OutputPinAction::Set(&mut self.step_mode3, High)])
    }

    fn step_trailing(&mut self) -> Result<[OutputPinAction<&mut Self::StepPin>; STEP_PIN_BUS_WIDTH ], Self::Error> {
        Ok([OutputPinAction::Set(&mut self.step_mode3, Low)])
    }
}
