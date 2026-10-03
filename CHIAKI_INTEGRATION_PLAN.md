# Plan de integración de Chiaki-ng con Mouseplay

**Estado:** core C externo y camino de sesión integrado. Chiaki-ng permanece como submódulo fijado; `libchiaki.dylib` se construye fuera del crate y se enlaza mediante `mouseplay/build.rs`. Mouseplay no implementa discovery, pairing ni registro: recibe los datos de conexión producidos por Chiaki y los entrega a `chiaki_session_init`.

**Referencia investigada:** `streetpea/chiaki-ng`, rama `main`, especialmente `lib/include/chiaki/`, `lib/src/`, `lib/CMakeLists.txt` y `CMakeLists.txt`. La API y las estructuras descritas abajo deben fijarse a un commit/tag concreto antes de implementar, porque `main` puede cambiar.

## 1. Arquitectura actual

Mouseplay ya separa las responsabilidades relevantes:

```text
Quartz Event Tap (macOS)
        ↓
RawInput
        ↓
Mapper
        ↓
ControllerState
```

- `mouseplay/src/input/macos_input.rs` captura keyboard/mouse y conserva deltas relativos.
- `mouseplay/src/mapper.rs` consume la representación independiente de plataforma y escribe botones, sticks y triggers.
- `mouseplay/src/controller/state.rs` contiene `ControllerState`, con sticks `u8` centrados en `128`, triggers `u8` y botones booleanos.
- `mouseplay/src/controller/ds4.rs` convierte únicamente reportes HID DS4 locales; no es un paquete Remote Play.
- `REMOTE_PLAY.md` confirma que el repositorio no contiene sockets, discovery, pairing, autenticación, cifrado ni sesión PS4/PS5.

Por tanto, la integración no debe extender `DS4` hacia la red. Debe añadir un adaptador separado después de `ControllerState`.

La build reproducible actual usa `scripts/build-chiaki-macos.sh`, CMake y Makefiles de macOS. El script configura `BUILD_SHARED_LIBS=ON`, `CMAKE_OSX_ARCHITECTURES=arm64`, deployment target 13.0 y desactiva GUI, CLI, tests, Steam Deck, Setsu, Speex, FFmpeg, Pi decoder, Opus y Steam shortcut. El resultado esperado es `target/chiaki-ng-arm64-shared/lib/libchiaki.dylib`.

## 2. Arquitectura propuesta

```text
Keyboard + Mouse
        ↓
Quartz Event Tap
        ↓
RawInput
        ↓
Mapper
        ↓
Mouseplay ControllerState
        ↓
Chiaki Adapter (Rust ↔ C)
        ↓
libchiaki / chiaki-lib
        ↓
Remote Play: discovery, registration, control, stream
        ↓
PS5
```

El adaptador debe traducir estados completos y publicar snapshots a una sesión existente. No debe conocer Quartz, `RawInput`, reportes DS4 ni APIs Win32.

La salida recomendada es `chiaki_session_set_controller_state()`. La API pública de `feedbacksender.h` también expone `chiaki_feedback_sender_set_controller_state()`, pero el feedback sender es un componente interno de la sesión; el adaptador no debería construir ni manipular `ChiakiTakion` directamente.

## 3. Componentes de Chiaki necesarios

### 3.1 Core reutilizable

Chiaki-ng tiene una biblioteca C separada del frontend Qt/QML. La guía de contribución identifica `lib/include/chiaki/` como headers públicos y `lib/src/` como el núcleo de protocolo, sesión, networking, recepción/envío de medios y control. La GUI vive en `gui/` y no es necesaria para el primer adaptador.

El target CMake `chiaki-lib` reúne `session.c`, `ctrl.c`, `takion.c`, `streamconnection.c`, `discovery.c`, `regist.c`, `feedbacksender.c`, `controller.c`, los receptores de vídeo/audio y los componentes criptográficos. Esto es la parte que implementa Remote Play; `loader` no debe comunicarse con el PS5 por cuenta propia.

### 3.2 Discovery

La API pública es `discovery.h` / `discoveryservice.h`:

- `chiaki_discovery_init()` / `chiaki_discovery_fini()` crean y destruyen el descubridor.
- `chiaki_discovery_thread_start()` o `chiaki_discovery_thread_start_oneshot()` ejecutan discovery con `ChiakiDiscoveryCb`.
- `ChiakiDiscoveryPacket` admite `CHIAKI_DISCOVERY_CMD_SRCH` y `CHIAKI_DISCOVERY_CMD_WAKEUP`.
- Para PS5, el header define puerto `9302` y versión de protocolo `00030010`.
- `chiaki_discovery_service_init()` ofrece el servicio que mantiene hosts y pings.

El adaptador debe recibir el `ChiakiDiscoveryHost` por callback y seleccionar el host; no debe duplicar el formato de discovery.

### 3.3 Pairing / registration

En Chiaki-ng la operación se expone como registration en `regist.h`, no como una API Rust independiente:

- `ChiakiRegistInfo` contiene target, host, PIN, console PIN, PSN online ID/account ID y la información opcional de PSN/hole-punch.
- `chiaki_regist_start()` inicia la operación asíncrona.
- `ChiakiRegistCb` recibe `FINISHED_SUCCESS`, `FINISHED_FAILED` o `FINISHED_CANCELED`.
- El resultado `ChiakiRegisteredHost` contiene, entre otros, `rp_regist_key`, target y datos de consola.

La registration debe persistir de forma segura la clave de registro y el identificador de cuenta necesarios para sesiones posteriores. Mouseplay no debe fabricar ni derivar esas credenciales.

### 3.4 Autenticación y cifrado

La sesión pública recibe `ChiakiConnectInfo`, que incluye `regist_key`, `morning`, `psn_account_id`, target PS4/PS5 y host. El core conserva y utiliza internamente:

- `rpcrypt` para la criptografía de Remote Play;
- `ecdh` para el intercambio de claves de sesión;
- `gkcrypt` y `aia` para las partes auxiliares del protocolo/registro;
- OpenSSL por defecto, o mbedTLS como alternativa de configuración.

La integración debe tratar esos campos como datos opacos. No se deben reimplementar hashes, intercambio ECDH, claves, nonces ni paquetes.

### 3.5 Conexión y sesión

La API pública mínima es:

```c
chiaki_lib_init();
chiaki_session_init(&session, &connect_info, log);
chiaki_session_set_event_cb(&session, event_cb, user);
chiaki_session_set_video_sample_cb(&session, video_cb, user);
chiaki_session_start(&session);
chiaki_session_set_controller_state(&session, &state);
chiaki_session_stop(&session);
chiaki_session_join(&session);
chiaki_session_fini(&session);
```

`ChiakiConnectInfo` debe incluir como mínimo `ps5`, `host`, `regist_key`, `morning`, `video_profile`, `audio_video_disabled`, `auto_regist` y, cuando corresponda, `psn_account_id`/hole-punch. La secuencia exacta de inicialización de log, profile y callbacks debe encapsularse en el futuro bridge.

`session.c` crea y coordina el control (`ChiakiCtrl`), la conexión de streaming y el estado de la sesión. El caller observa eventos como `CHIAKI_EVENT_CONNECTED`, `CHIAKI_EVENT_REGIST`, `CHIAKI_EVENT_LOGIN_PIN_REQUEST`, `CHIAKI_EVENT_QUIT`, rumble y efectos de trigger.

### 3.6 Keepalive / heartbeat

El keepalive no se expone como una función que Mouseplay deba llamar. Está dentro de las hebras de control/sesión y de los transportes de Chiaki. `ChiakiSession` mantiene estados internos de control, conexión y heartbeat, mientras `ctrl.c`, `takion.c` y `streamconnection.c` gestionan los mensajes y timeouts.

El bridge debe mantener viva la sesión y responder únicamente a callbacks públicos, especialmente PIN/login, quit y eventos de conexión. No debe crear un heartbeat paralelo.

### 3.7 Controller input

`controller.h` define `ChiakiControllerState` y el bitmask de botones. `feedbacksender.h` mantiene el estado anterior, numeración de secuencia, historial y cola de paquetes. `chiaki_session_set_controller_state()` copia el snapshot a la sesión y lo entrega al feedback sender cuando está activo. Ese componente es el que serializa y envía los estados al PS5 por el transporte de Chiaki.

## 4. API exacta utilizada por el bridge

### Discovery y registration

```c
ChiakiErrorCode chiaki_lib_init(void);
ChiakiErrorCode chiaki_discovery_init(ChiakiDiscovery *, ChiakiLog *, sa_family_t);
ChiakiErrorCode chiaki_discovery_thread_start_oneshot(
    ChiakiDiscoveryThread *, ChiakiDiscovery *, ChiakiDiscoveryCb, void *);
ChiakiErrorCode chiaki_regist_start(
    ChiakiRegist *, ChiakiLog *, const ChiakiRegistInfo *, ChiakiRegistCb, void *);
void chiaki_regist_stop(ChiakiRegist *);
void chiaki_regist_fini(ChiakiRegist *);
```

### Sesión y estado

```c
ChiakiErrorCode chiaki_session_init(
    ChiakiSession *, ChiakiConnectInfo *, ChiakiLog *);
void chiaki_session_set_event_cb(ChiakiSession *, ChiakiEventCallback, void *);
void chiaki_session_set_video_sample_cb(
    ChiakiSession *, ChiakiVideoSampleCallback, void *);
ChiakiErrorCode chiaki_session_start(ChiakiSession *);
ChiakiErrorCode chiaki_session_set_controller_state(
    ChiakiSession *, ChiakiControllerState *);
ChiakiErrorCode chiaki_session_stop(ChiakiSession *);
ChiakiErrorCode chiaki_session_join(ChiakiSession *);
void chiaki_session_fini(ChiakiSession *);
```

### Estado Chiaki

```c
void chiaki_controller_state_set_idle(ChiakiControllerState *);
int8_t chiaki_controller_state_start_touch(
    ChiakiControllerState *, uint16_t x, uint16_t y);
void chiaki_controller_state_stop_touch(ChiakiControllerState *, uint8_t id);
void chiaki_controller_state_set_touch_pos(
    ChiakiControllerState *, uint8_t id, uint16_t x, uint16_t y);
```

El bridge actual usa `chiaki_lib_init`, `chiaki_discovery_init`/`chiaki_discovery_thread_start_oneshot`, `chiaki_regist_start`/`chiaki_regist_fini`, `chiaki_session_init`, `chiaki_session_start`, `chiaki_session_set_controller_state`, `chiaki_session_stop`, `chiaki_session_join` y `chiaki_session_fini`. Discovery y registro son wrappers mínimos de esas APIs: no duplican el protocolo y devuelven `host_addr`, `rp_regist_key` y `rp_key`. `set_idle()` y escritura directa de campos públicos son suficientes para el estado actual; la API de touch solo debe usarse cuando Mouseplay incorpore coordenadas y gestos de touchpad.

## 5. ControllerState → Chiaki mapping

Los nombres de Chiaki usan la traducción de PlayStation `CROSS/MOON/BOX/PYRAMID`; corresponden a Cross/Circle/Square/Triangle de Mouseplay.

| Mouseplay | Chiaki | Conversión |
| --- | --- | --- |
| `left_stick.x` | `left_x` | `u8 0..255` centrado en `128` → `int16` firmado; preservar `128 → 0`, extremo bajo → aproximadamente `-32768`, extremo alto → `32767`. |
| `left_stick.y` | `left_y` | Igual que X; conservar el signo Y que ya produce el mapper. |
| `right_stick.x` | `right_x` | Misma conversión centrada y lineal. |
| `right_stick.y` | `right_y` | Misma conversión centrada y lineal; el movimiento de mouse en Y llega sin convertirse a coordenada absoluta. |
| `l2` | `l2_state` | Copia `0..255` directamente. |
| `r2` | `r2_state` | Copia `0..255` directamente. |
| `cross` | `CHIAKI_CONTROLLER_BUTTON_CROSS` | Activar/desactivar bit `1 << 0`. |
| `circle` | `CHIAKI_CONTROLLER_BUTTON_MOON` | Activar/desactivar bit `1 << 1`. |
| `square` | `CHIAKI_CONTROLLER_BUTTON_BOX` | Activar/desactivar bit `1 << 2`. |
| `triangle` | `CHIAKI_CONTROLLER_BUTTON_PYRAMID` | Activar/desactivar bit `1 << 3`. |
| `l1` | `CHIAKI_CONTROLLER_BUTTON_L1` | Activar/desactivar bit `1 << 8`. |
| `r1` | `CHIAKI_CONTROLLER_BUTTON_R1` | Activar/desactivar bit `1 << 9`. |
| `l3` | `CHIAKI_CONTROLLER_BUTTON_L3` | Activar/desactivar bit `1 << 10`. |
| `r3` | `CHIAKI_CONTROLLER_BUTTON_R3` | Activar/desactivar bit `1 << 11`. |
| `share` | `CHIAKI_CONTROLLER_BUTTON_SHARE` | Activar/desactivar bit `1 << 13`. |
| `options` | `CHIAKI_CONTROLLER_BUTTON_OPTIONS` | Activar/desactivar bit `1 << 12`. |
| `ps` | `CHIAKI_CONTROLLER_BUTTON_PS` | Activar/desactivar bit `1 << 15`. |
| `touchpad` / `touch` | `CHIAKI_CONTROLLER_BUTTON_TOUCHPAD` | El campo actual solo representa click booleano; activar/desactivar bit `1 << 14`. No inventar coordenadas. |

Los triggers tienen dos representaciones en Chiaki: `l2_state`/`r2_state` y los bits analógicos `CHIAKI_CONTROLLER_ANALOG_BUTTON_L2`/`R2` (`1 << 16`/`1 << 17`). La conversión debe conservar la distinción que ya hace Mouseplay: el valor `l2`/`r2` alimenta el estado analógico y `buttons.l2`/`buttons.r2` controla el bit digital. No se debe inferir una pulsación digital a partir de un valor analógico salvo que el mapping lo haya producido explícitamente.

La conversión de sticks debe estar centralizada en el futuro adaptador y cubierta por tests de los valores `0`, `128` y `255`. Una forma determinista es una escala por tramo: valores menores que `128` hacia `-32768..-1` y valores mayores que `128` hacia `1..32767`, con `128` exactamente en cero.

## 6. Dependencias

El target CMake de `chiaki-lib` requiere, según las opciones activadas:

- C11/CMake, `Threads` y sockets nativos.
- `nanopb` para protobuf generado.
- `Jerasure` y `gf-complete` para FEC.
- `json-c`.
- `miniupnpc`.
- `libevent`.
- `libcurl` con soporte HTTP/HTTPS/WebSocket requerido por la configuración seleccionada.
- OpenSSL por defecto, o mbedTLS si se elige `CHIAKI_LIB_ENABLE_MBEDTLS`.
- Opus si se habilita audio.
- FFmpeg (`avcodec`/`avutil`, y configuración asociada) si se habilita el decoder de vídeo.
- `CoreServices` en macOS para el core.

Para el primer objetivo de Mouseplay debe desactivarse la GUI Qt/QML, SDL de entrada y hardware específico del Steam Deck. Vídeo/audio pueden quedar inicialmente en el modo de configuración mínimo, pero la sesión funcional debe conservar las dependencias que el core necesite para compilar y enlazar.

El repositorio upstream usa submódulos para `nanopb`, `jerasure`, `gf-complete`, `curl` y otros componentes. La estrategia de dependencia debe fijar commit/tag y registrar licencias y avisos de cada submódulo.

## 7. FFI necesario

La opción mantenible es un bridge C muy pequeño, no bindings generados para toda la biblioteca:

1. CMake construye `chiaki-lib` y exporta los headers públicos.
2. Un header propio del bridge expone únicamente handles opacos y funciones de ciclo de vida.
3. Rust usa `extern "C"` para crear/detener sesión, pasar credenciales ya configuradas, responder PIN y enviar un `MouseplayControllerState` convertido.
4. Los callbacks C se convierten a eventos Rust mediante un `void *user` estable y una cola thread-safe; nunca se llama directamente a objetos Rust que puedan haberse destruido.
5. El bridge posee y libera `ChiakiSession`, `ChiakiLog`, `ChiakiRegist` y recursos de callbacks según un orden documentado.

El bridge debe ocultar de Rust los structs grandes y mutables (`ChiakiSession`, `ChiakiCtrl`, `ChiakiTakion`) para evitar acoplar el layout C al crate. Si se decide enlazar directamente desde Rust, `bindgen` debe fijarse a headers/versiones concretas y excluir internals; no es la primera opción.

No se requiere C++ para `chiaki-lib`: el core y su API pública son C. C++/Qt pertenece principalmente a la GUI. Un bridge C es preferible a un bridge C++ para reducir ABI y dependencias.

## 8. Compatibilidad `aarch64-apple-darwin`

Hay evidencia upstream de builds macOS arm64 publicados y de una configuración explícita para Apple Silicon. El CMake de chiaki-ng selecciona deployment target macOS 13.0 cuando el procesador host es `arm64`, enlaza `CoreServices` y el workflow upstream produce artefactos `macos_arm64`.

Eso demuestra que el proyecto completo puede compilar para Apple Silicon; no demuestra por sí solo que cualquier combinación futura de dependencias del core compile en este workspace. Antes de tocar Mouseplay se necesita una prueba reproducible independiente:

1. clonar chiaki-ng en un checkout de build separado;
2. inicializar submódulos;
3. configurar `CMAKE_OSX_ARCHITECTURES=arm64` y un deployment target acordado;
4. desactivar GUI, CLI, Steam Deck y funciones no necesarias;
5. producir y ejecutar tests del `chiaki-lib`/bridge en macOS arm64;
6. fijar los commits y los artefactos resultantes.

`cargo check --target aarch64-apple-darwin` de Mouseplay exige que exista esa build en `MOUSEPLAY_CHIAKI_BUILD_DIR` o en `target/chiaki-ng-arm64-shared`. `build.rs` añade los headers públicos y generados, enlaza `libchiaki.dylib` y copia la dylib junto a los binarios Cargo para las pruebas.

## 9. Licencias

### Chiaki-ng y libchiaki

Los headers y fuentes de `chiaki-ng/lib` llevan SPDX `LicenseRef-AGPL-3.0-only-OpenSSL`. El fichero `COPYING` es GNU AGPL v3 y contiene una autorización adicional para enlazar con OpenSSL, incluyendo la obligación de incluir el código fuente correspondiente de las partes de OpenSSL usadas al distribuir una forma no fuente.

`libchiaki` no debe tratarse como una biblioteca permisiva. La incorporación estática o dinámica dentro de un producto distribuido exige revisión de cumplimiento AGPL: conservar avisos y licencia, proporcionar el Corresponding Source de la combinación cubierta, respetar las condiciones de modificación/redistribución y atender la obligación AGPL de ofrecer el código fuente a usuarios que interactúen remotamente cuando aplique. La excepción OpenSSL no convierte el proyecto en MIT/BSD ni elimina AGPL.

### Dependencias

Las dependencias no comparten necesariamente una licencia única. Antes de distribuir Mouseplay hay que generar un inventario de SBOM/avisos de:

- `nanopb`;
- `Jerasure`/`gf-complete`;
- `curl`;
- `json-c`;
- `miniupnpc`;
- `libevent`;
- OpenSSL o mbedTLS;
- Opus;
- FFmpeg;
- cualquier framework o submódulo adicional.

No se debe asumir que el modo estático, un framework macOS o un DMG evita las obligaciones de las dependencias. La recomendación es mantener Chiaki-ng como dependencia externa fijada y conservar su fuente, `COPYING`, avisos de terceros y script reproducible de build en el proceso de distribución. La decisión final de incorporar y distribuir código AGPL requiere validación legal del proyecto antes de publicar binarios.

## 10. Plan de implementación por fases

1. **Fijar upstream y licencias.** Elegir un tag/commit de chiaki-ng, registrar submódulos y generar el inventario de licencias. No modificar Mouseplay todavía.
2. **Build aislado del core.** Compilar solo `chiaki-lib` para `aarch64-apple-darwin`, sin GUI/Qt/SDL/Steam Deck, con un toolchain y opciones documentadas.
3. **Bridge mínimo C.** Exponer handles opacos para `chiaki_lib_init`, discovery, registration y sesión; añadir una prueba de carga/enlace sin PS5.
4. **FFI Rust.** Añadir el bridge como dependencia de build sin mezclar Quartz con Chiaki; validar creación/destrucción y propagación de errores.
5. **Discovery.** Mapear `ChiakiDiscoveryHost` a un tipo Rust propio y permitir seleccionar un PS5 descubierto.
6. **Registration/pairing.** Implementar la entrada de PIN/credenciales y persistencia segura de la clave resultante; probar callbacks de éxito/error/cancelación.
7. **Sesión.** Construir `ChiakiConnectInfo`, iniciar/detener `ChiakiSession`, tratar eventos y mantener el ciclo de vida en un hilo controlado.
8. **Controller input.** Añadir el conversor `ControllerState → ChiakiControllerState`; verificar cada botón, trigger y stick con tests de extremos/centro.
9. **Mouse.** Verificar en el pipeline real `delta X/Y → right_stick.x/y → ControllerState → adapter`, sin introducir coordenadas absolutas ni depender de DS4.
10. **Vídeo.** Conectar `ChiakiVideoSampleCallback` a una salida macOS decidida; no bloquear el hilo de control.
11. **Audio.** Conectar sinks solo cuando el objetivo de producto los requiera; mantenerlo separado del input.
12. **Pruebas reales con PS5.** Probar discovery local, registration, sesión LAN, reconexión, stop/join, input sostenido, triggers, touch click y errores de permisos/red. Registrar resultados con una PS5 real.
13. **Distribución.** Verificar firma/notarización macOS, avisos AGPL/terceros, Corresponding Source y reproducibilidad del build.

## 11. Riesgos

- **AGPL y distribución:** la licencia de Chiaki-ng/libchiaki puede condicionar la forma de integrar y distribuir Mouseplay.
- **API inestable:** `main` no es un contrato de ABI; fijar un commit y evitar copiar internals.
- **Dependencias nativas:** json-c, miniupnpc, libevent, curl, OpenSSL/mbedTLS, Jerasure y FEC pueden complicar el cross-build arm64.
- **ABI/threads:** callbacks C son asíncronos; liberar Rust o Chiaki fuera de orden puede causar use-after-free.
- **Credenciales:** `regist_key`, `morning` y `psn_account_id` requieren almacenamiento seguro y no deben aparecer en logs.
- **Vídeo/audio:** obtener una sesión de control no equivale a renderizar vídeo o reproducir audio.
- **Input incompleto:** Mouseplay no representa coordenadas de touchpad, gyro, accelerometer ni haptics; el primer adaptador debe declarar esos campos como inactivos.
- **Semántica de ejes:** hay que fijar la escala y el signo Y con tests y una sesión real, no asumir equivalencia por el nombre del campo.
- **Reconexión/keepalive:** el transporte pertenece al core; el bridge debe respetar sus eventos y no crear bucles paralelos.
- **Target macOS:** los artefactos upstream arm64 prueban viabilidad del proyecto, no la integración concreta con el workspace Cargo.

## 12. Recomendación final

La estrategia más sencilla y mantenible es **compilar una versión fijada de `chiaki-lib` como biblioteca externa y añadir un bridge C mínimo con FFI Rust**, sin copiar el protocolo ni depender de la GUI Qt. El bridge debe usar las APIs públicas de `discovery.h`, `regist.h`, `session.h` y `controller.h`, y debe publicar snapshots convertidos mediante `chiaki_session_set_controller_state()`.

No se recomienda:

- reimplementar discovery, pairing, cifrado, keepalive o paquetes Remote Play en Rust;
- inyectar Mouseplay dentro de `RemotePlay.app` de Apple;
- emular primero un DS4 HID para que la aplicación oficial haga de transporte;
- enlazar todos los internals C mediante bindings automáticos;
- seguir `main` sin commit/tag reproducible.

La implementación actual se detiene después de obtener `chiaki-lib` compilable, enlazarlo al bridge y ejecutar `chiaki_lib_init()`. El contexto todavía no crea una `ChiakiSession` porque faltan los datos de discovery/registro. Solo después deben añadirse registration, input de sesión, vídeo/audio y pruebas con PS5.

## Fuentes consultadas

- Chiaki-ng: https://github.com/streetpea/chiaki-ng
- Guía de arquitectura: https://github.com/streetpea/chiaki-ng/blob/main/CONTRIBUTOR_GUIDE.md
- `controller.h`: https://github.com/streetpea/chiaki-ng/blob/main/lib/include/chiaki/controller.h
- `session.h`: https://github.com/streetpea/chiaki-ng/blob/main/lib/include/chiaki/session.h
- `discovery.h`: https://github.com/streetpea/chiaki-ng/blob/main/lib/include/chiaki/discovery.h
- `regist.h`: https://github.com/streetpea/chiaki-ng/blob/main/lib/include/chiaki/regist.h
- `feedbacksender.h`: https://github.com/streetpea/chiaki-ng/blob/main/lib/include/chiaki/feedbacksender.h
- `lib/CMakeLists.txt`: https://github.com/streetpea/chiaki-ng/blob/main/lib/CMakeLists.txt
- `CMakeLists.txt`: https://github.com/streetpea/chiaki-ng/blob/main/CMakeLists.txt
- `COPYING`: https://github.com/streetpea/chiaki-ng/blob/main/COPYING
- Build macOS arm64: https://github.com/streetpea/chiaki-ng/actions/workflows/build-weekly.yaml
