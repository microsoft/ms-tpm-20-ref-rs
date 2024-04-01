// Copyright (C) Microsoft Corporation. All rights reserved.

//! This code needs to be written in C, as it uses setjmp/longjmp, which cannot
//! be called from Rust without running into potential UB. There is a tracking
//! issue to add support for at least a subset of setjmp/longjmp to Rust, but
//! none of them are available on stable as of the time of writing (June 2021)

#include <setjmp.h>
#include <stdint.h>

jmp_buf s_jumpBuffer;

// implemented by the TPM library
void ExecuteCommand(
    uint32_t requestSize,    // IN: command buffer size
    unsigned char *request,  // IN: command buffer
    uint32_t *responseSize,  // IN/OUT: response buffer size
    unsigned char **response // IN/OUT: response buffer
);

// called by the TPM library on critical error
void _plat__Fail(void)
{
    longjmp(&s_jumpBuffer[0], 1);
}

// safe wrapper around ExecuteCommand that includes the required setjmp/longjmp
// error handling logic
void RunCommand(
    uint32_t requestSize, unsigned char *request,
    uint32_t *responseSize, unsigned char **response)
{
    // If the longjmp is taken, then the TPM will have been put in failure mode,
    // and ExecuteCommand will return with failure information immediately
    // without calling _plat__Fail again.
    setjmp(s_jumpBuffer);
    ExecuteCommand(requestSize, request, responseSize, response);
}
