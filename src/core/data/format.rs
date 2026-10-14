//! Formats de fichiers de données reconnus par Strata.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    Parquet,
    Csv,
    Tsv,
    /// JSON délimité par lignes (un objet par ligne).
    Json,
    /// Arrow IPC (fichier).
    Arrow,
}

impl FileFormat {
    /// Format d'après l'extension du fichier, `None` si non pris en charge.
    pub fn detect(path: &Path) -> Option<FileFormat> {
        let ext = path.extension()?.to_string_lossy().to_lowercase();
        match ext.as_str() {
            "parquet" | "pq" => Some(FileFormat::Parquet),
            "csv" => Some(FileFormat::Csv),
            "tsv" => Some(FileFormat::Tsv),
            "json" | "jsonl" | "ndjson" => Some(FileFormat::Json),
            "arrow" | "feather" | "ipc" => Some(FileFormat::Arrow),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FileFormat::Parquet => "Parquet",
            FileFormat::Csv => "CSV",
            FileFormat::Tsv => "TSV",
            FileFormat::Json => "JSON (une ligne par objet)",
            FileFormat::Arrow => "Arrow IPC",
        }
    }
}

/// Extensions proposées dans le sélecteur de fichiers.
pub const SUPPORTED_EXTENSIONS: &[&str] =
    &["parquet", "pq", "csv", "tsv", "json", "jsonl", "ndjson", "arrow", "feather", "ipc"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection() {
        assert_eq!(FileFormat::detect(Path::new("a.PARQUET")), Some(FileFormat::Parquet));
        assert_eq!(FileFormat::detect(Path::new("a.csv")), Some(FileFormat::Csv));
        assert_eq!(FileFormat::detect(Path::new("a.pdf")), None);
        assert_eq!(FileFormat::detect(Path::new("sans_extension")), None);
    }
}
