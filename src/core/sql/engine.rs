//! Exécution des requêtes SQL en arrière-plan.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use datafusion::prelude::SessionContext;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

use super::register::{TABLE_NAME, register_file};
use crate::core::data::{ColumnSummary, DataError, DataResult, FileFormat, Row, batches_to_rows};

/// Résultat d'une requête.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<ColumnSummary>,
    pub rows: Vec<Row>,
    /// Vrai si le résultat dépassait `max_rows` et a été coupé.
    pub truncated: bool,
    pub elapsed: Duration,
}

/// Possède le runtime tokio sur lequel tournent les requêtes.
pub struct SqlEngine {
    runtime: Runtime,
}

impl SqlEngine {
    pub fn new() -> std::io::Result<Self> {
        let runtime = Builder::new_multi_thread().enable_all().build()?;
        Ok(SqlEngine { runtime })
    }

    /// Lance la requête sur le runtime. Le JoinHandle peut être attendu depuis
    /// n'importe quel exécuteur (par exemple `slint::spawn_local`).
    pub fn spawn(&self, path: PathBuf, sql: String, max_rows: usize) -> JoinHandle<DataResult<QueryResult>> {
        self.runtime.spawn(async move { run_query(&path, &sql, max_rows).await })
    }
}

/// Exécute `sql` sur le fichier `path`, exposé comme la table `t`.
pub async fn run_query(path: &Path, sql: &str, max_rows: usize) -> DataResult<QueryResult> {
    let format = FileFormat::detect(path)
        .ok_or_else(|| DataError::Unsupported(path.display().to_string()))?;
    let started = Instant::now();

    let ctx = SessionContext::new();
    register_file(&ctx, TABLE_NAME, path, format).await?;

    let df = ctx.sql(sql).await?;
    let columns = df
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

    // Une ligne de plus que la limite pour savoir si le résultat a été coupé.
    let batches = df.limit(0, Some(max_rows + 1))?.collect().await?;
    let mut rows = batches_to_rows(&batches)?;
    let truncated = rows.len() > max_rows;
    rows.truncate(max_rows);

    Ok(QueryResult { columns, rows, truncated, elapsed: started.elapsed() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn csv_file() -> PathBuf {
        let path = std::env::temp_dir().join("strata_sql_test.csv");
        fs::write(&path, "region,montant\nGrand Est,10\nGrand Est,5\nBretagne,7\n").unwrap();
        path
    }

    #[test]
    fn agregation_sur_un_csv() {
        let engine = SqlEngine::new().unwrap();
        let sql = "SELECT region, SUM(montant) AS total FROM t GROUP BY region ORDER BY total DESC";
        let result = engine.runtime.block_on(run_query(&csv_file(), sql, 100)).unwrap();

        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows[0][0].as_deref(), Some("Grand Est"));
        assert_eq!(result.rows[0][1].as_deref(), Some("15"));
        assert!(!result.truncated);
    }

    #[test]
    fn resultat_coupe() {
        let engine = SqlEngine::new().unwrap();
        let result = engine.runtime.block_on(run_query(&csv_file(), "SELECT * FROM t", 2)).unwrap();
        assert_eq!(result.rows.len(), 2);
        assert!(result.truncated);
    }

    #[test]
    fn requete_sur_un_parquet() {
        use arrow::array::Int64Array;
        use arrow::datatypes::{DataType, Field, Schema};
        use arrow::record_batch::RecordBatch;
        use parquet::arrow::ArrowWriter;
        use std::sync::Arc;

        let path = std::env::temp_dir().join("strata_sql_test.parquet");
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int64, false)]));
        let batch = RecordBatch::try_new(schema.clone(), vec![Arc::new(Int64Array::from(vec![1, 2, 3]))]).unwrap();
        let mut writer = ArrowWriter::try_new(fs::File::create(&path).unwrap(), schema, None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();

        let engine = SqlEngine::new().unwrap();
        let result = engine.runtime.block_on(run_query(&path, "SELECT SUM(x) AS s FROM t", 10)).unwrap();
        assert_eq!(result.rows[0][0].as_deref(), Some("6"));
    }

    #[test]
    fn erreur_sql_lisible() {
        let engine = SqlEngine::new().unwrap();
        let err = engine.runtime.block_on(run_query(&csv_file(), "SELECT colonne_absente FROM t", 10));
        assert!(err.is_err());
    }
}
