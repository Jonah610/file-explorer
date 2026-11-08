mod rendering;
mod runtime;

use color_eyre::eyre::Result;

use runtime::runtime_loop::runtime_loop;

// TODO: support CLI arguments (start directory, --hidden, --no-icons, --help, --version)
// TODO: load a config file (keybindings, hidden files, sorting, icons/theme)
fn main() -> Result<()> {
    color_eyre::install()?;

    let terminal = ratatui::init();
    let result = runtime_loop(terminal);

    ratatui::restore();
    result
}
