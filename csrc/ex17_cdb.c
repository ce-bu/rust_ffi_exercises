/**
 * @file ex17_cdb.c
 * @brief A tiny key-value "database" library using opaque handles.
 *
 * This is a deliberately simple C library that follows a very common
 * pattern: the caller gets an opaque handle (pointer to an incomplete
 * type) and must pass it to every API function.  Internally, the
 * library manages its own memory.
 *
 * The Rust exercise wraps this C API in an idiomatic, safe Rust type.
 */

#include "exercises.h"
#include <stdlib.h>
#include <string.h>

/* ── Internal (hidden) definition of the handle ─────────────── */

#define CDB_MAX_ENTRIES 256
#define CDB_MAX_KEY     64
#define CDB_MAX_VALUE   256

typedef struct CdbEntry {
    char key[CDB_MAX_KEY];
    char value[CDB_MAX_VALUE];
    int  occupied;
} CdbEntry;

struct CdbHandle {
    CdbEntry entries[CDB_MAX_ENTRIES];
    size_t   count;
    int      closed;           /* non-zero after cdb_close */
};

/* ── Public API ─────────────────────────────────────────────── */

CdbHandle *cdb_open(const char *path) {
    if (!path || strlen(path) == 0) return NULL;

    CdbHandle *db = (CdbHandle *)calloc(1, sizeof(CdbHandle));
    /* In a real library, `path` would open a file.  We just
       ignore it and create an in-memory store. */
    return db;   /* NULL on OOM */
}

int cdb_put(CdbHandle *db, const char *key, const char *value) {
    if (!db || db->closed) return CDB_ERR_INVALID;
    if (!key || !value)    return CDB_ERR_INVALID;
    if (strlen(key) == 0)  return CDB_ERR_INVALID;
    if (strlen(key)   >= CDB_MAX_KEY)   return CDB_ERR_OVERFLOW;
    if (strlen(value) >= CDB_MAX_VALUE) return CDB_ERR_OVERFLOW;

    /* Update existing key? */
    for (size_t i = 0; i < CDB_MAX_ENTRIES; ++i) {
        if (db->entries[i].occupied &&
            strcmp(db->entries[i].key, key) == 0)
        {
            strncpy(db->entries[i].value, value, CDB_MAX_VALUE - 1);
            db->entries[i].value[CDB_MAX_VALUE - 1] = '\0';
            return CDB_OK;
        }
    }

    /* Insert into the first free slot */
    if (db->count >= CDB_MAX_ENTRIES) return CDB_ERR_OVERFLOW;
    for (size_t i = 0; i < CDB_MAX_ENTRIES; ++i) {
        if (!db->entries[i].occupied) {
            strncpy(db->entries[i].key,   key,   CDB_MAX_KEY   - 1);
            strncpy(db->entries[i].value, value, CDB_MAX_VALUE - 1);
            db->entries[i].key[CDB_MAX_KEY - 1]     = '\0';
            db->entries[i].value[CDB_MAX_VALUE - 1]  = '\0';
            db->entries[i].occupied = 1;
            db->count++;
            return CDB_OK;
        }
    }
    return CDB_ERR_OVERFLOW;  /* shouldn't happen */
}

int cdb_get(CdbHandle *db, const char *key,
            char *out_value, size_t out_len)
{
    if (!db || db->closed)        return CDB_ERR_INVALID;
    if (!key || !out_value)       return CDB_ERR_INVALID;

    for (size_t i = 0; i < CDB_MAX_ENTRIES; ++i) {
        if (db->entries[i].occupied &&
            strcmp(db->entries[i].key, key) == 0)
        {
            size_t vlen = strlen(db->entries[i].value);
            if (vlen + 1 > out_len) return CDB_ERR_OVERFLOW;
            strncpy(out_value, db->entries[i].value, out_len - 1);
            out_value[out_len - 1] = '\0';
            return CDB_OK;
        }
    }
    return CDB_ERR_NOT_FOUND;
}

int cdb_delete(CdbHandle *db, const char *key) {
    if (!db || db->closed) return CDB_ERR_INVALID;
    if (!key)              return CDB_ERR_INVALID;

    for (size_t i = 0; i < CDB_MAX_ENTRIES; ++i) {
        if (db->entries[i].occupied &&
            strcmp(db->entries[i].key, key) == 0)
        {
            db->entries[i].occupied = 0;
            db->count--;
            return CDB_OK;
        }
    }
    return CDB_ERR_NOT_FOUND;
}

size_t cdb_count(CdbHandle *db) {
    if (!db || db->closed) return 0;
    return db->count;
}

int cdb_close(CdbHandle *db) {
    if (!db)         return CDB_ERR_INVALID;
    if (db->closed)  return CDB_ERR_INVALID;
    db->closed = 1;
    free(db);
    return CDB_OK;
}
