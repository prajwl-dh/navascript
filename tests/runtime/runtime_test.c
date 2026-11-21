/* Unit tests for the C runtime. Compiled and run by tests/runtime/runtime.rs. */
#define NAVA_NO_MAIN
#include "../../runtime/nava_runtime.c"

#include <assert.h>

int nava_runtime_test_main(void) {
    void *a = nava_alloc(1);
    void *b = nava_alloc(7);
    assert(((uintptr_t)a % 8) == 0);
    assert(((uintptr_t)b % 8) == 0);
    assert((unsigned char *)b == (unsigned char *)a + 8);

    char *hello = nava_string_concat("hello ", "world");
    assert(strcmp(hello, "hello world") == 0);
    assert(nava_string_length(hello) == 11);
    assert(nava_string_equals(hello, "hello world"));
    assert(!nava_string_equals(hello, "hello"));
    char *first = nava_string_char_at(hello, 0);
    assert(strcmp(first, "h") == 0);
    char *as_text = nava_number_to_string(42.0);
    assert(strcmp(as_text, "42") == 0);

    NavaArray *numbers = nava_array_new(2);
    nava_array_set_f64(numbers, 0, 3.5);
    nava_array_set_f64(numbers, 1, 8.0);
    assert(nava_array_get_f64(numbers, 0) == 3.5);
    assert(nava_array_length(numbers) == 2);

    NavaArray *flags = nava_array_new(2);
    nava_array_set_bool(flags, 1, 1);
    assert(nava_array_get_bool(flags, 0) == 0);
    assert(nava_array_get_bool(flags, 1) == 1);
    assert(nava_array_get_f64(numbers, 1) == 8.0); /* bool slots don't corrupt neighbours */

    NavaObject *object = nava_object_new(3, "n\0name\0ok\0");
    nava_object_set_f64(object, 0, 2.0);
    nava_object_set_ptr(object, 1, hello);
    nava_object_set_bool(object, 2, 1);
    assert(nava_object_get_f64(object, 0) == 2.0);
    assert(strcmp((const char *)nava_object_get_ptr(object, 1), "hello world") == 0);
    assert(nava_object_get_bool(object, 2) == 1);

    puts("runtime unit tests passed");
    return 0;
}

#ifndef NAVA_SKIP_TEST_MAIN
int main(void) {
    return nava_runtime_test_main();
}
#endif
