// Copyright (C) Microsoft Corporation. All rights reserved.

//! Clock.c

use std::convert::TryInto;

use serde::Deserialize;
use serde::Serialize;

use super::super::MsTpm20RefPlatformImpl;

const CLOCK_NOMINAL: u32 = 30000;
const CLOCK_ADJUST_LIMIT: i32 = 5000;

const CLOCK_ADJUST_COARSE: i32 = 300;
const CLOCK_ADJUST_MEDIUM: i32 = 30;
const CLOCK_ADJUST_FINE: i32 = 1;

#[derive(Clone, Serialize, Deserialize)]
pub struct ClockState {
    adjust_rate: u32,

    timer_reset: bool,
    timer_stopped: bool,

    // These values are used to try to synthesize a long lived version of clock().
    last_system_time: u128,
    last_reported_time: u128,

    // This is the value returned the last time that the system clock was read. This
    // is only relevant for a simulator or virtual TPM.
    last_real_time: u128,

    // This is the rate adjusted value that is the equivalent of what would be read from
    // a hardware register that produced rate adjusted time.
    tpm_time: u128,
}

impl ClockState {
    pub fn new() -> ClockState {
        ClockState {
            adjust_rate: CLOCK_NOMINAL,

            timer_reset: true,
            timer_stopped: false,

            last_system_time: 0,
            last_reported_time: 0,
            last_real_time: 0,
            tpm_time: 0,
        }
    }
}

impl MsTpm20RefPlatformImpl {
    pub fn timer_reset(&mut self) {
        self.state.clock = ClockState::new();
    }
}

impl MsTpm20RefPlatformImpl {
    // Ported over from ms-tps-20-re/TPMCmd/Platform/src/Clock.c
    fn timer_read(&mut self) -> u64 {
        let ClockState {
            adjust_rate,
            last_system_time,
            last_reported_time,
            last_real_time,
            tpm_time,
            ..
        } = &mut self.state.clock;

        let now = self.callbacks.monotonic_timer().as_millis();

        if *last_system_time == 0 {
            *last_system_time = now;
            *last_reported_time = 0;
            *last_real_time = 0;
        }

        // The system time can bounce around and that's OK as long as we don't allow
        // time to go backwards. When the time does appear to go backwards, set
        // lastSystemTime to be the new value and then update the reported time.
        if now < *last_reported_time {
            *last_reported_time = now;
        }
        *last_reported_time = (*last_reported_time + now).wrapping_sub(*last_system_time);
        *last_system_time = now;

        // The code above produces a timeNow that is similar to the value returned
        // by Clock(). The difference is that timeNow does not max out, and it is
        // at a ms. rate rather than at a CLOCKS_PER_SEC rate. The code below
        // uses that value and does the rate adjustment on the time value.
        // If there is no difference in time, then skip all the computations
        if *last_real_time >= now {
            return (*tpm_time)
                .try_into()
                .expect("timestamp doesn't fit in 64 bits");
        }
        // Compute the amount of time since the last update of the system clock
        let time_diff = now - *last_real_time;

        // Do the time rate adjustment and conversion from CLOCKS_PER_SEC to mSec
        let adjusted_time_diff = (time_diff * CLOCK_NOMINAL as u128) / (*adjust_rate as u128);

        // update the TPM time with the adjusted timeDiff
        *tpm_time += adjusted_time_diff;

        // Might have some rounding error that would loose CLOCKS. See what is not
        // being used. As mentioned above, this could result in putting back more than
        // is taken out. Here, we are trying to recreate timeDiff.
        let readjusted_time_diff =
            (adjusted_time_diff * (*adjust_rate as u128)) / CLOCK_NOMINAL as u128;

        // adjusted is now converted back to being the amount we should advance the
        // previous sampled time. It should always be less than or equal to timeDiff.
        // That is, we could not have use more time than we started with.
        *last_real_time += readjusted_time_diff;

        (*tpm_time)
            .try_into()
            .expect("timestamp doesn't fit in 64 bits")
    }

    fn timer_was_reset(&mut self) -> bool {
        let ret = self.state.clock.timer_reset;
        self.state.clock.timer_reset = false;
        ret
    }

    fn timer_was_stopped(&mut self) -> bool {
        let ret = self.state.clock.timer_stopped;
        self.state.clock.timer_stopped = false;
        ret
    }

    fn clock_adjust_rate(&mut self, adjust: i32) {
        match adjust.abs() {
            CLOCK_ADJUST_COARSE | CLOCK_ADJUST_MEDIUM | CLOCK_ADJUST_FINE => {}
            _ => return, // ignore invalid values
        }

        self.state.clock.adjust_rate = ((self.state.clock.adjust_rate as i32) + adjust).clamp(
            (CLOCK_NOMINAL as i32) - CLOCK_ADJUST_LIMIT,
            (CLOCK_NOMINAL as i32) + CLOCK_ADJUST_LIMIT,
        ) as u32;
    }
}

mod c_api {
    // NOTE: The commented out functions are only ever called from the simulator,
    // and as such, they really shouldn't have been specified as part of the the
    // platform interface...

    // #[no_mangle]
    // pub unsafe extern "C" fn _plat__TimerReset() {
    //     platform!().timer_reset()
    // }

    // #[no_mangle]
    // pub unsafe extern "C" fn _plat__TimerRestart() {
    //     platform!().timer_restart()
    // }

    //
    // #[no_mangle]
    // pub unsafe extern "C" fn _plat__RealTime() -> u64 {
    //     platform!().real_time()
    // }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__TimerRead() -> u64 {
        platform!().timer_read()
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__TimerWasReset() -> i32 {
        platform!().timer_was_reset() as i32
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__TimerWasStopped() -> i32 {
        platform!().timer_was_stopped() as i32
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__ClockAdjustRate(adjust: i32) {
        platform!().clock_adjust_rate(adjust)
    }
}
