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

## Wave 2: nouveaux outils par application

- [x] Pixel : recadrage instantané aux pixels visibles.
- [x] Vector : sélection rapide par fond, contour, type et aspect.
- [x] Motion : inversion/étalement des images clés, ajustement du calque et durée automatique.
- [x] Cut : fermeture instantanée de tous les blancs.
- [x] Lumen : synchronisation des réglages de la photo courante vers la sélection.
- [x] Page : centrage et distribution rapides des blocs.
- [x] Folio : export PNG immédiat de la page courante.
- [x] Sound : suppression non destructive des silences de début et de fin.

### Checkpoint Wave 2

- [x] Tests ciblés des nouveaux algorithmes.
- [x] Compilation et tests du workspace.
- [x] Revue qualité et commit local.

## Wave 3: productivité dans toute la suite

### Slice A — édition et mise en page

- [x] Pixel : dupliquer, fusionner les calques visibles et aplatir depuis `Ctrl+K`.
- [x] Vector : sélectionner, inverser, dupliquer, grouper et dégrouper depuis `Ctrl+K`.
- [x] Motion : fractionner et dupliquer le calque actif depuis `Ctrl+K`.
- [x] Page : dupliquer, grouper, dégrouper, pivoter et retourner les blocs depuis `Ctrl+K`.
- [x] Folio : pivoter rapidement la page courante ou les pages sélectionnées.
- [x] Compiler et tester les applications concernées, puis créer un commit local.

### Slice B — médias, lots et assistant

- [x] Cut : normaliser l'audio sélectionné et appliquer des fondus rapides à tous les clips.
- [x] Lumen : noter, retenir, rejeter ou réinitialiser les photos choisies.
- [x] Sound : normalisation crête, normalisation −16 LUFS et micro-fondus de bord.
- [x] Assistant : distinguer une demande LUFS d'une simple normalisation de crête.
- [x] Mettre à jour les suggestions audio du Launcher.
- [x] Créer une feuille de route récapitulative dans `ROADMAP.md`.
- [x] Tests complets, revue qualité et commit local.

## Wave 4: palette commune plus intelligente

### Slice A — recherche et fiabilité

- [x] Afficher en premier les commandes récemment utilisées quand la recherche est vide.
- [x] Dédupliquer l'historique récent et le limiter à huit commandes.
- [x] Rendre la recherche insensible aux accents et à la casse.
- [x] Valider les identifiants, libellés et doublons de chaque registre de commandes.
- [x] Couvrir le moteur par des tests unitaires et créer un commit local.

### Slice B — repères visuels et finalisation

- [x] Ajouter des icônes distinctes pour fichiers, sélection, disposition, lots et audio.
- [x] Mettre à jour la feuille de route avec la vague 4.
- [x] Compiler et tester tout le workspace.
- [x] Effectuer la revue qualité et créer les commits locaux.
