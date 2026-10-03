#[cfg(target_os = "macos")]
mod macos_gui;

#[cfg(target_os = "macos")]
fn main() {
    if let Err(error) = macos_gui::run() {
        eprintln!("Mouseplay GUI error: {error}");
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("--- Mouseplay Loader para PS5 ---");

    // Tenta iniciar a lógica que está dentro da pasta mouseplay
    // Geralmente a função principal se chama 'run' ou 'start'
    if let Err(e) = mouseplay::run() {
        eprintln!("Erro ao iniciar a ferramenta: {}", e);
    }
}
