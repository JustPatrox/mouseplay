mod console;
mod controller;
mod input;
mod mapper;
mod platform;

#[cfg(windows)]
mod hooks;

#[cfg(windows)]
use winapi::um::winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};

// --- ESSA É A FUNÇÃO QUE O SEU LOADER PROCURA ---
// Adicionamos 'pub' para que o loader consiga vê-la.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Inicializa o console para ver mensagens
    console::init();

    // 2. Configura os ganchos (Hooks) do mouse
    platform::setup();

    // 3. Tenta carregar o arquivo de configuração
    // Usamos '?' para retornar o erro se o arquivo não existir
    mapper::load("mappings.json")?;

    println!("Mouseplay iniciado com sucesso! Pressione Ctrl+C para sair.");

    // 4. LOOP INFINITO IMPORTANTE
    // Sem isso, o loader.exe executa as linhas acima e fecha instantaneamente.
    loop {
        platform::tick();
        // Dorme um pouco para não usar 100% da CPU
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

// --- MANTEMOS O DLLMAIN PARA CASO QUEIRA USAR COMO DLL ---
#[cfg(windows)]
#[no_mangle]
extern "system" fn DllMain(_hinst: *const u8, reason: u32, _reserved: *const u8) -> u32 {
    match reason {
        DLL_PROCESS_ATTACH => {
            std::thread::spawn(|| {
                // Lógica separada para quando for injetado (sem loop infinito na thread principal)
                console::init();
                platform::setup();
                if let Err(e) = mapper::load("mappings.json") {
                    eprintln!("Erro no mapeamento: {}", e);
                }
            });
        }
        DLL_PROCESS_DETACH => {
            // Código de limpeza se necessário
        }
        _ => {}
    }
    1
}
