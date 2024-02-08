//! LocalityPlat.c

use serde::{Deserialize, Serialize};

use super::super::MsTpm20RefPlatformImpl;

#[derive(Clone, Serialize, Deserialize)]
pub struct LocalityState {
    pub locality: u8,
}

impl LocalityState {
    pub fn new() -> LocalityState {
        LocalityState { locality: 0 }
    }
}

impl MsTpm20RefPlatformImpl {
    fn locality_set(&mut self, mut locality: u8) {
        if (5..32).contains(&locality) {
            tracing::warn!(
                "tried to set invalid locality {}. defaulting to zero...",
                locality
            );
            locality = 0;
        }

        self.state.locality.locality = locality;
    }

    fn locality_get(&mut self) -> u8 {
        self.state.locality.locality
    }
}

mod c_api {
    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__LocalityGet() -> u8 {
        platform!().locality_get()
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__LocalitySet(locality: u8) {
        platform!().locality_set(locality)
    }
}
