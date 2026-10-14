//! Préparation des aperçus : quel type d'aperçu pour quel fichier, lecture du texte.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use crate::core::data::FileFormat;

/// Façon d'afficher un fichier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    /// Fichier de données : ouvert dans le Studio.
    Data,
    Text,
    Image,
    Pdf,
    /// Pas d'aperçu : ouvert avec l'application par défaut.
    Other,
}

const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "markdown", "log", "rs", "py", "sql", "js", "ts", "jsx", "tsx", "html", "htm", "css",
    "xml", "yaml", "yml", "toml", "ini", "cfg", "conf", "sh", "ps1", "bat", "c", "h", "cpp", "hpp",
    "java", "kt", "go", "rb", "php", "r", "slint", "env", "gitignore", "dockerfile",
];
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "svg", "webp"];

pub fn doc_kind(path: &Path) -> DocKind {
    if FileFormat::detect(path).is_some() {
        return DocKind::Data;
    }
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext == "pdf" {
        DocKind::Pdf
    } else if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        DocKind::Image
    } else if TEXT_EXTENSIONS.contains(&ext.as_str()) {
        DocKind::Text
    } else {
        DocKind::Other
    }
}

/// Lit au plus `max_bytes` octets en texte. Erreur `InvalidData` si le fichier semble binaire.
/// Renvoie le texte et `true` s'il a été coupé.
pub fn read_text(path: &Path, max_bytes: usize) -> io::Result<(String, bool)> {
    let file = File::open(path)?;
    let total = file.metadata()?.len() as usize;
    let mut buffer = Vec::with_capacity(max_bytes.min(total));
    file.take(max_bytes as u64).read_to_end(&mut buffer)?;

    // Un octet nul dans le début du fichier : on considère que c'est du binaire.
    if buffer.iter().take(8192).any(|&b| b == 0) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "fichier binaire"));
    }

    let mut text = String::from_utf8_lossy(&buffer).into_owned();
    // Les fins de ligne Windows s'affichent mal dans certains rendus : on les normalise.
    text = text.replace("\r\n", "\n");
    Ok((text, total > max_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn types_de_documents() {
        assert_eq!(doc_kind(Path::new("a.parquet")), DocKind::Data);
        assert_eq!(doc_kind(Path::new("notes.MD")), DocKind::Text);
        assert_eq!(doc_kind(Path::new("photo.jpg")), DocKind::Image);
        assert_eq!(doc_kind(Path::new("rapport.pdf")), DocKind::Pdf);
        assert_eq!(doc_kind(Path::new("archive.zip")), DocKind::Other);
    }

    #[test]
    fn texte_coupe_et_binaire() {
        let dir = std::env::temp_dir();
        let txt = dir.join("strata_preview.txt");
        fs::write(&txt, "ligne 1\r\nligne 2\r\n").unwrap();
        let (text, truncated) = read_text(&txt, 1000).unwrap();
        assert_eq!(text, "ligne 1\nligne 2\n");
        assert!(!truncated);
        assert!(read_text(&txt, 4).unwrap().1);

        let bin = dir.join("strata_preview.bin");
        fs::write(&bin, [0u8, 1, 2, 3]).unwrap();
        assert!(read_text(&bin, 1000).is_err());
    }
}
