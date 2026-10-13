//! Ce que la lecture d'un fichier renvoie, indépendamment du format.

/// Une colonne telle qu'affichée dans le schéma.
#[derive(Debug, Clone)]
pub struct ColumnSummary {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    /// Part de valeurs nulles (0.0 à 1.0) d'après les statistiques du fichier.
    /// `None` si le fichier ne les fournit pas.
    pub null_ratio: Option<f32>,
}

/// Ce qu'on sait d'un fichier sans lire ses lignes.
#[derive(Debug, Clone)]
pub struct FileSummary {
    pub num_rows: usize,
    pub columns: Vec<ColumnSummary>,
    /// Paires clé / valeur pour la vue Métadonnées.
    pub metadata: Vec<(String, String)>,
}

/// Une ligne : une cellule par colonne, `None` pour une valeur nulle.
pub type Row = Vec<Option<String>>;
