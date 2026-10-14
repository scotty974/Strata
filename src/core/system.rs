//! Actions système : ouvrir un fichier avec l'application par défaut.

use std::io;
use std::path::Path;
use std::process::Command;

/// Ouvre `path` avec l'application associée par le système (Explorateur sous Windows).
pub fn open_with_default_app(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = Command::new("explorer");
        c.arg(path);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = Command::new("open");
        c.arg(path);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut c = Command::new("xdg-open");
        c.arg(path);
        c
    };

    command.spawn().map(|_| ())
}

/// Ouvre l'Explorateur sur le dossier du fichier, avec le fichier sélectionné (Windows).
pub fn reveal_in_folder(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer").arg(format!("/select,{}", path.display())).spawn().map(|_| ())
    }
    #[cfg(not(target_os = "windows"))]
    {
        match path.parent() {
            Some(dir) => open_with_default_app(dir),
            None => Ok(()),
        }
    }
}
