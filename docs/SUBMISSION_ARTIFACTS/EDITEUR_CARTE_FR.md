# Éditeur de carte à l'écran

Annexe du dossier de certification de la version 1.0.0. Elle décrit l'éditeur de carte : un
gérant crée et modifie les catégories, les produits, les groupes d'options et les options
directement à la caisse, sans passer par un catalogue externe. Elle complète les §3 à §5 de
*MANUEL_PROGRAMMATION_FR.md*. La fonction n'a **aucun effet fiscal** : elle ne
produit aucun événement, ne touche à aucun compteur et ne modifie ni un ticket, ni un rapport,
ni une commande en cours (voir le §4).

## 1. Parcours du gérant

1. Dans **Config**, le panneau **« Carte »** (réservé au gérant) ouvre l'écran
   **« Édition de la carte »**.
2. Trois colonnes : les catégories, les produits de la catégorie choisie, et le formulaire de
   l'élément choisi. Les éléments masqués sont grisés et portent la mention « Masqué ».
3. **Catégorie** : noms par langue, couleur, ordre, visible ou masquée.
4. **Produit** : noms par langue, catégorie, prix (pavé numérique), code de TVA sur place et à
   emporter (A, B, C, D, X), code du menu, prix libre, envoi en cuisine, ordre, visible ou
   masqué. Un produit neuf doit d'abord être enregistré avant de recevoir des options.
5. **Groupe d'options** : noms, minimum et maximum de choix (le maximum est au moins 1 et au
   moins égal au minimum), ordre, visible ou masqué.
6. **Option** : noms, supplément de prix, ordre, visible ou masquée.
7. **Enregistrer** applique la modification ; **Annuler** referme le formulaire. Le bouton
   **« Historique »** liste les modifications : nature, élément, auteur, date et champs changés.

Saisie du texte : sur la caisse Linux, clavier à l'écran (majuscules, chiffres, accents é è ê
à ç ù, espace, `-'.,/&`) ; le nom chinois se saisit dans un champ ordinaire, sans clavier à
l'écran (la caisse n'a pas de méthode de saisie chinoise : collage depuis un clavier
physique). Les textes existent en français,
anglais et chinois.

## 2. Règles appliquées par le moteur

| Règle | Comportement |
|---|---|
| Droits | Toutes les écritures sont réservées au gérant (profil administrateur). Lire la carte d'édition et l'historique est aussi réservé au gérant. |
| Verrou | Tant qu'un catalogue externe gère la carte (HubRise), toute écriture est refusée avec le code `hubrise_menu_managed` ; l'écran affiche qui gère la carte et ne propose aucun bouton d'ajout. La lecture reste possible. |
| Validation | Le nom dans la langue fiscale de l'établissement est obligatoire (les autres sont facultatifs) ; code de TVA parmi A, B, C, D, X ; prix de 0 à 999 999 centimes ; supplément d'option positif ou nul ; minimum et maximum cohérents. Refus : `menu_invalid` avec le champ en cause (`field=`) ; élément ou parent inconnu : `menu_not_found`. |
| Aucune suppression | Un élément n'est jamais supprimé : il est masqué (`active = 0`). Les commandes et les ventes déjà enregistrées continuent de le référencer. |
| Atomicité | Une modification écrit la ligne, ses traductions et la ligne du journal dans une seule transaction. Une modification qui ne change rien n'écrit rien. |
| Options à la vente | Un produit propose le choix d'options dès qu'il possède au moins un groupe actif contenant au moins une option active (indicateur `has_modifiers`), avec ou sans HubRise. |

## 3. Journal

Chaque création et chaque modification est ajoutée à la table `menu_change_log` : type
d'élément (`category`, `product`, `modifier_group`, `modifier`), identifiant, action
(`create` ou `update`), la ligne avant et après en JSON (noms par langue compris), l'auteur
(numéro INSZ) et la date. La table est en ajout seulement : des déclencheurs
(`menu_change_log_no_update`, `menu_change_log_no_delete`) refusent toute modification et toute
suppression.

Requête de contrôle du journal :

```sql
SELECT changed_at, changed_by, entity_type, entity_id, action, before_json, after_json
FROM menu_change_log ORDER BY id;
```

## 4. Argument de l'article 44 de l'arrêté ministériel du 29 avril 2024

Une ligne de commande conserve, au moment où elle est ajoutée, un instantané du nom, du prix
unitaire, de l'étiquette de TVA et du département. L'événement signé et son archive JSON
contiennent ces valeurs, pas une référence vers la carte. Modifier le prix, le nom ou le code
de TVA d'un produit ne change donc ni une commande ouverte, ni un événement signé, ni un ticket
réimprimé, ni un rapport X ou Z déjà émis : seules les lignes ajoutées après la modification
utilisent les nouvelles valeurs. Le test `art_44_an_edit_never_changes_an_open_order_or_a_signed_event` le démontre : il ouvre
une commande, y ajoute deux Maki saumon à 6,50 EUR (TVA B), signe le pro forma, puis change le
nom, le prix (9,90 EUR) et la TVA du produit ainsi que le nom de la catégorie. Il vérifie que
les lignes déjà saisies sont intactes, que l'événement P signé est identique octet pour octet,
que la vente N signée ensuite porte encore le nom, le département, le prix unitaire, l'étiquette
de TVA et le total d'origine, et qu'une nouvelle commande utilise bien les nouvelles valeurs.

Un cas particulier concerne les rapports X et Z. Le nom du département d'une vente est celui de
la catégorie **au moment où la ligne a été ajoutée** (instantané ci-dessus) ; le rapport regroupe
les ventes par identifiant **et** par nom de département. Si un gérant renomme une catégorie
au milieu d'une période comptable, les ventes de cette catégorie faites avant et après le
changement apparaissent donc sur **deux lignes du même identifiant de département** (l'ancien
et le nouveau nom) dans les rapports X et Z de la période. Les montants et les totaux ne
changent pas : seule la ventilation par nom est scindée. Recommandation d'exploitation : renommer
une catégorie après le rapport Z, jamais en cours de période.

## 5. Tests

| Test | Ce qu'il démontre |
|---|---|
| `crates/pos-engine/tests/menu_editor.rs` | Création et modification des quatre éléments, codes de refus avec le champ, langue fiscale obligatoire, éléments inconnus, journal avant/après et immuable, verrou HubRise (lecture permise), indicateur `has_modifiers`, options vendues au nouveau prix, droits, article 44. |
| `crates/pos-engine/tests/access_control.rs` | Les six commandes sont réservées au gérant. |
| `crates/pos-engine/tests/error_codes.rs` | Les codes `menu_invalid` et `menu_not_found` sont listés et bien formés. |
| `crates/pos-db` (`migrate`) | La migration 42 est ajoutée ; le journal refuse update et delete. |
| `ui/e2e/menu-editor.spec.ts` | Parcours Vue : panneau, droits, saisie au clavier, chinois, groupes et options, refus, historique, lecture seule. |

Aucun cas du jeu d'essais de l'administration n'est concerné : aucun événement n'est produit
par l'éditeur.
