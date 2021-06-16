//! FFI for calling into the ms-tpm-20-ref C library
extern "C" {
    pub fn _TPM_Init();
    pub fn TPM_Manufacture(firstTime: ::std::os::raw::c_int) -> ::std::os::raw::c_int;

    // used by the sample platform
    pub fn _plat__RunCommand(
        requestSize: u32,
        request: *mut ::std::os::raw::c_uchar,
        responseSize: *mut u32,
        response: *mut *mut ::std::os::raw::c_uchar,
    );
    pub fn _plat__SetNvAvail();
    pub fn _plat__NVEnable(platParameter: *mut ::std::os::raw::c_void) -> ::std::os::raw::c_int;
    pub fn _plat__Signal_PowerOn() -> ::std::os::raw::c_int;
    pub fn _plat__NVNeedsManufacture() -> ::std::os::raw::c_int;
    pub fn _plat__Signal_PowerOff();

}
