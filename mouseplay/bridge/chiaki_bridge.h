#ifndef MOUSEPLAY_CHIAKI_BRIDGE_H
#define MOUSEPLAY_CHIAKI_BRIDGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MouseplayChiakiContext MouseplayChiakiContext;

enum {
    MOUSEPLAY_CHIAKI_SUCCESS = 0,
    MOUSEPLAY_CHIAKI_NOT_LINKED = 1,
    MOUSEPLAY_CHIAKI_INVALID_STATE = 2,
};

MouseplayChiakiContext *mouseplay_chiaki_context_new(void);
int mouseplay_chiaki_context_start(MouseplayChiakiContext *context);
int mouseplay_chiaki_context_set_controller_state(
    MouseplayChiakiContext *context, const void *chiaki_controller_state);
int mouseplay_chiaki_context_stop(MouseplayChiakiContext *context);
void mouseplay_chiaki_context_free(MouseplayChiakiContext *context);
size_t mouseplay_chiaki_controller_state_size(void);
size_t mouseplay_chiaki_controller_state_alignment(void);

#ifdef __cplusplus
}
#endif

#endif
