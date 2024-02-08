//! Cancel.c

use serde::{Deserialize, Serialize};

use super::super::MsTpm20RefPlatformImpl;

#[derive(Clone, Serialize, Deserialize)]
pub struct CancelState {
    pub flag: bool,
}

impl CancelState {
    pub fn new() -> CancelState {
        CancelState { flag: false }
    }
}

impl MsTpm20RefPlatformImpl {
    fn is_canceled(&self) -> bool {
        self.state.cancel.flag
    }

    pub fn set_cancel(&mut self) {
        self.state.cancel.flag = true;
    }

    pub fn clear_cancel(&mut self) {
        self.state.cancel.flag = false;
    }
}

mod c_api {
    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__IsCanceled() -> i32 {
        platform!().is_canceled() as i32
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__SetCancel() {
        platform!().set_cancel()
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__ClearCancel() {
        platform!().clear_cancel()
    }
}
