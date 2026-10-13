#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cell::RefCell;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::time::{Instant, SystemTime};

use slint::{ModelRc, PhysicalSize, VecModel, WindowSize};

use utils::indexing_file::{FileIndex, indexing_file};

mod utils;
slint::include_modules!();

/// Nombre maximum de résultats envoyés à l'interface.
const MAX_RESULTS: usize = 200;

fn main() -> Result<(), Box<dyn Error>> {
    let app = Home::new()?;
    app.window()
        .set_size(WindowSize::Physical(PhysicalSize::new(600, 800)));

    // Index partagé : écrit par le thread d'indexation, lu par la recherche.
    let index: Arc<RwLock<FileIndex>> = Arc::new(RwLock::new(Vec::new()));

    // Chemins des résultats affichés, pour retrouver le fichier au double-clic.
    let shown: Rc<RefCell<Vec<PathBuf>>> = Rc::new(RefCell::new(Vec::new()));

    start_indexing(&app, index.clone());

    // Recherche à chaque frappe
    let weak = app.as_weak();
    let search_index = index.clone();
    let search_shown = shown.clone();
    app.global::<SearchState>().on_query_changed(move |query| {
        let Some(app) = weak.upgrade() else { return };
        let state = app.global::<SearchState>();

        let query = query.trim().to_lowercase();
        if query.is_empty() {
            search_shown.borrow_mut().clear();
            state.set_results(ModelRc::default());
            state.set_info("".into());
            return;
        }

        let started = Instant::now();
        let Ok(files) = search_index.read() else { return };

        let mut total = 0;
        let mut paths = Vec::new();
        for file in files.iter().filter(|f| f.name_lower.contains(&query)) {
            total += 1;
            if paths.len() < MAX_RESULTS {
                paths.push(file.path.clone());
            }
        }
        drop(files);

        let hits: Vec<SearchHit> = paths.iter().map(|p| to_search_hit(p)).collect();
        state.set_results(ModelRc::new(VecModel::from(hits)));

        let elapsed = started.elapsed().as_millis();
        let info = if total > MAX_RESULTS {
            format!("{MAX_RESULTS} premiers sur {total} résultats · {elapsed} ms")
        } else {
            format!("{total} résultats · {elapsed} ms")
        };
        state.set_info(info.into());

        *search_shown.borrow_mut() = paths;
    });

    // Double-clic sur un résultat
    let open_shown = shown.clone();
    app.global::<SearchState>().on_result_opened(move |i| {
        if let Some(path) = open_shown.borrow().get(i as usize) {
            eprintln!("ouvrir : {}", path.display());
        }
    });

    app.run()?;
    Ok(())
}

/// Lance l'indexation dans un thread pour que la fenêtre s'affiche tout de suite.
fn start_indexing(app: &Home, index: Arc<RwLock<FileIndex>>) {
    let state = app.global::<SearchState>();
    state.set_indexing(true);
    state.set_index_text("Indexation…".into());

    let root = std::env::var("USERPROFILE").unwrap_or_else(|_| r"C:\".to_string());
    let weak = app.as_weak();

    std::thread::spawn(move || {
        let report = indexing_file(Path::new(&root));
        let count = report.files.len();

        if let Ok(mut shared) = index.write() {
            *shared = report.files;
        }

        // Retour sur le thread de l'interface pour mettre à jour l'affichage.
        let _ = weak.upgrade_in_event_loop(move |app| {
            let state = app.global::<SearchState>();
            state.set_indexing(false);
            state.set_index_text(format!("{count} fichiers indexés").into());
        });
    });
}

/// Convertit un chemin en ligne de résultat pour l'interface.
fn to_search_hit(path: &Path) -> SearchHit {
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
        family: family_of(&kind).into(),
        kind: kind.into(),
        name: name.into(),
        path: folder.into(),
        size: size.into(),
        modified: modified.into(),
        snippet: "".into(),
    }
}

/// Famille de fichier, qui pilote la couleur de l'icône.
fn family_of(ext: &str) -> &'static str {
    match ext {
        "parquet" | "csv" | "tsv" | "arrow" | "feather" | "json" | "jsonl" | "xlsx" | "xls" => "data",
        "pdf" => "pdf",
        "py" | "rs" | "sql" | "ipynb" | "js" | "ts" | "toml" | "yaml" | "yml" => "code",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => "image",
        _ => "doc",
    }
}

/// 1536 -> "1,5 Ko"
fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["o", "Ko", "Mo", "Go", "To"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} o")
    } else {
        format!("{value:.1} {}", UNITS[unit]).replace('.', ",")
    }
}

/// Date de modification relative : "il y a 3 j"
fn format_age(modified: SystemTime) -> String {
    let Ok(age) = SystemTime::now().duration_since(modified) else {
        return "à l'instant".into();
    };
    let secs = age.as_secs();
    match secs {
        0..=59 => "à l'instant".into(),
        60..=3599 => format!("il y a {} min", secs / 60),
        3600..=86_399 => format!("il y a {} h", secs / 3600),
        86_400..=2_591_999 => format!("il y a {} j", secs / 86_400),
        _ => format!("il y a {} mois", secs / 2_592_000),
    }
}