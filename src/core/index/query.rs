//! Recherche par nom dans l'index.

use std::path::PathBuf;

use super::FileIndex;

/// Résultat d'une recherche : les premiers chemins trouvés et le total.
#[derive(Debug, Default)]
pub struct SearchOutcome {
    pub paths: Vec<PathBuf>,
    pub total: usize,
}

/// Fichiers dont le nom contient `query` (insensible à la casse), au plus `limit` chemins.
pub fn search(index: &FileIndex, query: &str, limit: usize) -> SearchOutcome {
    let query = query.trim().to_lowercase();
    let mut outcome = SearchOutcome::default();
    if query.is_empty() {
        return outcome;
    }

    for file in index.iter().filter(|f| f.name_lower.contains(&query)) {
        outcome.total += 1;
        if outcome.paths.len() < limit {
            outcome.paths.push(file.path.clone());
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::index::IndexedFile;

    fn index() -> FileIndex {
        ["ventes_t3.parquet", "ventes_t4.parquet", "clients.csv"]
            .iter()
            .map(|n| IndexedFile { name_lower: n.to_string(), path: PathBuf::from(n) })
            .collect()
    }

    #[test]
    fn trouve_sans_tenir_compte_de_la_casse() {
        let outcome = search(&index(), "VENTES", 10);
        assert_eq!(outcome.total, 2);
    }

    #[test]
    fn respecte_la_limite_mais_compte_tout() {
        let outcome = search(&index(), "ventes", 1);
        assert_eq!(outcome.paths.len(), 1);
        assert_eq!(outcome.total, 2);
    }

    #[test]
    fn requete_vide() {
        assert_eq!(search(&index(), "  ", 10).total, 0);
    }
}
