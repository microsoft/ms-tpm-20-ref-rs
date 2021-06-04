//! PowerPlat.c

use serde::{Deserialize, Serialize};

use crate::error::Error;

use super::super::MsTpm20RefPlatformImpl;

#[derive(Clone, Serialize, Deserialize)]
pub struct PowerPlatState {
    power_lost: bool,
}

impl PowerPlatState {
    pub fn new() -> PowerPlatState {
        PowerPlatState { power_lost: false }
    }
}

impl MsTpm20RefPlatformImpl {
    pub fn signal_power_on(&mut self) -> Result<(), Error> {
        self.timer_reset();
        self.state.power_plat.power_lost = true;
        self.nv_enable()?;
        Ok(())
    }

    pub fn signal_power_off(&mut self) {
        self.nv_disable(false);
        self.act_enable_ticks(false);
    }

    fn signal_reset(&mut self) -> Result<(), Error> {
        self.timer_reset();
        self.state.locality.locality = 0;
        self.state.cancel.flag = false;

        // if we are doing reset but did not have a power failure, then we should
        // not need to reload NV ...

        Ok(())
    }

    fn was_power_lost(&mut self) -> bool {
        let ret = self.state.power_plat.power_lost;
        self.state.power_plat.power_lost = false;
        ret
    }
}

mod c_api {
    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__Signal_PowerOn() -> i32 {
        match platform!().signal_power_on() {
            Ok(()) => 0,
            Err(e) => {
                log::error!("error while powering on: {}", e);
                -1
            }
        }
    }

    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__WasPowerLost() -> i32 {
        platform!().was_power_lost() as i32
    }

    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__Signal_Reset() -> i32 {
        let ret = match platform!().signal_reset() {
            Ok(()) => 0,
            Err(e) => {
                log::error!("error while signalling reset: {}", e);
                -1
            }
        };

        // Must call _TPM_Init outside of the platform context to avoid deadlock
        //
        // SAFETY: _TPM_Init has no documented preconditions
        unsafe { crate::bindgen::_TPM_Init() };

        ret
    }

    #[no_mangle]
    #[log_derive::logfn(Trace)]
    #[log_derive::logfn_inputs(Trace)]
    pub unsafe extern "C" fn _plat__Signal_PowerOff() {
        platform!().signal_power_off()
    }
}
