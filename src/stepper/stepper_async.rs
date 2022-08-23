use crate::traits_async::SetDelayAsync;
use crate::Stepper;
use embedded_hal_async::delay::DelayUs;

/// Enable async features of the stepper by providing it with an implementation
/// of async DelayUs
impl<Driver: SetDelayAsync> Stepper<Driver> {
    fn set_delay<Delay: DelayUs>(
        &mut self,
        delay: Delay,
    ) -> Stepper<Driver::AsyncEnabled<Delay>> {
        Stepper {
            driver: self.driver.set_delay(delay),
        }
    }
}
