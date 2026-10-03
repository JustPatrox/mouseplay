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
#include <chiaki/ffmpegdecoder.h>
#include <libavutil/frame.h>
#include <libavutil/pixfmt.h>
#include <arpa/inet.h>
#include <errno.h>
#include <netdb.h>
#include <pthread.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>
#endif

#define MOUSEPLAY_CHIAKI_HOST_CAPACITY 256

#if defined(MOUSEPLAY_CHIAKI_LINKED)
struct MouseplayDiscoveryCallback {
    MouseplayChiakiDiscoveryResult *result;
    pthread_mutex_t mutex;
    pthread_cond_t condition;
    bool done;
};

static void mouseplay_chiaki_discovery_callback(ChiakiDiscoveryHost *host, void *user)
{
    struct MouseplayDiscoveryCallback *callback = user;
    if (!host || !callback || !callback->result || !host->host_addr)
        return;
    pthread_mutex_lock(&callback->mutex);
    strncpy(callback->result->host, host->host_addr, MOUSEPLAY_CHIAKI_HOST_CAPACITY - 1);
    callback->result->host[MOUSEPLAY_CHIAKI_HOST_CAPACITY - 1] = '\0';
    callback->result->ps5 = chiaki_discovery_host_is_ps5(host);
    callback->result->target = chiaki_discovery_host_system_version_target(host);
    callback->result->state = host->state;
    callback->done = true;
    pthread_cond_signal(&callback->condition);
    pthread_mutex_unlock(&callback->mutex);
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
    ChiakiFfmpegDecoder decoder;
    bool decoder_initialized;
    pthread_mutex_t video_mutex;
    uint8_t *video_rgba;
    size_t video_rgba_size;
    uint32_t video_width;
    uint32_t video_height;
#else
    int reserved;
#endif
};

#if defined(MOUSEPLAY_CHIAKI_LINKED)
static uint8_t mouseplay_clamp_u8(int value)
{
    return (uint8_t)(value < 0 ? 0 : value > 255 ? 255 : value);
}

static void mouseplay_yuv_to_bgra(uint8_t *dst, const AVFrame *frame)
{
    for (int y = 0; y < frame->height; y++) {
        for (int x = 0; x < frame->width; x++) {
            int y_value = frame->data[0][y * frame->linesize[0] + x];
            int uv_y = y / 2;
            int uv_x = x / 2;
            int u;
            int v;
            if (frame->format == AV_PIX_FMT_NV12) {
                u = frame->data[1][uv_y * frame->linesize[1] + uv_x * 2];
                v = frame->data[1][uv_y * frame->linesize[1] + uv_x * 2 + 1];
            } else {
                u = frame->data[1][uv_y * frame->linesize[1] + uv_x];
                v = frame->data[2][uv_y * frame->linesize[2] + uv_x];
            }
            int c = y_value - 16;
            int d = u - 128;
            int e = v - 128;
            uint8_t *pixel = dst + ((size_t)y * (size_t)frame->width + (size_t)x) * 4;
            pixel[0] = mouseplay_clamp_u8((298 * c + 516 * d + 128) >> 8);
            pixel[1] = mouseplay_clamp_u8((298 * c - 100 * d - 208 * e + 128) >> 8);
            pixel[2] = mouseplay_clamp_u8((298 * c + 409 * e + 128) >> 8);
            pixel[3] = 255;
        }
    }
}

static void mouseplay_video_frame_available(ChiakiFfmpegDecoder *decoder, void *user)
{
    MouseplayChiakiContext *context = user;
    int32_t frames_lost = 0;
    ChiakiFfmpegFrame decoded = chiaki_ffmpeg_decoder_pull_frame(decoder, &frames_lost);
    (void)frames_lost;
    if (!context || !decoded.frame || decoded.frame->width <= 0 || decoded.frame->height <= 0)
        goto done;
    if (decoded.frame->format != AV_PIX_FMT_YUV420P && decoded.frame->format != AV_PIX_FMT_NV12)
        goto done;
    if (!decoded.frame->data[0] || !decoded.frame->data[1]
        || decoded.frame->linesize[0] <= 0 || decoded.frame->linesize[1] <= 0
        || (decoded.frame->format == AV_PIX_FMT_YUV420P
            && (!decoded.frame->data[2] || decoded.frame->linesize[2] <= 0)))
        goto done;

    size_t size = (size_t)decoded.frame->width * (size_t)decoded.frame->height * 4;
    uint8_t *rgba = malloc(size);
    if (!rgba)
        goto done;
    mouseplay_yuv_to_bgra(rgba, decoded.frame);
    pthread_mutex_lock(&context->video_mutex);
    free(context->video_rgba);
    context->video_rgba = rgba;
    context->video_rgba_size = size;
    context->video_width = (uint32_t)decoded.frame->width;
    context->video_height = (uint32_t)decoded.frame->height;
    pthread_mutex_unlock(&context->video_mutex);
done:
    if (decoded.frame)
        av_frame_free(&decoded.frame);
}
#endif

MouseplayChiakiContext *mouseplay_chiaki_context_new(void)
{
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    MouseplayChiakiContext *context = calloc(1, sizeof(*context));
    if (!context || chiaki_lib_init() != CHIAKI_ERR_SUCCESS) {
        free(context);
        return NULL;
    }
    chiaki_log_init(&context->log, CHIAKI_LOG_ERROR, chiaki_log_cb_print, NULL);
    pthread_mutex_init(&context->video_mutex, NULL);
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
    pthread_mutex_init(&callback.mutex, NULL);
    pthread_cond_init(&callback.condition, NULL);
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
        if (error == CHIAKI_ERR_SUCCESS) {
            struct timespec deadline;
            clock_gettime(CLOCK_REALTIME, &deadline);
            deadline.tv_sec += (time_t)(timeout_ms / 1000);
            deadline.tv_nsec += (long)((timeout_ms % 1000) * 1000000);
            if (deadline.tv_nsec >= 1000000000L) {
                deadline.tv_sec++;
                deadline.tv_nsec -= 1000000000L;
            }
            pthread_mutex_lock(&callback.mutex);
            while (!callback.done && error == CHIAKI_ERR_SUCCESS) {
                int wait_error = pthread_cond_timedwait(
                    &callback.condition, &callback.mutex, &deadline);
                if (wait_error == ETIMEDOUT)
                    error = CHIAKI_ERR_TIMEOUT;
                else if (wait_error != 0)
                    error = CHIAKI_ERR_UNKNOWN;
            }
            pthread_mutex_unlock(&callback.mutex);
        }
        if (error != CHIAKI_ERR_SUCCESS || callback.done)
            chiaki_discovery_thread_stop(&thread);
    }
    if (error != CHIAKI_ERR_SUCCESS && error != CHIAKI_ERR_TIMEOUT) {
        pthread_cond_destroy(&callback.condition);
        pthread_mutex_destroy(&callback.mutex);
    } else {
        pthread_cond_destroy(&callback.condition);
        pthread_mutex_destroy(&callback.mutex);
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
    connect_info.audio_video_disabled = CHIAKI_AUDIO_DISABLED;
    connect_info.auto_regist = false;
    connect_info.packet_loss_max = 0.0;

    ChiakiErrorCode error = chiaki_session_init(&context->session, &connect_info, &context->log);
    if (error != CHIAKI_ERR_SUCCESS)
        return error;
    context->session_initialized = true;
    error = chiaki_ffmpeg_decoder_init(
        &context->decoder, &context->log, connect_info.video_profile.codec,
        connect_info.video_profile.max_fps, NULL, NULL,
        mouseplay_video_frame_available, context);
    if (error != CHIAKI_ERR_SUCCESS) {
        chiaki_session_fini(&context->session);
        context->session_initialized = false;
        return error;
    }
    context->decoder_initialized = true;
    chiaki_session_set_video_sample_cb(
        &context->session, chiaki_ffmpeg_decoder_video_sample_cb, &context->decoder);
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

int mouseplay_chiaki_context_take_video_frame(
    MouseplayChiakiContext *context,
    uint8_t **rgba,
    size_t *size,
    uint32_t *width,
    uint32_t *height)
{
#if defined(MOUSEPLAY_CHIAKI_LINKED)
    if (!context || !rgba || !size || !width || !height)
        return MOUSEPLAY_CHIAKI_INVALID_STATE;
    pthread_mutex_lock(&context->video_mutex);
    if (!context->video_rgba) {
        pthread_mutex_unlock(&context->video_mutex);
        return MOUSEPLAY_CHIAKI_REGISTRATION_FAILED;
    }
    *rgba = context->video_rgba;
    *size = context->video_rgba_size;
    *width = context->video_width;
    *height = context->video_height;
    context->video_rgba = NULL;
    context->video_rgba_size = 0;
    pthread_mutex_unlock(&context->video_mutex);
    return MOUSEPLAY_CHIAKI_SUCCESS;
#else
    (void)context; (void)rgba; (void)size; (void)width; (void)height;
    return MOUSEPLAY_CHIAKI_UNAVAILABLE;
#endif
}

void mouseplay_chiaki_video_frame_free(uint8_t *rgba)
{
    free(rgba);
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
            if (context->decoder_initialized) {
                chiaki_ffmpeg_decoder_fini(&context->decoder);
                context->decoder_initialized = false;
            }
            chiaki_session_fini(&context->session);
        }
        pthread_mutex_lock(&context->video_mutex);
        free(context->video_rgba);
        context->video_rgba = NULL;
        pthread_mutex_unlock(&context->video_mutex);
        pthread_mutex_destroy(&context->video_mutex);
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
