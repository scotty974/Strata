//! Lecture des fichiers de données. Parquet pour l'instant ; CSV et Arrow viendront ici.

mod error;
mod parquet_file;
mod types;

pub use error::DataResult;
pub use parquet_file::{read_rows, read_summary};
pub use types::{ColumnSummary, FileSummary, Row};
