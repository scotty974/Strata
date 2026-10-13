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
