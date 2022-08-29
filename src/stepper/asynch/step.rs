use crate::traits::Step;
use crate::SignalError;
use embedded_hal::digital::blocking::OutputPin;
use embedded_hal::digital::ErrorType;
use embedded_hal_async::delay::DelayUs;

/// Rotates the motor one step in the given direction
///
/// Steps the motor one step in the direction that was previously set,
/// according to the current entry in the pin firing configuration. To achieve a specific
/// speed, the user must call this method at an appropriate frequency.
///
/// You might need to call [`Stepper::enable_step_control`] to make this
/// method available.
pub async fn step_async<
    Driver,
    Delay,
    const TIMER_HZ: u32,
    const BUS_WIDTH: usize,
>(
    driver: &mut Driver,
    delay: &mut Delay,
) -> Result<
    (),
    SignalError<
        <Driver::StepPin as Step<BUS_WIDTH>>::Error,
        <Driver::StepPin as ErrorType>::Error,
        Delay::Error,
    >,
>
where
    Driver: Step<BUS_WIDTH> + OutputPin,
    Delay: DelayUs,
    <Driver as Step<BUS_WIDTH>>::StepPin: Step<BUS_WIDTH>,
    SignalError<
        <Driver::StepPin as Step<BUS_WIDTH>>::Error,
        <Driver::StepPin as ErrorType>::Error,
        Delay::Error,
    >: From<
        SignalError<
            <Driver as Step<BUS_WIDTH>>::Error,
            <Driver::StepPin as ErrorType>::Error,
            Delay::Error,
        >,
    >,
{
    driver
        .step()
        .map_err(|err| SignalError::PinUnavailable(err))?
        .set_high()
        .map_err(|err| SignalError::Pin(err))?;

    delay
        .delay_us(Driver::PULSE_LENGTH.to_micros())
        .await
        .map_err(|err| SignalError::Timer(err))?;

    driver
        .step()
        .map_err(|err| SignalError::PinUnavailable(err))?
        .set_low()
        .map_err(|err| SignalError::Pin(err))?;

    Ok(())
}
