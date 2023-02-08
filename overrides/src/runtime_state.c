// Hooks to save/restore all live global state of the TPM library
//
// This is not functionality built into ms-tpm-20-ref, as it's a pretty niche
// requirement (only really relevant for things like vTPMs, which must support
// live save/restore).

#include <stdbool.h>
#include <stdint.h>

#define GLOBAL_C
#include "Tpm.h"
#include "Global.h"

#define ARRAY_SIZE(a) (sizeof(a) / sizeof(a[0]))

//
// The header structure for vTPM run-time state blob.
//
typedef struct tag_TPM_RUNTIME_STATE_HEADER
{
    //
    // Contains a sequence of "VTPMRTST".
    //
    uint64_t HeaderMagic64;

    //
    // A number which has to match the local vTPM platform revision number to ensure the same set of static variables is getting saved and restored.
    //
    uint32_t Revision;

    //
    // Number of variables for which the data is present in the runtime state blob.
    //
    uint32_t VariableCount;

} TPM_RUNTIME_STATE_HEADER, *PTPM_RUNTIME_STATE_HEADER;

//
// Runtime state header magic value of "VTPMRTST".
//
static const uint64_t s_RuntimeStateHeaderMagic = 0x545354524D505456;

//
// Increment this revision on every change to the number or type of global static variables used by the TPM engine.
//
static const uint32_t s_RuntimeStateRevision = 3;

//
// Contains information about a single run-time variable.
//
typedef struct tag_TPM_RUNTIME_STATE_ENTRY
{
    //
    // Pointer to a variable.
    //
    const void *pbRuntimeVariable;

    //
    // Variable size.
    //
    const uint32_t cbVariableSize;

} TPM_RUNTIME_STATE_ENTRY;

//
// Enumerates all run-time variables inside the TPM engine and platform (as defined in Global.h).
//
static const TPM_RUNTIME_STATE_ENTRY s_TpmRuntimeVariables[] =
    {
        {(char *)&g_exclusiveAuditSession, sizeof(g_exclusiveAuditSession)},
        {(char *)&g_time, sizeof(g_time)},
        {(char *)&g_phEnable, sizeof(g_phEnable)},
        {(char *)&g_pcrReConfig, sizeof(g_pcrReConfig)},
        {(char *)&g_DRTMHandle, sizeof(g_DRTMHandle)},
        {(char *)&g_DrtmPreStartup, sizeof(g_DrtmPreStartup)},
        {(char *)&g_updateNV, sizeof(g_updateNV)},
        {(char *)&g_nvOk, sizeof(g_nvOk)},
        {(char *)&g_clearOrderly, sizeof(g_clearOrderly)},
        {(char *)&g_prevOrderlyState, sizeof(g_prevOrderlyState)},
        {(char *)&gp, sizeof(gp)},
        {(char *)&go, sizeof(go)},
        {(char *)&gc, sizeof(gc)},
        {(char *)&gr, sizeof(gr)},
        {(char *)s_sessionHandles, sizeof(s_sessionHandles)},
        {(char *)s_attributes, sizeof(s_attributes)},
        {(char *)s_associatedHandles, sizeof(s_associatedHandles)},
        {(char *)s_nonceCaller, sizeof(s_nonceCaller)},
        {(char *)s_inputAuthValues, sizeof(s_inputAuthValues)},
        {(char *)&s_encryptSessionIndex, sizeof(s_encryptSessionIndex)},
        {(char *)&s_decryptSessionIndex, sizeof(s_decryptSessionIndex)},
        {(char *)&s_auditSessionIndex, sizeof(s_auditSessionIndex)},
        {(char *)&s_cpHashForCommandAudit, sizeof(s_cpHashForCommandAudit)},
        {(char *)&s_DAPendingOnNV, sizeof(s_DAPendingOnNV)},
        {(char *)&s_selfHealTimer, sizeof(s_selfHealTimer)},
        {(char *)&g_NvStatus, sizeof(g_NvStatus)},
        {(char *)s_objects, sizeof(s_objects)},
        {(char *)s_pcrs, sizeof(s_pcrs)},
        {(char *)s_sessions, sizeof(s_sessions)},
        {(char *)&s_oldestSavedSession, sizeof(s_oldestSavedSession)},
        {(char *)&s_freeSessionSlots, sizeof(s_freeSessionSlots)},
        {(char *)&g_manufactured, sizeof(g_manufactured)},
        {(char *)&g_initialized, sizeof(g_initialized)},
        {(char *)&g_forceFailureMode, sizeof(g_forceFailureMode)},
        {(char *)&g_inFailureMode, sizeof(g_inFailureMode)},
        {(char *)&s_failFunction, sizeof(s_failFunction)},
        {(char *)&s_failLine, sizeof(s_failLine)},
        {(char *)&s_failCode, sizeof(s_failCode)}};

static uint32_t
GetRuntimeStateSize()
{
    uint32_t totalSize = 0;
    uint32_t i;

    for (i = 0; i < ARRAY_SIZE(s_TpmRuntimeVariables); i++)
    {
        totalSize += s_TpmRuntimeVariables[i].cbVariableSize;
    }

    return totalSize + sizeof(TPM_RUNTIME_STATE_HEADER);
}

// Returns:
// - 0 on success
// - 1 for invalid arg
// - 2 for insufficient size (setting pBufferSize to required size)
int INJECTED_GetRuntimeState(
    void *pBuffer,
    uint32_t *pBufferSize)
{
    if (pBufferSize == NULL ||
        (pBuffer == NULL && *pBufferSize != 0))
    {
        return 1;
    }

    uint32_t requiredSize = GetRuntimeStateSize();

    if (*pBufferSize < requiredSize)
    {
        *pBufferSize = requiredSize;
        return 2;
    }

    PTPM_RUNTIME_STATE_HEADER pHeader = (PTPM_RUNTIME_STATE_HEADER)pBuffer;

    pHeader->HeaderMagic64 = s_RuntimeStateHeaderMagic;
    pHeader->Revision = s_RuntimeStateRevision;
    pHeader->VariableCount = ARRAY_SIZE(s_TpmRuntimeVariables);

    const char *pRuntimeState = (const char *)(pHeader + 1);

    for (uint32_t i = 0; i < ARRAY_SIZE(s_TpmRuntimeVariables); i++)
    {
        memcpy(pRuntimeState, s_TpmRuntimeVariables[i].pbRuntimeVariable, s_TpmRuntimeVariables[i].cbVariableSize);

        pRuntimeState += s_TpmRuntimeVariables[i].cbVariableSize;
    }

    *pBufferSize = requiredSize;

    return 0;
}

// Returns:
// - 0 on success
// - 1 for invalid arg
// - 2 for size mismatch
// - 3 for format validation error
int INJECTED_ApplyRuntimeState(
    const void *pRuntimeStateBuffer,
    uint32_t runtimeStateBufferSize)
{
    if (pRuntimeStateBuffer == NULL)
    {
        return 1;
    }

    uint32_t requiredSize = GetRuntimeStateSize();

    if (runtimeStateBufferSize != requiredSize)
    {
        return 2;
    }

    PTPM_RUNTIME_STATE_HEADER pHeader = (PTPM_RUNTIME_STATE_HEADER)pRuntimeStateBuffer;

    if (pHeader->HeaderMagic64 != s_RuntimeStateHeaderMagic ||
        pHeader->Revision != s_RuntimeStateRevision ||
        pHeader->VariableCount != ARRAY_SIZE(s_TpmRuntimeVariables))
    {
        return 3;
    }

    char *pRuntimeState = (char *)(pHeader + 1);

    for (uint32_t i = 0; i < ARRAY_SIZE(s_TpmRuntimeVariables); i++)
    {
        memcpy(s_TpmRuntimeVariables[i].pbRuntimeVariable, pRuntimeState, s_TpmRuntimeVariables[i].cbVariableSize);

        pRuntimeState += s_TpmRuntimeVariables[i].cbVariableSize;
    }

    return 0;
}
