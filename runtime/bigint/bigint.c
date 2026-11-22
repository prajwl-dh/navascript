/* Bigint helpers: checked integer division, modulo and exponentiation.
 * Nava stores bigints as signed 64-bit integers today. `+`, `-` and `*`
 * follow LLVM i64 semantics (they wrap on overflow); these helpers diagnose
 * zero divisors, negative exponents and overflowing powers. */
#include "nava_runtime.h"

#include <stdint.h>

int64_t nava_bigint_div(int64_t lhs, int64_t rhs) {
    if (rhs == 0) {
        nava_runtime_error("division by zero");
    }
    if (lhs == INT64_MIN && rhs == -1) {
        nava_runtime_error("bigint overflow");
    }
    return lhs / rhs;
}

int64_t nava_bigint_mod(int64_t lhs, int64_t rhs) {
    if (rhs == 0) {
        nava_runtime_error("division by zero");
    }
    if (lhs == INT64_MIN && rhs == -1) {
        return 0;
    }
    return lhs % rhs;
}

int64_t nava_bigint_pow(int64_t base, int64_t exponent) {
    if (exponent < 0) {
        nava_runtime_error("negative bigint exponent");
    }
    __int128 acc = 1;
    __int128 factor = base;
    while (exponent > 0) {
        if (exponent & 1) {
            acc *= factor;
        }
        exponent >>= 1;
        if (exponent > 0) {
            factor *= factor;
        }
        if (acc > INT64_MAX || acc < INT64_MIN || factor > INT64_MAX || factor < INT64_MIN) {
            nava_runtime_error("bigint overflow");
        }
    }
    return (int64_t)acc;
}
