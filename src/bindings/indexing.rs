//! Dossiers indexés : liste persistante, indexation en arrière-plan, affichage dans l'UI.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use slint::{ComponentHandle, ModelRc, VecModel, Weak};

use crate::core::config::{folder_name, load_folders, save_folders};
use crate::core::index::{FileIndex, build_index};
use crate::core::system;
use crate::{AppState, Home, IndexState, IndexedFolder, SearchState};

/// Index partagé entre le thread d'indexation (écriture) et la recherche (lecture).
pub type SharedIndex = Arc<RwLock<FileIndex>>;

/// Nombre de fichiers trouvés par dossier, après indexation.
struct FolderCount {
    path: PathBuf,
    files: usize,
}

#[derive(Clone)]
pub struct Indexer {
    folders: Rc<RefCell<Vec<PathBuf>>>,
    index: SharedIndex,
    running: Arc<AtomicBool>,
    ui: Weak<Home>,
}

impl Indexer {
    /// Charge les dossiers enregistrés, branche IndexState et lance une première indexation.
    pub fn setup(app: &Home) -> Indexer {
        let indexer = Indexer {
            folders: Rc::new(RefCell::new(load_folders())),
            index: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            ui: app.as_weak(),
        };
        let state = app.global::<IndexState>();

        let s = indexer.clone();
        state.on_add_folder(move || s.add_folder());

        let s = indexer.clone();
        state.on_remove_folder(move |i| s.remove_folder(i as usize));

        let s = indexer.clone();
        state.on_reindex(move || s.reindex());

        let s = indexer.clone();
        state.on_folder_clicked(move |i| {
            let folder = s.folders.borrow().get(i as usize).cloned();
            if let Some(folder) = folder
                && let Err(e) = system::open_with_default_app(&folder)
            {
                s.show_error(&format!("Impossible d'ouvrir le dossier : {e}"));
            }
        });

        indexer.reindex();
        indexer
    }

    pub fn index(&self) -> SharedIndex {
        self.index.clone()
    }

    fn add_folder(&self) {
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Ajouter un dossier à indexer")
            .pick_folder()
        else {
            return;
        };
        if self.folders.borrow().contains(&folder) {
            return;
        }
        self.folders.borrow_mut().push(folder);
        self.save_and_reindex();
    }

    fn remove_folder(&self, index: usize) {
        {
            let mut folders = self.folders.borrow_mut();
            if index >= folders.len() {
                return;
            }
            folders.remove(index);
        }
        self.save_and_reindex();
    }

    fn save_and_reindex(&self) {
        if let Err(e) = save_folders(&self.folders.borrow()) {
            self.show_error(&format!("Impossible d'enregistrer les dossiers : {e}"));
        }
        self.reindex();
    }

    /// Reconstruit l'index de tous les dossiers dans un thread.
    pub fn reindex(&self) {
        // Une indexation à la fois.
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let folders = self.folders.borrow().clone();
        self.show_folders(folders.iter().map(|p| (p.clone(), None)).collect());
        if let Some(app) = self.ui.upgrade() {
            app.global::<IndexState>().set_indexing(true);
            app.global::<SearchState>().set_indexing(true);
            app.global::<SearchState>().set_index_text("Indexation…".into());
        }

        let index = self.index.clone();
        let running = self.running.clone();
        let weak = self.ui.clone();

        std::thread::spawn(move || {
            let mut all = Vec::new();
            let mut counts = Vec::new();
            let mut errors = 0;
            for folder in folders {
                let report = build_index(&folder);
                counts.push(FolderCount { path: folder, files: report.files.len() });
                errors += report.errors;
                all.extend(report.files);
            }
            let total = all.len();
            if let Ok(mut guard) = index.write() {
                *guard = all;
            }
            running.store(false, Ordering::SeqCst);

            // Données simples (Send) renvoyées vers le thread de l'interface.
            let rows: Vec<(PathBuf, usize)> = counts.into_iter().map(|c| (c.path, c.files)).collect();
            let _ = weak.upgrade_in_event_loop(move |app| {
                let folders: Vec<IndexedFolder> = rows
                    .iter()
                    .map(|(path, files)| to_indexed_folder(path, Some(*files)))
                    .collect();
                let index_state = app.global::<IndexState>();
                index_state.set_folders(ModelRc::new(VecModel::from(folders)));
                index_state.set_indexing(false);

                let search = app.global::<SearchState>();
                search.set_indexing(false);
                let text = if errors > 0 {
                    format!("{total} fichiers indexés · {errors} ignorés")
                } else {
                    format!("{total} fichiers indexés")
                };
                search.set_index_text(text.into());
            });
        });
    }

    fn show_folders(&self, folders: Vec<(PathBuf, Option<usize>)>) {
        if let Some(app) = self.ui.upgrade() {
            let items: Vec<IndexedFolder> =
                folders.iter().map(|(p, n)| to_indexed_folder(p, *n)).collect();
            app.global::<IndexState>().set_folders(ModelRc::new(VecModel::from(items)));
        }
    }

    fn show_error(&self, message: &str) {
        if let Some(app) = self.ui.upgrade() {
            app.global::<AppState>().set_error_text(message.into());
        }
    }
}

fn to_indexed_folder(path: &std::path::Path, files: Option<usize>) -> IndexedFolder {
    IndexedFolder {
        name: folder_name(path).into(),
        path: path.display().to_string().into(),
        files: files.map(format_count).unwrap_or_else(|| "…".into()).into(),
    }
}

/// 84312 -> "84 312"
fn format_count(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::format_count;

    #[test]
    fn separateur_de_milliers() {
        assert_eq!(format_count(12), "12");
        assert_eq!(format_count(84312), "84 312");
        assert_eq!(format_count(1284302), "1 284 302");
    }
}
