mod rendering;
mod runtime;

use std::{env, ffi::OsString, fs, path::Path, process::Command};

use color_eyre::eyre::Result;

use runtime::runtime_loop::runtime_loop;

// TODO: support CLI arguments (start directory, --hidden, --no-icons, --help, --version)
// TODO: load a config file (keybindings, hidden files, sorting, icons/theme)
fn main() -> Result<()> {
    color_eyre::install()?;

    let terminal = ratatui::init();
    let result = runtime_loop(terminal);

    ratatui::restore();

    enter_directory(&result?)
}

/// A process cannot change its parent shell's cwd, so replace fe with a shell
/// started in the directory the explorer was showing when it was quit. The rc
/// file is sourced with fastfetch suppressed, then the explorer's directory is
/// re-asserted in case the profile `cd`s elsewhere.
fn enter_directory(dir: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        let shell = env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/sh"));
        let is_bash = Path::new(&shell)
            .file_name()
            .is_some_and(|name| name == "bash");

        let rc_file = if is_bash {
            let path = env::temp_dir().join(format!("fe-exit-rc-{}", std::process::id()));
            fs::write(
                &path,
                "fastfetch() { :; }\n\
                 [ -f \"$HOME/.bashrc\" ] && . \"$HOME/.bashrc\"\n\
                 unset -f fastfetch\n\
                 cd -- \"$FE_EXIT_DIR\" 2>/dev/null || true\n\
                 rm -f -- \"$FE_RC_FILE\"\n",
            )?;
            Some(path)
        } else {
            None
        };

        let mut command = Command::new(&shell);
        command.current_dir(dir);

        if let Some(rc) = &rc_file {
            command
                .arg("--noprofile")
                .arg("--rcfile")
                .arg(rc)
                .env("FE_EXIT_DIR", dir)
                .env("FE_RC_FILE", rc);
        }

        let error = command.exec();
        if let Some(rc) = rc_file {
            let _ = fs::remove_file(rc);
        }

        Err(error.into())
    }

    #[cfg(windows)]
    {
        let shell = env::var_os("COMSPEC").unwrap_or_else(|| OsString::from("cmd.exe"));
        Command::new(shell).current_dir(dir).status()?;
        Ok(())
    }
}
