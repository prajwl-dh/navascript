/* Objects: named fields stored in 8-byte slots; field order comes from the
 * compiler's inferred type (sorted by name). */
#include "navascript_runtime.h"

#include <string.h>

NavaScriptObject *navascript_object_new(int64_t count, const char *names) {
    NavaScriptObject *object = (NavaScriptObject *)navascript_alloc(sizeof(NavaScriptObject));
    object->count = count;
    object->names = (const char **)navascript_alloc((size_t)count * sizeof(const char *));
    const char *cursor = names;
    for (int64_t i = 0; i < count; ++i) {
        object->names[i] = cursor;
        cursor += strlen(cursor) + 1;
    }
    object->slots = navascript_alloc((size_t)count * 8);
    return object;
}

static void check_field(NavaScriptObject *object, int64_t index) {
    if (index < 0 || index >= object->count) {
        navascript_runtime_error("unknown object field");
    }
}

void navascript_object_set_f64(NavaScriptObject *object, int64_t index, double value) {
    check_field(object, index);
    ((double *)object->slots)[index] = value;
}

double navascript_object_get_f64(NavaScriptObject *object, int64_t index) {
    check_field(object, index);
    return ((double *)object->slots)[index];
}

void navascript_object_set_i64(NavaScriptObject *object, int64_t index, int64_t value) {
    check_field(object, index);
    ((int64_t *)object->slots)[index] = value;
}

int64_t navascript_object_get_i64(NavaScriptObject *object, int64_t index) {
    check_field(object, index);
    return ((int64_t *)object->slots)[index];
}

void navascript_object_set_bool(NavaScriptObject *object, int64_t index, int value) {
    check_field(object, index);
    ((unsigned char *)object->slots)[index * 8] = (unsigned char)(value != 0);
}

int navascript_object_get_bool(NavaScriptObject *object, int64_t index) {
    check_field(object, index);
    return ((unsigned char *)object->slots)[index * 8] != 0;
}

void navascript_object_set_ptr(NavaScriptObject *object, int64_t index, void *value) {
    check_field(object, index);
    ((void **)object->slots)[index] = value;
}

void *navascript_object_get_ptr(NavaScriptObject *object, int64_t index) {
    check_field(object, index);
    return ((void **)object->slots)[index];
}
