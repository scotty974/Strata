//! Lecture des fichiers de données : Parquet en natif, les autres formats via DataFusion.

mod error;
mod format;
mod parquet_file;
mod rows;
mod tabular;
mod types;

use std::path::Path;

pub use error::{DataError, DataResult};
pub use format::{FileFormat, SUPPORTED_EXTENSIONS};
pub use rows::batches_to_rows;
pub use types::{ColumnSummary, FileSummary, Row};

/// Schéma, nombre de lignes et métadonnées du fichier.
pub fn read_summary(path: &Path) -> DataResult<FileSummary> {
    match detect(path)? {
        FileFormat::Parquet => parquet_file::read_summary(path),
        format => tabular::read_summary(path, format),
    }
}

/// `limit` lignes à partir de `offset`, cellules converties en texte.
pub fn read_rows(path: &Path, offset: usize, limit: usize) -> DataResult<Vec<Row>> {
    match detect(path)? {
        FileFormat::Parquet => parquet_file::read_rows(path, offset, limit),
        format => tabular::read_rows(path, format, offset, limit),
    }
}

fn detect(path: &Path) -> DataResult<FileFormat> {
    FileFormat::detect(path).ok_or_else(|| {
        let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
        DataError::Unsupported(if ext.is_empty() { "sans extension".into() } else { format!(".{ext}") })
    })
}
