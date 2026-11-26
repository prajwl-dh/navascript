/* Unit tests for the C runtime. Compiled and run by tests/runtime/runtime.rs. */
#include "navascript_runtime.h"

#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <string.h>

int navascript_runtime_test_main(void) {
    void *a = navascript_alloc(1);
    void *b = navascript_alloc(7);
    assert(((uintptr_t)a % 8) == 0);
    assert(((uintptr_t)b % 8) == 0);
    assert((uintptr_t)b >= (uintptr_t)a + 8);

    char *hello = navascript_string_concat("hello ", "world");
    assert(strcmp(hello, "hello world") == 0);
    assert(navascript_string_length(hello) == 11);
    assert(navascript_string_equals(hello, "hello world"));
    assert(!navascript_string_equals(hello, "hello"));
    char *first = navascript_string_char_at(hello, 0);
    assert(strcmp(first, "h") == 0);
    char *as_text = navascript_number_to_string(42.0);
    assert(strcmp(as_text, "42") == 0);
    assert(strcmp(navascript_number_to_string(NAN), "NaN") == 0);
    assert(strcmp(navascript_number_to_string(INFINITY), "Infinity") == 0);
    assert(strcmp(navascript_number_to_string(-INFINITY), "-Infinity") == 0);

    /* The arena grows beyond its initial chunk without moving earlier data. */
    void *large = navascript_alloc(2u * 1024u * 1024u);
    assert(((uintptr_t)large % 8) == 0);
    assert(strcmp(hello, "hello world") == 0);

    NavaScriptArray *numbers = navascript_array_new(2);
    navascript_array_set_f64(numbers, 0, 3.5);
    navascript_array_set_f64(numbers, 1, 8.0);
    assert(navascript_array_get_f64(numbers, 0) == 3.5);
    assert(navascript_array_length(numbers) == 2);

    NavaScriptArray *flags = navascript_array_new(2);
    navascript_array_set_bool(flags, 1, 1);
    assert(navascript_array_get_bool(flags, 0) == 0);
    assert(navascript_array_get_bool(flags, 1) == 1);
    assert(navascript_array_get_f64(numbers, 1) == 8.0); /* bool slots don't corrupt neighbours */

    NavaScriptObject *object = navascript_object_new(3, "n\0name\0ok\0");
    navascript_object_set_f64(object, 0, 2.0);
    navascript_object_set_ptr(object, 1, hello);
    navascript_object_set_bool(object, 2, 1);
    assert(navascript_object_get_f64(object, 0) == 2.0);
    assert(strcmp((const char *)navascript_object_get_ptr(object, 1), "hello world") == 0);
    assert(navascript_object_get_bool(object, 2) == 1);

    NavaScriptArray *bigints = navascript_array_new(1);
    navascript_array_set_i64(bigints, 0, 123);
    assert(navascript_array_get_i64(bigints, 0) == 123);
    NavaScriptObject *boxed = navascript_object_new(1, "v\0");
    navascript_object_set_i64(boxed, 0, -7);
    assert(navascript_object_get_i64(boxed, 0) == -7);
    assert(navascript_bigint_div(9, 2) == 4);
    assert(navascript_bigint_mod(9, 2) == 1);
    assert(navascript_bigint_pow(2, 10) == 1024);
    assert(navascript_bigint_pow(-3, 3) == -27);

    puts("runtime unit tests passed");
    return 0;
}

#ifndef NAVASCRIPT_SKIP_TEST_MAIN
int main(void) {
    return navascript_runtime_test_main();
}
#endif
