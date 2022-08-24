//! Traits that can be implemented by Stepper drivers using async logic
//!
//! Users are generally not expected to use these traits directly, except to
//! specify trait bounds, where necessary. Please check out [`Stepper`], which
//! uses these traits to provide a unified API.
//!
//! When constructed, drivers usually do not provide access to any of their
//! capabilities. This means users can specifically enable the capabilities they
//! need, and do not have have to provide hardware resources (like output pins)
//! for capabilities that they are not going to use.
//!
//! This approach also provides a lot of flexibility for non-standard use cases,
//! for example if not all driver capabilities are controlled by software.
//!
//! [`Stepper`]: crate::Stepper

use crate::traits::{EnableDirectionControl, EnableStepControl};
use crate::Direction;
use core::future::Future;
use embedded_hal_async::delay::DelayUs;

/// Placeholder trait to track Async functionality being enabled on a driver
pub trait DelayAsyncEnabled<Delay: DelayUs> {}

/// To satisfy https://github.com/rust-lang/rust/issues/87479
pub trait OutputFutureItem {
    /// The type of result being returned
    type OutputFutResult;

    /// The error that can occur while performing a step
    type Error;
}

/// Implemented by drivers that support controlling the DIR signal
pub trait SetDelayAsync {
    /// "Async Enabled" placeholder type
    type AsyncEnabled<Delay: DelayUs>: DelayAsyncEnabled<Delay>;

    /// Sets an implementation of async DelayUs to be used with async actions
    fn set_delay<Delay: DelayUs>(
        self,
        delay: Delay,
    ) -> Self::AsyncEnabled<Delay>;
}

/// Implemented by drivers that support controlling the DIR signal
pub trait SetDirectionAsync<Resources, Delay>
where
    Self: EnableDirectionControl<Resources> + DelayAsyncEnabled<Delay>,
    Delay: DelayUs,
{
    /// The type of the DIR pin
    // type Dir: OutputPin;

    /// The output future type
    type OutputFut<'r>: Future<Output = Result<(), Self::Error>>
    where
        Self: 'r;

    /// The error that can occur while accessing the DIR pin
    type Error;

    /// Provides access to the DIR pin
    fn set_dir_async<'r>(
        &'r mut self,
        direction: Direction,
    ) -> Self::OutputFut<'r>;
}

/// Implemented by drivers which handle firing of pins independently
pub trait StepAsync<Resources, Delay, const STEP_BUS_WIDTH: usize>:
    OutputFutureItem
where
    Self: EnableStepControl<Resources, STEP_BUS_WIDTH>, /*+ DelayAsyncEnabled<Delay>*/
    Delay: DelayUs,
{
    /// The output future type is defined here
    type OutputFut<'r>: Future<
        Output = Result<Self::OutputFutResult, Self::Error>,
    >
    where
        Self: 'r,
        Delay: 'r;

    /// Performs a single step per driver's specific logic
    fn step_async<'r>(
        &'r mut self,
        delay: &'r mut Delay,
    ) -> Self::OutputFut<'r>;
}

/// Implemented by drivers which have logic allowing to release motor coils
pub trait ReleaseAsync<Resources, Delay, const STEP_BUS_WIDTH: usize>
where
    Self:
        EnableStepControl<Resources, STEP_BUS_WIDTH> + DelayAsyncEnabled<Delay>,
    Delay: DelayUs,
{
    /// The output future type
    type OutputFut: Future<Output = Result<(), Self::Error>>;

    /// The error that can occur while performing a step
    type Error;

    /// Performs a single step per driver's specific logic
    fn release_async(&mut self) -> Self::OutputFut;
}

/// Implemented by drivers that have motion control capabilities
///
/// A software-based fallback implementation exists in the [`motion_control`]
/// module, for drivers that implement [SetDirection] and [Step].
///
/// [`motion_control`]: crate::motion_control
pub trait MotionControlAsync {
    /// Output future type
    type OutputFut: Future<Output = Result<(), Self::Error>>;

    /// The type used by the driver to represent velocity
    type Velocity: Copy;

    /// The type error that can happen when using this trait
    type Error;

    /// Move to the given position
    ///
    /// This method must arrange for the motion to start, but must not block
    /// until it is completed. If more attention is required during the motion,
    /// this should be handled in [`MotionControl::update`].
    fn move_to_position_async(
        &mut self,
        max_velocity: Self::Velocity,
        target_step: i32,
    ) -> Self::OutputFut;

    /// Reset internal position to the given value
    ///
    /// This method must not start a motion. Its only purpose is to change the
    /// driver's internal position value, for example for homing.
    // fn reset_position(&mut self, step: i32) -> Result<(), Self::Error>;

    /// Update an ongoing motion
    ///
    /// This method may contain any code required to maintain an ongoing motion,
    /// if required, or it might just check whether a motion is still ongoing.
    ///
    /// Return `true`, if motion is ongoing, `false` otherwise. If `false` is
    /// returned, the caller may assume that this method doesn't need to be
    /// called again, until starting another motion.
    ///
    // TODO: See if the `move_to_position_async` can return a "handle" object
    //       Which includes a future and the update method is part of that object
    fn update(&mut self) -> Result<bool, Self::Error>;
}
