/* Objects: named fields stored in 8-byte slots; field order comes from the
 * compiler's inferred type (sorted by name). */
#include "nava_runtime.h"

#include <string.h>

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
