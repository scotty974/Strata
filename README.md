# Strata 

## Résumé 
Strata est une application de bureau en Rust pour ouvrir, inspecter et interroger en SQL des fichiers Parquet, Arrow, et CSV, sans serveur ni script. Le but premier est de démontrer des compétences en Rust et ingénierie des données dans un portfolio.

## Problème et Cible
Un ingénieur de données doit souvent vérifier vite le contenu d'un fichier produit par une pipeline: schéma, types, valeurs nulles, volumes. Aujourd'hui cela passe par un script Python, la ligne de commande ou un outils tiers.

**Cible principale** : ingénieurs et analyste qui manipulent des fichiers Parquet en local.

**Cible Secondaire** : développeurs qui débuguent la sortie d'une pipeline. La taille de cette cible n'est pas mesurée; 

## Concurrence
- ParqEdit
- ParquetViewer
- Tad
- DuckDB Local
- Tecton

## Différenciation
- Comparer deux fichiers Parquet (schéma et ligne) et afficher les écarts.
- Produire un rapport de qualité de données exportable (null, unicité, types).
- Proposer un binaire Rust léger avec démarrage rapide, face à une application Electron comme Tad.
- Ouvrir un fichier data gouv

## Périmètre du MVP
Inclus : 
- Ouvrir un fichier Parquet ou CSV par glisser-déposer
- Onglet schéma et métadonnées
- Grille paginée qui reste fluide sur de gros fichiers.
- Editeur SQL et résultats dans la grille.
- Export du résultas en CSV ou Parquet.

## Stacks
- Lang : Rust
- Lecture fichiers : parquet et arrow
- Moteur SQL : [DataFusion](https://datafusion.apache.org/) ou DuckDB, à définir
- Interface : [Tauri](https://tauri.app/)
