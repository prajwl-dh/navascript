/* Arrays: fixed-length sequences of 8-byte slots with typed accessors. */
#include "nava_runtime.h"

#include <stdio.h>

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
