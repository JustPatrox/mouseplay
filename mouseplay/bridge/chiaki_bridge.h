#ifndef MOUSEPLAY_CHIAKI_BRIDGE_H
#define MOUSEPLAY_CHIAKI_BRIDGE_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MouseplayChiakiContext MouseplayChiakiContext;

enum {
    MOUSEPLAY_CHIAKI_SUCCESS = 0,
    MOUSEPLAY_CHIAKI_UNAVAILABLE = 1,
    MOUSEPLAY_CHIAKI_INVALID_STATE = 2,
    MOUSEPLAY_CHIAKI_SESSION_NOT_INITIALIZED = 3,
    MOUSEPLAY_CHIAKI_REGISTRATION_FAILED = 4,
};

typedef struct MouseplayChiakiDiscoveryResult {
    char host[256];
    bool ps5;
    int target;
    int state;
} MouseplayChiakiDiscoveryResult;

typedef struct MouseplayChiakiRegistrationResult {
    int target;
    uint8_t regist_key[16];
    uint8_t morning[16];
} MouseplayChiakiRegistrationResult;

MouseplayChiakiContext *mouseplay_chiaki_context_new(void);
int mouseplay_chiaki_discover(
    MouseplayChiakiContext *context,
    const char *address,
    bool ps5,
    uint64_t timeout_ms,
    MouseplayChiakiDiscoveryResult *result);
int mouseplay_chiaki_register(
    MouseplayChiakiContext *context,
    const char *host,
    int target,
    uint32_t pin,
    uint32_t console_pin,
    const uint8_t *psn_account_id,
    const char *psn_online_id,
    MouseplayChiakiRegistrationResult *result);
int mouseplay_chiaki_context_init_session(
    MouseplayChiakiContext *context,
    const char *host,
    bool ps5,
    const uint8_t *regist_key,
    const uint8_t *morning);
int mouseplay_chiaki_context_start(MouseplayChiakiContext *context);
int mouseplay_chiaki_context_set_controller_state(
    MouseplayChiakiContext *context, const void *chiaki_controller_state);
int mouseplay_chiaki_context_take_video_frame(
    MouseplayChiakiContext *context,
    uint8_t **rgba,
    size_t *size,
    uint32_t *width,
    uint32_t *height);
void mouseplay_chiaki_video_frame_free(uint8_t *rgba);
int mouseplay_chiaki_context_stop(MouseplayChiakiContext *context);
void mouseplay_chiaki_context_free(MouseplayChiakiContext *context);
size_t mouseplay_chiaki_controller_state_size(void);
size_t mouseplay_chiaki_controller_state_alignment(void);

#ifdef __cplusplus
}
#endif

#endif
