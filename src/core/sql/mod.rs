//! Moteur SQL (DataFusion) : le fichier actif est exposé comme la table `t`.

mod engine;
mod register;

pub use engine::{QueryResult, SqlEngine};
pub use register::{TABLE_NAME, register_file};
