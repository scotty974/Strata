//! Ouverture des fichiers : sélecteur natif, fichiers récents, résultats de recherche.

use std::path::Path;

use slint::ComponentHandle;

use super::studio::Studio;
use crate::core::data::{FileFormat, SUPPORTED_EXTENSIONS};
use crate::core::system;
use crate::{AppState, Home};

pub fn setup(app: &Home, studio: Studio) {
    let state = app.global::<AppState>();

    // Bouton « Ouvrir un fichier », « Parcourir… », + des onglets, Ctrl+O
    let s = studio.clone();
    state.on_open_file_dialog(move || {
        let picked = rfd::FileDialog::new()
            .set_title("Ouvrir un fichier de données")
            .add_filter("Fichiers de données", SUPPORTED_EXTENSIONS)
            .add_filter("Tous les fichiers", &["*"])
            .pick_file();
        if let Some(path) = picked {
            s.open(&path);
        }
    });

    // Fichier récent cliqué (ou chemin envoyé par un futur glisser-déposer)
    let weak = app.as_weak();
    state.on_open_path(move |path| {
        if let Some(app) = weak.upgrade() {
            open_any(&app, &studio, Path::new(path.as_str()));
        }
    });
}

/// Fichier de données : ouvert dans le Studio. Autre fichier : application par défaut du système.
pub fn open_any(app: &Home, studio: &Studio, path: &Path) {
    if FileFormat::detect(path).is_some() {
        studio.open(path);
    } else if let Err(e) = system::open_with_default_app(path) {
        app.global::<AppState>()
            .set_error_text(format!("Impossible d'ouvrir {} : {e}", path.display()).into());
    }
}
