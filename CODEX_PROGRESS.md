# CODEX_PROGRESS

## Fase 1 — desbloqueo de compilación y aislamiento de plataformas

Estado: completada.

Cambios verificados:
- Se eliminó el target Windows hardcodeado y se renombró `.cargo/config` a `.cargo/config.toml`.
- `winapi` quedó restringido a Windows; se eliminó `itoa`; se añadieron dependencias macOS `core-foundation` y `core-graphics`.
- `DllMain`, hooks, consola Win32 y Raw Input Win32 quedaron condicionados a Windows.
- Se añadió `platform::{windows,macos}` y un backend de entrada macOS temporal para mantener el límite de compilación hasta la Fase 4.
- La resolución de mappings usa `current_exe()` fuera de Windows.

Validación:
- `cargo check --target aarch64-apple-darwin` — salida literal: `Finished dev profile` — estado 0.
- `cargo test --target aarch64-apple-darwin` — salida literal: `test result: ok. 0 passed; 0 failed` — estado 0.
- `cargo build --release --target aarch64-apple-darwin --workspace` — salida literal: `Finished release profile` — estado 0.
- `cargo clippy --target aarch64-apple-darwin` — terminó con warnings existentes/no bloqueantes — estado 0.
- `cargo check --target i686-pc-windows-msvc` — no completó porque el target no está instalado: `can't find crate for core` — estado 101.

La compatibilidad Windows queda condicionada correctamente en el código; la validación cruzada requiere instalar el target `i686-pc-windows-msvc` en el entorno.
