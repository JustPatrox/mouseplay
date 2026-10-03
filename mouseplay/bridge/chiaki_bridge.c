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
#include <chiaki/discovery.h>
#include <chiaki/log.h>
#include <chiaki/regist.h>
#include <chiaki/session.h>
#include <arpa/inet.h>
#include <netdb.h>
#include <string.h>
#include <stdlib.h>
#endif

#define MOUSEPLAY_CHIAKI_HOST_CAPACITY 256

#if defined(MOUSEPLAY_CHIAKI_LINKED)
struct MouseplayDiscoveryCallback {
    MouseplayChiakiDiscoveryResult *result;
};

static void mouseplay_chiaki_discovery_callback(ChiakiDiscoveryHost *host, void *user)
{
    struct MouseplayDiscoveryCallback *callback = user;
    if (!host || !callback || !callback->result || !host->host_addr)
        return;
    strncpy(callback->result->host, host->host_addr, MOUSEPLAY_CHIAKI_HOST_CAPACITY - 1);
    callback->result->host[MOUSEPLAY_CHIAKI_HOST_CAPACITY - 1] = '\0';
    callback->result->ps5 = chiaki_discovery_host_is_ps5(host);
    callback->result->target = chiaki_discovery_host_system_version_target(host);
    callback->result->state = host->state;
}

struct MouseplayRegistrationCallback {
    MouseplayChiakiRegistrationResult *result;
    bool finished;
    bool success;
};

static void mouseplay_chiaki_registration_callback(ChiakiRegistEvent *event, void *user)
{
    struct MouseplayRegistrationCallback *callback = user;
    if (!event || !callback)
        return;
    callback->finished = true;
    callback->success = event->type == CHIAKI_REGIST_EVENT_TYPE_FINISHED_SUCCESS
        && event->registered_host;
    if (callback->success) {
        callback->result->target = event->registered_host->target;
        memcpy(callback->result->regist_key,
               event->registered_host->rp_regist_key,
               sizeof(callback->result->regist_key));
        memcpy(callback->result->morning,
               event->registered_host->rp_key,
               sizeof(callback->result->morning));
    }
}
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

int mouseplay_chiaki_discover(
    MouseplayChiakiContext *context,
    const char *address,
    bool ps5,
    uint64_t timeout_ms,
    MouseplayChiakiDiscoveryResult *result)
{
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    if (!context || !address || !result || timeout_ms == 0)
        return MOUSEPLAY_CHIAKI_INVALID_STATE;
    memset(result, 0, sizeof(*result));

    struct addrinfo hints = {0};
    hints.ai_socktype = SOCK_DGRAM;
    hints.ai_family = strchr(address, ':') ? AF_INET6 : AF_INET;
    struct addrinfo *addresses = NULL;
    if (getaddrinfo(address, NULL, &hints, &addresses) != 0)
        return CHIAKI_ERR_PARSE_ADDR;

    struct addrinfo *selected = NULL;
    for (struct addrinfo *candidate = addresses; candidate; candidate = candidate->ai_next) {
        if ((candidate->ai_family == AF_INET || candidate->ai_family == AF_INET6)
            && candidate->ai_socktype == SOCK_DGRAM) {
            selected = candidate;
            break;
        }
    }
    if (!selected) {
        freeaddrinfo(addresses);
        return CHIAKI_ERR_PARSE_ADDR;
    }

    struct sockaddr_storage destination = {0};
    memcpy(&destination, selected->ai_addr, selected->ai_addrlen);
    if (destination.ss_family == AF_INET)
        ((struct sockaddr_in *)&destination)->sin_port = htons(
            ps5 ? CHIAKI_DISCOVERY_PORT_PS5 : CHIAKI_DISCOVERY_PORT_PS4);
    else
        ((struct sockaddr_in6 *)&destination)->sin6_port = htons(
            ps5 ? CHIAKI_DISCOVERY_PORT_PS5 : CHIAKI_DISCOVERY_PORT_PS4);
    socklen_t destination_size = selected->ai_addrlen;
    freeaddrinfo(addresses);

    ChiakiDiscovery discovery;
    ChiakiErrorCode error = chiaki_discovery_init(&discovery, &context->log, destination.ss_family);
    if (error != CHIAKI_ERR_SUCCESS)
        return error;

    ChiakiDiscoveryThread thread;
    struct MouseplayDiscoveryCallback callback = { .result = result };
    error = chiaki_discovery_thread_start_oneshot(
        &thread, &discovery, mouseplay_chiaki_discovery_callback, &callback);
    if (error == CHIAKI_ERR_SUCCESS) {
        ChiakiDiscoveryPacket packet = {
            .cmd = CHIAKI_DISCOVERY_CMD_SRCH,
            .protocol_version = ps5
                ? CHIAKI_DISCOVERY_PROTOCOL_VERSION_PS5
                : CHIAKI_DISCOVERY_PROTOCOL_VERSION_PS4,
            .user_credential = 0,
        };
        error = chiaki_discovery_send(
            &discovery, &packet, (struct sockaddr *)&destination, destination_size);
        if (error == CHIAKI_ERR_SUCCESS)
            error = chiaki_thread_timedjoin(&thread.thread, NULL, timeout_ms);
        else
            chiaki_discovery_thread_stop(&thread);
        if (error == CHIAKI_ERR_TIMEOUT) {
            chiaki_discovery_thread_stop(&thread);
        } else if (error == CHIAKI_ERR_SUCCESS) {
            chiaki_stop_pipe_fini(&thread.stop_pipe);
        } else {
            chiaki_discovery_thread_stop(&thread);
        }
    }
    chiaki_discovery_fini(&discovery);
    if (error != CHIAKI_ERR_SUCCESS)
        return error;
    return result->host[0] ? MOUSEPLAY_CHIAKI_SUCCESS : MOUSEPLAY_CHIAKI_REGISTRATION_FAILED;
#else
    (void)context;
    (void)address;
    (void)ps5;
    (void)timeout_ms;
    (void)result;
    return MOUSEPLAY_CHIAKI_UNAVAILABLE;
#endif
}

int mouseplay_chiaki_register(
    MouseplayChiakiContext *context,
    const char *host,
    int target,
    uint32_t pin,
    uint32_t console_pin,
    const uint8_t *psn_account_id,
    const char *psn_online_id,
    MouseplayChiakiRegistrationResult *result)
{
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    if (!context || !host || !result || (!psn_account_id && !psn_online_id))
        return MOUSEPLAY_CHIAKI_INVALID_STATE;
    memset(result, 0, sizeof(*result));

    ChiakiRegistInfo info = {0};
    info.target = (ChiakiTarget)target;
    info.host = host;
    info.broadcast = false;
    info.psn_online_id = psn_online_id;
    if (psn_account_id)
        memcpy(info.psn_account_id, psn_account_id, CHIAKI_PSN_ACCOUNT_ID_SIZE);
    info.pin = pin;
    info.console_pin = console_pin;

    struct MouseplayRegistrationCallback callback = {
        .result = result,
        .finished = false,
        .success = false,
    };
    ChiakiRegist registration;
    ChiakiErrorCode error = chiaki_regist_start(
        &registration, &context->log, &info,
        mouseplay_chiaki_registration_callback, &callback);
    if (error != CHIAKI_ERR_SUCCESS)
        return error;
    chiaki_regist_fini(&registration);
    return callback.finished && callback.success
        ? MOUSEPLAY_CHIAKI_SUCCESS
        : MOUSEPLAY_CHIAKI_REGISTRATION_FAILED;
#else
    (void)context;
    (void)host;
    (void)target;
    (void)pin;
    (void)console_pin;
    (void)psn_account_id;
    (void)psn_online_id;
    (void)result;
    return MOUSEPLAY_CHIAKI_UNAVAILABLE;
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
