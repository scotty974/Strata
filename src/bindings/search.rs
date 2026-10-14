//! Branchement de SearchState : recherche à la frappe et ouverture d'un résultat.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use slint::{ComponentHandle, ModelRc, VecModel};

use super::Services;
use super::files::open_any;
use crate::core::format::{file_family, format_age, format_size};
use crate::core::index::search;
use crate::{Home, SearchHit, SearchState};

/// Nombre maximum de résultats envoyés à l'interface.
const MAX_RESULTS: usize = 200;

pub fn setup(app: &Home, services: Services) {
    let index = services.indexer.index();
    // Chemins des résultats affichés, pour retrouver le fichier au double-clic.
    let shown: Rc<RefCell<Vec<PathBuf>>> = Rc::new(RefCell::new(Vec::new()));
    let state = app.global::<SearchState>();

    let weak = app.as_weak();
    let query_shown = shown.clone();
    state.on_query_changed(move |query| {
        let Some(app) = weak.upgrade() else { return };
        let state = app.global::<SearchState>();

        let started = Instant::now();
        let outcome = match index.read() {
            Ok(files) => search(&files, &query, MAX_RESULTS),
            Err(_) => return,
        };

        if query.trim().is_empty() {
            state.set_info("".into());
        } else {
            let elapsed = started.elapsed().as_millis();
            state.set_info(info_text(outcome.paths.len(), outcome.total, elapsed).into());
        }

        let hits: Vec<SearchHit> = outcome.paths.iter().map(|p| to_search_hit(p)).collect();
        state.set_results(ModelRc::new(VecModel::from(hits)));
        *query_shown.borrow_mut() = outcome.paths;
    });

    // Clic simple : aperçu dans le panneau de droite
    let select_shown = shown.clone();
    let preview = services.preview.clone();
    state.on_result_selected(move |i| {
        if let Some(path) = select_shown.borrow().get(i as usize) {
            preview.show(path);
        }
    });

    // Double-clic : ouvrir
    let weak = app.as_weak();
    state.on_result_opened(move |i| {
        let Some(path) = shown.borrow().get(i as usize).cloned() else { return };
        if let Some(app) = weak.upgrade() {
            open_any(&app, &services, &path);
        }
    });
}

fn info_text(shown: usize, total: usize, elapsed_ms: u128) -> String {
    if total > shown {
        format!("{shown} premiers sur {total} résultats · {elapsed_ms} ms")
    } else {
        format!("{total} résultats · {elapsed_ms} ms")
    }
}

/// Convertit un chemin en ligne de résultat pour l'interface.
pub fn to_search_hit(path: &Path) -> SearchHit {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let kind = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let folder = path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    let meta = std::fs::metadata(path).ok();
    let size = meta.as_ref().map(|m| format_size(m.len())).unwrap_or_default();
    let modified = meta
        .and_then(|m| m.modified().ok())
        .map(format_age)
        .unwrap_or_default();

    SearchHit {
        family: file_family(&kind).into(),
        kind: kind.into(),
        name: name.into(),
        path: folder.into(),
        size: size.into(),
        modified: modified.into(),
        snippet: "".into(),
    }
}
