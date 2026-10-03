#include "chiaki_bridge.h"

#if defined(_WIN32)
#include <BaseTsd.h>
typedef SSIZE_T ssize_t;
#else
#include <sys/types.h>
#endif

#include <chiaki/controller.h>

#if defined(MOUSEPLAY_CHIAKI_LINKED)
#include <chiaki/common.h>
#include <chiaki/log.h>
#include <chiaki/session.h>
#include <stdlib.h>
#endif

struct MouseplayChiakiContext {
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    ChiakiSession *session;
    ChiakiLog log;
#else
    int reserved;
#endif
};

MouseplayChiakiContext *mouseplay_chiaki_context_new(void)
{
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    MouseplayChiakiContext *context = calloc(1, sizeof(*context));
    if (!context || chiaki_lib_init() != CHIAKI_ERR_SUCCESS) {
        free(context);
        return NULL;
    }
    chiaki_log_init(&context->log, CHIAKI_LOG_ERROR, chiaki_log_cb_print, NULL);
    return context;
#else
    return NULL;
#endif
}

int mouseplay_chiaki_context_start(MouseplayChiakiContext *context)
{
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    if (!context)
        return MOUSEPLAY_CHIAKI_INVALID_STATE;
    if (!context->session)
        return MOUSEPLAY_CHIAKI_SESSION_NOT_INITIALIZED;
    return chiaki_session_start(context->session);
#else
    (void)context;
    return MOUSEPLAY_CHIAKI_UNAVAILABLE;
#endif
}

int mouseplay_chiaki_context_set_controller_state(
    MouseplayChiakiContext *context, const void *chiaki_controller_state)
{
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    if (!context || !chiaki_controller_state)
        return MOUSEPLAY_CHIAKI_INVALID_STATE;
    if (!context->session)
        return MOUSEPLAY_CHIAKI_SESSION_NOT_INITIALIZED;
    return chiaki_session_set_controller_state(
        context->session, (ChiakiControllerState *)chiaki_controller_state);
#else
    (void)context;
    (void)chiaki_controller_state;
    return MOUSEPLAY_CHIAKI_UNAVAILABLE;
#endif
}

int mouseplay_chiaki_context_stop(MouseplayChiakiContext *context)
{
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    if (!context)
        return MOUSEPLAY_CHIAKI_INVALID_STATE;
    if (!context->session)
        return MOUSEPLAY_CHIAKI_SESSION_NOT_INITIALIZED;
    return chiaki_session_stop(context->session);
#else
    (void)context;
    return MOUSEPLAY_CHIAKI_UNAVAILABLE;
#endif
}

void mouseplay_chiaki_context_free(MouseplayChiakiContext *context)
{
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    if (context) {
        if (context->session)
            chiaki_session_fini(context->session);
        free(context);
    }
#else
    (void)context;
#endif
}

size_t mouseplay_chiaki_controller_state_size(void)
{
    return sizeof(ChiakiControllerState);
}

size_t mouseplay_chiaki_controller_state_alignment(void)
{
    return _Alignof(ChiakiControllerState);
}
