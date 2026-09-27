# Pili - charte de marque (résumé)

**Pili 霹雳** (pī lì, « coup de foudre ») : la foudre vient d'un nuage et elle est
instantanée, comme une caisse qui répond tout de suite. Marque indépendante, signée
« par Nuage Magique ».

## Logo
- **Frappe, avec les tons** : « pili » en grotesque géométrique minuscule ; un éclair en Z
  traverse le p ; les points des deux i sont les tons de pī lì (plat, puis descendant).
- Tons en **Volt** sur fond sombre, en encre sur fond clair. Composant : `ui/src/components/PiliLogo.vue`.
- **Icône** : le p frappé seul, Volt sur Encre (`src-tauri/icons/icon.svg`, PNG et ICO générés).
- Couleurs autorisées : Brume sur Encre, Encre sur Papier, Encre sur Volt. Pas de Volt sur
  clair, pas d'autres couleurs, pas de déformation ni d'inclinaison.
- Zone de protection : un tiers de la hauteur du logo. Taille minimale : 16 px à l'écran,
  6 mm imprimé ; sous 24 px, utiliser l'icône.

## Couleurs (une signification chacune)
| Nom | Hex | Rôle |
|---|---|---|
| Volt | `#D7FF3A` | action principale, un seul bouton Volt par écran |
| Encre | `#0B0D0E` | fond sombre, texte sur clair |
| Graphite | `#15181A` | surfaces |
| Ardoise | `#1F2327` | boutons secondaires |
| Brume | `#F2F4F2` | texte sur sombre, sélection |
| Papier | `#F4F5F2` | fond clair (documents) |
| Ambre | `#FFB020` | en attente, pas encore envoyé |
| Rouge | `#FF4D3D` | supprimer, erreur |
| Vert | `#22C55E` | OK, connecté, payé |
| Cyan | `#00D4FF` | information, en ligne (couleur de Nuage Magique) |
| Violet formation | `#9B59B6` / fond `#2D1B4E` | mode formation uniquement (guide UX §9) |

## Typographie
- **Noto Sans / Noto Sans SC** : écran de caisse (guide UX §10).
- **JetBrains Mono** : tous les chiffres (prix, heures, quantités, statuts).
- **Archivo** : marque et documents (titres, site, supports imprimés).

## Ton
Court et précis. Un verbe par bouton : Payer, Envoyer, Valider. Dire ce qui s'est passé
(« Envoyé en cuisine · 3 articles ») plutôt que « Succès ! ». Pas de point d'exclamation,
pas d'emoji.
