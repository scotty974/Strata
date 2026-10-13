//! Branchement de l'interface Slint sur la logique de `core`.
//! Chaque fichier gère un global de `ui/state.slint`.

mod indexing;
mod search;
mod studio;

use std::path::Path;

use slint::ComponentHandle;

use crate::{AppState, Home};
use studio::Studio;

/// Branche tous les callbacks. À appeler une fois, avant `app.run()`.
pub fn setup(app: &Home) {
    let index = indexing::start(app);
    let studio = Studio::setup(app);
    search::setup(app, index, studio.clone());

    // Fichier récent cliqué (ou chemin envoyé par un futur glisser-déposer)
    app.global::<AppState>().on_open_path(move |path| {
        studio.open(Path::new(path.as_str()));
    });
}
