//! Index des fichiers du disque : construction (walker) et recherche (query).

mod query;
mod walker;

pub use query::search;
pub use walker::build_index;

use std::path::PathBuf;

/// Un fichier indexé : nom en minuscules (pour la recherche) et chemin complet.
#[derive(Debug, Clone)]
pub struct IndexedFile {
    pub name_lower: String,
    pub path: PathBuf,
}

pub type FileIndex = Vec<IndexedFile>;
