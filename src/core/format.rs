//! Mise en forme des valeurs pour l'affichage (tailles, dates, familles de fichiers).

use std::time::SystemTime;

/// Famille de fichier d'après son extension (en minuscules) : pilote la couleur de l'icône.
pub fn file_family(ext: &str) -> &'static str {
    match ext {
        "parquet" | "csv" | "tsv" | "arrow" | "feather" | "json" | "jsonl" | "xlsx" | "xls" => "data",
        "pdf" => "pdf",
        "py" | "rs" | "sql" | "ipynb" | "js" | "ts" | "toml" | "yaml" | "yml" => "code",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => "image",
        _ => "doc",
    }
}

/// 1536 -> "1,5 Ko"
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["o", "Ko", "Mo", "Go", "To"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} o")
    } else {
        format!("{value:.1} {}", UNITS[unit]).replace('.', ",")
    }
}

/// Date de modification relative : "il y a 3 j"
pub fn format_age(modified: SystemTime) -> String {
    let Ok(age) = SystemTime::now().duration_since(modified) else {
        return "à l'instant".into();
    };
    let secs = age.as_secs();
    match secs {
        0..=59 => "à l'instant".into(),
        60..=3599 => format!("il y a {} min", secs / 60),
        3600..=86_399 => format!("il y a {} h", secs / 3600),
        86_400..=2_591_999 => format!("il y a {} j", secs / 86_400),
        _ => format!("il y a {} mois", secs / 2_592_000),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn tailles() {
        assert_eq!(format_size(512), "512 o");
        assert_eq!(format_size(1536), "1,5 Ko");
        assert_eq!(format_size(5 * 1024 * 1024), "5,0 Mo");
    }

    #[test]
    fn ages() {
        let now = SystemTime::now();
        assert_eq!(format_age(now), "à l'instant");
        assert_eq!(format_age(now - Duration::from_secs(7200)), "il y a 2 h");
    }

    #[test]
    fn familles() {
        assert_eq!(file_family("parquet"), "data");
        assert_eq!(file_family("rs"), "code");
        assert_eq!(file_family("inconnu"), "doc");
    }
}
