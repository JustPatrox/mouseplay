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
#include <string.h>
#include <stdlib.h>
#endif

struct MouseplayChiakiContext {
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    ChiakiSession session;
    ChiakiLog log;
    bool session_initialized;
    bool session_started;
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

int mouseplay_chiaki_context_init_session(
    MouseplayChiakiContext *context,
    const char *host,
    bool ps5,
    const uint8_t *regist_key,
    const uint8_t *morning)
{
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    if (!context || !host || !regist_key || !morning || context->session_initialized)
        return MOUSEPLAY_CHIAKI_INVALID_STATE;

    ChiakiConnectInfo connect_info = {0};
    connect_info.ps5 = ps5;
    connect_info.host = host;
    memcpy(connect_info.regist_key, regist_key, sizeof(connect_info.regist_key));
    memcpy(connect_info.morning, morning, sizeof(connect_info.morning));
    chiaki_connect_video_profile_preset(
        &connect_info.video_profile,
        CHIAKI_VIDEO_RESOLUTION_PRESET_720p,
        CHIAKI_VIDEO_FPS_PRESET_60);
    connect_info.video_profile_auto_downgrade = true;
    connect_info.enable_keyboard = false;
    connect_info.enable_dualsense = false;
    connect_info.audio_video_disabled = CHIAKI_AUDIO_VIDEO_DISABLED;
    connect_info.auto_regist = false;
    connect_info.packet_loss_max = 0.0;

    ChiakiErrorCode error = chiaki_session_init(&context->session, &connect_info, &context->log);
    if (error != CHIAKI_ERR_SUCCESS)
        return error;
    context->session_initialized = true;
    return MOUSEPLAY_CHIAKI_SUCCESS;
#else
    (void)context;
    (void)host;
    (void)ps5;
    (void)regist_key;
    (void)morning;
    return MOUSEPLAY_CHIAKI_UNAVAILABLE;
#endif
}

int mouseplay_chiaki_context_start(MouseplayChiakiContext *context)
{
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    if (!context)
        return MOUSEPLAY_CHIAKI_INVALID_STATE;
    if (!context->session_initialized)
        return MOUSEPLAY_CHIAKI_SESSION_NOT_INITIALIZED;
    if (context->session_started)
        return MOUSEPLAY_CHIAKI_INVALID_STATE;
    int error = chiaki_session_start(&context->session);
    if (error == CHIAKI_ERR_SUCCESS)
        context->session_started = true;
    return error;
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
    if (!context->session_initialized)
        return MOUSEPLAY_CHIAKI_SESSION_NOT_INITIALIZED;
    return chiaki_session_set_controller_state(
        &context->session, (ChiakiControllerState *)chiaki_controller_state);
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
    if (!context->session_initialized)
        return MOUSEPLAY_CHIAKI_SESSION_NOT_INITIALIZED;
    int error = chiaki_session_stop(&context->session);
    context->session_started = false;
    return error;
#else
    (void)context;
    return MOUSEPLAY_CHIAKI_UNAVAILABLE;
#endif
}

void mouseplay_chiaki_context_free(MouseplayChiakiContext *context)
{
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    if (context) {
        if (context->session_initialized) {
            if (context->session_started) {
                chiaki_session_stop(&context->session);
                chiaki_session_join(&context->session);
            }
            chiaki_session_fini(&context->session);
        }
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
