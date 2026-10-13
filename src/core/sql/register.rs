//! Déclaration d'un fichier comme table DataFusion.

use std::path::Path;

use datafusion::error::Result;
use datafusion::execution::options::ArrowReadOptions;
use datafusion::prelude::{CsvReadOptions, JsonReadOptions, ParquetReadOptions, SessionContext};

use crate::core::data::FileFormat;

/// Nom de la table qui représente le fichier actif dans les requêtes.
pub const TABLE_NAME: &str = "t";

/// Enregistre `path` sous le nom `table` dans `ctx`, selon son format.
pub async fn register_file(
    ctx: &SessionContext,
    table: &str,
    path: &Path,
    format: FileFormat,
) -> Result<()> {
    let location = path.to_string_lossy();
    // DataFusion filtre les fichiers par extension : on lui donne l'extension réelle.
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    match format {
        FileFormat::Parquet => {
            let options = ParquetReadOptions::new().file_extension(&ext);
            ctx.register_parquet(table, location.as_ref(), options).await
        }
        FileFormat::Csv => {
            let options = CsvReadOptions::new().file_extension(&ext);
            ctx.register_csv(table, location.as_ref(), options).await
        }
        FileFormat::Tsv => {
            let options = CsvReadOptions::new().delimiter(b'\t').file_extension(&ext);
            ctx.register_csv(table, location.as_ref(), options).await
        }
        FileFormat::Json => {
            let options = JsonReadOptions::default().file_extension(&ext);
            ctx.register_json(table, location.as_ref(), options).await
        }
        FileFormat::Arrow => {
            let options = ArrowReadOptions { file_extension: &ext, ..Default::default() };
            ctx.register_arrow(table, location.as_ref(), options).await
        }
    }
}
