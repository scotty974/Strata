//! Conversion des RecordBatch Arrow en lignes de texte pour la grille.

use arrow::array::Array;
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use arrow::util::display::{ArrayFormatter, FormatOptions};

use super::Row;

/// Ajoute à `rows` les lignes du batch, chaque cellule en texte (`None` = nul).
pub fn push_batch_rows(batch: &RecordBatch, rows: &mut Vec<Row>) -> Result<(), ArrowError> {
    let options = FormatOptions::default();
    let columns = batch.columns();
    let formatters = columns
        .iter()
        .map(|c| ArrayFormatter::try_new(c.as_ref(), &options))
        .collect::<Result<Vec<_>, _>>()?;

    for r in 0..batch.num_rows() {
        let row = columns
            .iter()
            .zip(&formatters)
            .map(|(col, fmt)| {
                if col.is_null(r) {
                    None
                } else {
                    Some(fmt.value(r).to_string())
                }
            })
            .collect();
        rows.push(row);
    }
    Ok(())
}

/// Convertit une liste de batches en lignes.
pub fn batches_to_rows(batches: &[RecordBatch]) -> Result<Vec<Row>, ArrowError> {
    let mut rows = Vec::new();
    for batch in batches {
        push_batch_rows(batch, &mut rows)?;
    }
    Ok(rows)
}
