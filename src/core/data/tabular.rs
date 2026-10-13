//! Lecture des formats autres que Parquet (CSV, TSV, JSON, Arrow) via DataFusion.

use std::future::Future;
use std::path::Path;

use datafusion::prelude::SessionContext;
use tokio::runtime::Builder;

use super::{ColumnSummary, DataResult, FileFormat, FileSummary, Row, batches_to_rows};
use crate::core::sql::{TABLE_NAME, register_file};

/// Exécute une tâche async DataFusion depuis du code synchrone.
fn block_on<F: Future>(future: F) -> DataResult<F::Output> {
    let runtime = Builder::new_current_thread().enable_all().build()?;
    Ok(runtime.block_on(future))
}

async fn context(path: &Path, format: FileFormat) -> DataResult<SessionContext> {
    let ctx = SessionContext::new();
    register_file(&ctx, TABLE_NAME, path, format).await?;
    Ok(ctx)
}

/// Schéma, nombre de lignes et métadonnées. Le comptage relit tout le fichier.
pub fn read_summary(path: &Path, format: FileFormat) -> DataResult<FileSummary> {
    let file_size = std::fs::metadata(path)?.len();

    block_on(async {
        let ctx = context(path, format).await?;
        let df = ctx.table(TABLE_NAME).await?;

        let columns: Vec<ColumnSummary> = df
            .schema()
            .fields()
            .iter()
            .map(|f| ColumnSummary {
                name: f.name().clone(),
                data_type: f.data_type().to_string(),
                nullable: f.is_nullable(),
                null_ratio: None,
            })
            .collect();
        let num_rows = df.count().await?;

        let metadata = vec![
            ("Format".into(), format.label().to_string()),
            ("Lignes".into(), num_rows.to_string()),
            ("Colonnes".into(), columns.len().to_string()),
            ("Taille du fichier".into(), format!("{file_size} octets")),
            ("Types".into(), "déduits des premières lignes".into()),
        ];

        Ok(FileSummary { num_rows, columns, metadata })
    })?
}

/// `limit` lignes à partir de `offset`.
pub fn read_rows(path: &Path, format: FileFormat, offset: usize, limit: usize) -> DataResult<Vec<Row>> {
    block_on(async {
        let ctx = context(path, format).await?;
        let batches = ctx.table(TABLE_NAME).await?.limit(offset, Some(limit))?.collect().await?;
        Ok(batches_to_rows(&batches)?)
    })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn csv_schema_et_pages() {
        let path = std::env::temp_dir().join("strata_tabular_test.csv");
        fs::write(&path, "id,nom\n1,a\n2,b\n3,\n").unwrap();

        let summary = read_summary(&path, FileFormat::Csv).unwrap();
        assert_eq!(summary.num_rows, 3);
        assert_eq!(summary.columns.len(), 2);

        let rows = read_rows(&path, FileFormat::Csv, 1, 5).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][1].as_deref(), Some("b"));
        assert_eq!(rows[1][1], None);
    }
}
