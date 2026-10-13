//! Conversions des types de `core` vers les types Slint.

use std::path::Path;

use slint::{ModelRc, SharedString, VecModel};

use crate::core::data::{FileSummary, Row};
use crate::{ColumnInfo, FileTab, KeyValue};

pub fn columns(summary: &FileSummary) -> ModelRc<ColumnInfo> {
    let items: Vec<ColumnInfo> = summary
        .columns
        .iter()
        .map(|c| ColumnInfo {
            name: c.name.clone().into(),
            data_type: c.data_type.clone().into(),
            nullable: c.nullable,
            null_ratio: c.null_ratio.unwrap_or(0.0),
            summary: if c.null_ratio.is_none() { "nuls inconnus".into() } else { "".into() },
        })
        .collect();
    ModelRc::new(VecModel::from(items))
}

pub fn metadata(summary: &FileSummary) -> ModelRc<KeyValue> {
    let items: Vec<KeyValue> = summary
        .metadata
        .iter()
        .map(|(k, v)| KeyValue { key: k.into(), value: v.into() })
        .collect();
    ModelRc::new(VecModel::from(items))
}

/// Lignes de la grille : `None` devient le marqueur de valeur nulle attendu par l'interface.
pub fn rows(rows: Vec<Row>, null_marker: &SharedString) -> ModelRc<ModelRc<SharedString>> {
    let items: Vec<ModelRc<SharedString>> = rows
        .into_iter()
        .map(|row| {
            let cells: Vec<SharedString> = row
                .into_iter()
                .map(|cell| cell.map(SharedString::from).unwrap_or_else(|| null_marker.clone()))
                .collect();
            ModelRc::new(VecModel::from(cells))
        })
        .collect();
    ModelRc::new(VecModel::from(items))
}

pub fn tab(path: &Path) -> FileTab {
    FileTab {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
            .into(),
        path: path.display().to_string().into(),
        kind: path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
            .into(),
    }
}
