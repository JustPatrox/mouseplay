# Bridge de Chiaki-ng para Mouseplay

## Versión fijada

Mouseplay usa el submódulo `third-party/chiaki-ng` en:

```text
a9a2805884cfa83865fdfcc09ca3ddfcd628aa42
```

La dependencia se incorpora como código fuente externo; no se copian sus fuentes a los crates Rust.

## Frontera actual

```text
ControllerState
      ↓
remote_play::to_chiaki_controller_state
      ↓
repr(C) ChiakiControllerState
      ↓
mouseplay/bridge/chiaki_bridge.c
      ↓
libchiaki (enlace pendiente)
```

El bridge incluye el header público fijado de Chiaki-ng. Rust solo ve `ChiakiRemotePlay`, `RemotePlayError` y `ControllerState`; los handles C son opacos.

## Funciones C expuestas

- `mouseplay_chiaki_context_new`
- `mouseplay_chiaki_context_start`
- `mouseplay_chiaki_context_set_controller_state`
- `mouseplay_chiaki_context_stop`
- `mouseplay_chiaki_context_free`
- funciones de comprobación de tamaño/alineación de `ChiakiControllerState`

Con `libchiaki.dylib` construido, `context_new` ejecuta `chiaki_lib_init()` real y devuelve un contexto operativo. `start`, `send_controller_state` y `stop` devuelven `SessionNotInitialized` hasta que una subfase posterior cree `ChiakiSession` con sus datos de conexión. En targets sin core enlazable el bridge se mantiene no disponible para no fingir una sesión.

## Conversión

- Sticks Mouseplay `0..255`, centro `128`, se convierten a `int16` firmado de Chiaki.
- `l2` y `r2` se copian a `l2_state` y `r2_state`.
- Los botones usan las constantes públicas de `controller.h`.
- `l2`/`r2` digitales se mantienen separados de los valores analógicos.
- Touchpad representa únicamente click; touch coordinates, gyro, acelerómetro y orientación quedan en el estado idle porque Mouseplay aún no los modela.

## Reproducción

```bash
git clone --recurse-submodules <mouseplay-url>
# o, en un checkout existente:
git submodule update --init --recursive
cargo check --target aarch64-apple-darwin
cargo test --target aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin --workspace
```

El bridge se compila mediante `mouseplay/build.rs` y la dependencia `cc`; no depende de una ruta absoluta local.

## Licencia

Chiaki-ng/libchiaki se mantiene bajo AGPL-3.0-only con permiso adicional para enlazar con OpenSSL. Deben conservarse `COPYING`, los avisos de terceros y el código fuente correspondiente al distribuir una combinación cubierta. Mouseplay conserva su licencia propia; la integración no la cambia.

## No implementado todavía

El enlace con `libchiaki` ya es real y `chiaki_lib_init()` se ejecuta en el test de integración del bridge. No existe todavía sesión configurada: discovery, pairing, registro, credenciales, autenticación completa, conexión PS5, keepalive operativo, vídeo, audio ni UI siguen pendientes.
