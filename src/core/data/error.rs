//! Erreurs de lecture des fichiers de données.

use std::fmt;

use arrow::error::ArrowError;
use datafusion::error::DataFusionError;
use parquet::errors::ParquetError;

#[derive(Debug)]
pub enum DataError {
    Io(std::io::Error),
    Parquet(ParquetError),
    Arrow(ArrowError),
    Sql(DataFusionError),
    /// Extension non prise en charge.
    Unsupported(String),
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataError::Io(e) => write!(f, "Impossible d'ouvrir le fichier : {e}"),
            DataError::Parquet(e) => write!(f, "Fichier Parquet illisible : {e}"),
            DataError::Arrow(e) => write!(f, "Erreur de conversion : {e}"),
            DataError::Sql(e) => write!(f, "{e}"),
            DataError::Unsupported(ext) => write!(f, "Format non pris en charge : {ext}"),
        }
    }
}

impl std::error::Error for DataError {}

impl From<std::io::Error> for DataError {
    fn from(e: std::io::Error) -> Self {
        DataError::Io(e)
    }
}
impl From<ParquetError> for DataError {
    fn from(e: ParquetError) -> Self {
        DataError::Parquet(e)
    }
}
impl From<ArrowError> for DataError {
    fn from(e: ArrowError) -> Self {
        DataError::Arrow(e)
    }
}
impl From<DataFusionError> for DataError {
    fn from(e: DataFusionError) -> Self {
        DataError::Sql(e)
    }
}

pub type DataResult<T> = Result<T, DataError>;
