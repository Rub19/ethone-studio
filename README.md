# Ethone Studio

Neuf apps de création pour Windows : photo, dessin vectoriel, montage, animation, mise en page, PDF et son. Gratuites, sans abonnement, sans compte. Tout reste sur ton PC.

**Site et téléchargements : [studio.ethone.dev](https://studio.ethone.dev)**

| App | Pour | Fichiers Adobe lus |
|---|---|---|
| **Pixel** | Retouche photo et compositing par calques | PSD, ACO |
| **Lumen** | Développement RAW et catalogue photo | XMP (préréglages Lightroom), CUBE |
| **Vector** | Dessin vectoriel, logos et typographie | ASE |
| **Cut** | Montage vidéo multipiste | médias PRPROJ |
| **Motion** | Animation, titres et effets visuels | |
| **Page** | Mise en page, magazines et export PDF | IDML |
| **Folio** | PDF : lecture, annotation, OCR, signature, organisation | AI (compatible PDF) |
| **Sound** | Édition et nettoyage audio | |
| **Ethone Studio** | Le lanceur : installe, met à jour et ouvre les apps | |

## Installer

Sur la page des versions GitHub (ou sur le site) :

- `Ethone-Studio-Setup.exe` : le lanceur, qui installe ensuite les apps de ton choix.
- `<App>-Setup.exe` : une seule app, sans le lanceur.
- `<App>-Web-Setup.exe` : un petit fichier qui télécharge la dernière version et vérifie son empreinte SHA-256.

L'installation se fait pour ton compte Windows, sans droits administrateur. `/S` installe sans fenêtre, `--uninstall` désinstalle (`--purge` efface aussi les préférences). Les apps ne sont pas encore signées : si Windows affiche « Windows a protégé votre ordinateur », choisis « Informations complémentaires » puis « Exécuter quand même ».

## Ce qu'elles ont en commun

- **Ctrl+K** : une palette pour toutes les commandes, outils, looks et réglages, en français.
- **Ctrl+,** : les préférences (taille de l'interface, couleur d'accent, coins, historique, sections propres à chaque app).
- **Envoyer vers** : les apps se passent les fichiers entre elles en un clic.
- **Hors ligne** : pas de compte ni de cloud. La seule IA, facultative et locale, sert à retirer un filigrane si tu le demandes.

## Compiler

Rust 2024 (stable), Windows 10 ou 11.

```bash
cargo build --release -p pixel
```

Chaque app est un paquet du dossier `apps/` (`pixel`, `lumen`, `vector`, `cut`, `motion`, `page`, `folio`, `sound`, et `ethone-studio` pour le lanceur). Le thème et les composants partagés sont dans `crates/studio-ui`, les effets dans `crates/studio-fx`. Les icônes se régénèrent avec `cargo run -p studio-ui --example gen_icons`.

Une version se publie en poussant un tag `vX.Y.Z` : GitHub Actions compile les apps, prépare les installateurs et le manifeste lu par le lanceur.

## Documentation

- [Nouveautés](docs/NOUVEAUTES.md)
- [Feuille de route](ROADMAP.md)
- Site : dossier `site/` (Cloudflare Workers, `npx wrangler deploy`)
