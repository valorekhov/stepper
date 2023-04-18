//! Software implementation of motion control capability
//!
//! See [`SoftwareMotionControl`] for more information.

mod conversion;
mod error;
mod state;

pub use self::{
    conversion::DelayToTicks,
    error::{BusyError, Error, TimeConversionError},
};

use core::convert::Infallible;

use embedded_hal::digital::ErrorType;
use embedded_hal_async::delay::DelayUs;
use fugit::NanosDurationU32 as Nanoseconds;
use ramp_maker::MotionProfile;
use replace_with::replace_with_and_return;

use crate::stepper::set_step_mode::SetStepModeFuture;
use crate::traits::OutputPinAction;
use crate::{
    traits::{
        EnableMotionControl, MotionControl, SetDirection, SetStepMode, Step,
    },
    util::ref_mut::RefMut,
    Direction, SetDirectionFuture,
};

use self::state::State;

/// Software implementation of motion control capability
///
/// Some driver natively support motion control capability. This is a software
/// implementation of the [`MotionControl`] trait for those drivers that don't.
/// It wraps a driver that implements [`SetDirection`] and [`Step`], and in turn
/// acts like a driver itself, adding to the wrapped driver's capabilities.
///
/// You can use `SoftwareMotionControl` directly, but like a driver, it is
/// designed to be used through the [`Stepper`] API.
///
/// [`Stepper`]: crate::Stepper
pub struct SoftwareMotionControl<
    'r,
    Driver,
    Delay: DelayUs,
    Profile: MotionProfile,
    Convert,
    const TIMER_HZ: u32,
    const STEP_BUS_WIDTH: usize,
> {
    state: State<'r, Driver, Delay, Profile, TIMER_HZ, STEP_BUS_WIDTH>,
    new_motion: Option<Direction>,
    profile: Profile,
    current_step: i32,
    current_direction: Direction,
    convert: Convert,
}

impl<
        'r,
        Driver,
        Delay,
        Profile,
        Convert,
        const TIMER_HZ: u32,
        const STEP_BUS_WIDTH: usize,
    >
    SoftwareMotionControl<
        'r,
        Driver,
        Delay,
        Profile,
        Convert,
        TIMER_HZ,
        STEP_BUS_WIDTH,
    >
where
    Profile: MotionProfile,
    Delay: DelayUs,
{
    /// Construct a new instance of `SoftwareMotionControl`
    ///
    /// Instead of using this constructor directly, you can instead use
    /// [`Stepper::enable_motion_control`] with any driver that implements
    /// [`SetDirection`] and [`Step`], providing timer and a motion profile.
    /// This module provides a blanket implementation of [`EnableMotionControl`]
    /// to make this work.
    ///
    /// [`Stepper::enable_motion_control`]: crate::Stepper::enable_motion_control
    pub fn new(
        driver: Driver,
        delay: Delay,
        profile: Profile,
        convert: Convert,
    ) -> Self {
        Self {
            state: State::Idle { driver, delay },
            new_motion: None,
            profile,
            current_step: 0,
            // Doesn't matter what we initialize it with. We're only using it
            // during an ongoing movement, and it will have been overridden at
            // that point.
            current_direction: Direction::Forward,
            convert,
        }
    }

    /// Access a reference to the wrapped driver
    ///
    /// This is only possible if there is no ongoing movement.
    pub fn driver(&self) -> Option<&Driver> {
        if let State::Idle { driver, .. } = &self.state {
            return Some(driver);
        }

        None
    }

    /// Access a mutable reference to the wrapped driver
    ///
    /// This is only possible if there is no ongoing movement.
    pub fn driver_mut(&mut self) -> Option<&mut Driver> {
        if let State::Idle { driver, .. } = &mut self.state {
            return Some(driver);
        }

        None
    }

    /// Access a reference to the wrapped timer
    ///
    /// This is only possible if there is no ongoing movement.
    pub fn timer(&self) -> Option<&Delay> {
        if let State::Idle { delay: timer, .. } = &self.state {
            return Some(timer);
        }

        None
    }

    /// Access a mutable reference to the wrapped timer
    ///
    /// This is only possible if there is no ongoing movement.
    pub fn timer_mut(&mut self) -> Option<&mut Delay> {
        if let State::Idle { delay: timer, .. } = &mut self.state {
            return Some(timer);
        }

        None
    }

    /// Access a reference to the wrapped motion profile
    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    /// Access a mutable reference to the wrapped motion profile
    pub fn profile_mut(&mut self) -> &mut Profile {
        &mut self.profile
    }

    /// Access the current step
    pub fn current_step(&self) -> i32 {
        self.current_step
    }

    /// Access the current direction
    pub fn current_direction(&self) -> Direction {
        self.current_direction
    }

    /// Set step mode of the wrapped driver
    ///
    /// This method is a more convenient alternative to
    /// [`Stepper::set_step_mode`], which requires a timer, while this methods
    /// reuses the timer that `SoftwareMotionControl` already owns.
    ///
    /// However, while [`Stepper::set_step_mode`] is part of the generic API,
    /// this method is only available, if you statically know that you're
    /// working with a driver wrapped by `SoftwareMotionControl`.
    ///
    /// # Errors
    ///
    /// Returns [`BusyError::Busy`], if a motion is ongoing.
    ///
    /// [`Stepper::set_step_mode`]: crate::Stepper::set_step_mode
    pub fn set_step_mode(
        &mut self,
        step_mode: Driver::StepMode,
    ) -> Result<
        SetStepModeFuture<RefMut<Driver>, RefMut<Delay>>,
        BusyError<Infallible>,
    >
    where
        Driver: SetStepMode,
        Delay: DelayUs,
    {
        let future = match &mut self.state {
            State::Idle { driver, delay } => {
                SetStepModeFuture::new(step_mode, RefMut(driver), RefMut(delay))
            }
            _ => return Err(BusyError::Busy),
        };

        Ok(future)
    }

    /// Set direction of the wrapped driver
    ///
    /// This method is a more convenient alternative to
    /// [`Stepper::set_direction`], which requires a timer, while this methods
    /// reuses the timer that `SoftwareMotionControl` already owns.
    ///
    /// However, while [`Stepper::set_direction`] is part of the generic API,
    /// this method is only available, if you statically know that you're
    /// working with a driver wrapped by `SoftwareMotionControl`.
    ///
    /// # Errors
    ///
    /// Returns [`BusyError::Busy`], if a motion is ongoing.
    ///
    /// [`Stepper::set_direction`]: crate::Stepper::set_direction
    pub fn set_direction(
        &mut self,
        direction: Direction,
    ) -> Result<
        SetDirectionFuture<RefMut<Driver>, RefMut<Delay>>,
        BusyError<Infallible>,
    >
    where
        Driver: SetDirection,
        Delay: DelayUs,
    {
        let future = match &mut self.state {
            State::Idle { driver, delay: timer } => SetDirectionFuture::new(
                direction,
                RefMut(driver),
                RefMut(timer),
            ),
            _ => return Err(BusyError::Busy),
        };

        Ok(future)
    }

    /// Tell the wrapped driver to move the motor one step
    ///
    /// This method is a more convenient alternative to [`Stepper::step`], which
    /// requires a timer, while this methods reuses the timer that
    /// `SoftwareMotionControl` already owns.
    ///
    /// However, while [`Stepper::step`] is part of the generic API, this method
    /// is only available, if you statically know that you're working with a
    /// driver wrapped by `SoftwareMotionControl`.
    ///
    /// # Errors
    ///
    /// Returns [`BusyError::Busy`], if a motion is ongoing.
    ///
    /// [`Stepper::step`]: crate::Stepper::step
    pub fn step(
        &mut self,
    ) -> Result<Driver::OutputStepFutureResult, BusyError<Infallible>>
    where
        Driver: Step,
        Delay: DelayUs,
    {
        let future = match &mut self.state {
            State::Idle {
                driver,
                delay,
            } => driver.step(delay),
            _ => return Err(BusyError::Busy),
        };

        Ok(future)
    }
}

impl<
        'r,
        Driver,
        Delay: DelayUs,
        Profile,
        Convert,
        const TIMER_HZ: u32,
        const STEP_BUS_WIDTH: usize,
    > MotionControl
    for SoftwareMotionControl<
        'r,
        Driver,
        Delay,
        Profile,
        Convert,
        TIMER_HZ,
        STEP_BUS_WIDTH,
    >
where
    Driver: SetDirection + Step,
    Profile: MotionProfile,
    Delay: DelayUs,
    Profile::Velocity: Copy,
    Convert: DelayToTicks<Profile::Delay, TIMER_HZ>,
{
    type Velocity = Profile::Velocity;
    type Error = Error<
        <Driver as SetDirection>::Error,
        <<Driver as SetDirection>::Dir as ErrorType>::Error,
        <Driver as Step>::OutputStepFutureError,
        Infallible,
        Delay::Error,
        Convert::Error,
    >;

    async fn move_to_position(
        &mut self,
        max_velocity: Self::Velocity,
        target_step: i32,
    ) -> Result<(), Error<
        <Driver as SetDirection>::Error,
        <<Driver as SetDirection>::Dir as ErrorType>::Error,
        <Driver as Step>::OutputStepFutureError,
        Infallible,
        Delay::Error,
        Convert::Error,
    >> {
        let steps_from_here = target_step - self.current_step;

        self.profile
            .enter_position_mode(max_velocity, steps_from_here.abs() as u32);

        let direction = if steps_from_here > 0 {
            Direction::Forward
        } else {
            Direction::Backward
        };
        self.new_motion = Some(direction);

        Ok(())
    }

    fn reset_position(&mut self, step: i32) -> Result<(), Self::Error> {
        self.current_step = step;
        Ok(())
    }

    // async fn update(&mut self) -> Result<bool, Self::Error> {
    //     // Otherwise the closure will borrow all of `self`.
    //     let new_motion = &mut self.new_motion;
    //     let profile = &mut self.profile;
    //     let current_step = &mut self.current_step;
    //     let current_direction = &mut self.current_direction;
    //     let convert = &self.convert;
    //
    //     replace_with_and_return(
    //         &mut self.state,
    //         || State::Invalid,
    //         |state| {
    //             state::update(
    //                 state,
    //                 new_motion,
    //                 profile,
    //                 current_step,
    //                 current_direction,
    //                 convert,
    //             )
    //         },
    //     )
    // }
}

// We could also implement the various "enable" traits here, but those
// implementations can only work while we have access to the driver, which
// mostly means we'd have to be idle. Since the "enable" traits are infallible,
// we'd have to panic, and I don't know if that would be worth it.

impl<
        'r,
        Driver,
        Delay: DelayUs,
        Profile,
        Convert,
        const TIMER_HZ: u32,
        const STEP_BUS_WIDTH: usize,
    > SetStepMode
    for SoftwareMotionControl<
        'r,
        Driver,
        Delay,
        Profile,
        Convert,
        TIMER_HZ,
        STEP_BUS_WIDTH,
    >
where
    Driver: SetStepMode,
    Profile: MotionProfile,
{
    const SETUP_TIME: Nanoseconds = Driver::SETUP_TIME;
    const HOLD_TIME: Nanoseconds = Driver::HOLD_TIME;

    type Error = BusyError<Driver::Error>;
    type StepMode = Driver::StepMode;

    fn apply_mode_config(
        &mut self,
        step_mode: Self::StepMode,
    ) -> Result<(), Self::Error> {
        match self.driver_mut() {
            Some(driver) => driver
                .apply_mode_config(step_mode)
                .map_err(|err| BusyError::Other(err)),
            None => Err(BusyError::Busy),
        }
    }

    fn enable_driver(&mut self) -> Result<(), Self::Error> {
        match self.driver_mut() {
            Some(driver) => {
                driver.enable_driver().map_err(|err| BusyError::Other(err))
            }
            None => Err(BusyError::Busy),
        }
    }
}

impl<
        'r,
        Driver,
        Delay: DelayUs,
        Profile,
        Convert,
        const TIMER_HZ: u32,
        const STEP_BUS_WIDTH: usize,
    > SetDirection
    for SoftwareMotionControl<
        'r,
        Driver,
        Delay,
        Profile,
        Convert,
        TIMER_HZ,
        STEP_BUS_WIDTH,
    >
where
    Driver: SetDirection,
    Profile: MotionProfile,
{
    const SETUP_TIME: Nanoseconds = Driver::SETUP_TIME;

    type Dir = Driver::Dir;
    type Error = BusyError<Driver::Error>;

    fn dir(
        &mut self,
        direction: Direction,
    ) -> Result<OutputPinAction<&mut Self::Dir>, Self::Error> {
        match self.driver_mut() {
            Some(driver) => {
                driver.dir(direction).map_err(|err| BusyError::Other(err))
            }
            None => Err(BusyError::Busy),
        }
    }
}

impl<
        'r,
        Driver,
        Delay: DelayUs,
        Profile,
        Convert,
        const TIMER_HZ: u32,
        const STEP_BUS_WIDTH: usize,
    > Step
    for SoftwareMotionControl<
        'r,
        Driver,
        Delay,
        Profile,
        Convert,
        TIMER_HZ,
        STEP_BUS_WIDTH,
    >
where
    Driver: Step,
    Profile: MotionProfile,
    //Convert: 'r,
{
    type OutputStepFutureResult = ();
    type OutputStepFutureError = BusyError<Driver::OutputStepFutureError>;

    async fn step<Delay2: DelayUs>(
        &mut self,
        delay: &mut Delay2,
    ) -> Result<(), BusyError<Driver::OutputStepFutureError>> {
        match self.driver_mut() {
            Some(driver) => {
                let result = driver.step(delay).await;
                match result {
                    Ok(_) => Ok(()),
                    Err(err) => Err(BusyError::Other(err)),
                }
            }
            None =>
                Err(BusyError::Busy),
        }
    }
}

// Blanket implementation of `EnableMotionControl` for all STEP/DIR stepper
// drivers.
impl<
        'r,
        Driver,
        Delay,
        Profile,
        Convert,
        const TIMER_HZ: u32,
        const STEP_BUS_WIDTH: usize,
    > EnableMotionControl<(Delay, Profile, Convert), TIMER_HZ, STEP_BUS_WIDTH>
    for &'r Driver
where
    Driver: SetDirection + Step,
    Profile: MotionProfile,
    Delay: DelayUs + 'r,
    Profile::Velocity: Copy,
    Convert: DelayToTicks<Profile::Delay, TIMER_HZ>,
{
    type WithMotionControl = SoftwareMotionControl<'r,
        Driver,
        Delay,
        Profile,
        Convert,
        TIMER_HZ,
        STEP_BUS_WIDTH,
    >;

    fn enable_motion_control(
        self,
        (delay, profile, convert): (Delay, Profile, Convert),
    ) -> Self::WithMotionControl {
        // The issue is due to 'r lifetime used in the "for" generic type on line 517
        SoftwareMotionControl::new(self, delay, profile, convert)
    }
}
