/**
 * @file ex24_session.c
 * @brief A toy "network session" library that owns opaque handles.
 *
 * The handle is allocated by C (calloc) and returned via an
 * out-parameter — the caller never sees the internal layout.
 * Every function takes the handle as its first argument, much
 * like a C++ `this` pointer.
 *
 * Lifecycle:
 *   session_create(&s)  →  configure  →  connect  →  use  →
 *   disconnect  →  session_destroy(s)
 */

#include <stdlib.h>
#include <string.h>

/* ── Internal (opaque) representation ─────────────────────── */

#define MAX_OPTIONS   4
#define MAX_KEY_LEN  64
#define MAX_VAL_LEN  64
#define MAX_HOST_LEN 128
#define RECV_BUF_LEN 256

typedef struct Session {
    char host[MAX_HOST_LEN];
    int  connected;

    char   opt_keys[MAX_OPTIONS][MAX_KEY_LEN];
    char   opt_vals[MAX_OPTIONS][MAX_VAL_LEN];
    int    option_count;

    unsigned char recv_buf[RECV_BUF_LEN];
    size_t        recv_len;
} Session;

/* ── Status codes ─────────────────────────────────────────── */

#define SESSION_OK            0
#define SESSION_ERR_NULL     -1
#define SESSION_ERR_STATE    -2
#define SESSION_ERR_OVERFLOW -3

/* ── API ──────────────────────────────────────────────────── */

int session_create(Session **out) {
    if (!out) return SESSION_ERR_NULL;
    Session *s = (Session *)calloc(1, sizeof(Session));
    if (!s) return SESSION_ERR_NULL;
    *out = s;
    return SESSION_OK;
}

int session_set_option(Session *s, const char *key, const char *value) {
    if (!s || !key || !value) return SESSION_ERR_NULL;
    if (s->connected)         return SESSION_ERR_STATE;
    if (s->option_count >= MAX_OPTIONS) return SESSION_ERR_OVERFLOW;

    strncpy(s->opt_keys[s->option_count], key,   MAX_KEY_LEN - 1);
    s->opt_keys[s->option_count][MAX_KEY_LEN - 1] = '\0';

    strncpy(s->opt_vals[s->option_count], value,  MAX_VAL_LEN - 1);
    s->opt_vals[s->option_count][MAX_VAL_LEN - 1] = '\0';

    s->option_count++;
    return SESSION_OK;
}

int session_get_option(Session *s, const char *key,
                       char *out_value, size_t out_len) {
    if (!s || !key || !out_value) return SESSION_ERR_NULL;
    for (int i = 0; i < s->option_count; i++) {
        if (strcmp(s->opt_keys[i], key) == 0) {
            size_t vlen = strlen(s->opt_vals[i]);
            if (vlen + 1 > out_len) return SESSION_ERR_OVERFLOW;
            memcpy(out_value, s->opt_vals[i], vlen + 1);
            return SESSION_OK;
        }
    }
    return SESSION_ERR_NULL; /* not found */
}

int session_connect(Session *s, const char *host) {
    if (!s || !host)  return SESSION_ERR_NULL;
    if (s->connected) return SESSION_ERR_STATE;

    strncpy(s->host, host, MAX_HOST_LEN - 1);
    s->host[MAX_HOST_LEN - 1] = '\0';
    s->connected = 1;
    return SESSION_OK;
}

int session_is_connected(Session *s) {
    if (!s) return 0;
    return s->connected;
}

int session_send(Session *s, const unsigned char *data, size_t len) {
    if (!s || !data)    return SESSION_ERR_NULL;
    if (!s->connected)  return SESSION_ERR_STATE;

    /* Simulate an echo server: store the data in the recv buffer. */
    if (len > RECV_BUF_LEN) len = RECV_BUF_LEN;
    memcpy(s->recv_buf, data, len);
    s->recv_len = len;
    return SESSION_OK;
}

int session_recv(Session *s, unsigned char *buf,
                 size_t buf_len, size_t *out_len) {
    if (!s || !buf || !out_len) return SESSION_ERR_NULL;
    if (!s->connected)          return SESSION_ERR_STATE;

    if (s->recv_len == 0) {
        *out_len = 0;
        return SESSION_OK;
    }

    if (s->recv_len > buf_len) return SESSION_ERR_OVERFLOW;

    memcpy(buf, s->recv_buf, s->recv_len);
    *out_len     = s->recv_len;
    s->recv_len  = 0;           /* consume */
    return SESSION_OK;
}

int session_disconnect(Session *s) {
    if (!s)             return SESSION_ERR_NULL;
    if (!s->connected)  return SESSION_ERR_STATE;

    s->connected = 0;
    s->host[0]   = '\0';
    s->recv_len  = 0;
    return SESSION_OK;
}

void session_destroy(Session *s) {
    free(s);  /* free(NULL) is a no-op per the C standard */
}
