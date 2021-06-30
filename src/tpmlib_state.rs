use serde::{Deserialize, Serialize};

// TODO: actually dump all the C library's global variables.
//
// This is a real PITA, since the library doesn't come with any off-the-shelf
// way to dump it's global state. Instead, a human has to parse the `Globals.h`
// header from the C library, and write a custom function that dumps
// de/serialized this state.
//
// At the moment, HvLite doesn't support classic VM-style full-suspend save
// states, and as such, we can punt this feature down the line.
#[derive(Clone, Serialize, Deserialize)]
pub struct MsTpm20RefLibraryState {}

/// NOTE: THIS FUNCTION IS CURRENTLY `unimplemented!`
pub fn restore_runtime_state(_state: MsTpm20RefLibraryState) {
    unimplemented!()
}

/// NOTE: THIS FUNCTION IS CURRENTLY `unimplemented!`
pub fn get_runtime_state() -> MsTpm20RefLibraryState {
    unimplemented!()
}
