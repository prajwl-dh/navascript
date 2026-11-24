/* console.log overloads called from generated code. */
#include "navascript_runtime.h"

#include <stdio.h>

void navascript_console_log_f64(double value) {
    char buffer[64];
    navascript_format_number(value, buffer, sizeof(buffer));
    printf("%s\n", buffer);
}

void navascript_console_log_i64(int64_t value) {
    printf("%lld\n", (long long)value);
}

void navascript_console_log_bool(int value) {
    printf("%s\n", value ? "true" : "false");
}

void navascript_console_log_str(const char *value) {
    printf("%s\n", value ? value : "null");
}
