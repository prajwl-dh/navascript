/* console.log overloads called from generated code. */
#include "nava_runtime.h"

#include <stdio.h>

void nava_console_log_f64(double value) {
    char buffer[64];
    nava_format_number(value, buffer, sizeof(buffer));
    printf("%s\n", buffer);
}

void nava_console_log_i64(int64_t value) {
    printf("%lld\n", (long long)value);
}

void nava_console_log_bool(int value) {
    printf("%s\n", value ? "true" : "false");
}

void nava_console_log_str(const char *value) {
    printf("%s\n", value ? value : "null");
}
