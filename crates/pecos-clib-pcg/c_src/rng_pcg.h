/*
 * PCG Random Number Generation for C.
 *
 * Copyright 2014 Melissa O'Neill <oneill@pcg-random.org>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 * For additional information about the PCG random number generation scheme,
 * including its license and other licensing options, visit
 *
 *       http://www.pcg-random.org
 */

#pragma once

#include <stdint.h>

#if __cplusplus
extern "C" {
#endif

// Thread-safe RNG state structure
typedef struct pcg_state_setseq_64 {
    uint64_t state;
    uint64_t inc;
} pcg32_random_t;

// Thread-safe versions that take explicit state
uint32_t pcg32_random_r(pcg32_random_t* rng);
uint32_t pcg32_boundedrand_r(pcg32_random_t* rng, uint32_t bound);
void pcg32_srandom_r(pcg32_random_t* rng, uint64_t initstate, uint64_t initseq);

// Global state versions (for compatibility)
uint32_t pcg32_random();
uint32_t pcg32_boundedrand(uint32_t bound);
double pcg32_frandom();
void pcg32_srandom(uint64_t seq);

#if __cplusplus
}
#endif
