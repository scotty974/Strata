//! Indexation en arrière-plan et affichage de son état dans la sidebar.

use std::path::Path;
use std::sync::{Arc, RwLock};

use slint::ComponentHandle;

use crate::core::index::{FileIndex, build_index};
use crate::{Home, SearchState};

/// Index partagé entre le thread d'indexation (écriture) et la recherche (lecture).
pub type SharedIndex = Arc<RwLock<FileIndex>>;

/// Lance l'indexation dans un thread pour que la fenêtre s'affiche tout de suite.
pub fn start(app: &Home) -> SharedIndex {
    let index: SharedIndex = Arc::new(RwLock::new(Vec::new()));

    let state = app.global::<SearchState>();
    state.set_indexing(true);
    state.set_index_text("Indexation…".into());

    let root = std::env::var("USERPROFILE").unwrap_or_else(|_| r"C:\".to_string());
    let weak = app.as_weak();
    let shared = index.clone();

    std::thread::spawn(move || {
        let report = build_index(Path::new(&root));
        let count = report.files.len();
        let errors = report.errors;

        if let Ok(mut guard) = shared.write() {
            *guard = report.files;
        }

        // Retour sur le thread de l'interface pour mettre à jour l'affichage.
        let _ = weak.upgrade_in_event_loop(move |app| {
            let state = app.global::<SearchState>();
            state.set_indexing(false);
            let text = if errors > 0 {
                format!("{count} fichiers indexés · {errors} ignorés")
            } else {
                format!("{count} fichiers indexés")
            };
            state.set_index_text(text.into());
        });
    });

    index
}
