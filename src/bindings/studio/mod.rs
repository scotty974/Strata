//! Branchement de StudioState : onglets, schéma, pagination.
//! La lecture des fichiers est dans `core::data` ; ici on ne fait que remplir l'interface.

mod convert;
mod model;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel, Weak};

use crate::core::data;
use crate::{AppState, FileTab, Home, Page, StudioState};
use model::StudioModel;

/// Taille d'une page de la grille.
const PAGE_SIZE: usize = 200;

/// Poignée clonable pour piloter le Studio depuis d'autres callbacks (recherche, récents...).
#[derive(Clone)]
pub struct Studio {
    model: Rc<RefCell<StudioModel>>,
    ui: Weak<Home>,
}

impl Studio {
    /// Branche les callbacks de StudioState. À appeler une fois, avant `run()`.
    pub fn setup(app: &Home) -> Studio {
        let studio = Studio {
            model: Rc::new(RefCell::new(StudioModel::default())),
            ui: app.as_weak(),
        };
        let state = app.global::<StudioState>();
        state.set_page_size(PAGE_SIZE as i32);

        let s = studio.clone();
        state.on_tab_selected(move |i| s.select(i as usize));

        let s = studio.clone();
        state.on_tab_closed(move |i| s.close(i as usize));

        let s = studio.clone();
        state.on_page_requested(move |page| s.load_page(page.max(0) as usize));

        let s = studio.clone();
        state.on_run_query(move |_sql| {
            s.with_state(|state| {
                state.set_query_error("Le moteur SQL n'est pas encore branché.".into())
            });
        });

        let s = studio.clone();
        state.on_export_requested(move |format| {
            s.show_error(&format!("Export {format} : pas encore disponible."));
        });

        studio
    }

    /// Ouvre un fichier dans un nouvel onglet (ou réactive l'onglet s'il est déjà ouvert).
    pub fn open(&self, path: &Path) {
        let existing = self.model.borrow().position(path);
        let index = match existing {
            Some(i) => i,
            None => match data::read_summary(path) {
                Ok(summary) => {
                    let i = self.model.borrow_mut().push(path.to_path_buf(), summary);
                    self.refresh_tabs();
                    i
                }
                Err(e) => return self.show_error(&e.to_string()),
            },
        };
        self.select(index);
        self.go_to_studio();
    }

    fn select(&self, index: usize) {
        let Some(app) = self.ui.upgrade() else { return };
        let state = app.global::<StudioState>();
        {
            let model = self.model.borrow();
            let Some(file) = model.files.get(index) else { return };
            let total = file.summary.num_rows;

            state.set_active_tab(index as i32);
            state.set_columns(convert::columns(&file.summary));
            state.set_metadata(convert::metadata(&file.summary));
            state.set_total_rows(total as i32);
            state.set_page_count(total.div_ceil(PAGE_SIZE) as i32);
            state.set_selected_row(-1);
            state.set_selected_column(-1);
            state.set_query_error("".into());
        }
        self.model.borrow_mut().active = Some(index);
        self.load_page(0);
    }

    fn close(&self, index: usize) {
        let next = self.model.borrow_mut().close(index);
        self.refresh_tabs();
        match next {
            Some(i) => self.select(i),
            None => self.clear(),
        }
    }

    fn load_page(&self, page: usize) {
        let Some(path) = self.model.borrow().active_file().map(|f| f.path.clone()) else {
            return;
        };
        match data::read_rows(&path, page * PAGE_SIZE, PAGE_SIZE) {
            Ok(rows) => self.with_state(|state| {
                let null = state.get_null_marker();
                state.set_rows(convert::rows(rows, &null));
                state.set_page_index(page as i32);
                state.set_selected_row(-1);
            }),
            Err(e) => self.show_error(&e.to_string()),
        }
    }

    fn refresh_tabs(&self) {
        let tabs: Vec<FileTab> = self.model.borrow().files.iter().map(|f| convert::tab(&f.path)).collect();
        self.with_state(|state| state.set_tabs(ModelRc::new(VecModel::from(tabs))));
    }

    fn clear(&self) {
        self.with_state(|state| {
            state.set_active_tab(-1);
            state.set_columns(ModelRc::default());
            state.set_metadata(ModelRc::default());
            state.set_rows(ModelRc::default());
            state.set_total_rows(0);
            state.set_page_count(0);
            state.set_page_index(0);
        });
    }

    fn go_to_studio(&self) {
        if let Some(app) = self.ui.upgrade() {
            app.global::<AppState>().set_page(Page::Studio);
        }
    }

    fn show_error(&self, message: &str) {
        if let Some(app) = self.ui.upgrade() {
            app.global::<AppState>().set_error_text(message.into());
        }
    }

    /// Exécute `f` sur StudioState si la fenêtre existe encore.
    fn with_state(&self, f: impl FnOnce(StudioState<'_>)) {
        if let Some(app) = self.ui.upgrade() {
            f(app.global::<StudioState>());
        }
    }
}
