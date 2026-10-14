//! Ouverture des fichiers : sélecteur natif, fichiers récents, résultats de recherche, palette.

use std::path::Path;

use slint::ComponentHandle;

use super::Services;
use crate::core::data::SUPPORTED_EXTENSIONS;
use crate::core::preview::{DocKind, doc_kind};
use crate::core::system;
use crate::{AppState, Home};

pub fn setup(app: &Home, services: Services) {
    let state = app.global::<AppState>();

    // Bouton « Ouvrir un fichier », « Parcourir… », + des onglets, Ctrl+O
    let s = services.clone();
    let weak = app.as_weak();
    state.on_open_file_dialog(move || {
        let picked = rfd::FileDialog::new()
            .set_title("Ouvrir un fichier")
            .add_filter("Fichiers de données", SUPPORTED_EXTENSIONS)
            .add_filter("Tous les fichiers", &["*"])
            .pick_file();
        if let (Some(path), Some(app)) = (picked, weak.upgrade()) {
            open_any(&app, &s, &path);
        }
    });

    // Fichier récent cliqué (ou chemin envoyé par un futur glisser-déposer)
    let weak = app.as_weak();
    state.on_open_path(move |path| {
        if let Some(app) = weak.upgrade() {
            open_any(&app, &services, Path::new(path.as_str()));
        }
    });
}

/// Données : Studio. Texte, image, PDF : page Aperçu. Le reste : application par défaut.
pub fn open_any(app: &Home, services: &Services, path: &Path) {
    match doc_kind(path) {
        DocKind::Data => services.studio.open(path),
        DocKind::Text | DocKind::Image | DocKind::Pdf => services.preview.open_page(path),
        DocKind::Other => {
            if let Err(e) = system::open_with_default_app(path) {
                app.global::<AppState>()
                    .set_error_text(format!("Impossible d'ouvrir {} : {e}", path.display()).into());
            }
        }
    }
}
