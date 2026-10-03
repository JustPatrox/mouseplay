#include "chiaki_bridge.h"

#if defined(_WIN32)
#include <BaseTsd.h>
typedef SSIZE_T ssize_t;
#else
#include <sys/types.h>
#endif

#include <chiaki/controller.h>

struct MouseplayChiakiContext {
    int reserved;
};

MouseplayChiakiContext *mouseplay_chiaki_context_new(void)
{
    /* Session construction is deferred to the next subphase. */
    return NULL;
}

int mouseplay_chiaki_context_start(MouseplayChiakiContext *context)
{
    return context ? MOUSEPLAY_CHIAKI_NOT_LINKED : MOUSEPLAY_CHIAKI_INVALID_STATE;
}

int mouseplay_chiaki_context_set_controller_state(
    MouseplayChiakiContext *context, const void *chiaki_controller_state)
{
    return context && chiaki_controller_state
        ? MOUSEPLAY_CHIAKI_NOT_LINKED
        : MOUSEPLAY_CHIAKI_INVALID_STATE;
}

int mouseplay_chiaki_context_stop(MouseplayChiakiContext *context)
{
    return context ? MOUSEPLAY_CHIAKI_NOT_LINKED : MOUSEPLAY_CHIAKI_INVALID_STATE;
}

void mouseplay_chiaki_context_free(MouseplayChiakiContext *context)
{
    (void)context;
}

size_t mouseplay_chiaki_controller_state_size(void)
{
    return sizeof(ChiakiControllerState);
}

size_t mouseplay_chiaki_controller_state_alignment(void)
{
    return _Alignof(ChiakiControllerState);
}
