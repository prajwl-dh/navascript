/* Nava runtime ABI — see runtime/README.md. */
#ifndef NAVA_RUNTIME_H
#define NAVA_RUNTIME_H

#include <stddef.h>
#include <stdint.h>

/* ------------------------------ arena ----------------------------------- */
void *nava_alloc(size_t bytes);
void nava_runtime_error(const char *message);

/* ------------------------------ numbers --------------------------------- */
void nava_format_number(double value, char *buffer, size_t capacity);
char *nava_number_to_string(double value);

/* ------------------------------ bigints ---------------------------------- */
int64_t nava_bigint_div(int64_t lhs, int64_t rhs);
int64_t nava_bigint_mod(int64_t lhs, int64_t rhs);
int64_t nava_bigint_pow(int64_t base, int64_t exponent);

/* ------------------------------ strings --------------------------------- */
char *nava_string_concat(const char *left, const char *right);
int nava_string_equals(const char *left, const char *right);
int64_t nava_string_length(const char *string);
char *nava_string_char_at(const char *string, int64_t index);

/* ------------------------------ arrays ---------------------------------- */
typedef struct {
    int64_t length;
    void *slots; /* length * 8 bytes */
} NavaArray;

NavaArray *nava_array_new(int64_t length);
void nava_array_set_f64(NavaArray *array, int64_t index, double value);
double nava_array_get_f64(NavaArray *array, int64_t index);
void nava_array_set_i64(NavaArray *array, int64_t index, int64_t value);
int64_t nava_array_get_i64(NavaArray *array, int64_t index);
void nava_array_set_bool(NavaArray *array, int64_t index, int value);
int nava_array_get_bool(NavaArray *array, int64_t index);
void nava_array_set_ptr(NavaArray *array, int64_t index, void *value);
void *nava_array_get_ptr(NavaArray *array, int64_t index);
int64_t nava_array_length(NavaArray *array);

/* ------------------------------ objects --------------------------------- */
typedef struct {
    int64_t count;
    const char **names; /* metadata for future reflection */
    void *slots;        /* count * 8 bytes */
} NavaObject;

NavaObject *nava_object_new(int64_t count, const char *names);
void nava_object_set_f64(NavaObject *object, int64_t index, double value);
double nava_object_get_f64(NavaObject *object, int64_t index);
void nava_object_set_i64(NavaObject *object, int64_t index, int64_t value);
int64_t nava_object_get_i64(NavaObject *object, int64_t index);
void nava_object_set_bool(NavaObject *object, int64_t index, int value);
int nava_object_get_bool(NavaObject *object, int64_t index);
void nava_object_set_ptr(NavaObject *object, int64_t index, void *value);
void *nava_object_get_ptr(NavaObject *object, int64_t index);

/* ---------------------------- console.log ------------------------------- */
void nava_console_log_f64(double value);
void nava_console_log_i64(int64_t value);
void nava_console_log_bool(int value);
void nava_console_log_str(const char *value);

#endif
