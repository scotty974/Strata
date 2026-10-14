//! Rendu des pages PDF avec Pdfium (bibliothèque chargée à l'exécution).
//!
//! `pdfium.dll` (Windows) doit se trouver à côté de l'exécutable, ou dans le PATH.
//! Binaires : https://github.com/bblanchon/pdfium-binaries

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::mpsc::{self, Sender, SyncSender};
use std::thread;

use pdfium_render::prelude::{PdfRenderConfig, Pdfium};

/// Page rendue en pixels RGBA.
pub struct RenderedPage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub page_count: u32,
}

struct RenderRequest {
    path: PathBuf,
    index: u32,
    target_width: i32,
    reply: SyncSender<Result<RenderedPage, String>>,
}

/// Canal vers le thread Pdfium, créé une seule fois.
static PDFIUM_WORKER: OnceLock<Sender<RenderRequest>> = OnceLock::new();

/// Charge Pdfium : d'abord à côté de l'exécutable, puis dans les bibliothèques du système.
fn pdfium() -> Result<Pdfium, String> {
    let next_to_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Pdfium::pdfium_platform_library_name_at_path));

    let bindings = next_to_exe
        .map(Pdfium::bind_to_library)
        .and_then(Result::ok)
        .map(Ok)
        .unwrap_or_else(Pdfium::bind_to_system_library)
        .map_err(|e| {
            format!(
                "Pdfium introuvable ({e}). Placez pdfium.dll à côté de strata-app.exe."
            )
        })?;

    Ok(Pdfium::new(bindings))
}

/// Rend la page `index` (commence à 0) avec une largeur cible en pixels.
///
/// La demande est traitée par un thread permanent qui conserve l'instance Pdfium.
pub fn render_page(
    path: &Path,
    index: u32,
    target_width: i32,
) -> Result<RenderedPage, String> {
    let sender = PDFIUM_WORKER
        .get_or_init(|| {
            let (sender, receiver) = mpsc::channel::<RenderRequest>();

            thread::Builder::new()
                .name("pdfium-renderer".into())
                .spawn(move || {
                    // Pdfium est initialisé une seule fois et reste vivant
                    // pendant toute la durée de vie de ce thread.
                    let pdfium = pdfium();

                    while let Ok(request) = receiver.recv() {
                        let result = match &pdfium {
                            Ok(pdfium) => render_page_with_pdfium(
                                pdfium,
                                &request.path,
                                request.index,
                                request.target_width,
                            ),
                            Err(error) => Err(error.clone()),
                        };

                        let _ = request.reply.send(result);
                    }
                })
                .expect("Impossible de créer le thread Pdfium");

            sender
        })
        .clone();

    let (reply_sender, reply_receiver) = mpsc::sync_channel(1);

    sender
        .send(RenderRequest {
            path: path.to_path_buf(),
            index,
            target_width,
            reply: reply_sender,
        })
        .map_err(|e| format!("Impossible d'envoyer la demande au thread Pdfium : {e}"))?;

    reply_receiver
        .recv()
        .map_err(|e| format!("Impossible de recevoir le résultat Pdfium : {e}"))?
}

/// Rend une page avec une instance Pdfium déjà initialisée.
fn render_page_with_pdfium(
    pdfium: &Pdfium,
    path: &Path,
    index: u32,
    target_width: i32,
) -> Result<RenderedPage, String> {
    let document = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| format!("PDF illisible : {e}"))?;

    let pages = document.pages();
    let page_count = pages.len().max(0) as u32;

    if page_count == 0 {
        return Err("Le PDF ne contient aucune page.".into());
    }

    let page = pages
        .get(index.min(page_count - 1) as i32)
        .map_err(|e| format!("Page introuvable : {e}"))?;

    let config = PdfRenderConfig::new()
        .set_target_width(target_width)
        .set_maximum_height(target_width * 2);

    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("Rendu impossible : {e}"))?;

    Ok(RenderedPage {
        width: bitmap.width() as u32,
        height: bitmap.height() as u32,
        rgba: bitmap.as_rgba_bytes(),
        page_count,
    })
}
