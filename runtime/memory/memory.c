/* Program-lifetime bump arena (ADR-0005). */
#include "navascript_runtime.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define NAVASCRIPT_ARENA_SIZE (16u * 1024u * 1024u)

_Alignas(8) static unsigned char navascript_arena[NAVASCRIPT_ARENA_SIZE];
static size_t navascript_arena_used = 0;

void *navascript_alloc(size_t bytes) {
    size_t aligned = (bytes + 7u) & ~(size_t)7u;
    if (aligned == 0) {
        aligned = 8;
    }
    if (navascript_arena_used + aligned > NAVASCRIPT_ARENA_SIZE) {
        fputs("navascript: runtime arena exhausted\n", stderr);
        exit(1);
    }
    void *pointer = navascript_arena + navascript_arena_used;
    navascript_arena_used += aligned;
    memset(pointer, 0, aligned);
    return pointer;
}

void navascript_runtime_error(const char *message) {
    fprintf(stderr, "navascript: runtime error: %s\n", message);
    exit(1);
}
