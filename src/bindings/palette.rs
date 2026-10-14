//! Branchement de PaletteState : fichiers de l'index et actions de navigation.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};

use super::Services;
use super::files::open_any;
use crate::core::format::file_family;
use crate::core::index::search;
use crate::{AppState, Home, Page, PaletteItem, PaletteState, SearchState};

/// Nombre de fichiers proposés dans la palette.
const MAX_FILES: usize = 8;

/// Ce que fait une ligne de la palette quand on la choisit.
#[derive(Clone)]
enum Action {
    OpenFile(PathBuf),
    OpenDialog,
    GoTo(Page),
    SearchFor(String),
}

struct Entry {
    item: PaletteItem,
    action: Action,
}

pub fn setup(app: &Home, services: Services) {
    let entries: Rc<RefCell<Vec<Action>>> = Rc::new(RefCell::new(Vec::new()));
    let state = app.global::<PaletteState>();

    let weak = app.as_weak();
    let index = services.indexer.index();
    let query_entries = entries.clone();
    state.on_query_changed(move |query| {
        let Some(app) = weak.upgrade() else { return };
        let query = query.trim().to_string();

        let mut list = Vec::new();
        if !query.is_empty()
            && let Ok(files) = index.read()
        {
            for path in search(&files, &query, MAX_FILES).paths {
                list.push(file_entry(&path));
            }
        }
        list.extend(action_entries(&query));

        let items: Vec<PaletteItem> = list.iter().map(|e| e.item.clone()).collect();
        *query_entries.borrow_mut() = list.into_iter().map(|e| e.action).collect();
        let state = app.global::<PaletteState>();
        state.set_items(ModelRc::new(VecModel::from(items)));
        state.set_current(0);
    });

    let weak = app.as_weak();
    state.on_activated(move |i| {
        let Some(app) = weak.upgrade() else { return };
        let Some(action) = entries.borrow().get(i as usize).cloned() else { return };
        match action {
            Action::OpenFile(path) => open_any(&app, &services, &path),
            Action::OpenDialog => app.global::<AppState>().invoke_open_file_dialog(),
            Action::GoTo(page) => app.global::<AppState>().set_page(page),
            Action::SearchFor(text) => {
                app.global::<AppState>().set_page(Page::Search);
                let search = app.global::<SearchState>();
                search.set_query(text.clone().into());
                search.invoke_query_changed(text.into());
            }
        }
    });
}

fn file_entry(path: &Path) -> Entry {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let family = file_family(&ext);
    Entry {
        item: PaletteItem {
            group: "FICHIERS".into(),
            title: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default().into(),
            subtitle: path.parent().map(|p| p.display().to_string()).unwrap_or_default().into(),
            icon: if family == "data" { "table".into() } else { "file".into() },
            family: family.into(),
            shortcut: "".into(),
        },
        action: Action::OpenFile(path.to_path_buf()),
    }
}

/// Actions fixes, filtrées par le texte saisi.
fn action_entries(query: &str) -> Vec<Entry> {
    let action = |title: &str, subtitle: &str, icon: &str, shortcut: &str, action: Action| Entry {
        item: PaletteItem {
            group: "ACTIONS".into(),
            title: title.into(),
            subtitle: subtitle.into(),
            icon: icon.into(),
            family: "".into(),
            shortcut: shortcut.into(),
        },
        action,
    };

    let mut list = vec![
        action("Ouvrir un fichier…", "Parquet, CSV, JSON, Arrow, PDF, texte", "upload", "Ctrl O", Action::OpenDialog),
        action("Accueil", "Fichiers récents", "home", "", Action::GoTo(Page::Home)),
        action("Data Studio", "Schéma, grille, SQL", "table", "", Action::GoTo(Page::Studio)),
        action("Recherche", "Chercher dans les dossiers indexés", "search", "", Action::GoTo(Page::Search)),
        action("Paramètres", "Dossiers indexés", "settings", "", Action::GoTo(Page::Settings)),
    ];

    let q = query.to_lowercase();
    if !q.is_empty() {
        list.retain(|e| {
            e.item.title.to_lowercase().contains(&q) || e.item.subtitle.to_lowercase().contains(&q)
        });
        list.push(action(
            &format!("Rechercher « {query} »"),
            "Afficher tous les résultats",
            "search",
            "",
            Action::SearchFor(query.to_string()),
        ));
    }
    list
}
