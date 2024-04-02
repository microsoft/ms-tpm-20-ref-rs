// Copyright (C) Microsoft Corporation. All rights reserved.

//! PPPlat.c

use super::super::MsTpm20RefPlatformImpl;

// TODO: model physical presence using `PlaformCallbacks`?
//
// Also, on a more general note, shouldn't this API just be the one
// `physical_presence_asserted` function? i.e: that function should encapsulate
// the machinery to detect if a user is physically present... i.e: why would you
// ever call the other two functions instead of just updating your own internal
// state directly??
impl MsTpm20RefPlatformImpl {
    fn physical_presence_asserted(&mut self) -> bool {
        false
    }

    fn signal_physical_presence_on(&mut self) {}

    fn signal_physical_presence_off(&mut self) {}
}

mod c_api {
    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__PhysicalPresenceAsserted() -> i32 {
        platform!().physical_presence_asserted() as i32
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__Signal_PhysicalPresenceOn() {
        platform!().signal_physical_presence_on()
    }

    #[no_mangle]
    #[tracing::instrument(level = "trace")]
    pub unsafe extern "C" fn _plat__Signal_PhysicalPresenceOff() {
        platform!().signal_physical_presence_off()
    }
}
