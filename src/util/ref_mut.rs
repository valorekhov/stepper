//! Generic wrapper around a mutable reference
//!
//! See [`RefMut`] for more information.

use crate::Direction;
use fugit::{
    NanosDurationU32 as Nanoseconds, TimerDurationU32 as TimerDuration,
    TimerInstantU32 as TimerInstant,
};
use fugit_timer::Timer;

use crate::traits::{
    MotionControl, OutputPinAction, SetDirection, SetStepMode, Step,
};

/// Generic wrapper around a mutable reference
///
/// This is used as a means of implementing traits that are already implemented
/// for `T` for `&mut T` too. While this is redundant for the traits from this
/// crate, we couldn't do this for `embedded_hal::timer::CountDown` without a
/// crate-local type.
///
/// The purpose of this is to make the future types more flexible, making it
/// possible to move types into them, or just provide mutable references.
pub struct RefMut<'r, T>(pub &'r mut T);

impl<'r, T, const TIMER_HZ: u32> Timer<TIMER_HZ> for RefMut<'r, T>
where
    T: Timer<TIMER_HZ>,
{
    type Error = T::Error;

    fn now(&mut self) -> TimerInstant<TIMER_HZ> {
        self.0.now()
    }

    fn start(
        &mut self,
        duration: TimerDuration<TIMER_HZ>,
    ) -> Result<(), Self::Error> {
        self.0.start(duration)
    }

    fn cancel(&mut self) -> Result<(), Self::Error> {
        self.0.cancel()
    }

    fn wait(&mut self) -> nb::Result<(), Self::Error> {
        self.0.wait()
    }
}

impl<'r, T> MotionControl for RefMut<'r, T>
where
    T: MotionControl,
{
    type Velocity = T::Velocity;
    type Error = T::Error;

    fn move_to_position(
        &mut self,
        max_velocity: Self::Velocity,
        target_step: i32,
    ) -> Result<(), Self::Error> {
        self.0.move_to_position(max_velocity, target_step)
    }

    fn reset_position(&mut self, step: i32) -> Result<(), Self::Error> {
        self.0.reset_position(step)
    }

    fn update(&mut self) -> Result<bool, Self::Error> {
        self.0.update()
    }
}

impl<'r, T> SetDirection for RefMut<'r, T>
where
    T: SetDirection,
{
    const SETUP_TIME: Nanoseconds = T::SETUP_TIME;

    type Dir = T::Dir;
    type Error = T::Error;

    fn dir(
        &mut self,
        direction: Direction,
    ) -> Result<OutputPinAction<&mut Self::Dir>, Self::Error> {
        self.0.dir(direction)
    }
}

impl<'r, T> SetStepMode for RefMut<'r, T>
where
    T: SetStepMode,
{
    const SETUP_TIME: Nanoseconds = T::SETUP_TIME;
    const HOLD_TIME: Nanoseconds = T::HOLD_TIME;

    type Error = T::Error;
    type StepMode = T::StepMode;

    fn apply_mode_config(
        &mut self,
        step_mode: Self::StepMode,
    ) -> Result<(), Self::Error> {
        self.0.apply_mode_config(step_mode)
    }

    fn enable_driver(&mut self) -> Result<(), Self::Error> {
        self.0.enable_driver()
    }
}

impl<'r, T, const STEP_BUS_WIDTH: usize> Step<STEP_BUS_WIDTH> for RefMut<'r, T>
where
    T: Step<STEP_BUS_WIDTH>,
{
    const PULSE_LENGTH: Nanoseconds = T::PULSE_LENGTH;

    type StepPin = T::StepPin;
    type Error = T::Error;

    fn step_leading(
        &mut self,
    ) -> Result<
        [OutputPinAction<&mut Self::StepPin>; STEP_BUS_WIDTH],
        Self::Error,
    > {
        self.0.step_leading()
    }

    fn step_trailing(
        &mut self,
    ) -> Result<
        [OutputPinAction<&mut Self::StepPin>; STEP_BUS_WIDTH],
        Self::Error,
    > {
        self.0.step_trailing()
    }
}
