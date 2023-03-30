//! PlatformACT.c

use super::super::MsTpm20RefPlatformImpl;

// TODO: model ACTs using `PlaformCallbacks`?
impl MsTpm20RefPlatformImpl {
    fn act_get_implemented(&mut self, _act: u32) -> bool {
        true // must report true, or else TPM_Manufacture fails
    }

    fn act_get_remaining(&mut self, _act: u32) -> u32 {
        0
    }

    fn act_get_signaled(&mut self, _act: u32) -> i32 {
        0
    }

    fn act_set_signaled(&mut self, _act: u32, _on: i32) {}

    fn act_get_pending(&mut self, _act: u32) -> i32 {
        0
    }

    fn act_update_counter(&mut self, _act: u32, _new_value: u32) -> bool {
        true
    }

    pub fn act_enable_ticks(&mut self, _enable: bool) {}

    fn act_tick(&mut self) {}

    fn act_initialize(&mut self) -> bool {
        false
    }
}

mod c_api {
    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__ACT_GetImplemented(act: u32) -> i32 {
        platform!().act_get_implemented(act) as i32
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__ACT_GetRemaining(act: u32) -> u32 {
        platform!().act_get_remaining(act)
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__ACT_GetSignaled(act: u32) -> i32 {
        platform!().act_get_signaled(act)
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__ACT_SetSignaled(act: u32, on: i32) {
        platform!().act_set_signaled(act, on)
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__ACT_GetPending(act: u32) -> i32 {
        platform!().act_get_pending(act)
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__ACT_UpdateCounter(act: u32, new_value: u32) -> i32 {
        platform!().act_update_counter(act, new_value) as i32
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__ACT_EnableTicks(enable: i32) {
        platform!().act_enable_ticks(enable != 0)
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__ACT_Tick() {
        platform!().act_tick()
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__ACT_Initialize() -> i32 {
        platform!().act_initialize() as i32
    }
}
