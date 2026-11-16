# Implementation Plan: bibliothèque d'options et icônes

## Overview

Étendre la palette commune sans encombrer les interfaces : les 3 000 variantes sont générées une fois, filtrées par recherche et appliquées par le modèle d'effet existant.

## Architecture Decisions

- Génération paresseuse dans `studio-ui` pour éviter 3 000 allocations à chaque image d'interface.
- Encodage stable `fxv:<effet>:<intensité>` et décodage centralisé.
- Réutilisation de `Fx.amount` afin que chaque option produise une différence réelle sans inventer de nouveaux algorithmes.

## Task List

### Phase 1: Fondation
- [x] Task 1: Générateur, décodeur et tests des 3 000 variantes.
- [x] Task 2: Icônes de catégorie dans chaque ligne de la palette.

### Checkpoint: Fondation
- [x] Tests ciblés et compilation de `studio-ui`.

### Phase 2: Intégration
- [x] Task 3: Intégrer Pixel, Vector et Motion.
- [x] Task 4: Intégrer Cut, Lumen et Page.

### Checkpoint: Complete
- [x] 3 000 identifiants uniques et exécutables.
- [x] Tests et compilation du workspace.
- [x] Revue finale et commit local.

## Risks and Mitigations

| Risque | Impact | Mitigation |
|---|---|---|
| Palette lente | Moyen | Bibliothèque en `OnceLock`, filtrage par références |
| Options dupliquées | Moyen | Test d'unicité des identifiants |
| Intensité invalide | Élevé | Décodage strict et valeur bornée 5–100 % |
| Interface encombrée | Élevé | Résultats uniquement dans la recherche `Ctrl+K`, maximum visible inchangé |

## Open Questions

Aucune.
