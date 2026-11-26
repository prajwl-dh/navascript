/* Growable program-lifetime arena with conservative mark-sweep reclamation.
 *
 * Phase 8.1: every runtime allocation carries a small header, so the collector
 * can find block boundaries. Roots are found by scanning the live stack range,
 * callee-saved registers and the payload of every marked block. False positives
 * (a number that happens to look like a pointer) only retain garbage; a live
 * reference is found because LLVM must spill values it needs across the
 * `collect()` call, and callee-saved registers are scanned explicitly.
 */
#include "navascript_runtime.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define NAVASCRIPT_INITIAL_CHUNK (1u * 1024u * 1024u)
#define NAVASCRIPT_MAX_CHUNK (64u * 1024u * 1024u)
#define NAVASCRIPT_BLOCK_MAGIC UINT64_C(0x4e41564153435249) /* "NAVASCRI" */

typedef struct NavaScriptChunk {
    struct NavaScriptChunk *next;
    size_t capacity;
    size_t used;
    _Alignas(8) unsigned char data[];
} NavaScriptChunk;

typedef struct NavaScriptBlock {
    struct NavaScriptBlock *next;
    size_t size;
    uint64_t magic;
    uint64_t mark;
    _Alignas(8) unsigned char payload[];
} NavaScriptBlock;

static NavaScriptChunk *navascript_chunks = NULL;
static size_t navascript_next_capacity = NAVASCRIPT_INITIAL_CHUNK;
static NavaScriptBlock *navascript_blocks = NULL;
static NavaScriptBlock *navascript_free_blocks = NULL;
static uintptr_t navascript_stack_base = 0;

void navascript_gc_init(void) {
    uintptr_t local = 0;
    navascript_stack_base = (uintptr_t)&local;
}

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

static void *navascript_chunk_alloc(size_t bytes) {
    if (navascript_chunks == NULL
        || navascript_chunks->used + bytes > navascript_chunks->capacity) {
        navascript_add_chunk(bytes);
    }
    NavaScriptChunk *chunk = navascript_chunks;
    void *pointer = chunk->data + chunk->used;
    chunk->used += bytes;
    return pointer;
}

static NavaScriptBlock *navascript_find_block(uintptr_t candidate) {
    for (NavaScriptBlock *block = navascript_blocks; block != NULL; block = block->next) {
        if ((uintptr_t)block->payload == candidate) {
            return block;
        }
    }
    return NULL;
}

static void navascript_mark_words(const unsigned char *start, size_t bytes) {
    for (size_t offset = 0; offset + sizeof(uintptr_t) <= bytes; offset += sizeof(uintptr_t)) {
        uintptr_t candidate;
        memcpy(&candidate, start + offset, sizeof(candidate));
        NavaScriptBlock *block = navascript_find_block(candidate);
        if (block != NULL && block->mark == 0) {
            block->mark = 1;
            navascript_mark_words(block->payload, block->size);
        }
    }
}

static void navascript_scan_registers(void) {
    uint64_t value = 0;
    __asm__ volatile("mov %0, x19" : "=r"(value));
    navascript_mark_words((const unsigned char *)&value, sizeof(value));
    __asm__ volatile("mov %0, x20" : "=r"(value));
    navascript_mark_words((const unsigned char *)&value, sizeof(value));
    __asm__ volatile("mov %0, x21" : "=r"(value));
    navascript_mark_words((const unsigned char *)&value, sizeof(value));
    __asm__ volatile("mov %0, x22" : "=r"(value));
    navascript_mark_words((const unsigned char *)&value, sizeof(value));
    __asm__ volatile("mov %0, x23" : "=r"(value));
    navascript_mark_words((const unsigned char *)&value, sizeof(value));
    __asm__ volatile("mov %0, x24" : "=r"(value));
    navascript_mark_words((const unsigned char *)&value, sizeof(value));
    __asm__ volatile("mov %0, x25" : "=r"(value));
    navascript_mark_words((const unsigned char *)&value, sizeof(value));
    __asm__ volatile("mov %0, x26" : "=r"(value));
    navascript_mark_words((const unsigned char *)&value, sizeof(value));
    __asm__ volatile("mov %0, x27" : "=r"(value));
    navascript_mark_words((const unsigned char *)&value, sizeof(value));
    __asm__ volatile("mov %0, x28" : "=r"(value));
    navascript_mark_words((const unsigned char *)&value, sizeof(value));
    __asm__ volatile("mov %0, x29" : "=r"(value));
    navascript_mark_words((const unsigned char *)&value, sizeof(value));
}

void navascript_collect(void) {
    for (NavaScriptBlock *block = navascript_blocks; block != NULL; block = block->next) {
        block->mark = 0;
    }
    navascript_scan_registers();
    uintptr_t current = 0;
    uintptr_t low = (uintptr_t)&current;
    if (navascript_stack_base > low) {
        navascript_mark_words((const unsigned char *)low,
                              (size_t)(navascript_stack_base - low));
    }

    NavaScriptBlock **link = &navascript_blocks;
    while (*link != NULL) {
        NavaScriptBlock *block = *link;
        if (block->mark == 0) {
            *link = block->next;
            block->next = navascript_free_blocks;
            navascript_free_blocks = block;
        } else {
            link = &block->next;
        }
    }
}

double navascript_live_bytes(void) {
    size_t total = 0;
    for (NavaScriptBlock *block = navascript_blocks; block != NULL; block = block->next) {
        total += block->size;
    }
    return (double)total;
}

void *navascript_alloc(size_t bytes) {
    size_t aligned = (bytes + 7u) & ~(size_t)7u;
    if (aligned == 0) {
        aligned = 8;
    }
    size_t total = aligned + sizeof(NavaScriptBlock);

    NavaScriptBlock **slot = &navascript_free_blocks;
    while (*slot != NULL && (*slot)->size < aligned) {
        slot = &(*slot)->next;
    }
    NavaScriptBlock *block = *slot;
    if (block != NULL) {
        *slot = block->next;
        block->size = aligned;
        // A recycled block must be visible to the marker again, otherwise its
        // children can be collected while it is still live.
        block->next = navascript_blocks;
        navascript_blocks = block;
    } else {
        block = (NavaScriptBlock *)navascript_chunk_alloc(total);
        block->size = aligned;
        block->next = navascript_blocks;
        navascript_blocks = block;
    }
    block->magic = NAVASCRIPT_BLOCK_MAGIC;
    block->mark = 0;
    memset(block->payload, 0, aligned);
    return block->payload;
}

void navascript_runtime_error(const char *message) {
    fprintf(stderr, "navascript: runtime error: %s\n", message);
    exit(1);
}
