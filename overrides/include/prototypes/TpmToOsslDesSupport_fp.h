/* Microsoft Reference Implementation for TPM 2.0
 *
 *  The copyright in this software is being made available under the BSD License,
 *  included below. This software may be subject to other third party and
 *  contributor rights, including patent rights, and no such rights are granted
 *  under this license.
 *
 *  Copyright (c) Microsoft Corporation
 *
 *  All rights reserved.
 *
 *  BSD License
 *
 *  Redistribution and use in source and binary forms, with or without modification,
 *  are permitted provided that the following conditions are met:
 *
 *  Redistributions of source code must retain the above copyright notice, this list
 *  of conditions and the following disclaimer.
 *
 *  Redistributions in binary form must reproduce the above copyright notice, this
 *  list of conditions and the following disclaimer in the documentation and/or
 *  other materials provided with the distribution.
 *
 *  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS ""AS IS""
 *  AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 *  IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 *  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR
 *  ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
 *  (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 *  LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
 *  ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 *  (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 *  SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/*(Auto-generated)
 *  Created by TpmPrototypes; Version 3.0 July 18, 2017
 *  Date: Apr  2, 2019  Time: 03:18:00PM
 */

#ifndef _TPM_TO_OSSL_DES_SUPPORT_FP_H_
#define _TPM_TO_OSSL_DES_SUPPORT_FP_H_

#if (defined SYM_LIB_OSSL) && ALG_TDES

//**Functions
//*** TDES_set_encyrpt_key()
// This function makes creation of a TDES key look like the creation of a key for
// any of the other OpenSSL block ciphers. It will create three key schedules,
// one for each of the DES keys. If there are only two keys, then the third schedule
// is a copy of the first.
void TDES_set_encrypt_key(
    const BYTE *key,
    UINT16 keySizeInBits,
    tpmKeyScheduleTDES *keySchedule);

//*** TDES_encyrpt()
// The TPM code uses one key schedule. For TDES, the schedule contains three
// schedules. OpenSSL wants the schedules referenced separately. This function
// does that.
void TDES_encrypt(
    const BYTE *in,
    BYTE *out,
    tpmKeyScheduleTDES *ks);

//*** TDES_decrypt()
// As with TDES_encypt() this function bridges between the TPM single schedule
// model and the OpenSSL three schedule model.
void TDES_decrypt(
    const BYTE *in,
    BYTE *out,
    tpmKeyScheduleTDES *ks);
#endif // SYM_LIB_OSSL

#endif // _TPM_TO_OSSL_DES_SUPPORT_FP_H_
