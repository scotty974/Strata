//! Branchement de l'interface Slint sur la logique de `core`.
//! Chaque fichier gère un global de `ui/state.slint`.

mod files;
mod indexing;
mod palette;
mod preview;
mod search;
mod studio;

use crate::Home;
use indexing::Indexer;
use preview::Previewer;
use studio::Studio;

/// Les contrôleurs partagés entre les callbacks (tous clonables, sur le thread de l'interface).
#[derive(Clone)]
pub struct Services {
    pub indexer: Indexer,
    pub studio: Studio,
    pub preview: Previewer,
}

/// Branche tous les callbacks. À appeler une fois, avant `app.run()`.
pub fn setup(app: &Home) {
    let services = Services {
        indexer: Indexer::setup(app),
        studio: Studio::setup(app),
        preview: Previewer::setup(app),
    };
    search::setup(app, services.clone());
    palette::setup(app, services.clone());
    files::setup(app, services);
}
