/* Arrays: fixed-length sequences of 8-byte slots with typed accessors. */
#include "navascript_runtime.h"

#include <stdio.h>

NavaScriptArray *navascript_array_new(int64_t length) {
    if (length < 0) {
        navascript_runtime_error("negative array length");
    }
    NavaScriptArray *array = (NavaScriptArray *)navascript_alloc(sizeof(NavaScriptArray));
    array->length = length;
    array->slots = navascript_alloc((size_t)length * 8);
    return array;
}

static void check_index(NavaScriptArray *array, int64_t index) {
    if (index < 0 || index >= array->length) {
        navascript_runtime_error("array index out of bounds");
    }
}

void navascript_array_set_f64(NavaScriptArray *array, int64_t index, double value) {
    check_index(array, index);
    ((double *)array->slots)[index] = value;
}

double navascript_array_get_f64(NavaScriptArray *array, int64_t index) {
    check_index(array, index);
    return ((double *)array->slots)[index];
}

void navascript_array_set_i64(NavaScriptArray *array, int64_t index, int64_t value) {
    check_index(array, index);
    ((int64_t *)array->slots)[index] = value;
}

int64_t navascript_array_get_i64(NavaScriptArray *array, int64_t index) {
    check_index(array, index);
    return ((int64_t *)array->slots)[index];
}

void navascript_array_set_bool(NavaScriptArray *array, int64_t index, int value) {
    check_index(array, index);
    ((unsigned char *)array->slots)[index * 8] = (unsigned char)(value != 0);
}

int navascript_array_get_bool(NavaScriptArray *array, int64_t index) {
    check_index(array, index);
    return ((unsigned char *)array->slots)[index * 8] != 0;
}

void navascript_array_set_ptr(NavaScriptArray *array, int64_t index, void *value) {
    check_index(array, index);
    ((void **)array->slots)[index] = value;
}

void *navascript_array_get_ptr(NavaScriptArray *array, int64_t index) {
    check_index(array, index);
    return ((void **)array->slots)[index];
}

int64_t navascript_array_length(NavaScriptArray *array) {
    return array->length;
}
