//! Branchement de PreviewState : aperçu des textes, images et PDF.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer, Weak};

use crate::core::data::FileFormat;
use crate::core::format::format_size;
use crate::core::preview::{DocKind, doc_kind, read_text};
use crate::core::{pdf, system};
use crate::{AppState, Home, Page, PreviewKind, PreviewState};

/// Taille maximale lue pour un aperçu texte.
const MAX_TEXT_BYTES: usize = 512 * 1024;
/// Largeur de rendu des pages PDF, en pixels.
const PDF_RENDER_WIDTH: i32 = 1400;

#[derive(Clone)]
pub struct Previewer {
    current: Rc<RefCell<Option<PathBuf>>>,
    /// Numéro du dernier rendu demandé : un rendu plus ancien qui arrive en retard est ignoré.
    generation: Arc<AtomicU64>,
    ui: Weak<Home>,
}

impl Previewer {
    pub fn setup(app: &Home) -> Previewer {
        let previewer = Previewer {
            current: Rc::new(RefCell::new(None)),
            generation: Arc::new(AtomicU64::new(0)),
            ui: app.as_weak(),
        };
        let state = app.global::<PreviewState>();

        let p = previewer.clone();
        state.on_page_requested(move |i| {
            if let Some(path) = p.current.borrow().clone() {
                p.render_pdf(path, i.max(0) as u32);
            }
        });

        let p = previewer.clone();
        state.on_open_external(move || {
            if let Some(path) = p.current.borrow().as_ref() {
                p.report(system::open_with_default_app(path), "ouvrir le fichier");
            }
        });

        let p = previewer.clone();
        state.on_reveal_in_folder(move || {
            if let Some(path) = p.current.borrow().as_ref() {
                p.report(system::reveal_in_folder(path), "afficher le dossier");
            }
        });

        let p = previewer.clone();
        state.on_closed(move || p.clear());

        previewer
    }

    /// Affiche l'aperçu de `path` (dans le panneau de recherche et la page Aperçu).
    pub fn show(&self, path: &Path) {
        let Some(app) = self.ui.upgrade() else { return };
        let state = app.global::<PreviewState>();
        *self.current.borrow_mut() = Some(path.to_path_buf());
        self.generation.fetch_add(1, Ordering::SeqCst);

        let size = std::fs::metadata(path).map(|m| format_size(m.len())).unwrap_or_default();
        state.set_name(file_name(path).into());
        state.set_path(path.display().to_string().into());
        state.set_loading(false);
        state.set_text_truncated(false);
        state.set_page_index(0);
        state.set_page_count(0);
        state.set_message("".into());

        match doc_kind(path) {
            DocKind::Text => match read_text(path, MAX_TEXT_BYTES) {
                Ok((text, truncated)) => {
                    state.set_kind(PreviewKind::Text);
                    state.set_text(text.into());
                    state.set_text_truncated(truncated);
                    state.set_info(format!("Texte · {size}").into());
                }
                Err(e) => self.set_error(&state, &format!("Lecture impossible : {e}")),
            },
            DocKind::Image => match Image::load_from_path(path) {
                Ok(image) => {
                    let s = image.size();
                    state.set_kind(PreviewKind::Image);
                    state.set_image(image);
                    state.set_info(format!("Image · {} × {} px · {size}", s.width, s.height).into());
                }
                Err(_) => self.set_error(&state, "Format d'image non pris en charge."),
            },
            DocKind::Pdf => {
                state.set_kind(PreviewKind::Pdf);
                state.set_info(format!("PDF · {size}").into());
                self.render_pdf(path.to_path_buf(), 0);
            }
            DocKind::Data if is_text_data(path) => match read_text(path, MAX_TEXT_BYTES) {
                // CSV, TSV, JSON : on montre le début du fichier brut
                Ok((text, truncated)) => {
                    state.set_kind(PreviewKind::Text);
                    state.set_text(text.into());
                    state.set_text_truncated(truncated);
                    state.set_info(format!("Données · {size} · double-clic pour le Data Studio").into());
                }
                Err(e) => self.set_error(&state, &format!("Lecture impossible : {e}")),
            },
            DocKind::Data => {
                state.set_kind(PreviewKind::Unsupported);
                state.set_info(format!("Données · {size}").into());
                state.set_message("Double-cliquez pour l'ouvrir dans le Data Studio.".into());
            }
            DocKind::Other => {
                state.set_kind(PreviewKind::Unsupported);
                state.set_info(size.into());
                state.set_message("Ce type de fichier n'a pas d'aperçu dans Strata.".into());
            }
        }
    }

    /// Affiche le document en plein écran (page Aperçu).
    pub fn open_page(&self, path: &Path) {
        self.show(path);
        if let Some(app) = self.ui.upgrade() {
            let state = app.global::<AppState>();
            if state.get_page() != Page::Preview {
                state.set_previous_page(state.get_page());
            }
            state.set_page(Page::Preview);
        }
    }

    /// Rendu d'une page PDF dans un thread ; l'image est créée sur le thread de l'interface.
    fn render_pdf(&self, path: PathBuf, page: u32) {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        if let Some(app) = self.ui.upgrade() {
            app.global::<PreviewState>().set_loading(true);
        }
        let latest = self.generation.clone();
        let weak = self.ui.clone();

        std::thread::spawn(move || {
            let result = pdf::render_page(&path, page, PDF_RENDER_WIDTH);
            if latest.load(Ordering::SeqCst) != generation {
                return; // un autre document ou une autre page a été demandé entre-temps
            }
            let _ = weak.upgrade_in_event_loop(move |app| {
                let state = app.global::<PreviewState>();
                state.set_loading(false);
                match result {
                    Ok(rendered) => {
                        let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
                            &rendered.rgba,
                            rendered.width,
                            rendered.height,
                        );
                        state.set_image(Image::from_rgba8(buffer));
                        state.set_page_index(page.min(rendered.page_count.saturating_sub(1)) as i32);
                        state.set_page_count(rendered.page_count as i32);
                        let info = state.get_info();
                        let base = info.split(" · ").last().unwrap_or("").to_string();
                        state.set_info(format!("PDF · {} pages · {base}", rendered.page_count).into());
                    }
                    Err(message) => {
                        state.set_kind(PreviewKind::Error);
                        state.set_message(message.into());
                    }
                }
            });
        });
    }

    fn clear(&self) {
        *self.current.borrow_mut() = None;
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(app) = self.ui.upgrade() {
            let state = app.global::<PreviewState>();
            state.set_kind(PreviewKind::None);
            state.set_text("".into());
            state.set_image(Image::default());
        }
    }

    fn set_error(&self, state: &PreviewState<'_>, message: &str) {
        state.set_kind(PreviewKind::Error);
        state.set_message(message.into());
    }

    fn report(&self, result: std::io::Result<()>, action: &str) {
        if let Err(e) = result
            && let Some(app) = self.ui.upgrade()
        {
            app.global::<AppState>().set_error_text(format!("Impossible d'{action} : {e}").into());
        }
    }
}

/// Fichiers de données lisibles tels quels (texte).
fn is_text_data(path: &Path) -> bool {
    matches!(
        FileFormat::detect(path),
        Some(FileFormat::Csv | FileFormat::Tsv | FileFormat::Json)
    )
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}
