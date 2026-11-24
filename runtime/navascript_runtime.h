/* NavaScript runtime ABI — see runtime/README.md. */
#ifndef NAVASCRIPT_RUNTIME_H
#define NAVASCRIPT_RUNTIME_H

#include <stddef.h>
#include <stdint.h>

/* ------------------------------ arena ----------------------------------- */
void *navascript_alloc(size_t bytes);
void navascript_runtime_error(const char *message);

/* ------------------------------ numbers --------------------------------- */
void navascript_format_number(double value, char *buffer, size_t capacity);
char *navascript_number_to_string(double value);

/* ------------------------------ bigints ---------------------------------- */
int64_t navascript_bigint_div(int64_t lhs, int64_t rhs);
int64_t navascript_bigint_mod(int64_t lhs, int64_t rhs);
int64_t navascript_bigint_pow(int64_t base, int64_t exponent);

/* ------------------------------ strings --------------------------------- */
char *navascript_string_concat(const char *left, const char *right);
int navascript_string_equals(const char *left, const char *right);
int64_t navascript_string_length(const char *string);
char *navascript_string_char_at(const char *string, int64_t index);

/* ------------------------------ arrays ---------------------------------- */
typedef struct {
    int64_t length;
    void *slots; /* length * 8 bytes */
} NavaScriptArray;

NavaScriptArray *navascript_array_new(int64_t length);
void navascript_array_set_f64(NavaScriptArray *array, int64_t index, double value);
double navascript_array_get_f64(NavaScriptArray *array, int64_t index);
void navascript_array_set_i64(NavaScriptArray *array, int64_t index, int64_t value);
int64_t navascript_array_get_i64(NavaScriptArray *array, int64_t index);
void navascript_array_set_bool(NavaScriptArray *array, int64_t index, int value);
int navascript_array_get_bool(NavaScriptArray *array, int64_t index);
void navascript_array_set_ptr(NavaScriptArray *array, int64_t index, void *value);
void *navascript_array_get_ptr(NavaScriptArray *array, int64_t index);
int64_t navascript_array_length(NavaScriptArray *array);

/* ------------------------------ objects --------------------------------- */
typedef struct {
    int64_t count;
    const char **names; /* metadata for future reflection */
    void *slots;        /* count * 8 bytes */
} NavaScriptObject;

NavaScriptObject *navascript_object_new(int64_t count, const char *names);
void navascript_object_set_f64(NavaScriptObject *object, int64_t index, double value);
double navascript_object_get_f64(NavaScriptObject *object, int64_t index);
void navascript_object_set_i64(NavaScriptObject *object, int64_t index, int64_t value);
int64_t navascript_object_get_i64(NavaScriptObject *object, int64_t index);
void navascript_object_set_bool(NavaScriptObject *object, int64_t index, int value);
int navascript_object_get_bool(NavaScriptObject *object, int64_t index);
void navascript_object_set_ptr(NavaScriptObject *object, int64_t index, void *value);
void *navascript_object_get_ptr(NavaScriptObject *object, int64_t index);

/* ---------------------------- console.log ------------------------------- */
void navascript_console_log_f64(double value);
void navascript_console_log_i64(int64_t value);
void navascript_console_log_bool(int value);
void navascript_console_log_str(const char *value);

#endif
