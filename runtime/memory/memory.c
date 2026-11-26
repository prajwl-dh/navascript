/* Growable program-lifetime bump arena (ADR-0005).
 *
 * Chunks are never reclaimed during execution, but the arena grows on demand
 * instead of failing at a fixed 16 MiB. Pointers remain valid for the whole
 * process because chunks are linked, never moved or freed.
 */
#include "navascript_runtime.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define NAVASCRIPT_INITIAL_CHUNK (1u * 1024u * 1024u)
#define NAVASCRIPT_MAX_CHUNK (64u * 1024u * 1024u)

typedef struct NavaScriptChunk {
    struct NavaScriptChunk *next;
    size_t capacity;
    size_t used;
    _Alignas(8) unsigned char data[];
} NavaScriptChunk;

static NavaScriptChunk *navascript_chunks = NULL;
static size_t navascript_next_capacity = NAVASCRIPT_INITIAL_CHUNK;

static void navascript_add_chunk(size_t minimum) {
    size_t capacity = navascript_next_capacity;
    if (capacity < minimum) {
        capacity = minimum;
    }
    if (capacity > NAVASCRIPT_MAX_CHUNK) {
        capacity = minimum;
    }
    NavaScriptChunk *chunk = (NavaScriptChunk *)malloc(sizeof(NavaScriptChunk) + capacity);
    if (chunk == NULL) {
        fputs("navascript: cannot grow runtime arena\n", stderr);
        exit(1);
    }
    chunk->next = navascript_chunks;
    chunk->capacity = capacity;
    chunk->used = 0;
    navascript_chunks = chunk;
    if (navascript_next_capacity < NAVASCRIPT_MAX_CHUNK) {
        navascript_next_capacity *= 2;
    }
}

void *navascript_alloc(size_t bytes) {
    size_t aligned = (bytes + 7u) & ~(size_t)7u;
    if (aligned == 0) {
        aligned = 8;
    }
    if (navascript_chunks == NULL
        || navascript_chunks->used + aligned > navascript_chunks->capacity) {
        navascript_add_chunk(aligned);
    }
    NavaScriptChunk *chunk = navascript_chunks;
    void *pointer = chunk->data + chunk->used;
    chunk->used += aligned;
    memset(pointer, 0, aligned);
    return pointer;
}

void navascript_runtime_error(const char *message) {
    fprintf(stderr, "navascript: runtime error: %s\n", message);
    exit(1);
}
