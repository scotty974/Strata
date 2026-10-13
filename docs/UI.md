# Interface Strata (Slint 1.18)

L'interface ne contient aucune logique métier. Elle lit des propriétés et déclenche des callbacks
regroupés dans 4 globals (`ui/state.slint`). Tout le branchement se fait côté Rust.

## Arborescence

```
ui/
  app-window.slint        fenêtre `Home` (no-frame), raccourcis, pages, poignées de redimensionnement
  theme.slint             Theme (couleurs clair/sombre, typo, espacements), Icons, Format
  types.slint             structs et enums partagés avec Rust
  state.slint             le contrat : AppState, HomeState, StudioState, SearchState
  components/
    primitives.slint      Icon, Button, Chip, Segmented, Badge, Kbd, FileGlyph, TextField...
    title_bar_custom.slint  barre de titre (WindowMoveArea, boutons système, recherche)
    side-bar.slint        navigation (modifie AppState.page)
    search_bar.slint      champ de la page Recherche
    common.slint          FileTabs, DropZone, EmptyState, StatusBar, ErrorBanner
    data_grid.slint       grille typée virtualisée (ListView)
    schema_panel.slint    panneau Schéma
    sql_editor.slint      éditeur SQL + Exécuter
  pages/
    home_page.slint       dépôt + fichiers récents
    studio_page.slint     onglets, schéma, SQL, Données / Schéma / Métadonnées
    search_page.slint     recherche dans l'index
    placeholder_page.slint  Agent, Paramètres (à venir)
  dev/preview.slint       aperçu avec données fictives (non compilé par build.rs)
```

## Aperçu sans Rust

Ouvrir `ui/dev/preview.slint` dans l'aperçu Slint de VS Code (extension Slint). La propriété
`start-page` choisit l'écran affiché.

## Le contrat (ui/state.slint)

Accès en Rust : `ui.global::<StudioState>()`. Les noms passent en snake_case
(`null-ratio` devient `null_ratio`, `run-query` devient `on_run_query`).

### AppState

| Nom | Sens | Rôle |
| --- | --- | --- |
| `page` | in-out | Écran courant (`Page::Home`, `Studio`, `Search`, `Agent`, `Settings`) |
| `status-text`, `engine-text` | in | Textes de la barre d'état (gauche, droite) |
| `busy` | in | Point animé dans la barre d'état |
| `error-text` | in-out | Bandeau d'erreur sous la barre de titre ; l'utilisateur peut le fermer |
| `open-file-dialog()` | callback | Bouton « Ouvrir un fichier », « Parcourir… », `+` des onglets, Ctrl+O |
| `open-path(path)` | callback | Clic sur un fichier récent |
| `search-requested()` | callback | Ctrl+K ou recherche de la barre de titre (la page Recherche s'ouvre déjà côté UI) |
| `start-resize(edge)` | callback | Bords de la fenêtre : `"n"`, `"s"`, `"e"`, `"w"`, `"ne"`, `"nw"`, `"se"`, `"sw"` |

### HomeState

| Nom | Sens | Rôle |
| --- | --- | --- |
| `recent-files` | in | `[RecentFile]` (tailles et dates déjà formatées) |
| `drop-hover` | in | Met la zone de dépôt en surbrillance |
| `clear-recent()` | callback | Bouton « Effacer » |

### StudioState

| Nom | Sens | Rôle |
| --- | --- | --- |
| `tabs`, `active-tab` | in / in-out | Fichiers ouverts. Aucun onglet = zone de dépôt |
| `tab-selected(i)`, `tab-closed(i)` | callback | L'UI met déjà `active-tab` à jour au clic |
| `view` | in-out | `StudioView::Data`, `Schema`, `Metadata` |
| `columns` | in | `[ColumnInfo]` : `name`, `data_type`, `nullable`, `null_ratio` (0 à 1), `summary` |
| `file-rows` | in | Nombre de lignes du fichier (pied du panneau Schéma) |
| `grid-columns` | in | Colonnes de la grille : celles du fichier, ou celles du résultat SQL |
| `showing-query` | in | Vrai quand la grille affiche un résultat SQL (puce « Résultat SQL ») |
| `query-cleared()` | callback | Croix de la puce « Résultat SQL » : revenir aux lignes du fichier |
| `selected-column`, `selected-row` | in-out | Sélection partagée entre schéma et grille |
| `metadata` | in | `[KeyValue]` pour la vue Métadonnées |
| `rows` | in | `[[string]]` : uniquement la page courante, une cellule texte par colonne |
| `null-marker` | in | Une cellule égale à cette valeur s'affiche `null` en orange |
| `total-rows`, `page-index`, `page-size`, `page-count` | in | Pagination de ce qu'affiche la grille (`page-index` commence à 0) |
| `page-requested(i)` | callback | Flèches précédente / suivante |
| `query` | in-out | Texte de l'éditeur SQL |
| `query-running`, `query-error`, `query-info` | in | État de la requête |
| `run-query(sql)` | callback | Bouton Exécuter ou Ctrl+Entrée |
| `export-requested(format)` | callback | `"csv"` ou `"parquet"` |

### SearchState

| Nom | Sens | Rôle |
| --- | --- | --- |
| `query` | in-out | Texte saisi |
| `results` | in | `[SearchHit]` ; `family` pilote la couleur : `data`, `doc`, `pdf`, `code`, `image` |
| `info` | in | Ex. « 38 résultats · 24 ms » |
| `indexing`, `index-text` | in | Statut affiché en bas de la sidebar |
| `query-changed(text)` | callback | À chaque frappe |
| `result-opened(i)` | callback | Double-clic sur un résultat |

### Theme

`Theme.dark` suit le thème système ; `ui.global::<Theme>().set_dark(true)` le force.

## Exemple de branchement

Vérifié avec `cargo check` sur ce projet (il n'est pas ajouté au code) :

```rust
use slint::{ModelRc, SharedString, VecModel};

let studio = ui.global::<StudioState>();
studio.set_tabs(ModelRc::new(VecModel::from(vec![FileTab {
    name: "ventes.parquet".into(),
    path: r"C:\data\ventes.parquet".into(),
    kind: "parquet".into(),
}])));
studio.set_active_tab(0);
studio.set_columns(ModelRc::new(VecModel::from(vec![ColumnInfo {
    name: "order_id".into(),
    data_type: "int64".into(),
    nullable: false,
    null_ratio: 0.0,
    summary: "".into(),
}])));

// Lignes : une ModelRc par ligne. Pour une valeur nulle, utiliser studio.get_null_marker().
let rows: Vec<ModelRc<SharedString>> = vec![
    ModelRc::new(VecModel::from(vec![SharedString::from("1048211")])),
];
studio.set_rows(ModelRc::new(VecModel::from(rows)));
studio.set_total_rows(1);
studio.set_page_count(1);

let weak = ui.as_weak();
studio.on_run_query(move |sql| {
    let ui = weak.unwrap();
    ui.global::<StudioState>().set_query_info(format!("{} caractères", sql.len()).into());
});

ui.global::<AppState>().on_open_file_dialog(|| { /* sélecteur de fichier */ });
```

## Points à traiter côté Rust

- **Glisser-déposer depuis l'Explorateur** : dans le code du backend winit de Slint 1.18.1, je n'ai
  pas trouvé de relais des fichiers déposés par le système vers `DropArea`. L'UI expose donc
  `HomeState.drop-hover` et `AppState.open-path`, à piloter depuis les évènements winit
  `WindowEvent::HoveredFile`, `HoveredFileCancelled` et `DroppedFile`, par exemple via
  `WinitWindowAccessor::on_winit_window_event` (feature `unstable-winit-030`).
- **Redimensionnement** : `AppState.start-resize(edge)` est à brancher sur
  `drag_resize_window()` (feature `unstable-winit-030`). Sans branchement, les bords ne font rien.
- **Polices** : Inter et JetBrains Mono doivent être installées, sinon Slint utilise la police
  système. Pour les embarquer : placer les .ttf dans `ui/fonts/` et ajouter
  `import "fonts/Inter.ttf";` en tête de `app-window.slint`.
- **Taille de fenêtre** : `Home` déclare `preferred-width: 1280px` / `preferred-height: 800px` et un
  minimum de 560 x 420. Le `set_size(600, 800)` de `main.rs` reste prioritaire au démarrage.
