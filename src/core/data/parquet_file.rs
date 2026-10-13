//! Lecture Parquet : schéma, métadonnées et pages de lignes.

use std::fs::File;
use std::path::Path;

use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use super::rows::push_batch_rows;
use super::{ColumnSummary, DataResult, FileSummary, Row};

/// Lit le schéma et les métadonnées (ne lit que le pied de page du fichier : rapide).
pub fn read_summary(path: &Path) -> DataResult<FileSummary> {
    let file = File::open(path)?;
    let file_size = file.metadata()?.len();
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;

    let meta = builder.metadata();
    let file_meta = meta.file_metadata();
    let num_rows = file_meta.num_rows().max(0) as usize;

    // Nulls par colonne feuille, additionnés sur tous les row groups.
    let leaf_count = file_meta.schema_descr().num_columns();
    let mut null_counts: Vec<Option<u64>> = vec![Some(0); leaf_count];
    for rg in meta.row_groups() {
        for (i, col) in rg.columns().iter().enumerate() {
            let count = col.statistics().and_then(|s| s.null_count_opt());
            null_counts[i] = match (null_counts[i], count) {
                (Some(total), Some(n)) => Some(total + n),
                _ => None,
            };
        }
    }

    let fields = builder.schema().fields();
    // Les statistiques sont par colonne feuille : on ne les associe aux colonnes
    // que si le schéma est plat (pas de struct / list imbriquée).
    let flat = fields.len() == leaf_count;

    let columns = fields
        .iter()
        .enumerate()
        .map(|(i, field)| ColumnSummary {
            name: field.name().clone(),
            data_type: field.data_type().to_string(),
            nullable: field.is_nullable(),
            null_ratio: if flat && num_rows > 0 {
                null_counts[i].map(|n| n as f32 / num_rows as f32)
            } else {
                None
            },
        })
        .collect();

    let compressed: i64 = meta.row_groups().iter().map(|rg| rg.compressed_size()).sum();
    let compression = meta
        .row_groups()
        .first()
        .and_then(|rg| rg.columns().first())
        .map(|c| format!("{:?}", c.compression()))
        .unwrap_or_else(|| "-".into());

    let metadata = vec![
        ("Format".into(), format!("Parquet (version {})", file_meta.version())),
        ("Créé par".into(), file_meta.created_by().unwrap_or("-").to_string()),
        ("Lignes".into(), num_rows.to_string()),
        ("Colonnes".into(), fields.len().to_string()),
        ("Row groups".into(), meta.num_row_groups().to_string()),
        ("Compression".into(), compression),
        ("Taille du fichier".into(), format!("{file_size} octets")),
        ("Données compressées".into(), format!("{compressed} octets")),
    ];

    Ok(FileSummary { num_rows, columns, metadata })
}

/// Lit `limit` lignes à partir de `offset`, chaque cellule convertie en texte.
pub fn read_rows(path: &Path, offset: usize, limit: usize) -> DataResult<Vec<Row>> {
    let file = File::open(path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)?
        .with_offset(offset)
        .with_limit(limit)
        .with_batch_size(limit.max(1))
        .build()?;

    let mut rows = Vec::with_capacity(limit);
    for batch in reader {
        push_batch_rows(&batch?, &mut rows)?;
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::error::DataError;
    use std::sync::Arc;

    use arrow::array::{Float64Array, Int64Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use parquet::arrow::ArrowWriter;

    /// Crée un petit Parquet de 10 lignes dans le dossier temporaire.
    fn sample_file(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("region", DataType::Utf8, false),
            Field::new("montant", DataType::Float64, true),
        ]));
        let ids = Int64Array::from((1..=10).collect::<Vec<i64>>());
        let regions = StringArray::from(vec!["Grand Est"; 10]);
        let montants = Float64Array::from(
            (1..=10).map(|i| if i % 5 == 0 { None } else { Some(i as f64 * 1.5) }).collect::<Vec<_>>(),
        );
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(ids), Arc::new(regions), Arc::new(montants)],
        )
        .unwrap();

        let mut writer = ArrowWriter::try_new(File::create(&path).unwrap(), schema, None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
        path
    }

    #[test]
    fn lit_le_schema_et_les_nulls() {
        let path = sample_file("strata_summary_test.parquet");
        let summary = read_summary(&path).unwrap();

        assert_eq!(summary.num_rows, 10);
        assert_eq!(summary.columns.len(), 3);
        assert_eq!(summary.columns[0].name, "id");
        assert_eq!(summary.columns[0].data_type, "Int64");
        assert!(summary.columns[2].nullable);
        assert_eq!(summary.columns[2].null_ratio, Some(0.2));
    }

    #[test]
    fn lit_une_page_avec_offset() {
        let path = sample_file("strata_rows_test.parquet");
        let rows = read_rows(&path, 3, 4).unwrap();

        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0][0].as_deref(), Some("4"));
        assert_eq!(rows[1][2], None); // ligne 5 : montant nul
    }

    #[test]
    fn fichier_absent_renvoie_une_erreur() {
        let err = read_summary(Path::new("n_existe_pas.parquet")).unwrap_err();
        assert!(matches!(err, DataError::Io(_)));
    }
}
