/* String values are NUL-terminated arena buffers; literals are LLVM globals. */
#include "nava_runtime.h"

#include <ctype.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void nava_format_number(double value, char *buffer, size_t capacity) {
    char general[64];

    if (value == (double)(long long)value && value >= -9e18 && value <= 9e18) {
        snprintf(buffer, capacity, "%lld", (long long)value);
        return;
    }

    snprintf(general, sizeof(general), "%.15g", value);
    char *exponent = strchr(general, 'e');
    if (exponent == NULL) {
        snprintf(buffer, capacity, "%s", general);
        return;
    }

    int power = atoi(exponent + 1);
    double magnitude = fabs(value);
    if (magnitude < 1e-6 || magnitude >= 1e21) {
        snprintf(buffer, capacity, "%s", general);
        return;
    }

    int digits = 0;
    for (char *cursor = general; cursor < exponent; ++cursor) {
        if (isdigit((unsigned char)*cursor)) {
            ++digits;
        }
    }
    int precision = digits - (power + 1);
    if (precision < 0) {
        precision = 0;
    }
    if (precision > 17) {
        precision = 17;
    }
    snprintf(buffer, capacity, "%.*f", precision, value);
    size_t length = strlen(buffer);
    while (length > 0 && buffer[length - 1] == '0') {
        buffer[--length] = '\0';
    }
    if (length > 0 && buffer[length - 1] == '.') {
        buffer[--length] = '\0';
    }
}

char *nava_number_to_string(double value) {
    char buffer[64];
    nava_format_number(value, buffer, sizeof(buffer));
    size_t length = strlen(buffer);
    char *result = (char *)nava_alloc(length + 1);
    memcpy(result, buffer, length + 1);
    return result;
}

char *nava_string_concat(const char *left, const char *right) {
    size_t left_length = strlen(left);
    size_t right_length = strlen(right);
    char *result = (char *)nava_alloc(left_length + right_length + 1);
    memcpy(result, left, left_length);
    memcpy(result + left_length, right, right_length);
    result[left_length + right_length] = '\0';
    return result;
}

int nava_string_equals(const char *left, const char *right) {
    return strcmp(left, right) == 0;
}

int64_t nava_string_length(const char *string) {
    return (int64_t)strlen(string);
}

char *nava_string_char_at(const char *string, int64_t index) {
    size_t length = strlen(string);
    if (index < 0 || (size_t)index >= length) {
        nava_runtime_error("string index out of bounds");
    }
    char *result = (char *)nava_alloc(2);
    result[0] = string[index];
    result[1] = '\0';
    return result;
}
