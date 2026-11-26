/* Standard-library builtins needed by a compiler written in NavaScript:
 * file I/O, process arguments, string helpers and array growth.
 * All buffers live in the program-lifetime arena (ADR-0005).
 */
#include "navascript_runtime.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int navascript_argc = 0;
static char **navascript_argv = NULL;
static NavaScriptArray *navascript_process_args = NULL;

void navascript_init_io(int argc, char **argv) {
    navascript_argc = argc;
    navascript_argv = argv;
}

char *navascript_read_file(const char *path) {
    if (path == NULL) {
        navascript_runtime_error("read_file: null path");
    }
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        navascript_runtime_error("read_file: cannot open file");
    }
    if (fseek(file, 0, SEEK_END) != 0) {
        fclose(file);
        navascript_runtime_error("read_file: cannot seek");
    }
    long length = ftell(file);
    if (length < 0) {
        fclose(file);
        navascript_runtime_error("read_file: cannot determine size");
    }
    rewind(file);
    char *buffer = (char *)navascript_alloc((size_t)length + 1);
    size_t read = fread(buffer, 1, (size_t)length, file);
    fclose(file);
    buffer[read] = '\0';
    return buffer;
}

void navascript_write_file(const char *path, const char *contents) {
    if (path == NULL || contents == NULL) {
        navascript_runtime_error("write_file: null argument");
    }
    FILE *file = fopen(path, "wb");
    if (file == NULL) {
        navascript_runtime_error("write_file: cannot open file");
    }
    size_t length = strlen(contents);
    if (fwrite(contents, 1, length, file) != length) {
        fclose(file);
        navascript_runtime_error("write_file: write failed");
    }
    fclose(file);
}

NavaScriptArray *navascript_get_argv(void) {
    if (navascript_process_args == NULL) {
        int count = navascript_argc > 0 ? navascript_argc - 1 : 0;
        navascript_process_args = navascript_array_new(count);
        for (int index = 0; index < count; ++index) {
            navascript_array_set_ptr(
                navascript_process_args, index, navascript_argv[index + 1]);
        }
    }
    return navascript_process_args;
}

char *navascript_str_slice(const char *value, double start_value, double end_value) {
    if (value == NULL) {
        navascript_runtime_error("str_slice: null string");
    }
    int64_t length = (int64_t)strlen(value);
    int64_t start = navascript_number_index(start_value);
    int64_t end = navascript_number_index(end_value);
    if (start < 0) {
        start += length;
    }
    if (end < 0) {
        end += length;
    }
    if (start < 0) {
        start = 0;
    }
    if (end > length) {
        end = length;
    }
    if (end < start) {
        end = start;
    }
    size_t count = (size_t)(end - start);
    char *result = (char *)navascript_alloc(count + 1);
    memcpy(result, value + start, count);
    result[count] = '\0';
    return result;
}

double navascript_str_index_of(const char *value, const char *needle) {
    if (value == NULL || needle == NULL) {
        navascript_runtime_error("str_index_of: null argument");
    }
    const char *found = strstr(value, needle);
    return (double)(found == NULL ? -1 : (int64_t)(found - value));
}

double navascript_str_char_code_at(const char *value, double index_value) {
    if (value == NULL) {
        navascript_runtime_error("str_char_code_at: null string");
    }
    int64_t index = navascript_number_index(index_value);
    if (index >= (int64_t)strlen(value)) {
        return -1.0;
    }
    return (double)(unsigned char)value[index];
}

static NavaScriptArray *push_copy(NavaScriptArray *array, int64_t count) {
    NavaScriptArray *result = navascript_array_new(count + 1);
    for (int64_t index = 0; index < count; ++index) {
        double number = navascript_array_get_f64(array, index);
        navascript_array_set_f64(result, index, number);
    }
    return result;
}

double navascript_str_compare(const char *left, const char *right) {
    if (left == NULL || right == NULL) {
        navascript_runtime_error("str_compare: null argument");
    }
    int compared = strcmp(left, right);
    return compared < 0 ? -1.0 : (compared > 0 ? 1.0 : 0.0);
}

int64_t navascript_number_index(double value) {
    if (!isfinite(value) || value != trunc(value) || value < 0.0
        || value > (double)INT64_MAX) {
        navascript_runtime_error("index must be a non-negative integer");
    }
    return (int64_t)value;
}

NavaScriptArray *navascript_array_push_number(NavaScriptArray *array, double value) {
    if (array == NULL) {
        navascript_runtime_error("array_push_number: null array");
    }
    int64_t count = navascript_array_length(array);
    NavaScriptArray *result = push_copy(array, count);
    navascript_array_set_f64(result, count, value);
    return result;
}

NavaScriptArray *navascript_array_push_string(NavaScriptArray *array, const char *value) {
    if (array == NULL || value == NULL) {
        navascript_runtime_error("array_push_string: null argument");
    }
    int64_t count = navascript_array_length(array);
    NavaScriptArray *result = navascript_array_new(count + 1);
    for (int64_t index = 0; index < count; ++index) {
        navascript_array_set_ptr(result, index, navascript_array_get_ptr(array, index));
    }
    navascript_array_set_ptr(result, count, (void *)value);
    return result;
}

/* Tagged values for `any` and mixed unions (Phase 8.1). The tag is checked on
 * every unbox, so a wrong cast is a runtime error rather than a reinterpreted
 * pointer. */
NavaScriptValue *navascript_box_number(double value) {
    NavaScriptValue *boxed = (NavaScriptValue *)navascript_alloc(sizeof(NavaScriptValue));
    boxed->tag = 0;
    boxed->payload.number = value;
    return boxed;
}

NavaScriptValue *navascript_box_bigint(int64_t value) {
    NavaScriptValue *boxed = (NavaScriptValue *)navascript_alloc(sizeof(NavaScriptValue));
    boxed->tag = 1;
    boxed->payload.bigint = value;
    return boxed;
}

NavaScriptValue *navascript_box_bool(int32_t value) {
    NavaScriptValue *boxed = (NavaScriptValue *)navascript_alloc(sizeof(NavaScriptValue));
    boxed->tag = 2;
    boxed->payload.boolean = value;
    return boxed;
}

NavaScriptValue *navascript_box_string(const char *value) {
    NavaScriptValue *boxed = (NavaScriptValue *)navascript_alloc(sizeof(NavaScriptValue));
    boxed->tag = 3;
    boxed->payload.ref = (void *)value;
    return boxed;
}

NavaScriptValue *navascript_box_null(void) {
    NavaScriptValue *boxed = (NavaScriptValue *)navascript_alloc(sizeof(NavaScriptValue));
    boxed->tag = 5;
    boxed->payload.ref = NULL;
    return boxed;
}

static NavaScriptValue *expect_tag(NavaScriptValue *value, int64_t tag, const char *name) {
    if (value == NULL || value->tag != tag) {
        navascript_runtime_error(name);
    }
    return value;
}

double navascript_as_number(NavaScriptValue *value) {
    return expect_tag(value, 0, "as_number: value is not a number")->payload.number;
}

int64_t navascript_as_bigint(NavaScriptValue *value) {
    return expect_tag(value, 1, "as_bigint: value is not a bigint")->payload.bigint;
}

int32_t navascript_as_bool(NavaScriptValue *value) {
    return expect_tag(value, 2, "as_bool: value is not a boolean")->payload.boolean;
}

const char *navascript_as_string(NavaScriptValue *value) {
    return (const char *)expect_tag(value, 3, "as_string: value is not a string")->payload.ref;
}

double navascript_tag_of(NavaScriptValue *value) {
    if (value == NULL) {
        navascript_runtime_error("tag_of: null value");
    }
    return (double)value->tag;
}
