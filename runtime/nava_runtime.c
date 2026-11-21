/* Nava native runtime (Phase 5).
 *
 * Memory strategy (ADR-0005): one program-lifetime bump arena. Runtime objects
 * are never individually freed; the process releases the arena on exit. */
#include <ctype.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* macOS entry point bridges into Nava's synthetic `main` function. */
#ifndef NAVA_NO_MAIN
extern void nava_main(void);

int main(void) {
    nava_main();
    return 0;
}
#endif

/* ------------------------------ arena ----------------------------------- */

#define NAVA_ARENA_SIZE (16u * 1024u * 1024u)

_Alignas(8) static unsigned char nava_arena[NAVA_ARENA_SIZE];
static size_t nava_arena_used = 0;

void *nava_alloc(size_t bytes) {
    size_t aligned = (bytes + 7u) & ~(size_t)7u;
    if (aligned == 0) {
        aligned = 8;
    }
    if (nava_arena_used + aligned > NAVA_ARENA_SIZE) {
        fputs("nava: runtime arena exhausted\n", stderr);
        exit(1);
    }
    void *pointer = nava_arena + nava_arena_used;
    nava_arena_used += aligned;
    memset(pointer, 0, aligned);
    return pointer;
}

void nava_runtime_error(const char *message) {
    fprintf(stderr, "nava: runtime error: %s\n", message);
    exit(1);
}

void nava_format_number(double value, char *buffer, size_t capacity);

/* ------------------------------ strings --------------------------------- */
/* Strings are NUL-terminated char buffers in the arena. String literals are
 * emitted as LLVM global constants and passed directly as char pointers. */

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

/* ------------------------------ arrays ---------------------------------- */

typedef struct {
    int64_t length;
    void *slots; /* length * 8 bytes */
} NavaArray;

NavaArray *nava_array_new(int64_t length) {
    if (length < 0) {
        nava_runtime_error("negative array length");
    }
    NavaArray *array = (NavaArray *)nava_alloc(sizeof(NavaArray));
    array->length = length;
    array->slots = nava_alloc((size_t)length * 8);
    return array;
}

static void check_index(NavaArray *array, int64_t index) {
    if (index < 0 || index >= array->length) {
        nava_runtime_error("array index out of bounds");
    }
}

void nava_array_set_f64(NavaArray *array, int64_t index, double value) {
    check_index(array, index);
    ((double *)array->slots)[index] = value;
}

double nava_array_get_f64(NavaArray *array, int64_t index) {
    check_index(array, index);
    return ((double *)array->slots)[index];
}

void nava_array_set_bool(NavaArray *array, int64_t index, int value) {
    check_index(array, index);
    ((unsigned char *)array->slots)[index * 8] = (unsigned char)(value != 0);
}

int nava_array_get_bool(NavaArray *array, int64_t index) {
    check_index(array, index);
    return ((unsigned char *)array->slots)[index * 8] != 0;
}

void nava_array_set_ptr(NavaArray *array, int64_t index, void *value) {
    check_index(array, index);
    ((void **)array->slots)[index] = value;
}

void *nava_array_get_ptr(NavaArray *array, int64_t index) {
    check_index(array, index);
    return ((void **)array->slots)[index];
}

int64_t nava_array_length(NavaArray *array) {
    return array->length;
}

/* ------------------------------ objects --------------------------------- */

typedef struct {
    int64_t count;
    const char **names; /* NUL-separated name buffer, one per field */
    void *slots;        /* count * 8 bytes */
} NavaObject;

NavaObject *nava_object_new(int64_t count, const char *names) {
    NavaObject *object = (NavaObject *)nava_alloc(sizeof(NavaObject));
    object->count = count;
    object->names = (const char **)nava_alloc((size_t)count * sizeof(const char *));
    const char *cursor = names;
    for (int64_t i = 0; i < count; ++i) {
        object->names[i] = cursor;
        cursor += strlen(cursor) + 1;
    }
    object->slots = nava_alloc((size_t)count * 8);
    return object;
}

static void check_field(NavaObject *object, int64_t index) {
    if (index < 0 || index >= object->count) {
        nava_runtime_error("unknown object field");
    }
}

void nava_object_set_f64(NavaObject *object, int64_t index, double value) {
    check_field(object, index);
    ((double *)object->slots)[index] = value;
}

double nava_object_get_f64(NavaObject *object, int64_t index) {
    check_field(object, index);
    return ((double *)object->slots)[index];
}

void nava_object_set_bool(NavaObject *object, int64_t index, int value) {
    check_field(object, index);
    ((unsigned char *)object->slots)[index * 8] = (unsigned char)(value != 0);
}

int nava_object_get_bool(NavaObject *object, int64_t index) {
    check_field(object, index);
    return ((unsigned char *)object->slots)[index * 8] != 0;
}

void nava_object_set_ptr(NavaObject *object, int64_t index, void *value) {
    check_field(object, index);
    ((void **)object->slots)[index] = value;
}

void *nava_object_get_ptr(NavaObject *object, int64_t index) {
    check_field(object, index);
    return ((void **)object->slots)[index];
}

/* ---------------------------- console.log ------------------------------- */

void nava_console_log_bool(int value) {
    printf("%s\n", value ? "true" : "false");
}

void nava_console_log_str(const char *value) {
    printf("%s\n", value ? value : "null");
}

/* Format a number the way TypeScript's Number.toString mostly does. */
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

void nava_console_log_f64(double value) {
    char buffer[64];
    nava_format_number(value, buffer, sizeof(buffer));
    printf("%s\n", buffer);
}
