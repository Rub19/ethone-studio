# Capability Map: bibliothèque d'options et icônes de commandes

| Module id | Responsabilité | Dépend de |
|---|---|---|
| command-visuals | Donner un repère visuel cohérent à chaque résultat de la palette `Ctrl+K` | — |
| option-library | Fournir 3 000 variantes d'effets recherchables, stables et réellement exécutables | command-visuals |
| option-integration | Exécuter les variantes dans Pixel, Vector, Motion, Cut, Lumen et Page | option-library |

Ordre de construction : `command-visuals` → `option-library` → `option-integration`.
