/* Program-lifetime bump arena (ADR-0005). */
#include "nava_runtime.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define NAVA_ARENA_SIZE (16u * 1024u * 1024u)

_Alignas(8) static unsigned char nava_arena[NAVA_ARENA_SIZE];
static size_t nava_arena_used = 0;

void *nava_alloc(size_t bytes) {
    size_t aligned = (bytes + 7u) & ~(size_t)7u;
    if (aligned == 0) {
        aligned = 8;
    }
    if (nava_arena_used + aligned > NAVA_ARENA_SIZE) {
        fputs("nava: runtime arena exhausted\n", stderr);
        exit(1);
    }
    void *pointer = nava_arena + nava_arena_used;
    nava_arena_used += aligned;
    memset(pointer, 0, aligned);
    return pointer;
}

void nava_runtime_error(const char *message) {
    fprintf(stderr, "nava: runtime error: %s\n", message);
    exit(1);
}
