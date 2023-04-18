use core::future::Future;
use core::pin::Pin;
use crate::legacy_future::LegacyFuture;
use core::task::{Context, Poll};

use crate::traits::MotionControl;

/// The "future" returned by [`Stepper::move_to_position`]
///
/// Please note that this type provides a custom API and does not implement
/// [`core::future::Future`]. This might change, when using futures for embedded
/// development becomes more practical.
///
/// [`Stepper::move_to_position`]: crate::Stepper::move_to_position
#[must_use]
pub struct MoveToFuture<Driver: MotionControl> {
    driver: Driver,
    state: State<Driver::Velocity>,
}

impl<Driver> MoveToFuture<Driver>
where
    Driver: MotionControl,
{
    /// Create new instance of `MoveToFuture`
    ///
    /// This constructor is public to provide maximum flexibility for
    /// non-standard use cases. Most users can ignore this and just use
    /// [`Stepper::move_to_position`] instead.
    ///
    /// [`Stepper::move_to_position`]: crate::Stepper::move_to_position
    pub fn new(
        driver: Driver,
        max_velocity: Driver::Velocity,
        target_step: i32,
    ) -> Self {
        Self {
            driver,
            state: State::Initial {
                max_velocity,
                target_step,
            },
        }
    }

    /// Drop the future and release the resources that were moved into it
    pub fn release(self) -> Driver {
        self.driver
    }
}

impl<Driver> LegacyFuture for MoveToFuture<Driver>
where
    Driver: MotionControl,
{
    type DriverError = Driver::Error;
    type TimerError = ();
    type FutureOutput = Result<(), Driver::Error>;

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
    fn poll(&mut self) -> Poll<Self::FutureOutput> {
        todo!()
    }
}

impl<Driver> Future for MoveToFuture<Driver>
    where
        Driver: MotionControl,
{
    type Output = Result<(), Driver::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.state {
            State::Initial {
                max_velocity,
                target_step,
            } => {
                self.driver.move_to_position(max_velocity, target_step)?;
                self.state = State::Moving;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            State::Moving => {
                let still_moving = self.driver.update()?;
                cx.waker().wake_by_ref();
                if still_moving {
                    Poll::Pending
                } else {
                    self.state = State::Finished;
                    Poll::Ready(Ok(()))
                }
            }
            State::Finished => Poll::Ready(Ok(())),
        }
    }
}

enum State<Velocity> {
    Initial {
        max_velocity: Velocity,
        target_step: i32,
    },
    Moving,
    Finished,
}
