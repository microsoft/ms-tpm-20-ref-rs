use serde::{Deserialize, Serialize};

// TODO: dump all the C library's global variables
#[derive(Clone, Serialize, Deserialize)]
pub struct MsTpm20RefLibraryState {}

pub fn restore_runtime_state(_state: MsTpm20RefLibraryState) {
    // TODO
}

pub fn get_runtime_state() -> MsTpm20RefLibraryState {
    // TODO
    MsTpm20RefLibraryState {}
}
