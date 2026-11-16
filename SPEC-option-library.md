# Spec: bibliothèque de 3 000 options

## Objective

Compléter les icônes de la palette de commandes commune et ajouter exactement 3 000 variantes utiles des effets existants. Une variante combine un effet pris en charge avec un niveau d'intensité ; elle doit être recherchable et exécutable dans toutes les applications qui savent déjà recevoir des effets.

## Tech Stack

Rust 2024, egui/eframe, `studio-ui` et `studio-fx`. Aucune nouvelle dépendance.

## Commands

- Build : `cargo check --workspace`
- Tests ciblés : `cargo test -p studio-ui` et `cargo test -p studio-fx`
- Tests complets : `cargo test --workspace`
- Format ciblé : `rustfmt --edition 2024 --check <fichiers modifiés>`

## Project Structure

- `crates/studio-ui/src/palette.rs` : affichage, recherche et bibliothèque globale des commandes.
- `crates/studio-fx/src/stack.rs` : modèle d'effet non destructif et décodage des variantes.
- `apps/*/src/main.rs` : application de la commande selon le document actif.
- `tasks/` : plan et suivi de l'initiative.

## Code Style

```rust
if let Some(effect) = studio_ui::palette::effect_from_command(id) {
    target.effects.push(effect);
}
```

Les identifiants sont stables, les fonctions de génération sont déterministes et les valeurs sont bornées.

## Testing Strategy

- Tests unitaires pour le nombre exact de variantes, l'unicité des identifiants et le décodage des intensités.
- Tests du workspace pour prévenir les régressions d'intégration.
- Pas de test visuel automatisé : l'utilisateur a demandé de ne pas ouvrir les applications.

## Boundaries

- Toujours : commandes exécutables, identifiants uniques, intensité bornée, aucune dépendance nouvelle.
- Demander d'abord : changement de format de projet ou de fichier utilisateur.
- Jamais : 3 000 boutons visibles simultanément, commandes factices, ouverture automatique d'une application.

## Success Criteria

- La palette contient exactement 3 000 variantes supplémentaires.
- Chaque variante possède un identifiant unique et se décode vers un effet valide.
- Chaque résultat de la palette affiche une icône de catégorie.
- Pixel, Vector, Motion, Cut, Lumen et Page exécutent les variantes.
- `cargo test --workspace` et `cargo check --workspace` réussissent.

## Open Questions

Aucune : l'utilisateur délègue le choix des options « must do ».
