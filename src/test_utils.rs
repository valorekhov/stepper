use core::convert::Infallible;
use embedded_hal::digital::OutputPin;
use embedded_hal::digital::ErrorType;
use embedded_hal_async::delay::DelayUs;
use fugit::{TimerDurationU32, TimerInstantU32};
use fugit_timer::Timer;
use mockall::mock;

use std::time::{SystemTime, UNIX_EPOCH};

mock! {
        pub Pin{}
        impl ErrorType for Pin {
            type Error = Infallible;
        }

        impl OutputPin for Pin {
            fn set_low(&mut self) -> Result<(), <Self as ErrorType>::Error>;
            fn set_high(&mut self) -> Result<(), <Self as ErrorType>::Error>;
       }
}

pub(crate) struct SysClockTimer<const TIMER_HZ: u32> {
    start_time: u32,
    end_time: Option<TimerInstantU32<TIMER_HZ>>,
}

impl<const TIMER_HZ: u32> SysClockTimer<TIMER_HZ> {
    pub fn new() -> Self {
        Self {
            start_time: Self::epoch(),
            end_time: None,
        }
    }

    pub fn epoch() -> u32 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u32
    }

    pub fn now(&self) -> TimerInstantU32<TIMER_HZ> {
        TimerInstantU32::from_ticks(Self::epoch() - self.start_time)
    }
}

impl<const TIMER_HZ: u32> Timer<TIMER_HZ> for SysClockTimer<TIMER_HZ> {
    type Error = Infallible;

    fn now(&mut self) -> TimerInstantU32<TIMER_HZ> {
        SysClockTimer::now(self)
    }

    fn start(
        &mut self,
        duration: TimerDurationU32<TIMER_HZ>,
    ) -> Result<(), Self::Error> {
        let now = self.now();
        self.end_time.replace(now + duration);
        Ok(())
    }

    fn cancel(&mut self) -> Result<(), Self::Error> {
        self.end_time.take();
        Ok(())
    }

    fn wait(&mut self) -> nb::Result<(), Self::Error> {
        match self.end_time.map(|end| end <= self.now()) {
            Some(true) => {
                self.end_time.take();
                Ok(())
            }
            _ => Err(nb::Error::WouldBlock),
        }
    }
}

pub struct OkTimer<const TIMER_HZ: u32> {}
impl<const TIMER_HZ: u32> OkTimer<TIMER_HZ> {
    pub fn new() -> Self {
        Self {}
    }
}
impl<const TIMER_HZ: u32> fugit_timer::Timer<TIMER_HZ> for OkTimer<TIMER_HZ> {
    type Error = ();

    fn now(&mut self) -> TimerInstantU32<TIMER_HZ> {
        todo!()
    }

    fn start(
        &mut self,
        _duration: TimerDurationU32<TIMER_HZ>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn cancel(&mut self) -> Result<(), Self::Error> {
        todo!()
    }

    fn wait(&mut self) -> nb::Result<(), Self::Error> {
        Ok(())
    }
}

pub struct NoDelay;

impl DelayUs for NoDelay {
    type Error = ();

    async fn delay_us(&mut self, us: u32) -> Result<(), Self::Error> {
        core::future::ready(Ok(())).await
    }

    async fn delay_ms(&mut self, ms: u32) -> Result<(), Self::Error> {
        core::future::ready(Ok(())).await
    }
}
