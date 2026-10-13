//! Parcours parallèle du disque pour construire l'index.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

use ignore::{DirEntry, WalkBuilder, WalkState};

use super::{FileIndex, IndexedFile};

/// Résultat de l'indexation : les fichiers et le nombre d'entrées illisibles.
pub struct IndexReport {
    pub files: FileIndex,
    pub errors: usize,
}

/// Dossiers jamais parcourus (comparaison en minuscules).
const EXCLUDED_DIRS: &[&str] = &[
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "$recycle.bin",
    "system volume information",
    "node_modules",
    "target",
    ".git",
    "appdata",
    "wsiaccount",
    "perflogs",
    "system repair",
    "fichiers de conversation microsoft teams",
    "tmp",
];

/// Garde l'entrée sauf si c'est un dossier exclu.
fn keep_entry(entry: &DirEntry) -> bool {
    let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
    if !is_dir {
        return true;
    }
    let name = entry.file_name().to_string_lossy().to_lowercase();
    !EXCLUDED_DIRS.contains(&name.as_str())
}

/// Parcourt `root` en parallèle et renvoie tous les fichiers (pas les dossiers).
pub fn build_index(root: &Path) -> IndexReport {
    let (tx, rx) = mpsc::channel::<IndexedFile>();
    let errors = AtomicUsize::new(0);

    WalkBuilder::new(root)
        .standard_filters(false)
        .follow_links(false)
        .filter_entry(keep_entry)
        .build_parallel()
        .run(|| {
            // Appelé une fois par thread : chaque thread a son propre émetteur.
            let tx = tx.clone();
            let errors = &errors;
            Box::new(move |result| {
                let entry = match result {
                    Ok(entry) => entry,
                    Err(_) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                        return WalkState::Continue;
                    }
                };

                if entry.file_type().is_some_and(|t| t.is_file()) {
                    let file = IndexedFile {
                        name_lower: entry.file_name().to_string_lossy().to_lowercase(),
                        path: entry.into_path(),
                    };
                    // Échec d'envoi = le récepteur n'existe plus : on arrête.
                    if tx.send(file).is_err() {
                        return WalkState::Quit;
                    }
                }
                WalkState::Continue
            })
        });

    // Ferme l'émetteur d'origine pour que la collecte se termine.
    drop(tx);

    IndexReport {
        files: rx.into_iter().collect(),
        errors: errors.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn indexe_les_fichiers_et_ignore_les_dossiers_exclus() {
        let dir = std::env::temp_dir().join("strata_walker_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("data")).unwrap();
        fs::create_dir_all(dir.join("node_modules")).unwrap();
        fs::write(dir.join("data").join("Ventes.parquet"), b"x").unwrap();
        fs::write(dir.join("node_modules").join("lib.js"), b"x").unwrap();

        let report = build_index(&dir);

        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].name_lower, "ventes.parquet");
    }
}
