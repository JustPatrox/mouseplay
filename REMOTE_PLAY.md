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

La dependencia concreta es el submódulo C de chiaki-ng fijado en `a9a2805884cfa83865fdfcc09ca3ddfcd628aa42`, construido como `libchiaki.dylib`. Mouseplay recibe una configuración de conexión ya registrada (host, `regist_key` y `morning`) y la pasa a `chiaki_session_init`; no implementa discovery, pairing ni registro.

Después de configurar la sesión, `ChiakiRemotePlay::start()` llama a `chiaki_session_start()` y `send_controller_state()` llama a `chiaki_session_set_controller_state()`. El feedback sender, la autenticación, el cifrado, la red y el keepalive permanecen en libchiaki.

La API Rust también expone `discover()` y `register_ps5()`. Son wrappers delgados: discovery devuelve el `host_addr`/target de `ChiakiDiscoveryHost`, y registro devuelve `ChiakiRegisteredHost.rp_regist_key` como `regist_key` y `ChiakiRegisteredHost.rp_key` como `morning`. El caller puede convertir esas credenciales con `into_connection_config()` y configurar la sesión sin un DualSense físico.

El binario `loader` incluye una ventana nativa AppKit en macOS con botones para buscar, registrar, conectar y desconectar, además de estado y logs básicos. La UI no procesa eventos de juego: el worker mantiene Quartz → RawInput → Mapper → ControllerState y envía snapshots a Chiaki.

## Estado

Fase 5: investigación y límite de integración documentados; transporte Remote Play todavía no implementado.

La ruta de integración puede llegar hasta `chiaki_session_start()` con credenciales válidas. No se declara una conexión PS5 probada en este entorno.

## Fase 5A/5B — core externo y frontera FFI

Se fijó Chiaki-ng como dependencia Git externa en el commit:

`a9a2805884cfa83865fdfcc09ca3ddfcd628aa42`

La referencia está en `third-party/chiaki-ng` como submódulo; Mouseplay no copia sus fuentes dentro de sus crates. El bridge C de `mouseplay/bridge/` incluye únicamente el header público `chiaki/controller.h` del submódulo. La compilación Cargo genera la biblioteca estática del bridge con `cc` y verifica el layout C/Rust de `ChiakiControllerState`.

La API Rust expuesta es `mouseplay::remote_play::ChiakiRemotePlay` con `new`, `start`, `send_controller_state` y `stop`. La conversión cubre sticks, triggers, botones, click de touchpad y valores neutrales de motion/touch no representados por Mouseplay. La API C mantiene handles opacos y reserva las operaciones de contexto/sesión para la siguiente subfase, cuando exista un build enlazable de `chiaki-lib`.

Estado actual: el bridge compila y enlaza contra `libchiaki.dylib`; `ChiakiRemotePlay::new()` ejecuta `chiaki_lib_init()` real. El contexto no crea todavía una `ChiakiSession` porque esa operación requiere datos de discovery/registro, que pertenecen a la siguiente subfase. No existe conexión PS5, discovery, pairing, registro, vídeo, audio ni autenticación implementada.

Para reproducir el checkout se deben inicializar submódulos (`git clone --recurse-submodules` o `git submodule update --init --recursive`) y conservar el commit fijado. La licencia del submódulo sigue siendo AGPL-3.0-only con el permiso adicional de OpenSSL descrito en `CHIAKI_INTEGRATION_PLAN.md`.
