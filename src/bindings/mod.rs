//! Branchement de l'interface Slint sur la logique de `core`.
//! Chaque fichier gère un global de `ui/state.slint`.

mod files;
mod indexing;
mod search;
mod studio;

use crate::Home;
use studio::Studio;

/// Branche tous les callbacks. À appeler une fois, avant `app.run()`.
pub fn setup(app: &Home) {
    let index = indexing::start(app);
    let studio = Studio::setup(app);
    search::setup(app, index, studio.clone());
    files::setup(app, studio);
}
