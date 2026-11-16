# Feuille de route Ethone Studio

Ce document recense les améliorations ajoutées pendant les vagues de développement récentes et fixe les prochaines priorités de la suite.

## Vision

Faire d'Ethone Studio une suite créative cohérente où chaque application reste spécialisée, partage la même palette `Ctrl+K`, échange facilement ses fichiers avec les autres outils et expose les actions répétitives à l'assistant.

## Livré

### Vague 1 — suppression et bibliothèque créative

- Pixel : suppression d'un filigrane depuis une sélection ou par détection automatique.
- Assistant : compréhension des demandes de retrait de filigrane en français et en anglais.
- Palette commune : exactement 3 000 variantes d'effets recherchables.
- Icônes de catégorie ajoutées aux résultats de la palette.
- Les variantes sont exécutables dans Pixel, Vector, Motion, Cut, Lumen et Page.
- Optimisation : la bibliothèque de 3 000 options n'est parcourue que lorsqu'une recherche est saisie.

### Vague 2 — outils essentiels par application

| Application | Ajouts |
|---|---|
| Pixel | Recadrage automatique aux pixels visibles. |
| Vector | Sélection par fond, contour, type d'objet ou apparence complète. |
| Motion | Inversion et répartition des images clés, ajustement à la composition, durée adaptée aux calques. |
| Cut | Fermeture de tous les espaces vides de la piste. |
| Lumen | Synchronisation des réglages de la photo courante vers les photos choisies. |
| Page | Centrage sur la page et distribution horizontale ou verticale. |
| Folio | Export PNG immédiat de la page courante. |
| Sound | Rognage des silences de début et de fin sans supprimer les pauses internes. |
| Assistant / MCP | Nouvelle opération `trim_silence_edges` et suggestion correspondante. |

### Vague 3 — productivité dans toute la suite

| Application | Ajouts |
|---|---|
| Pixel | Dupliquer le calque actif, fusionner les calques visibles, aplatir l'image depuis `Ctrl+K`. |
| Vector | Tout sélectionner, inverser la sélection, dupliquer, grouper et dégrouper depuis `Ctrl+K`. |
| Motion | Fractionner le calque à la tête et dupliquer le calque depuis `Ctrl+K`. |
| Cut | Normalisation de l'audio sélectionné et fondus rapides de 0,5 ou 1 seconde sur tous les clips. |
| Lumen | Attribution de 5 étoiles, drapeaux retenu/rejeté et remise à zéro par lot. |
| Page | Dupliquer, grouper, dégrouper, pivoter et retourner les blocs depuis `Ctrl+K`. |
| Folio | Rotation rapide de la page courante ou des pages sélectionnées. |
| Sound | Normalisation de crête à −1 dB, normalisation à −16 LUFS et micro-fondus anti-clic. |
| Launcher / Assistant | Suggestion −16 LUFS et distinction entre volume perçu et normalisation de crête. |

### Vague 4 — palette commune plus intelligente

- Les huit éditeurs affichent les commandes récemment utilisées en premier.
- L'historique récent est dédupliqué et limité aux huit dernières commandes.
- La recherche est insensible à la casse et aux accents.
- Les registres `Ctrl+K` sont validés en développement : identifiants vides, libellés vides et doublons sont refusés.
- Les commandes affichent des icônes distinctes pour les fichiers, calques, sélections, lots, actions audio et dispositions.
- La requête normalisée est calculée une seule fois, y compris lors d'une recherche dans les 3 000 variantes.

### Vague 5 — sécurité des actions globales

- Les commandes sensibles de `Ctrl+K` affichent un avertissement et exigent une seconde validation identique.
- Échap, une nouvelle recherche ou l'exécution d'une commande normale annule la confirmation en attente.
- Un clic déplace la sélection afin qu'Entrée confirme toujours la même commande.
- Pixel protège l'aplatissement complet de l'image.
- Cut protège les transitions globales, coupes de piste, changements de format, fermeture des blancs et fondus appliqués à tous les clips.
- Lumen protège les rejets et remises à zéro par lot.
- Sound protège la suppression globale des silences, le rognage des bords et la création de boucles.

### Corrections incluses

- Folio : une action visant les pages sélectionnées agit désormais sur la page courante quand aucune vignette n'est sélectionnée, au lieu de modifier toutes les pages.
- Pixel : un recadrage impossible ne laisse plus un mauvais libellé pour la prochaine entrée d'historique.
- Sound : la détection des bords audibles fonctionne en mémoire constante et conserve les silences internes.

## Prochaines vagues

### Vague 6 — cohérence et sécurité des actions

- Uniformiser les messages de réussite, d'erreur et les états vides dans les huit éditeurs.
- Ajouter des tests de registre garantissant que chaque commande `Ctrl+K` possède un identifiant unique et une action associée.
- Vérifier les historiques annuler/rétablir pour les nouvelles commandes de lot.

### Vague 7 — workflows inter-applications

- Conserver davantage de métadonnées lors des envois entre applications.
- Ajouter des recettes multi-applications : développer puis retoucher, monter puis mixer, mettre en page puis exporter.
- Afficher la destination et le format avant chaque envoi.
- Ajouter un historique des transferts récents dans le Launcher.

### Vague 8 — outils avancés

- Pixel : traitement par lot et comparaison multi-versions.
- Vector : symboles réutilisables, variantes et export SVG optimisé.
- Motion : précompositions, expressions simples et modèles réutilisables.
- Cut : sous-titres automatiques, détection de scènes et proxy média.
- Lumen : piles HDR, panorama et profils d'objectifs.
- Page : gabarits maîtres, styles de texte et contrôle prépresse.
- Folio : OCR par lot, comparaison de versions et optimisation PDF.
- Sound : réduction de bruit adaptative, analyse de silence et export multi-format.

## Qualité et validation

- État actuel : `120` tests réussis, `8` tests ignorés intentionnellement et compilation complète validée.
- Chaque vague doit compiler avec `cargo check --workspace`.
- Tous les tests doivent réussir avec `cargo test --workspace`.
- Les nouveaux algorithmes et les corrections doivent commencer par un test de régression.
- Chaque tranche fonctionnelle reçoit un commit local séparé et réversible.
- Les applications ne sont pas lancées pendant les vérifications automatisées.

## Historique des points de sauvegarde

- `47056e9` — retrait des filigranes dans Pixel et l'assistant.
- `e9a1a08` — 3 000 variantes d'effets recherchables.
- `397a473` — chargement des variantes uniquement pendant la recherche.
- `66d9c73` — outils rapides de la vague 2.
- `da9c2e9` — commandes d'édition et correction de portée Folio de la vague 3.
- `1ca5b29` — actions média, lots Lumen et compréhension −16 LUFS de la vague 3.
- `c148638` — commandes récentes, recherche sans accents et validation des registres.
- `a2ba87f` — icônes distinctes par famille de commande.
- `82529d8` — moteur de confirmation partagé pour les commandes sensibles.
- `6a7c985` — protection des actions globales dans les éditeurs.

