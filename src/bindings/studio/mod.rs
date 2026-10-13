//! Branchement de StudioState : onglets, schéma, pagination, requêtes SQL.
//! La lecture des fichiers est dans `core::data`, le SQL dans `core::sql` ;
//! ici on ne fait que remplir l'interface.

mod convert;
mod model;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel, Weak};
use tokio::task::JoinError;

use crate::core::data::{self, DataResult};
use crate::core::sql::{QueryResult, SqlEngine};
use crate::{AppState, FileTab, Home, Page, StudioState};
use model::StudioModel;

/// Taille d'une page de la grille.
const PAGE_SIZE: usize = 200;
/// Nombre maximum de lignes gardées en mémoire pour un résultat SQL.
const MAX_QUERY_ROWS: usize = 10_000;

/// Poignée clonable pour piloter le Studio depuis d'autres callbacks (recherche, récents...).
#[derive(Clone)]
pub struct Studio {
    model: Rc<RefCell<StudioModel>>,
    engine: Option<Rc<SqlEngine>>,
    ui: Weak<Home>,
}

impl Studio {
    /// Branche les callbacks de StudioState. À appeler une fois, avant `run()`.
    pub fn setup(app: &Home) -> Studio {
        let engine = match SqlEngine::new() {
            Ok(engine) => Some(Rc::new(engine)),
            Err(e) => {
                app.global::<AppState>()
                    .set_error_text(format!("Moteur SQL indisponible : {e}").into());
                None
            }
        };
        let studio = Studio {
            model: Rc::new(RefCell::new(StudioModel::default())),
            engine,
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
        state.on_run_query(move |sql| s.run_query(sql.to_string()));

        let s = studio.clone();
        state.on_query_cleared(move || s.show_file_rows());

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

    // ------------------------------------------------------------ Onglets

    fn select(&self, index: usize) {
        {
            let mut model = self.model.borrow_mut();
            if index >= model.files.len() {
                return;
            }
            model.active = Some(index);
            model.query = None;
        }
        let model = self.model.borrow();
        let Some(file) = model.active_file() else { return };
        self.with_state(|state| {
            state.set_active_tab(index as i32);
            state.set_columns(convert::columns(&file.summary.columns));
            state.set_metadata(convert::metadata(&file.summary));
            state.set_file_rows(file.summary.num_rows as i32);
            state.set_selected_column(-1);
            state.set_query_error("".into());
            state.set_query_info("".into());
        });
        drop(model);
        self.show_file_rows();
    }

    fn close(&self, index: usize) {
        let next = self.model.borrow_mut().close(index);
        self.refresh_tabs();
        match next {
            Some(i) => self.select(i),
            None => self.clear(),
        }
    }

    fn refresh_tabs(&self) {
        let tabs: Vec<FileTab> = self.model.borrow().files.iter().map(|f| convert::tab(&f.path)).collect();
        self.with_state(|state| state.set_tabs(ModelRc::new(VecModel::from(tabs))));
    }

    // ------------------------------------------------------------ Grille

    /// La grille montre les lignes du fichier actif.
    fn show_file_rows(&self) {
        let model = self.model.borrow();
        let Some(file) = model.active_file() else { return };
        let total = file.summary.num_rows;
        self.with_state(|state| {
            state.set_grid_columns(convert::columns(&file.summary.columns));
            state.set_showing_query(false);
            state.set_total_rows(total as i32);
            state.set_page_count(total.div_ceil(PAGE_SIZE) as i32);
        });
        drop(model);
        self.model.borrow_mut().query = None;
        self.load_page(0);
    }

    /// Affiche une page : lue dans le fichier, ou découpée dans le résultat SQL en mémoire.
    fn load_page(&self, page: usize) {
        let offset = page * PAGE_SIZE;
        let rows = {
            let model = self.model.borrow();
            if let Some(query) = &model.query {
                let end = (offset + PAGE_SIZE).min(query.rows.len());
                Ok(query.rows.get(offset..end).map(|r| r.to_vec()).unwrap_or_default())
            } else if let Some(file) = model.active_file() {
                data::read_rows(&file.path, offset, PAGE_SIZE)
            } else {
                return;
            }
        };

        match rows {
            Ok(rows) => self.with_state(|state| {
                let null = state.get_null_marker();
                state.set_rows(convert::rows(rows, &null));
                state.set_page_index(page as i32);
                state.set_selected_row(-1);
            }),
            Err(e) => self.show_error(&e.to_string()),
        }
    }

    // ------------------------------------------------------------ SQL

    fn run_query(&self, sql: String) {
        let Some(engine) = self.engine.clone() else {
            return self.with_state(|s| s.set_query_error("Moteur SQL indisponible.".into()));
        };
        let Some(path) = self.active_path() else { return };

        self.with_state(|state| {
            state.set_query_running(true);
            state.set_query_error("".into());
            state.set_query_info("".into());
        });

        // La requête tourne sur le runtime tokio ; on attend le résultat
        // sur la boucle de l'interface pour pouvoir mettre l'UI à jour.
        let handle = engine.spawn(path.clone(), sql, MAX_QUERY_ROWS);
        let studio = self.clone();
        let spawned = slint::spawn_local(async move {
            let result = handle.await;
            studio.apply_query(&path, result);
        });
        if let Err(e) = spawned {
            self.with_state(|state| {
                state.set_query_running(false);
                state.set_query_error(format!("Impossible de lancer la requête : {e}").into());
            });
        }
    }

    fn apply_query(&self, path: &Path, result: Result<DataResult<QueryResult>, JoinError>) {
        self.with_state(|state| state.set_query_running(false));

        // L'utilisateur a changé d'onglet pendant la requête : on ignore le résultat.
        if self.active_path().as_deref() != Some(path) {
            return;
        }

        let query = match result {
            Ok(Ok(query)) => query,
            Ok(Err(e)) => return self.with_state(|s| s.set_query_error(e.to_string().into())),
            Err(e) => {
                return self.with_state(|s| {
                    s.set_query_error(format!("La requête a été interrompue : {e}").into())
                });
            }
        };

        let total = query.rows.len();
        let mut info = format!("{total} lignes · {} ms", query.elapsed.as_millis());
        if query.truncated {
            info.push_str(&format!(" · limité à {MAX_QUERY_ROWS} lignes"));
        }

        self.with_state(|state| {
            state.set_query_info(info.into());
            state.set_grid_columns(convert::columns(&query.columns));
            state.set_showing_query(true);
            state.set_total_rows(total as i32);
            state.set_page_count(total.div_ceil(PAGE_SIZE) as i32);
            state.set_selected_column(-1);
        });
        self.model.borrow_mut().query = Some(query);
        self.load_page(0);
    }

    // ------------------------------------------------------------ Divers

    fn active_path(&self) -> Option<PathBuf> {
        self.model.borrow().active_file().map(|f| f.path.clone())
    }

    fn clear(&self) {
        self.with_state(|state| {
            state.set_active_tab(-1);
            state.set_columns(ModelRc::default());
            state.set_grid_columns(ModelRc::default());
            state.set_metadata(ModelRc::default());
            state.set_rows(ModelRc::default());
            state.set_showing_query(false);
            state.set_file_rows(0);
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
