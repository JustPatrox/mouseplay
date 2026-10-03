# Remote Play en Mouseplay

## Resultado de la inspección de Fase 5

El repositorio no contiene una implementación del protocolo Remote Play hacia PS4/PS5.

La implementación existente en `mouseplay/src/hooks.rs` no es un cliente Remote Play:

- localiza `RpCtrlWrapper.dll` dentro de Remote Play para Windows;
- intercepta `CreateFileW`, `ReadFile`, `WriteFile` e `IsDebuggerPresent` mediante la IAT;
- recibe un reporte HID DS4 físico de 64 bytes;
- transforma ese reporte localmente y lo devuelve al proceso de Windows.

No existen en este repositorio:

- sockets UDP/TCP;
- negociación de sesión con PS4/PS5;
- pairing o registro de consola;
- autenticación PSN;
- cifrado Remote Play;
- keepalive/heartbeat de sesión;
- transporte de paquetes de control, audio o vídeo.

Por tanto, `DS4` representa únicamente el reporte HID local. No es un paquete de red Remote Play.

## Decisión técnica

No se añadió un protocolo inventado ni se modificó `hooks.rs` para macOS. La capa de salida queda pendiente de integrar con una implementación real de Remote Play.

La vía técnicamente verificable es reutilizar un cliente existente como Chiaki/chiaki-ng. Su documentación de arquitectura sitúa la sesión, transporte, cifrado y networking en `lib/`, y la emisión del estado de mando en `feedbacksender.c`. Su API pública expone un estado lógico de controlador y una operación para enviarlo al host.

La integración futura debe adoptar esta forma:

```text
ControllerState de Mouseplay
        ↓ adaptación de rangos y botones
ChiakiControllerState
        ↓ API de feedback sender
Sesión Remote Play de chiaki-ng
        ↓ protocolo existente
PS4/PS5
```

Esa integración requiere escoger una estrategia de dependencia concreta —biblioteca C de chiaki-ng, submódulo/build reproducible o proceso externo— y disponer de una sesión configurada con consola registrada. Ninguna de esas dependencias existe actualmente en este workspace.

## Estado

Fase 5: investigación y límite de integración documentados; transporte Remote Play todavía no implementado.

No se declara conexión PS5 funcional.
