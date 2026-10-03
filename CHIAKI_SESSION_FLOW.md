# Flujo de sesión de Chiaki-ng

## Alcance

Esta investigación corresponde exactamente al submódulo `third-party/chiaki-ng` en:

```text
a9a2805884cfa83865fdfcc09ca3ddfcd628aa42
```

No se modificó código de Mouseplay ni se implementó discovery, registro o autenticación.

## Respuesta corta

`libchiaki` no expone una única función que devuelva automáticamente `host`, `regist_key` y `morning`.

El flujo real está dividido en dos operaciones:

1. **Discovery** obtiene la dirección/identidad de una consola.
2. **Registration** obtiene las credenciales de Remote Play.
3. El resultado se copia a `ChiakiConnectInfo`.
4. `ChiakiSession` usa esos datos para iniciar Remote Play.

Para una integración futura, Mouseplay debería llamar las APIs públicas de libchiaki en este orden:

```text
PS5
  ↓
chiaki_discovery_service_init / callback
  ↓  host_addr, target, estado
chiaki_regist_start / ChiakiRegistCb
  ↓  ChiakiRegisteredHost
  ├─ rp_regist_key → ChiakiConnectInfo.regist_key
  └─ rp_key        → ChiakiConnectInfo.morning
  ↓
chiaki_session_init
  ↓
chiaki_session_start
  ↓
chiaki_session_set_controller_state
```

No hay que derivar `morning`, calcular claves ni implementar paquetes: `regist.c`, `session.c` y `rpcrypt.c` hacen ese trabajo.

## 1. Discovery: obtener `host`

### API

Header: `third-party/chiaki-ng/lib/include/chiaki/discovery.h`

Para una consulta puntual:

```c
chiaki_discovery_init(
    ChiakiDiscovery *discovery,
    ChiakiLog *log,
    sa_family_t family);
chiaki_discovery_thread_start_oneshot(
    ChiakiDiscoveryThread *thread,
    ChiakiDiscovery *discovery,
    ChiakiDiscoveryCb callback,
    void *user);
chiaki_discovery_send(
    ChiakiDiscovery *discovery,
    ChiakiDiscoveryPacket *packet,
    struct sockaddr *address,
    size_t address_size);
chiaki_discovery_thread_stop(...);
chiaki_discovery_fini(...);
```

Para descubrimiento continuo, el header `discoveryservice.h` expone:

```c
chiaki_discovery_service_init(
    ChiakiDiscoveryService *service,
    ChiakiDiscoveryServiceOptions *options,
    ChiakiLog *log);
chiaki_discovery_service_fini(...);
```

`ChiakiDiscoveryServiceOptions.cb` recibe un `ChiakiDiscoveryServiceCb` con una lista de `ChiakiDiscoveryHost`.

### Datos producidos

`ChiakiDiscoveryHost`, definido en `discovery.h`, contiene entre otros:

- `host_addr`: dirección de red de la consola;
- `host_request_port`;
- `host_name`;
- `host_id`;
- `system_version`;
- `state` (`READY` o `STANDBY`);
- datos para determinar el target con:
  - `chiaki_discovery_host_is_ps5()`;
  - `chiaki_discovery_host_system_version_target()`.

El valor que acaba en `ChiakiConnectInfo.host` es la dirección de host obtenida/seleccionada por la aplicación. La propia GUI la obtiene desde `DiscoveryHost` o desde un `ManualHost`; el callback de discovery no produce credenciales.

### Cómo lo usa Chiaki-ng

- CLI: `third-party/chiaki-ng/cli/src/discover.c` inicializa `ChiakiDiscovery`, inicia el hilo one-shot, envía `CHIAKI_DISCOVERY_CMD_SRCH` para PS4 y PS5 y muestra `ChiakiDiscoveryHost` en `discovery_cb`.
- GUI: `third-party/chiaki-ng/gui/src/discoverymanager.cpp` configura `ChiakiDiscoveryServiceOptions`, inicia el servicio y transforma cada `ChiakiDiscoveryHost` en su modelo `DiscoveryHost`.

Discovery no registra la consola y no genera `regist_key` ni `morning`.

## 2. Registro/pairing: obtener credenciales

### API

Header: `third-party/chiaki-ng/lib/include/chiaki/regist.h`

La API pública es:

```c
chiaki_regist_start(
    ChiakiRegist *regist,
    ChiakiLog *log,
    const ChiakiRegistInfo *info,
    ChiakiRegistCb callback,
    void *user);
chiaki_regist_stop(ChiakiRegist *regist);
chiaki_regist_fini(ChiakiRegist *regist);
```

`ChiakiRegistInfo` necesita, según el tipo de registro:

- `target` (`CHIAKI_TARGET_PS5_1` para PS5);
- `host` obtenido por discovery o introducido manualmente;
- `pin` de Remote Play;
- `console_pin` cuando corresponda;
- `psn_account_id` o `psn_online_id` según el target/flujo;
- información de hole-punch/RUDP solo para el flujo PSN.

### Resultado

El callback recibe `ChiakiRegistEvent`.

Cuando `event->type == CHIAKI_REGIST_EVENT_TYPE_FINISHED_SUCCESS`,
`event->registered_host` apunta a `ChiakiRegisteredHost`, que contiene:

| Campo | Uso posterior |
| --- | --- |
| `rp_regist_key[0x10]` | Copiar literalmente a `ChiakiConnectInfo.regist_key` |
| `rp_key[0x10]` | Copiar literalmente a `ChiakiConnectInfo.morning` |
| `target` | Determinar PS4/PS5 y versión Remote Play |
| `server_mac`, `server_nickname` | Identificar y asociar la consola |
| `console_pin` | Persistencia/uso posterior de la configuración |

`regist.c` obtiene estos valores de la respuesta de registro y los parsea en `regist_parse_response_payload()`. No se deben calcular desde `pin`, `host` o `server_mac`.

### Cómo lo usa la GUI

`third-party/chiaki-ng/gui/src/qmlbackend.cpp` construye `ChiakiRegistInfo` en `QmlBackend::registerHost()` y crea `QmlRegist`. El constructor de `QmlRegist` llama directamente a:

```cpp
chiaki_regist_start(&chiaki_regist, &chiaki_log, &regist_info,
                    &QmlRegist::regist_cb, this);
```

En `regist_cb`, el evento de éxito copia el `ChiakiRegisteredHost` al modelo `RegisteredHost` y lo guarda mediante `settings->AddRegisteredHost()`.

## 3. De dónde sale `morning`

`morning` no se genera en Mouseplay.

En Chiaki-ng, el valor persistido como `rp_key` es el valor que después se usa como `morning`:

```text
ChiakiRegisteredHost.rp_key
    → GUI RegisteredHost::GetRPKey()
    → StreamSessionConnectInfo.morning
    → ChiakiConnectInfo.morning
```

La relación está verificada en:

- `third-party/chiaki-ng/gui/include/host.h`: `GetRPKey()` devuelve `rp_key`;
- `third-party/chiaki-ng/gui/src/host.cpp`: `SaveToSettings()`/`LoadFromSettings()` persisten `rp_key` bajo la clave `"rp_key"`;
- `third-party/chiaki-ng/gui/src/main.cpp`: para `stream`, carga `temphost.GetRPKey()` en `morning`;
- `third-party/chiaki-ng/gui/src/streamsession.cpp`: copia `connect_info.morning` a `ChiakiConnectInfo.morning`;
- `third-party/chiaki-ng/lib/src/session.c`: usa `connect_info->morning` para inicializar `rpcrypt`.

Tras un registro automático dentro de una sesión, `lib/src/session.c` también copia:

```c
registered_host->rp_key        → session->connect_info.morning
registered_host->rp_regist_key → session->connect_info.regist_key
```

Ese camino automático pertenece al flujo interno de registro de Chiaki; no convierte a Mouseplay en implementador del protocolo.

## 4. De dónde sale `regist_key`

`regist_key` tampoco se deriva en Mouseplay.

El valor persistido como `rp_regist_key` se obtiene de `ChiakiRegisteredHost.rp_regist_key`:

```text
ChiakiRegisteredHost.rp_regist_key
    → GUI RegisteredHost::GetRPRegistKey()
    → StreamSessionConnectInfo.regist_key
    → ChiakiConnectInfo.regist_key
```

La relación está verificada en:

- `third-party/chiaki-ng/lib/include/chiaki/regist.h`;
- `third-party/chiaki-ng/lib/src/regist.c`, que extrae `rp_regist_key` de la respuesta de registro;
- `third-party/chiaki-ng/gui/src/host.cpp`, que persiste `rp_regist_key` bajo `"rp_regist_key"`;
- `third-party/chiaki-ng/gui/src/main.cpp`, que carga `GetRPRegistKey()` para el comando `stream`;
- `third-party/chiaki-ng/gui/src/streamsession.cpp`, que copia el valor al `ChiakiConnectInfo`.

Para el comando `wakeup`, `rp_regist_key` también se interpreta como credencial hexadecimal de 64 bits, pero eso solo sirve para despertar una consola y no sustituye la configuración de una sesión.

## 5. Persistencia reutilizable

### GUI

La GUI sí persiste las credenciales en su configuración mediante Qt `QSettings`.

Archivo: `third-party/chiaki-ng/gui/src/host.cpp`.

`RegisteredHost::SaveToSettings()` guarda:

- `target`;
- `server_mac`;
- `server_nickname`;
- `rp_regist_key`;
- `rp_key_type`;
- `rp_key`;
- datos auxiliares de la consola;
- `console_pin`.

`RegisteredHost::LoadFromSettings()` vuelve a cargar esos valores.

La dirección IP/host no forma parte de `ChiakiRegisteredHost` ni de esos campos de credenciales. La GUI la mantiene asociada mediante `ManualHost`/`DiscoveryHost`, y el flujo de sesión usa `server.GetHostAddr()`.

### Core

`libchiaki` no ofrece una API pública de almacenamiento de hosts/configuración estilo `QSettings`. La persistencia es responsabilidad del frontend. El core ofrece las estructuras C (`ChiakiRegisteredHost`, `ChiakiConnectInfo`) y los callbacks, no un archivo de configuración portable.

Por tanto, Mouseplay no puede asumir que exista un archivo de Chiaki-ng que el core vaya a leer automáticamente.

## 6. Flujo completo del CLI/GUI

### Consola no registrada

```text
1. Discovery
   GUI DiscoveryManager o CLI discover
   → ChiakiDiscoveryHost
   → host_addr, target, estado

2. Registro
   GUI registerHost()
   → ChiakiRegistInfo { target, host, pin, account/online id }
   → chiaki_regist_start()
   → ChiakiRegistCb
   → ChiakiRegisteredHost

3. Persistencia frontend
   GUI RegisteredHost::SaveToSettings()
   → rp_regist_key y rp_key quedan guardados
   → host queda asociado por ManualHost/DiscoveryHost

4. Sesión
   GUI StreamSession
   → ChiakiConnectInfo { host, regist_key, morning, target }
   → chiaki_session_init()
   → chiaki_session_start()

5. Entrada
   StreamSession::SendFeedbackState()
   → chiaki_session_set_controller_state()
   → feedback sender de libchiaki
```

### Consola ya registrada

El comando `stream` de `third-party/chiaki-ng/gui/src/main.cpp` puede cargar `regist_key` y `morning` desde `Settings::GetRegisteredHosts()`, o recibirlos mediante `--registkey` y `--morning`. El host se recibe como argumento final del comando.

La GUI normal sigue este camino en `qmlbackend.cpp`:

```text
DisplayServer seleccionado
  → server.GetHostAddr()
  → registered_host.GetRPRegistKey()
  → registered_host.GetRPKey()
  → StreamSessionConnectInfo
  → StreamSession
```

La construcción final de `ChiakiConnectInfo` está en `gui/src/streamsession.cpp`, aproximadamente en las líneas 469–551 del commit fijado.

## 7. Configuración de la sesión

Header: `third-party/chiaki-ng/lib/include/chiaki/session.h`.

La API mínima para una consola ya registrada es:

```c
ChiakiConnectInfo info = {0};
info.ps5 = true;
info.host = host;
memcpy(info.regist_key, registered_host->rp_regist_key,
       sizeof(info.regist_key));
memcpy(info.morning, registered_host->rp_key,
       sizeof(info.morning));
chiaki_connect_video_profile_preset(
    &info.video_profile,
    CHIAKI_VIDEO_RESOLUTION_PRESET_720p,
    CHIAKI_VIDEO_FPS_PRESET_60);
info.video_profile_auto_downgrade = true;
info.audio_video_disabled = CHIAKI_NONE_DISABLED;
info.auto_regist = false;

chiaki_session_init(&session, &info, &log);
chiaki_session_start(&session);
chiaki_session_set_controller_state(&session, &controller_state);
```

`chiaki_session_init()` valida/resuelve el host y copia las credenciales necesarias a su estado interno. `chiaki_session_start()` inicia el hilo de sesión; la autenticación, cifrado, control, keepalive y transporte se ejecutan internamente en libchiaki.

Para la sesión de Mouseplay, los datos mínimos a obtener son:

- `host`: `ChiakiDiscoveryHost.host_addr` o dirección manual equivalente;
- `regist_key`: `ChiakiRegisteredHost.rp_regist_key`;
- `morning`: `ChiakiRegisteredHost.rp_key`;
- target PS5: `chiaki_discovery_host_is_ps5()`/`ChiakiRegisteredHost.target`.

## 8. ¿Existe una API única para hacerlo todo?

No en el flujo local normal.

- `chiaki_discovery_*` descubre hosts, pero no registra.
- `chiaki_regist_start()` registra y devuelve `ChiakiRegisteredHost`, pero no ofrece la dirección como configuración persistente global.
- `chiaki_session_init()` consume los datos; no busca automáticamente un `RegisteredHost` guardado por la GUI.
- `chiaki_session_start()` inicia la sesión; no reemplaza el registro local.

Existe una ruta especial de sesión con `auto_regist` y un flujo PSN/hole-punch dentro de `lib/src/session.c`, pero requiere `ChiakiHolepunchSession`, cuenta PSN y RUDP ya configurados. No es una alternativa local automática para una PS5 no registrada.

## 9. Qué necesitaría Mouseplay

Sin implementar todavía nada, la interfaz futura puede seguir este contrato:

1. Ejecutar discovery de Chiaki y seleccionar un `ChiakiDiscoveryHost`.
2. Ejecutar `chiaki_regist_start()` con host, target, PIN y datos de cuenta requeridos; recibir `ChiakiRegisteredHost` en `ChiakiRegistCb`.
3. Persistir o entregar al adaptador:
   - `host_addr`/host;
   - `rp_regist_key[16]`;
   - `rp_key[16]` como `morning`;
   - target PS5.
4. Construir `ChiakiConnectInfo` y llamar a `chiaki_session_init()`.
5. Llamar `chiaki_session_start()` y enviar snapshots mediante `chiaki_session_set_controller_state()`.

El `ChiakiConnectionConfig` actual de Mouseplay ya representa el último contrato, pero sus tres valores deben provenir de estas APIs/estructuras reales, no de valores inventados.

## 10. Conclusión

La respuesta exacta es:

```text
host:
  ChiakiDiscoveryHost.host_addr
  (o dirección manual conservada por el frontend)

regist_key:
  ChiakiRegisteredHost.rp_regist_key
  obtenido por chiaki_regist_start / ChiakiRegistCb

morning:
  ChiakiRegisteredHost.rp_key
  obtenido por chiaki_regist_start / ChiakiRegistCb

sesión:
  ChiakiConnectInfo
  → chiaki_session_init
  → chiaki_session_start
  → chiaki_session_set_controller_state
```

Mouseplay debe reutilizar esas APIs y estructuras. No debe calcular credenciales, leer supuestos archivos Qt sin una decisión explícita de integración, ni implementar discovery/registro/autenticación propios.
