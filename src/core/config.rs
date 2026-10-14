//! Réglages persistants : la liste des dossiers indexés.
//! Stockée en texte simple (un chemin par ligne) dans le dossier de configuration utilisateur.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// `%APPDATA%\Strata` sous Windows, `~/.config/strata` ailleurs.
pub fn config_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return PathBuf::from(appdata).join("Strata");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config").join("strata")
}

fn folders_file() -> PathBuf {
    config_dir().join("folders.txt")
}

/// Dossier proposé au premier lancement : le dossier utilisateur.
pub fn default_folders() -> Vec<PathBuf> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(|p| vec![PathBuf::from(p)])
        .unwrap_or_default()
}

/// Dossiers indexés enregistrés, ou le dossier par défaut si rien n'est enregistré.
pub fn load_folders() -> Vec<PathBuf> {
    match fs::read_to_string(folders_file()) {
        Ok(content) => parse_folders(&content),
        Err(_) => default_folders(),
    }
}

pub fn save_folders(folders: &[PathBuf]) -> io::Result<()> {
    fs::create_dir_all(config_dir())?;
    fs::write(folders_file(), format_folders(folders))
}

fn parse_folders(content: &str) -> Vec<PathBuf> {
    content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn format_folders(folders: &[PathBuf]) -> String {
    folders
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Nom court d'un dossier pour l'affichage : « Documents » pour C:\Users\fab\Documents.
pub fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour_texte() {
        let folders = vec![PathBuf::from(r"C:\Users\fab\Documents"), PathBuf::from("/home/fab/data")];
        let parsed = parse_folders(&format_folders(&folders));
        assert_eq!(parsed, folders);
    }

    #[test]
    fn lignes_vides_ignorees() {
        assert_eq!(parse_folders("\n  \n/a\n\n").len(), 1);
    }

    #[test]
    fn nom_court() {
        assert_eq!(folder_name(Path::new("/home/fab/Documents")), "Documents");
    }
}
