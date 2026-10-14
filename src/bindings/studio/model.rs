//! État interne du Studio côté Rust : fichiers ouverts et onglet actif.

use std::path::{Path, PathBuf};

use crate::core::data::FileSummary;
use crate::core::sql::QueryResult;

pub struct OpenFile {
    pub path: PathBuf,
    pub summary: FileSummary,
}

#[derive(Default)]
pub struct StudioModel {
    pub files: Vec<OpenFile>,
    pub active: Option<usize>,
    /// Résultat SQL affiché dans la grille (None = la grille montre le fichier).
    pub query: Option<QueryResult>,
}

impl StudioModel {
    pub fn position(&self, path: &Path) -> Option<usize> {
        self.files.iter().position(|f| f.path == path)
    }

    pub fn active_file(&self) -> Option<&OpenFile> {
        self.active.and_then(|i| self.files.get(i))
    }

    /// Ajoute un fichier et renvoie son index.
    pub fn push(&mut self, path: PathBuf, summary: FileSummary) -> usize {
        self.files.push(OpenFile { path, summary });
        self.files.len() - 1
    }

    /// Ferme un onglet et renvoie l'onglet à afficher ensuite (None s'il n'en reste plus).
    pub fn close(&mut self, index: usize) -> Option<usize> {
        if index >= self.files.len() {
            return self.active;
        }
        self.files.remove(index);
        if self.files.is_empty() {
            self.active = None;
        } else {
            self.active = Some(index.min(self.files.len() - 1));
        }
        self.active
    }
}
