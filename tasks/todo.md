# Bibliothèque d'options et icônes

## Task 1: Générateur et décodeur

- [x] Exactement 3 000 variantes sont générées avec des identifiants uniques.
- [x] Chaque identifiant se décode vers un `Fx` valide et une intensité bornée.
- [x] Vérifier avec `cargo test -p studio-ui`.

## Task 2: Icônes de palette

- [x] Chaque résultat affiche une icône selon sa catégorie.
- [x] Les libellés et raccourcis existants restent lisibles.
- [x] Vérifier avec `cargo check -p studio-ui`.

## Task 3: Pixel, Vector et Motion

- [x] Les trois applications exécutent une variante décodée.
- [x] Pixel conserve un aperçu réglable ; Vector et Motion ajoutent un effet non destructif.
- [x] Vérifier avec les tests des trois paquets.

## Task 4: Cut, Lumen et Page

- [x] Les trois applications exécutent une variante décodée.
- [x] Les erreurs de sélection restent explicites.
- [x] Vérifier avec les tests des trois paquets.

## Checkpoint final

- [x] `cargo test --workspace` réussit.
- [x] `cargo check --workspace` réussit.
- [x] Diff revu, sans secret ni changement hors périmètre.

## Vague 2 — nouveaux outils

- [x] Pixel : recadrer aux pixels visibles depuis `Ctrl+K`.
- [x] Vector : sélectionner les objets de même apparence depuis `Ctrl+K`.
- [x] Motion : outils temporels et ajustement rapide depuis `Ctrl+K`.
- [x] Cut : fermer tous les blancs depuis `Ctrl+K`.
- [x] Lumen : synchroniser les réglages vers les photos choisies.
- [x] Page : centrer et distribuer les blocs.
- [x] Folio : exporter la page courante en PNG.
- [x] Sound : rogner les silences de bord avec test de régression.
- [x] Tests et compilation complets réussis.

## Vague 3 — productivité dans toute la suite

- [x] Slice A : nouvelles commandes Pixel, Vector, Motion, Page et Folio.
- [x] Slice A : compilation et tests ciblés réussis.
- [x] Slice A : commit local créé.
- [x] Slice B : nouvelles commandes Cut, Lumen et Sound.
- [x] Slice B : demande « −16 LUFS » comprise et couverte par un test.
- [x] Slice B : suggestions du Launcher enrichies.
- [x] Checkpoint final : workspace compilé et testé.
- [x] Checkpoint final : diff revu et commit local créé.

## Vague 4 — palette commune

- [x] Slice A : commandes récentes, recherche sans accents et validation des registres.
- [x] Slice A : tests ciblés et commit local.
- [x] Slice B : icônes de familles de commandes.
- [x] Slice B : feuille de route actualisée.
- [x] Checkpoint final : workspace compilé, testé et revu.

## Vague 5 — sécurité des actions globales

- [x] Slice A : moteur de confirmation partagé et tests.
- [x] Slice A : commit local créé.
- [x] Slice B : commandes sensibles annotées dans Pixel, Cut, Lumen et Sound.
- [x] Slice B : workspace compilé et testé.
- [x] Checkpoint final : feuille de route, revue et commits locaux.

## Vague 6 — états indisponibles

- [x] Slice A : état désactivé partagé, raison visible et tests.
- [ ] Slice A : commit local créé.
- [ ] Slice B : intégration Pixel, Vector, Motion et Page.
- [ ] Slice B : tests ciblés et commit local.
- [ ] Slice C : intégration Cut, Lumen, Folio et Sound.
- [ ] Checkpoint final : workspace, feuille de route, revue et commits.
