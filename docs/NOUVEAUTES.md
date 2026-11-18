# Nouveautés Ethone Studio

Version **0.9.1**. Les installateurs (Pixel-Setup.exe…) sont téléchargeables sur [studio.ethone.dev](https://studio.ethone.dev) et sur la page des versions GitHub du projet.

## Prochaine version

- **Anglais** : Ctrl+, › Interface › « Langue / Language ». Les 9 apps, le lanceur et l'installateur sont traduits (menus, outils, panneaux, options).
- **Couleurs** : les profils ICC des photos (AdobeRGB, Display P3…) sont respectés à l'ouverture, sRGB est intégré aux exports ; Pixel ajoute l'épreuvage CMJN et l'export TIFF CMJN pour l'impression.
- **Récupération après plantage** : Pixel, Vector, Page, Motion et Cut enregistrent le travail non sauvegardé chaque minute et le rouvrent au démarrage.
- **Pixel** : format de projet .epix avec calques, masques et calques de réglage (Ctrl+S).
- **Pixel › Texte de la photo** : la police de chaque ligne est reconnue parmi 30 polices Windows ; seuls les mots modifiés sont effacés et réécrits ; l’écriture détachée est recomposée avec les lettres de la photo, l’écriture liée avec la police manuscrite la plus proche à la taille, l’épaisseur et l’encre de l’original ; grain et netteté imités ; fond uni (papier) recalculé sans traces d’encre. Texte illisible pour l'OCR : « Sélection → ligne » puis taper ce qui est écrit.
- **Panneaux déplaçables** : glisser l'en-tête, clic droit pour masquer, menu Espace de travail (Pixel, Vector, Page, Motion, Cut, Sound).
- **Icônes uniformes** dans les 9 apps, aides en info-bulles (i), raccourcis regroupés dans la barre d'état.

## 0.9.1

- **Page** : le panneau Pages affiche le vrai rendu de chaque page, photos comprises.
- **Lumen** : poignées des curseurs plus petites, comme dans Lightroom.
- **Motion** : palette « Ajouter un calque » compacte sur 4 colonnes.
- **Sound** : barre de transport plus fine (boutons carrés, lecture et minutage plus discrets).

## Nouvelle identité (0.9.0)

- **Icônes et logo** : icônes plates aux couleurs de chaque app, nouveau logo Ethone Studio (un E modulaire vermillon et crème).
- **Interface pro** : gris neutres, champs plus sombres que les panneaux, accent vermillon, contrôles et barres plus compacts (menus 34 px, barre d'état 22 px), coins de 3 px, curseurs fins. Les 9 apps et le lanceur partagent le même thème.
- **Préférences** : le vermillon devient la couleur d'accent par défaut (le bleu reste disponible) ; les coins vont de carrés à 8 px.
- **Site** : studio.ethone.dev, avec une scène par app et de vraies captures.

Toutes les nouvelles options se trouvent dans la palette **Ctrl+K** de chaque app : tape un mot (« look », « palette », « duotone », « écho », « filigrane »…) et choisis. Rien n'encombre l'interface tant que tu ne cherches pas.

## Préférences (Ctrl+,)

Comme Édition > Préférences dans Photoshop, dans les 9 apps : **Ctrl+,** ou Ctrl+K « Préférences… ». Tout est enregistré dès qu'on le change.

- **Communes à la suite** : taille de l'interface (85 à 150 %), couleur d'accentuation, coins carrés ou arrondis, animations, info-bulles, nombre d'étapes d'annulation (pris en compte par les 8 apps), dossier des préférences, valeurs par défaut.
- **Propres à chaque app** :

| App | Onglets | Réglages |
|---|---|---|
| Pixel | Curseurs, Repères et grille, Affichage | Curseur de peinture, grille de pixels, grille et subdivisions, magnétisme, plan de travail, cadre des calques, lissage |
| Vector | Repères et grille, Unités et règles | Grille et pas, magnétisme, repères, règles, pas d'angle avec Maj |
| Page | Repères et grille, Affichage | Marges, colonnes et grille, magnétisme, règles, notes |
| Lumen | Bibliothèque, Développement | Taille des vignettes, superposition de recadrage |
| Cut | Lecture, Montage, Repères | Qualité de l'aperçu, magnétisme, formes d'onde, vignettes, zones de sécurité, grille, repères |
| Motion | Lecture, Repères et grille, Transparence et pelure | Qualité, règles, grille, zones de sécurité, damier, pelure d'oignon |
| Sound | Affichage, Lecture | Vue au démarrage, échelle des fréquences, suivi de la lecture |
| Folio | Affichage, Annotations | Zoom à l'ouverture, panneau de gauche, couleur, épaisseur et taille du texte des annotations |

## Moins d'IA

L'IA est retirée partout, sauf pour le retrait automatique des filigranes dans Pixel (le mode sans IA reste disponible). Plus d'assistant IA dans l'aide (F1 ouvre les tutoriels), plus de résumé de PDF dans Folio, plus de traduction dans Pixel. L'Assistant du lanceur reste : il comprend les demandes avec des règles, sans IA.

## Interface revue

Audit des 9 apps avec un vrai document ouvert, corrigé et vérifié à l'écran :

- Cut et Motion : la barre de lecture et la barre d'options s'adaptent à la largeur, plus rien ne se superpose.
- Lumen : barres d'outils et d'état sans chevauchement, noms longs raccourcis avec « … ».
- Sound : barre de zoom remise en ordre. Lanceur : la liste des catégories ne passe plus sous « Paramètres ».
- Pixel : le panneau de droite ne passe plus sous l'image ; « Lissage » n'est plus coupé.
- Page : un document InDesign garde ses retours à la ligne, son alignement et sa police.
- Correction importante : choisir une option des bibliothèques Ctrl+K ne faisait rien, elles s'exécutent maintenant.

## 300 nouveaux outils

Dans Ctrl+K, tape « Outil » (ou « Outil PDF » dans Folio). Chaque outil fait une vraie action en un clic et est testé automatiquement.

| App | Outils | Exemples |
|---|---:|---|
| Pixel | 38 | Kaléidoscope, glitch, VHS, tri de pixels, tramages, réduction à 8 ou 16 couleurs, noir et blanc sauf une couleur, vision thermique, infrarouge, anaglyphe 3D, coins arrondis, découpe en cercle, autocollant, ombre portée, motif sans couture, reflet, cadre photo instantanée, pluie, neige |
| Sound | 38 | Téléphone, radio AM, mégaphone, sous l'eau, cathédrale, effet Haas, largeur stéréo, ping-pong, doubleur, vinyle, cassette, lo-fi, hachage rythmique, pompage, dé-esseur, anti-ronflement 50 / 60 Hz, karaoké, voix seule, déclic, bip de censure, gel, granulaire, arrêt de bande |
| Vector | 38 | Disposer en cercle ou en grille, progressions de couleur, taille, opacité et rotation, rosace, répétitions, copie miroir, ombre longue, halo, néon, centrer, remplir la page, coins, pastel, échanger fond et contour |
| Lumen | 38 | Exposition, points noir et blanc, contraste et vibrance automatiques, ciel, verdure, peau, coucher de soleil, nuit américaine, HDR, mat, vintage, netteté web ou impression, recadrages carré, 16:9 et 4:5 |
| Cut | 38 | Mélanger, trier, montage express, rythme alterné, Ken Burns, transitions variées, accélération, étalonnage partagé, boomerang, arrêt sur image, titre d'ouverture, carton de fin, noms des plans, marqueurs |
| Motion | 38 | Vitesse et retard des clés, paliers, rebond, boucle, aller-retour, placements, écho, répétition, ombre, lueur, machine à écrire, formats 9:16 et 1:1, grain, bandes cinéma |
| Page | 37 | Pages, numérotation, en-têtes, marges, formats A5, Lettre, carré, paysage, grilles, bandeaux, cadre, titre, citation, encadré, alignements, empiler, rangée, coins arrondis |
| Folio | 35 | Dupliquer, pages blanches, remise en ordre d'un scan recto verso, rotations, rognage, 2 ou 4 pages par feuille, signets automatiques, date, page de garde et de fin, ligne de signature, cadre |

## Installateurs autonomes

Chaque app peut s'installer seule, sans le lanceur, comme Photoshop, en deux versions :

- **Hors ligne** (Pixel-Setup.exe…, 20 à 25 Mo) : l'app est dans l'installateur.
- **Web** (Pixel-Web-Setup.exe…, 13,7 Mo) : télécharge la dernière version publiée et vérifie qu'elle est intacte, comme le Set-up.exe d'Adobe.

Installation pour ton compte Windows sans droits administrateur, raccourcis Démarrer et Bureau, entrée dans « Applications installées ». Tes projets (.evec, .ecut, .emot, .epage) s'ouvrent dans leur app ; images, sons, vidéos, PDF et PSD proposent les apps dans « Ouvrir avec » sans changer l'app par défaut. La désinstallation propose de supprimer aussi les préférences, et `--silent` installe sans fenêtre. Testé en vrai : installation et désinstallation silencieuses de Sound, tout est nettoyé.

Fabrication : `scripts\package-setups.ps1` (compile toute la suite en version finale, à lancer quand le PC est libre).

## Suite allégée

- Looks pellicule stockés en binaire (730 Ko → 100 Ko).
- Bruitages inutiles retirés.
- Compilation finale réglée pour des exécutables plus petits (taille mesurée à la prochaine compilation finale).

## Fichiers Adobe

| Fichier | App | Ce qui se passe |
|---|---|---|
| Photoshop .psd | Pixel | Ouvert calque par calque (nom, visibilité, opacité) ; aussi importable comme calque |
| Photoshop .psd | Lumen, Page, Motion | Lu comme une image (aplatie) |
| Illustrator .ai | Folio | Ouvert comme un PDF s'il a été enregistré « compatible PDF » (réglage par défaut d'Illustrator) ; enregistré en .pdf, l'original n'est jamais écrasé |
| Lightroom / Camera Raw .xmp | Lumen | Ctrl+K « Importer un préréglage Lightroom » : exposition, contraste, tons, balance des blancs, présence, TSL par couleur, virage partiel, vignette, grain, netteté, bruit, noir et blanc |
| Nuanciers .ase / .aco | Pixel, Vector | Ctrl+K « Importer un nuancier Adobe » : RVB, CMJN, gris et Lab convertis |
| LUT .cube | Lumen | Ctrl+K « Importer un LUT .cube » (Premiere, Resolve, Photoshop…) |

Premiere (.prproj), After Effects (.aep) et InDesign (.indd) sont des formats fermés : pas encore lus.

## Banques en ligne intégrées

Deux banques libres récupérées sur internet et embarquées dans les apps (aucune connexion nécessaire) :

| Banque | Licence | Où | Ce que ça donne |
|---|---|---|---|
| Lucide | ISC | Vector, Page, Motion | 2 359 icônes, recherche aussi par mots-clés anglais (« house », « arrow », « heart »…) ; tracés modifiables dans Vector, image vectorielle dans Page et calque dans Motion |
| color-name-list (meodai) | MIT | Pixel, Vector | 5 000 couleurs nommées : couleur du pinceau dans Pixel, remplissage ou contour de la sélection dans Vector |
| derby-film | MIT | Lumen | 7 looks pellicule (Kodachrome 64, Portra 400 et 800, Gold 200, Ektachrome E100VS, Agfa Ultra 100, Superia X-TRA 400) × 10 intensités |
| Kenney (Interface, RPG, Digital) + « 100 CC0 SFX » | CC0 | Sound | 311 bruitages (clics, portes, livres, lasers, bonus, cloches, vaisselle…) insérés au curseur ou à la place de la sélection ; cherche « bruitage porte », « laser », « clic » |

Vector gagne 12 359 options (18 359 au total), Pixel 5 000 (11 744), Page et Motion 2 359 chacun, Sound 311.

Nouvel export **SVG optimisé** dans Vector : nombres arrondis, attributs par défaut retirés, fichier sur une ligne (case dans Exporter… et entrée dans Exporter rapidement).

## Bibliothèques d'options (≈ 52 800 nouvelles options)

Chaque option est une vraie combinaison des réglages de l'app, nommée en français et exécutable en un clic.

| App | Options | Ce que c'est | Exemples à taper |
|---|---:|---|---|
| Pixel | 6 744 | Duotones (2 256), teintes par zone ombres / tons moyens / hautes lumières (2 160), cadres nets ou arrondis (576), dégradés colorés depuis 8 directions (960), vignettes colorées (288), pixel art avec 8 palettes rétro (504) | « duotone marine », « teinte 30° ombres », « cadre or arrondi », « pixel art console » |
| Lumen | 6 240 | Les 78 looks × 10 intensités × 8 balances des blancs, sur la photo courante ou les photos choisies ; le recadrage et le redressement de chaque photo sont conservés | « look portra 60 % chaud », « cyberpunk froid » |
| Vector | 6 000 | 60 teintes × 10 harmonies (monochrome, analogue, complémentaire, triade, tétrade…) × 10 ambiances (vif, pastel, néon, terreux…) ; la recoloration garde la hiérarchie claire / foncée du dessin | « palette corail triade pastel », « néon carré » |
| Motion | 6 750 | 15 animations × 10 durées × 9 courbes (douce, élastique, rebond…) × 5 délais, calées sur le calque sélectionné | « animer zoom 0.8 s élastique », « flottement après 0.5 » |
| Cut | 7 047 | Looks × intensité × température × saturation, étalonnage, contraste, grain, rétro, vitesse, 25 transitions × 12 durées × 3 alignements, cadrages, Ken Burns, fondus, opacité | « look 40 % chaud », « transition 1.5 s », « ken burns droite » |
| Page | 6 336 | 22 polices × 24 tailles × 12 couleurs, appliquées au bloc de texte sélectionné | « style georgia 36 pt marine » |
| Folio | 6 720 | 30 filigranes (CONFIDENTIEL, PAYÉ, SPÉCIMEN…) × 8 couleurs × 7 positions × 4 opacités, sur toutes les pages | « filigrane payé vert » |
| Sound | 6 973 | Hauteur, vitesse, étirement, gain, égaliseur 3 bandes (4 912 réglages), trémolo, auto-pan, bitcrusher, modulation en anneau, écho, réverbération, compresseur, limiteur, filtres, normalisations, fondus | « écho 250 ms », « égaliseur graves +3 », « trémolo 6 hz » |

## Interface

- La palette Ctrl+K annonce qu'elle cherche aussi dans la bibliothèque de l'app.
- Recherche instantanée : les mots-clés des milliers d'options sont préparés une seule fois.
- Une commande impossible reste visible mais grisée, avec la raison (« Sélectionne d'abord un clip », « Ouvre d'abord un PDF »…), dans les huit apps.
- Lanceur : section **Transferts récents** sous l'Assistant (50 derniers envois entre apps, Rouvrir, Dossier, Effacer).

## Nouveaux préréglages

- Motion : pulsation continue, balancement, clignotement, flottement, rotation continue, zoom de sortie, sortie vers la droite.
- Lumen : ambiances heure dorée, heure bleue, brume chaude, hiver lumineux, désert, cyberpunk, rétro 70s, sombre et intense.

## Rappel des versions précédentes

- v0.9.1 : miniatures de Page, curseurs de Lumen, palette de Motion, transport de Sound.
- v0.9.0 : nouvelle identité et interface pro (section du haut).
- v0.8.0 : préférences, installateurs, ressources et outils décrits sur cette page.

- v0.7.0 : détachant de filigrane (sélection ou IA), 3 000 variantes d'effets, palette intelligente, actions sensibles confirmées.
- v0.6.0 / v0.5.0 : Folio (PDF), Assistant automatique, barre de titre intégrée, texte de la photo (OCR).
