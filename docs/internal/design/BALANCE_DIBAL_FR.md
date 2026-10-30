# Balance de caisse Dibal DPOS-400 (note de conception)

Statut : phases 1 et 2 écrites sur la branche `feat/scale-dialog06` (voir « État de l'implémentation » ci-dessous),
essayées contre le simulateur uniquement, jamais sur une balance. Cible : `pili-pos-android` 1.0.0, pas encore soumise.
La vente au poids et la DPOS-400 font partie de la v1 (décision du propriétaire, 2026-10-30).
Public : le responsable du prochain lot et le propriétaire. Langue : français (interne).
Décisions à venir : préfixe `WGT-D` dans `docs/android/DECISIONS.md`.

## État de l'implémentation (2026-10-30)

Écrit et testé : `crates/pos-scale` (protocole sans E/S et pilote TCP), `crates/pos-scale-mock`, la section `scale` de
`pos.json`, la migration 80, la ligne pesée (moteur, ticket, rapports, remboursement, écho HubRise, refus en ligne), le
rejeu de UC4112, les trois commandes, et les écrans Vue et Compose. Hypothèse de départ du propriétaire : la balance
parle bien Dialog 06 (code 24 du menu).

Ce qui s'écarte du texte ci-dessous, et prévaut sur lui :

- **TCP seulement.** Le `transport` à plusieurs valeurs, `android_usb`, `ScaleLink` et le pont Kotlin (7.3) ne sont pas
  écrits : sans balance sur la table, on ne connaît ni la puce ni la classe USB. La section est
  `{ "tcp_address", "price_digits", "check_values", "weigh_timeout_ms" }`. Le raccordement réel reste la phase 3.
  Un serveur série vers TCP devrait marcher, sans avoir été essayé.
- **Enregistrement 20.** Dans la spécification Bizerba, 20 va de la caisse vers la balance (version logique active ou
  non). Le verdict du contrôle est l'enregistrement 11 : `D0` 30h refusé, 31h accepté, 32h demande avec `Z`.
- **Enregistrement 01.** Le schéma de la spécification met `ESC` avant `ETX`, l'exemple de sa page 6 non. Le pilote suit
  le schéma. À vérifier sur une vraie balance.
- **Somme de contrôle.** L'algorithme est inconnu. Le pilote envoie les valeurs fixes de `scale.check_values` et ignore
  `Z`. Le simulateur accepte une chaîne fixe : réussir contre lui prouve le dialogue, pas l'acceptation par une balance.
- **`scale_weighings`** garde poids, prix, montant et tare, sans la trame brute ni le transport.
- **Pas encore fait :** la ligne de la balance dans les alertes au propriétaire (le voyant rouge existe), la bibliothèque
  légale séparée (6.3), le gel rc4, le dossier et les preuves (phase 5, après la décision de métrologie).

## 1. Objet

Permettre à une caisse pili-pos de vendre au poids avec une balance de caisse Dibal DPOS-400, par
exemple un bol de malatang composé par le client et payé au kilo. La note couvre le volet fiscal
(vente au poids, SCE 2.0), le protocole de la balance, la métrologie légale, le matériel et un plan
par phases. Rien n'a été commandé, testé sur une balance ni demandé à une administration.

Deux autorités sont concernées, et l'accord de l'une ne vaut pas pour l'autre :

- **SPF Finances** : le système de caisse (la ligne pesée signée, le ticket, le dossier).
- **SPF Économie, service Métrologie** : l'instrument de pesage et, selon sa réponse, le logiciel de
  caisse qui reçoit le poids.

## 2. Ce que propose cette note

| Sujet | Proposition | Section |
| --- | --- | --- |
| Périmètre | UC4112 et la balance entrent dans la v1 déclarée (décidé) ; le gel `android-v1.0.0-rc3` est repris, le dossier et les preuves suivent | 4.10 |
| **Blocage possible** | Les questions de métrologie sont maintenant sur le chemin de la soumission. Le volet fiscal et le pilote avancent sans elles ; le gel attend leur réponse ou une décision du propriétaire | 6.4 |
| Modèle de données | Pas de quantité décimale dans `OrderLine` : une pesée est une ligne de quantité 1 qui porte son poids et son montant, en colonnes ajoutées | 4.3 |
| Montant | Celui de la balance, repris tel quel, entiers uniquement, contrôlé contre le recalcul de la caisse | 4.5 |
| Remboursement | Une pesée se rembourse entière en première version | 4.8 |
| Pilote de balance | Crate `pos-scale` sans entrée-sortie, transport interchangeable, simulateur `pos-scale-mock`, section `scale` de `pos.json`, absente par défaut | 7.3 |
| Raccordement | USB d'abord, RJ11 série en repli, selon les essais sur le vrai terminal | 7.2 |
| Logiciel légal | Partie légalement pertinente isolée dans une bibliothèque séparée, pour qu'une mise à jour ordinaire ne change pas la somme de contrôle | 6.3 |
| Premier pas | Questions au SPF Économie et à Dibal dès maintenant : leur réponse décide de la voie et de la date de soumission | 6.4 et 8 |

## 3. Statut des sources

| Élément | Source | Statut |
| --- | --- | --- |
| Ligne pesée, UC4112, FAQ Q201 | documents SPF Finances du dépôt | lu dans le dépôt |
| État du code (section 4.2), largeur 58 mm, ports du D3 Pro dans `PLATEFORME_ANDROID_FR.md` | `origin/main`, commit `aa37600a` | lu dans le dépôt |
| État du gel (section 4.10) | `scripts/android/check_android_frozen.sh` lancé à ce commit | constaté |
| Protocole Dialog 06 (section 5) | relevé fourni pour cette note, issu de recherches ; le manuel Dibal n'a pas été lu | **non vérifié** sur le manuel et sur une balance |
| Algorithme de la somme de contrôle (enregistrement 10) | non trouvé | **inconnu** |
| Lecture WELMEC (section 6) | résumés des guides trouvés en ligne, guides non relus en entier | **non vérifié** |
| Accès au port série du D3 Pro | aucune documentation Sunmi trouvée le 2026-10-30 | **inconnu** |

## 4. Volet fiscal

### 4.1 Ce que demande la spécification

| Source | Exigence |
| --- | --- |
| `docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md`, `quantity` | Numérique, multiple de 0.0001, positif, négatif ou nul ; `negQuantityReason` obligatoire si négatif. |
| Même fichier, `quantityType` et `unitPrice` | `KILOGRAM` existe ; `unitPrice` est un multiple de 0.0001. |
| Même fichier, `vat.price` et `lineTotal` | Deux décimales. `lineTotal` = somme de `vat.price` et des `priceChange`. Le texte ne donne aucune formule `quantity x unitPrice`. |
| `docs/20260717_FAQ_PRODUCTEURS_FR.md`, Q201 (produit pesé en temps réel, salade-bar) | a) `KILOGRAM`, prix au kg en `unitPrice`, poids en `quantity`. b) Une option payante (sauce, riz, nouilles) est une ligne distincte. c) Une réduction sur le prix au kg (client qui apporte son contenant) est un `priceChange` sur la ligne pesée. d) Un format S / M / L avec surcoût est une ligne distincte et ne change pas le prix au kg. e) `PIECE` ne s'applique pas à un produit pesé. |
| `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/UC4112_WeightedProduct/` | Vente directe : 1 Perrier (A, 2,50) et 0,278 kg de « Catch of the day » à 45 EUR/kg (B, 12,51). Total 15,01, payé en espèces, arrondi -0,01. |
| Manuel de certification, tableaux des périphériques et des fonctions | Une ligne « Balance » (« Pour la fonction "prix au kg" ») et une ligne « Prix au poids » à cocher. |

La FAQ Q201 décrit exactement le bol composé : le bol est une ligne pesée, et un supplément vendu à
prix fixe est une autre ligne. Aucune recette de bol ni aucun produit composé n'est nécessaire.

### 4.2 État du code

Le protocole fiscal est prêt, mais toute la chaîne commande suppose un entier.

| Endroit | Aujourd'hui | Conséquence pour une ligne pesée |
| --- | --- | --- |
| `pos-fiscal/src/inputs.rs`, `types.rs` | `quantity: f64`, `QuantityType::Kilogram` | Rien à changer |
| `pos-core/src/order.rs`, `OrderLine` | `quantity: i32`, `line_total()` = prix x quantité | À étendre (4.3) |
| `pos-fiscal/src/event.rs`, `build_transaction` | `QuantityType::Piece` en dur, totaux recalculés à deux endroits | À passer par `line_total()` et par le poids |
| `pos-db/src/schema.rs` | `order_lines.quantity` et `order_entries.quantity` en `INTEGER` ; `order_entries` protégé par des déclencheurs d'ajout seul | Colonnes à ajouter (4.4) |
| `pos-db/src/queries/orders.rs`, `recalculate_order_totals` | `SUM(unit_price_cents * quantity)` | À corriger |
| `pos-print/src/receipt.rs`, `format_qty` | Une quantité non entière passe par `format_price` : 0,278 s'imprime **« 0,28 »** | Juste à la pièce, **faux pour un poids**, à corriger (4.6) |
| `pos-engine/src/commands/refund.rs`, `already_refunded_quantities` | `(-quantity).round() as i64` : -0,278 compte pour **0** | Juste à la pièce, **faux pour un poids** : le garde-fou contre le double remboursement ne verrait rien (4.8) |
| `pos-fiscal/src/ticket.rs`, `TicketLine` | `quantity: f64`, `unit_price` présent mais non imprimé, pas d'unité | Unité à ajouter |
| `pos-hubrise-types/src/money.rs`, `parse_quantity` ; `pos-hubrise/src/webshop.rs` | Une quantité non entière est refusée (`FractionalQuantity`) | Comportement à garder (4.9) |
| `pos-engine/src/commands/hubrise_push.rs` | `l.quantity.to_string()` pour l'écho d'une vente | Enverrait « 1 » au prix du kilo : à corriger (4.9) |
| `pos-print/src/bar_ticket.rs`, `kitchen_ticket.rs`, `customer_slip.rs` | Quantités entières | Rien à changer avec le modèle 4.3 : « 1 x Bol » |
| `pos-engine/src/commands/invoice.rs` (`unit_code`), `pos-print/src/invoice.rs`, `pos-peppol/src/ubl.rs` | Déjà `KGM` et `quantity_milli`, lus dans l'archive signée | Rien à changer, à couvrir par un test |
| `pos-engine/src/commands/sales_report.rs` (`milli`), `pos-export/src/stats.rs` (`quantity REAL`) | Déjà décimaux, lus dans l'archive signée | Affichage de l'unité seulement |
| `pos-fiscal/src/report_totals.rs` | `negQuantityCount` compte des lignes, les montants viennent de la requête signée | Rien à changer, à couvrir par un test |
| `pos-fdm-mock` | Contrôle le multiple de 0.0001 ; ne compare pas `quantity x unitPrice` | Rien à changer |
| `pos-fiscal/tests/advanced_set_fixtures.rs` | UC4112 passe au niveau du protocole (`COUVERTURE_ADVANCED_SET_FR.md`, ✓) ; aucun scénario moteur | Scénario à ajouter (8) |

### 4.3 Modèle de la ligne pesée

**Proposition : une pesée est une ligne de quantité 1 (ou -1) qui porte son poids et son montant.**

```rust
pub struct OrderLine {
    // ... champs existants
    pub quantity: i32,                       // 1 pour une pesée, -1 pour sa correction ou son remboursement
    pub unit_price_cents: i64,               // le prix au kilo pour une pesée
    #[serde(default)]
    pub weight_grams: Option<i64>,           // None : ligne à la pièce
    #[serde(default)]
    pub weighed_amount_cents: Option<i64>,   // montant de la pesée, toujours positif
}

impl OrderLine {
    pub fn line_total(&self) -> i64 {
        self.weighed_amount_cents.unwrap_or(self.unit_price_cents) * i64::from(self.quantity)
    }
}
```

La requête signée dérive de ces champs : `quantity = quantity x weight_grams / 1000` (0.278 ou -0.278),
`quantityType: KILOGRAM`, `unitPrice` = prix au kilo, `vats[0].price` et `lineTotal` = `line_total()`.
La conversion passe par des entiers, puis une seule division, comme la fonction `eur` existante.

Pourquoi pas une quantité décimale dans `OrderLine` :

- SQLite ne change pas le type d'une colonne, et les déclencheurs de `order_entries` interdisent de
  réécrire les lignes du journal. Une conversion demanderait de recréer une table en ajout seul.
- `quantity` paraît dans 136 fichiers Rust, tests compris, dont 52 occurrences dans `queries/orders.rs`
  et 26 dans `refund.rs`, ainsi que dans les enregistrements FFI en `i32`, le Kotlin et le Vue. Chaque
  occurrence devrait être relue pour un cas qui ne concerne que les lignes pesées.
- Les journaux, les corrections, les temps de service et les bons de cuisine fonctionnent tels quels
  sur un 1 et un -1, sans toucher aux lignes à la pièce ni à leurs tests.
- Le prix en virgule flottante reste exclu (règle du dépôt).

Limite assumée : le poids n'est pas additionnable par la colonne `quantity`. Deux pesées restent deux
lignes, et l'addition des kilos se lit dans l'archive signée (ce que font déjà les rapports).

### 4.4 Base de données

Groupe de migrations 80 et suivantes, dans `schema_weight.rs`, sans fonctionnalité Cargo. Les plages
1 à 11, 20 à 28, 30, 31, 40 à 42, 60 à 63 et 70 sont prises et 12 à 19 sont réservées aux correctifs
certifiés.

- `menu_items.sold_by_weight INTEGER NOT NULL DEFAULT 0` : `price_cents` est alors le prix au kilo.
- `menu_items.scale_tare_grams INTEGER` : tare fixe du contenant envoyée à la balance, facultative.
- `order_lines` et `order_entries` : `weight_grams INTEGER`, `weighed_amount_cents INTEGER`, nulles pour
  les lignes à la pièce, avec une contrainte : les deux nulles ou les deux positives.
- Un second déclencheur d'ajout seul pour ces deux colonnes de `order_entries`, car
  `order_entries_no_rewrite` nomme ses colonnes une à une.
- `scale_weighings`, en ajout seul : ligne de commande, horodatage, poids, prix unitaire, montant, statut,
  enregistrement 02 brut, transport, résultat du dernier contrôle de la balance. C'est la preuve d'audit
  entre ce que la balance a dit et ce qui a été signé. Elle reste locale et n'entre dans aucun message FDM.
- `scale_check_log`, en ajout seul : demandes de contrôle (enregistrements 11, 10, 20) et leur résultat.

La table `fiscal_events` ne change pas : l'archive contient déjà tout ce que signe le FDM.

### 4.5 Montant et arrondi

Le prix est calculé en entiers, jamais en flottants, et arrondi au centime supérieur à partir du demi :

```
montant_cents = (poids_grammes x prix_kilo_cents + 500) / 1000
```

| Poids | Prix au kilo | Exact | Montant |
| --- | --- | --- | --- |
| 278 g | 45,00 | 12,51 | 12,51 (UC4112) |
| 612 g | 19,90 | 12,1788 | 12,18 |
| 250 g | 12,30 | 3,075 | 3,08 (cas du demi) |

Règles :

- **Avec une balance, le montant est celui de la balance**, jamais le recalcul de la caisse : la ligne
  signée et imprimée reprend le poids, le prix unitaire et le prix de l'enregistrement 02.
- La caisse recalcule quand même et refuse la lecture (`scale_price_mismatch`) si l'écart dépasse 1 centime,
  ou si le prix unitaire renvoyé diffère de celui envoyé. Dialog 06 ne porte qu'une parité par caractère
  dans le relevé disponible, et ce contrôle attrape une trame abîmée.
- La règle d'arrondi de la balance n'est pas connue (question D5). Si elle diffère d'un centime, la ligne
  signée a un `vat.price` différent de `quantity x unitPrice` : la spécification ne l'interdit pas, le
  simulateur module2 reste à essayer (F2).
- TVA : le code de la ligne suit le type de commande (`vat_code`, `vat_code_takeaway`), comme aujourd'hui.
  Le montant est TVA comprise et le FDM calcule la ventilation (`vatCalc`), sans changement.
- L'arrondi au 0,05 EUR reste au niveau du paiement en espèces, comme dans UC4112.

### 4.6 Panier et ticket

- Toucher un produit au poids ouvre la fenêtre de pesée (section 5.3). La ligne n'apparaît qu'après une
  lecture valide de la balance, et le poids vient du moteur, jamais de l'interface.
- La ligne pesée n'a ni bouton + ni bouton - et ne fusionne pas avec une autre. La changer revient à
  la retirer (correction de -1, même poids) et à repeser. `do_update_order_line_quantity` la refuse
  (`line_weighed_fixed`).
- Les remises à la caisse fonctionnent : FAQ Q201 c) confirme le `priceChange` sur la ligne pesée. Le
  ticket imprime alors le montant de la balance, puis la remise sur sa propre ligne.
- Copies (C), additions (P), formation (T) et ticket numérique passent par `TicketLine`, qui reçoit un
  champ `quantity_type` avec `#[serde(default)]` pour relire les tickets déjà stockés.
- `format_qty` imprime trois décimales et l'unité pour un poids. Le prix au kilo s'imprime toujours pour
  une ligne pesée, sur une seconde ligne.

Ticket de 80 mm (48 colonnes) :

```
0,278 kg  Catch of the day              12,51  B
    45,00 €/kg
```

Ticket de 58 mm (35 colonnes, `item_line` passe le prix à la ligne quand le nom ne laisse pas la place) :

```
0,612 kg  Bol malatang sur place
                           12,18  B
    19,90 €/kg
```

Remboursement : `-0,278 kg  Catch of the day  -12,51  B`, le prix au kilo restant positif. Le test
`assert_58_mm_prints_the_80_mm_words` s'applique à ces lignes. La présentation exacte dépend de la
réponse du SPF Économie (E4) ; le formatage reste dans la partie isolée (6.3).

Les écrans (`OrderLineItem`, `CartSidebar`, `cartAmounts.ts`, `CartAmounts.kt`, l'aperçu de remboursement)
et la ventilation `vat_split` de la FFI utilisent le montant de la pesée au lieu du prix x quantité.

### 4.7 Rapports

Les X et Z viennent des requêtes signées (`report_totals.rs`) : ils ne dépendent ni d'une quantité entière
ni de la pièce. Aucun changement attendu, à prouver par un test Z après une vente pesée, une correction et
un remboursement. Le rapport des ventes par produit additionne déjà en millièmes ; il doit afficher
« 1,234 kg » au lieu d'un nombre sans unité. `pili-accounting` ne lit aucune quantité.

### 4.8 Remboursement

**Proposition : une pesée se rembourse entière**, avec le poids et le montant signés à l'origine
(`quantity` -0.278, `negQuantityReason: REFUND`, `vat.price` -12.51).

- `RefundItem.quantity` vaut 1 et `line_id` devient obligatoire pour une ligne pesée.
- `Bucket` reçoit le poids, sinon deux pesées du même produit au même prix se confondraient.
- `already_refunded_quantities` lit `quantityType` : un `KILOGRAM` compte une pesée de ce poids au lieu
  d'arrondir la quantité (section 4.2).
- Refusé : une quantité supérieure à 1 sur une pesée (`refund_weighed_line_whole`), et le rejeu d'une pesée
  déjà remboursée (`check_sold`).

Un remboursement partiel (-0,100 kg) est admis par la spécification mais ne vient d'aucune pesée : il
crée un calcul et un arrondi propres à la caisse, et les arrondis successifs peuvent dépasser le montant
de la ligne. Il attend un besoin réel (P3). L'écran de remboursement l'annonce : « pesée remboursée entière ».

### 4.9 HubRise

- Catalogue : étiquette `pos:sold_by_weight` lue dans `catalog_map.rs`, comme `pos:open_price`. Le prix du
  SKU est alors le prix au kilo. `pos-catalog` (modèle et `apply.rs`) reçoit le champ.
- Commande en ligne : personne ne pèse à distance. Une commande reçue sur un produit au poids est refusée
  à l'ingestion avec un problème explicite, et le webshop vend un SKU distinct à prix fixe. FAQ Q201 e) réserve
  `PIECE` aux articles pré-emballés : un bol à prix fixe non pesé reste à confirmer (F4). `parse_quantity` continue
  de refuser les quantités non entières.
- Écho d'une vente de caisse (`hubrise_push.rs`) : envoyer `"quantity": "0.278"` et le prix au kilo. L'acceptation
  de cette forme par HubRise reste à vérifier : `pos-hubrise-mock` d'abord, puis `scripts/live_hubrise.sh` sur la
  localisation de test, à la main, avec le quota gratuit de 5 commandes par mois.
- Stock : `sold_out` fonctionne sans changement, car il porte sur le produit et non sur la quantité.

### 4.10 Périmètre de la v1 : gel, dossier et preuves

**Décision du propriétaire (2026-10-30) : UC4112 et la DPOS-400 font partie de la v1, qui n'a pas encore été
soumise.** À inscrire dans `docs/android/DECISIONS.md` (`WGT-D1`). Le dossier dit aujourd'hui le contraire :

| Fichier de `docs/android/dossier` | Ce qu'il dit aujourd'hui | Ce qu'il doit dire |
| --- | --- | --- |
| `REPONSES_MANUEL_CERT_FR.md` | Balance ☐ « Pas de prix au poids » ; Prix au poids ☐ « Pas de balance » ; périphérique supplémentaire ☐ | Les trois cochés, avec renvoi à l'annexe |
| `LIGNES_PRODUITS_FR.md`, graine `seed_spf_test.sql` | Produit 22005 ☐ « non supporté en v1.0.0 », `active = 0` ; 9 produits sur 25 non proposés | 22005 proposé ; 8 sur 25 |
| `COUVERTURE_ADVANCED_SET_FR.md` | UC4112 ✓ au niveau du protocole, sans écran opérateur | Ligne « proposé » au tableau des écrans, avec le test moteur |

**Le gel.** `android-v1.0.0-rc3` ne vaut déjà plus : à `aa37600a`, `check_android_frozen.sh` répond « frozen paths
differ » sur 36 fichiers, dont l'impression 58 mm. La reprise était donc due avant la balance, et
`ANDROID_FREEZE.md` précise qu'un changement de chemin gelé demande l'accord de l'utilisateur et un nouveau gel.
Proposition : un **nouveau candidat `android-v1.0.0-rc4`**, l'étiquette rc3 restant comme historique, posé
**une seule fois**, quand les lots sont fusionnés et que la voie de métrologie est tranchée (6.4). La procédure
de `ANDROID_FREEZE.md` s'applique telle quelle :

1. Porte complète du dépôt, dont `p5_gate.sh`, `p6_gate.sh`, les deux `check_dossier.py` et `lane_paths.py --check`,
   avec des règles pour les nouvelles crates. `check_ipc_wrappers.py`, `check_command_parity` et `check_ui_parity`
   comptent les nouvelles commandes (148 aujourd'hui).
2. `plat_about_engine_ref` passe à `pili-pos 1.0.0, android-v1.0.0-rc4` dans `strings_plat_about.xml`, pour toutes
   les langues, et la capture A82 est refaite.
3. `check_android_frozen.sh --ref HEAD`, puis `check_repro.sh --commit HEAD` : deux constructions identiques.
4. Étiquette locale, jamais poussée ; `FREEZE_SHA_RECORDED` dans `p4_gate.sh` ; section Values de `ANDROID_FREEZE.md` ;
   étiquette par défaut de `check_android_frozen.sh`.
5. Les fichiers qui nomment rc3 : `CADRE_DEMANDE_FR.md`, `CHECKLIST_DEPLOIEMENT_FR.md`, `EXPORT_NOCTURNE_FR.md`,
   `GARANTIE_MAJ_LANGUES_FR.md`, `DOSSIER_EVIDENCE.md`.

`pos-scale` et, en voie A, `pos-scale-legal` entrent d'eux-mêmes dans les chemins gelés (`cargo tree` de `pos-ffi`).
`check_apk.sh` doit connaître le manifeste modifié (7.2). Les valeurs de référence EV-04 et EV-05 restent celles de
l'étiquette finale `android-v1.0.0` (INT-D47).

**Les preuves** (`docs/android/DOSSIER_EVIDENCE.md`). Celles qui sont faites deviennent à refaire, car elles citent
un autre APK :

| Preuve | Effet |
| --- | --- |
| EV-04, EV-05, EV-01 à EV-03 | Portent sur la version finale, comme prévu ; le candidat rc3 disparaît |
| EV-06 (résultats UC) | UC4112 passe par le moteur et l'écran, plus seulement par le protocole |
| EV-08 (tickets, faite) | **À refaire** : ligne pesée à 80 et à 58 mm, copie, addition, remboursement, facture |
| EV-09 (captures, faite) | **À refaire en partie** : fenêtre de pesée, panier, remboursement, réglages, voyant, A82 ; runbook `CAPTURES_ECRAN_RUNBOOK_FR.md` |
| EV-10, EV-11, EV-12 | Déroulement avec une vente pesée ; 24 h avec la liaison active ; coupure de courant pendant une pesée |
| EV-14 (module2 en direct, sur accord) | Une vente pesée, un écart d'un centime s'il existe (F2), un remboursement |
| EV-15 (vrai D3 Pro) | S'ajoutent le port USB ou RJ11 et la balance |
| EV-16 | Vérifier que l'identifiant de l'appareil reste juste après la reprise |
| EV-17 | Nouveaux comptes de tests |
| EV-21 à EV-28 | Nouvelles preuves de la section 8, à ajouter à `DOSSIER_EVIDENCE.md` |

**La machine de test** (`CADRE_DEMANDE_FR.md` §8) est le profil émulé `sunmi_d3_pro_gms_api33`, sans USB ni port série,
avec une imprimante et un FDM simulés. La balance y serait `pos-scale-mock` par le transport `tcp`, sur le même modèle,
sauf si le SPF Finances demande une vraie balance (F5). Ce transport fait donc partie de l'APK déclaré.

**Le dossier.** Outre le tableau ci-dessus : `CADRE_DEMANDE_FR.md`, `INDEX_DOSSIER_FR.md`, `MANIFEST.txt`,
`LISTE_FONCTIONNALITES_FR.md`, `MAPPING_CONFORMITE_FR.md`, `TICKETS_EXEMPLES_FR.md`, `FLUX_REMBOURSEMENT_FR.md`,
`FLUX_DONNEES_FR.md`, `SCHEMA_BD_FR.md`, `SCHEMA_ER_FR.md`, `SCHEMA_ASSEMBLAGE_FR.md`, `PLATEFORME_ANDROID_FR.md`,
`CHECKLIST_DEPLOIEMENT_FR.md`, `MANUEL_UTILISATEUR_FR.md`, `MANUEL_PROGRAMMATION_FR.md`, `MANUEL_INSTALLATION_FR.md`,
`RESULTATS_TESTS_UC_FR.md`, `RAPPORT_TESTS_BASELINE_FR.md`, `JOURNAL_TESTS_PRE_SOUMISSION_FR.md`, et une nouvelle annexe
`BALANCE_FR.md`. `docs/SUBMISSION_ARTIFACTS` (Linux, non soumis) n'est pas concerné.

Après la certification, toute évolution passe par la règle du SPF (Introduction à la certification, 2026-09-03, §4) :
une mise à jour ayant un impact sur le volet SCE n'est installée chez les clients qu'une fois certifiée.

## 5. Protocole : Bizerba « Checkout-Dialog 06 » sur la DPOS-400

Tout ce qui suit est **à confirmer sur le manuel Dibal** (table des matières vue en ligne : réf. 49-TGDP4ES02,
V.1.08, sections « Dialog 06 » et variante sans poids minimal) et sur une balance.

### 5.1 Liaison

9600 bauds, 7 bits de données, parité impaire, 1 bit d'arrêt ; seulement TX, RX et masse (ni RTS ni CTS).
Protocole 24 dans le menu de test de la balance, avec des variantes sans poids minimal et sans tare reçue.

Caractères de contrôle : `EOT` 04h, `ENQ` 05h, `STX` 02h, `ETX` 03h, `ESC` 1Bh, `NAK` 15h.

### 5.2 Enregistrements

| Sens | Enregistrement | Forme | Rôle |
| --- | --- | --- | --- |
| Caisse vers balance | 01 | `EOT STX "01" ESC <prix 5/6 chiffres> ESC ETX` | Prix unitaire |
| Caisse vers balance | 03 | 01 + `ESC <tare 4 chiffres>` | Prix et tare |
| Caisse vers balance | 04 | 01 + `ESC <texte 13 car.>` | Prix et texte |
| Caisse vers balance | 05 | prix, tare et texte | Les trois |
| Caisse vers balance | `EOT ENQ` | | Demande du poids |
| Balance vers caisse | 02 | `STX "02" ESC <statut, 33h = kg> ESC <poids 5 chiffres> ESC <prix unitaire> ESC <prix de vente 6 chiffres> ETX` | Résultat de la pesée |
| Balance vers caisse | `NAK` | | Pas de résultat, demander le statut |
| Caisse vers balance | 08 | | Statut après un `NAK` |
| Balance vers caisse | 09 | `S1 S0` | Code du statut (5.4) |
| Balance vers caisse | 11 | `32h` + nombre aléatoire `Z` | Demande de contrôle du logiciel |
| Caisse vers balance | 10 | Somme(s) de contrôle + valeur de correction, ASCII hexadécimal | Réponse au contrôle |
| Balance vers caisse | 11 | `30h` ou `31h` | Résultat du contrôle (refusé, accepté) |
| Caisse vers balance | 20 | | Version logique active ou non |

Exemple avec UC4112, sous réserve du cadrage réel (nombre de chiffres, décimales selon le réglage) :

```
caisse  -> EOT STX "01" ESC "04500" ESC ETX
caisse  -> EOT ENQ
balance -> STX "02" ESC "3" ESC "00278" ESC "04500" ESC "001251" ETX
```

### 5.3 Déroulement d'une pesée

1. L'opérateur touche le produit. Le moteur vérifie que le contrôle du logiciel est valide, sinon il l'attend.
2. Le moteur envoie 01, 03, 04 ou 05 selon le produit (`scale_tare_grams`, nom). Le texte tient en 13 caractères
   ASCII : le nom français, sans accents et coupé. Un nom sans transcription (chinois) ne s'envoie pas : le moteur
   utilise 01 ou 03.
3. La balance affiche le prix au kilo, la fenêtre de la caisse affiche « Poser le bol sur la balance ».
4. Le moteur envoie `EOT ENQ` à intervalle court, avec un délai total (`weigh_timeout_ms`). Sur `NAK`, il envoie 08
   et lit le code 09.
5. Sur un 02 valide et contrôlé (4.5), le moteur écrit `scale_weighings` et la ligne dans la même transaction.
6. Annulation à tout moment : aucune ligne, aucun événement. La pesée n'est pas un événement fiscal : la ligne suit
   ensuite le chemin habituel (P ou N).

Cadence, délais, accusés de réception et comportement au retrait du plateau restent à mesurer.

### 5.4 Codes d'état (enregistrement 09)

| Code | Sens | Comportement de la caisse |
| --- | --- | --- |
| 20 | Encore en mouvement | Attendre et redemander, message « Stabilisation » |
| 21 | Aucun mouvement depuis la dernière pesée | Message « Retirer puis reposer », pas de nouvelle ligne |
| 22 | Prix pas encore disponible | Attendre et redemander |
| 30 | Sous le minimum | Message et aucune ligne (`scale_below_min`) |
| 31 | Sous-charge | Message et aucune ligne (`scale_underload`) |
| 32 | Surcharge | Message et aucune ligne (`scale_overload`) |
| 11, 12, 13 | Prix, tare ou texte invalide | Abandon, journal, alerte de configuration (`scale_price_refused`) |
| 01 | Erreur générale | Abandon, nouvelle tentative au choix de l'opérateur |
| 02 | Erreur de parité | Une répétition automatique, puis abandon |

Le code 21 protège du double comptage d'un même bol : il n'est jamais contourné par la caisse.

### 5.5 Contrôle du logiciel (enregistrements 11, 10, 20)

La balance demande le contrôle à la mise sous tension, après une erreur et toutes les 50 pesées. La caisse
répond par l'enregistrement 10, avec la ou les sommes de contrôle et la valeur de correction de son logiciel
légalement pertinent, calculées avec le nombre aléatoire `Z`. Sans « contrôle OK », la balance refuse les prix
unitaires.

Conséquences de conception :

- Le pilote a un état : `Unverified`, `Checking`, `Verified`. Aucun prix n'est envoyé hors `Verified` dans ce mode.
- Une demande 11 peut arriver à tout moment, y compris entre deux pesées d'un service chargé. Le pilote la traite
  en priorité et la fenêtre de pesée attend sans rien perdre.
- Un refus laisse la balance inutilisable : voyant rouge, ventes à la pièce intactes, alerte au propriétaire
  (`alerts/`), et **aucune option pour contourner** le contrôle.
- L'algorithme est inconnu. Un rapport trouvé en ligne évoque un CRC16 avec rotation de bits, refusé par une balance
  de modèle non précisé : il ne sert pas de référence.
- La caisse imprime le poids, le prix unitaire et le prix renvoyés, sans les modifier (4.5).

## 6. Métrologie légale

Tout ce qui suit est une **lecture non vérifiée** : ni le SPF Économie ni Dibal n'ont été consultés.

### 6.1 Pourquoi la question se pose

L'enregistrement 10 montre que Dialog 06 traite le logiciel de caisse comme la partie légalement pertinente d'un
ensemble avec l'instrument de pesage à fonctionnement non automatique (NAWI). Les guides concernés :

- **WELMEC 2.2** (Issue 3, 2007) : terminaux de point de vente raccordés à un NAWI pour la vente directe au public,
  par une interface numérique protectrice. D'après son résumé, il ne fixe pas lui-même les exigences logicielles et
  suit les principes du guide 2.5.
- **WELMEC 2.5** : approche modulaire, ordinateurs et périphériques numériques raccordés à un NAWI.
- **WELMEC 2.3, 7.2 et 7.5** : logiciel. Lequel s'applique aujourd'hui en Belgique reste à établir (E6).

### 6.2 Trois voies possibles

| Voie | Principe | Effet sur pili-pos | Dépend de |
| --- | --- | --- | --- |
| A. Dialog 06 avec contrôle | Protocole 24, somme de contrôle enregistrée | La caisse est légalement pertinente : isolation du logiciel, enregistrement, une partie à chaque mise à jour touchant cette partie | E1, E2, D1 à D3 |
| B. Protocole sans contrôle | La balance affiche poids, prix unitaire et prix au client ; la caisse ne fait que reprendre | Pas d'enregistrement, même pilote sans l'état de contrôle | E2, D4 |
| C. Balance autonome | Le caissier lit et saisit le poids | Aucune liaison, aucune trame, mais une saisie qui peut se tromper | E3 |

Préférence à ce stade : B si le SPF Économie l'accepte, A sinon. C n'est retenue que si elle est explicitement admise.
Le pilote est écrit pour A et B : seule la phase 4 du plan en dépend.

### 6.3 Isoler la partie légalement pertinente

Si une somme portait sur tout l'APK, chaque mise à jour la changerait, et chaque correctif sans rapport avec la
pesée demanderait une nouvelle valeur enregistrée. Options :

| Option | Effet | Avis |
| --- | --- | --- |
| Somme sur tout l'APK ou sur `libpos_ffi.so` | Change à chaque version | À écarter sauf exigence du SPF |
| Constante compilée | Ne prouve rien sur le code exécuté | À écarter |
| Bibliothèque séparée (`pos-scale-legal`, un `cdylib`), somme sur ses octets à l'exécution | Ne change que si la partie légale change ; les constructions reproductibles (`check_repro.sh`) permettent de le prouver | **Proposée**, sous réserve de D2 |

La partie isolée contient : le décodage de l'enregistrement 02, le contrôle (4.5), la production des valeurs de la
ligne signée et le formatage de la ligne de ticket. Le reste (`receipt.rs`, le moteur) reçoit des valeurs figées et ne
les recalcule pas. Un test échoue si `pos-scale-legal` change sans que sa version déclarée change.

### 6.4 Blocage possible de la soumission v1

La balance étant dans la v1, les questions de métrologie (section 11) sont sur le chemin de la soumission :

- Une vraie DPOS-400 en protocole 24 refuse les prix sans « contrôle OK » (5.5). Sans l'algorithme (D1) et sans valeur
  attribuée (E1, D2), la voie A ne fonctionne pas de bout en bout.
- La réponse change les octets de l'APK : bibliothèque séparée, manifeste, ligne de ticket (E4). Répondue après le gel,
  elle impose un nouveau gel ; répondue après la certification, une mise à jour à certifier.
- Le délai du SPF Économie est inconnu, et une évaluation par un organisme n'est pas exclue.

Les documents du SPF Finances lus ici ne demandent aucune preuve métrologique pour la ligne « Balance » : ils ne
mentionnent que la fonction « prix au kg ». Cela ne vaut pas accord (F6).

**Ce qui peut se faire d'abord**, car cela ne dépend d'aucune réponse :

| Ordre | Travail | Pourquoi maintenant |
| --- | --- | --- |
| 1 | Le propriétaire envoie les questions au SPF Économie (`metrology.regulation@economie.fgov.be`) et à Dibal ou à son revendeur ; le présent document ne les envoie pas | C'est le seul vrai délai ; le revendeur connaît sans doute la pratique des autres éditeurs de caisse |
| 2 | Manuel Dibal et balance de prêt, essai à vide (EV-22) | Lève D1, D5 et D6, et le choix USB ou RJ11 |
| 3 | Volet fiscal (phase 1) | Nécessaire dans toutes les voies, testable sans balance |
| 4 | `pos-scale` et le simulateur, sans le contrôle (phase 2) | Nécessaires en voies A et B ; le contrôle s'ajoute ensuite |

**Ce qui attend la réponse** : la partie légale (phase 4), le gel rc4, les captures et les exemples de tickets définitifs.
Les faire plus tôt, c'est risquer de les refaire.

**Point de décision.** Le propriétaire fixe une date limite (P1). À cette date :

| Situation | Suite |
| --- | --- |
| Voie B acceptée par écrit (E2) | Gel rc4 sans partie légale, soumission |
| Voie A exigée, valeurs obtenues | Phase 4, puis gel rc4, soumission |
| Voie C acceptée par écrit (E3) | Gel rc4 avec poids saisi, liaison retirée de la v1 |
| Pas de réponse, ou voie A hors délai | Choix du propriétaire : **(a)** attendre et décaler la soumission, ou **(b)** soumettre le volet fiscal et le pilote avec la section `scale` éteinte par défaut, et n'activer la liaison chez un client qu'après la réponse écrite. La voie (b) suppose l'accord de F6 et risque une mise à jour à certifier si la réponse change le code |

Recommandation : **(b) si F6 l'admet**, car les installations sans balance ne dépendent pas de la métrologie et la date
de soumission reste entre nos mains ; (a) sinon. Dans tous les cas, **aucun restaurant ne vend au poids par la liaison
avant une réponse écrite du SPF Économie**, et aucun contournement du contrôle n'est livré pour tenir une date.

## 7. Matériel et architecture

### 7.1 Ce qui est connu

- **D3 Pro** (`PLATEFORME_ANDROID_FR.md` §1, fiche du fabricant) : 1 RJ11 série,
  1 USB-A 3.0, 2 USB-A 2.0, 1 USB-C 2.0. Un seul port série, et les badges USB utilisent déjà un port A.
- **DPOS-400** : RS-232 et USB, un seul câble pour l'alimentation et les données ; 40 protocoles sélectionnables selon
  sa fiche, et des protocoles supplémentaires téléchargeables.

### 7.2 Raccordement

| Critère | USB (pilote USB-série dans l'application) | RJ11 vers DB9 (port série du D3 Pro) |
| --- | --- | --- |
| Accès depuis Android | API `UsbManager`, standard ; 7 bits et parité à confirmer selon la puce | Aucune API Sunmi trouvée ; nœud `/dev/tty*` et droits SELinux d'une application non système inconnus |
| Câble | Standard si la balance se présente comme un port série virtuel | Sur mesure ; brochage du D3 Pro inconnu, à mesurer avant tout branchement |
| Autorisation | Filtre `USB_DEVICE_ATTACHED` et `device_filter.xml` ; dialogue éventuel en mode kiosque à mesurer | Rien à déclarer si le nœud est accessible |
| Alimentation | Par le câble, courant du port à vérifier | Alimentation séparée de la balance |
| Autre terminal, Linux | Même code | Propre au D3 Pro |
| Inconnue principale | Classe USB et puce de la balance (D6) | Accès au nœud sur l'image de série |

**Décision proposée : USB d'abord**, parce que tout y est standard et que le code reste valable sur un autre terminal.
Le RJ11 sert de repli si le protocole 24 n'est pas disponible en USB (D6). Le choix final suit l'essai EV-22.
Sur la machine de test émulée, ni l'un ni l'autre n'existe : le transport y est `tcp` (4.10).

Côté application : `android.hardware.usb.host` avec `required="false"` dans le manifeste, comme le NFC ; bibliothèque
USB-série à évaluer (`usb-serial-for-android`). `DISALLOW_USB_FILE_TRANSFER` vise le transfert vers un ordinateur et
ne devrait pas gêner l'hôte USB, à confirmer. Tout cela modifie le manifeste, donc `PLATEFORME_ANDROID_FR.md` §3.

### 7.3 Moteur et pilote

Le moteur ne connaît pas le matériel, comme pour les badges (`badge_tap`) : il ne connaît ni l'USB ni le RJ11.
La différence avec le badge est que la balance dialogue, donc le protocole vit en Rust et seul le transport change.

```
              +-----------------------------+
 écrans  -->  | pos-engine commands/scale   |  commandes, événement scale-reading
              +--------------+--------------+
                             |
              +--------------v--------------+
              | pos-scale                   |  machine d'états Dialog 06, sans E/S
              |   pos-scale-legal (cdylib)  |  décodage, contrôle, ligne de ticket
              +--------------+--------------+
                             | trait ScaleTransport (octets)
        +--------------------+---------------------+
        |                    |                     |
  AndroidBridge             Tcp                 Scripted
  (Kotlin : USB, RJ11)      (pos-scale-mock)    (tests, fonctionnalité scale-scripted)
```

- **`crates/pos-scale`** : `Dialog06Session` reçoit des octets et rend des octets et des événements, sans horloge ni
  fil d'exécution, donc entièrement testable. Le moteur fournit le temps.
- **`crates/pos-scale-mock`** (`publish = false`, comme `pos-printer-mock` et `pos-eft-mock`) : une balance qui parle
  Dialog 06 sur TCP, avec un petit serveur HTTP pour poser un poids, simuler un mouvement, renvoyer chaque code 09,
  couper la liaison, compter jusqu'à 50 pesées, accepter ou refuser le contrôle, et abîmer une trame.
- **Android** : un objet `ScaleLink` dans `pos-ffi`, sur le modèle d'`EventStream` (`next_tx()` attendu dans une
  coroutine) mais avec une file sans perte, plus `scale_rx(bytes)`. La diffusion des événements peut perdre des messages
  (`Resync`) : elle ne convient pas à des octets.
- **Commandes** : `get_scale_status`, `scale_start_weighing`, `scale_cancel_weighing`, et une ligne ajoutée par le moteur
  à partir de sa propre lecture. Aucune commande n'accepte un poids venant de l'interface (sauf voie C).
- **Linux** : reste compilé et à parité (TCP en essai) ; pas de transport série tant que le produit n'est pas soumis.

Section de `pos.json`, absente par défaut (absente : aucun écran, aucune liaison, ventes inchangées) :

```json
"scale": {
  "kind": "dibal_dialog06",
  "transport": "android_usb",
  "tcp_address": null,
  "price_digits": 6,
  "software_check": "required",
  "weigh_timeout_ms": 15000
}
```

Une entrée `scale` dans `problem_code` et `PROBLEM_CODES` ; un produit `sold_by_weight` sans balance configurée reste
hors vente (bouton grisé, `scale_not_configured`). Erreurs à déclarer dans `commands/errors.rs` et à traduire : celles
des sections 4 et 5, plus `scale_unreachable` et `scale_check_failed`. Voyant de balance dans `StatusLights`, et ligne
de la balance dans le panneau des alertes au propriétaire.

## 8. Plan par phases

| Phase | Contenu | Tests | Preuves | Condition de sortie |
| --- | --- | --- | --- | --- |
| 0. Questions et essai à vide | Envoi des questions par le propriétaire, manuel Dibal, balance de prêt | Aucun | EV-21, EV-22 | Questions parties ; raccordement choisi |
| 1. Volet fiscal | Sections 4.3 à 4.9, sans lien balance ; pesée fournie par `Scripted` | Scénario `uc4112_weighed_product` | EV-23 | Rejeu de UC4112 identique, tests verts |
| 2. Pilote et simulateur | `pos-scale`, `pos-scale-mock`, commandes, écrans | Protocole contre le simulateur | EV-24 | Toutes les branches 5.3 à 5.5 couvertes |
| 3. Raccordement réel | Transport Android, autorisations | Banc avec vraie balance | EV-25 | 200 pesées sans perte ni écart |
| **Point de décision** | Voie de métrologie arrêtée (6.4) | | EV-21 | Voie A, B ou C, ou choix (a) ou (b) du propriétaire |
| 4. Partie légale (voie A seulement) | `pos-scale-legal`, somme, enregistrement | Contrôle accepté par la vraie balance | EV-26 | Valeurs attribuées, contrôle OK |
| 5. Gel et dossier | Gel rc4, preuves à refaire, dossier, annexe (4.10) | Porte complète, parcours sur l'émulateur et le vrai terminal | EV-27 | `check_repro.sh` et `check_dossier.py` verts |
| 6. Soumission | Dossier v1 déposé par le propriétaire | | EV-18 | Certification de la v1 |
| 7. Premier restaurant | Un restaurant, un service, après la certification | Rapprochement Z et pesées | EV-28 | Service sans incident |

Les phases 0 à 3 peuvent commencer tout de suite et se chevaucher. Les phases 4 et 5 attendent le point de décision.
La phase 7 attend la certification et la réponse écrite du SPF Économie.

### Tests

**Fiscal (phase 1)**

- `uc4112_weighed_product` dans `spf_e2e_fixtures.rs` : rejeu moteur champ par champ avec `assert_matches_fixture`
  (`quantity` 0.278, `KILOGRAM`, `unitPrice` 45, `vats` 12.51, total 15.01, arrondi -0,01), produit 22005 du jeu de test.
- Unitaires : arrondi entier (4.5), dont le demi et les bornes (99,999 kg et prix maximal) ; `line_total()` ; sérialisation
  à au plus trois décimales pour tous les poids de 1 à 99 999 g.
- Correction avant paiement, remise sur une ligne pesée (FAQ Q201 c), pesée et supplément sur des lignes séparées (Q201 b),
  commande P puis N, transfert de table, formation, ligne pesée sur un bon de cuisine et en temps de service.
- Remboursement : pesée entière ; deuxième remboursement refusé (le piège de 4.2) ; deux pesées du même produit ;
  quantité supérieure à 1 refusée ; pesée et ligne à la pièce dans le même ticket.
- Rapports : X, Z, Z utilisateur et rapport des ventes après vente, correction et remboursement ; `pili-accounting` rapproché ;
  facture avec `KGM` ; export statistique.
- Impression : tous les formats à 80 et à 58 mm, y compris copie, addition et remboursement.
- HubRise : étiquette lue et écho contre `pos-hubrise-mock`, commande en ligne refusée (`crates/pos-engine/tests/hubrise_*.rs`).
- Non-régression : une installation sans `scale` signe et imprime à l'octet près comme avant (exemples officiels inchangés).
- FFI : contrats des nouveaux champs ; Roborazzi et Playwright pour la fenêtre de pesée et la ligne.

**Protocole, avec `pos-scale-mock` (phase 2)**

- Chaque enregistrement (01, 03, 04, 05, 02, 08, 09, 10, 11, 20) et chaque code 09 du tableau 5.4.
- Mouvement puis stabilisation ; retrait non fait (code 21) ; annulation à chaque étape ; délai dépassé ; liaison coupée
  en pleine trame ; trame abîmée, parité, écart de prix et prix renvoyé différent.
- Contrôle à la mise sous tension, à la 50e pesée pendant une pesée en cours, contrôle refusé (plus aucun prix envoyé).
- Propriété : quelle que soit la suite d'octets reçue, aucune ligne n'est créée sans un 02 complet et contrôlé.
- Intégrité : le montant signé est bien celui de `scale_weighings` pour chaque pesée.

### Preuves à réunir

| Réf. | Preuve |
| --- | --- |
| EV-21 | Réponses écrites du SPF Économie et de Dibal ou de son revendeur, et manuel Dibal |
| EV-22 | Essai à vide : `lsusb`, classe et puce de la balance, 7 bits et parité, nœud série et brochage du D3 Pro mesurés, dialogue d'autorisation en mode kiosque, courant consommé |
| EV-23 | Requête et réponse UC4112 archivées, tickets de 80 et 58 mm, Z, et le même scénario sur le simulateur module2 (exécution en direct à faire autoriser) |
| EV-24 | Rapport des tests du simulateur et couverture des codes |
| EV-25 | Transcription de 200 pesées, écarts à zéro, règle d'arrondi de la balance relevée (D5) |
| EV-26 | Sommes, valeur de correction, reçu d'enregistrement, contrôle OK à la mise sous tension et à la 50e pesée (voie A) |
| EV-27 | Preuves refaites (4.10), dossier à jour, certificat ou accusé de la balance (vérification métrologique) |
| EV-28 | Rapprochement d'un service : `scale_weighings`, tickets signés, Z |

## 9. Effort (jours-homme, estimation sans les délais d'administration)

| Volet | Jours |
| --- | --- |
| Moteur et fiscal (ligne pesée, remboursement) | 5 |
| Base de données | 1 |
| Impression (80 et 58 mm) | 1,5 |
| HubRise | 1,5 |
| Vue | 2 |
| Android et FFI | 3 |
| `pos-scale` et `pos-scale-mock` | 6 |
| Transport Android et banc | 5 (RJ11 : à réestimer après EV-22) |
| Partie légale (voie A seulement) | 5 à 10, algorithme inconnu |
| Tests et jeux d'essai | 5 |
| Dossier | 5 |
| Reprise du gel et des preuves (due en partie sans la balance, 58 mm) | 3 |
| **Total** | **environ 38 en voie B, 43 à 48 en voie A** |

## 10. Risques

- **Métrologie et date de soumission** : une réponse tardive ou défavorable peut retarder la v1, voire exiger une évaluation
  du logiciel par un organisme, au coût inconnu. D'où les questions dès maintenant et le point de décision (6.4).
- **Gel posé trop tôt** : chaque reprise refait les preuves. Un seul gel rc4, après le point de décision.
- **Machine de test** : une balance simulée peut ne pas suffire au SPF Finances (F5).
- **Cadence des mises à jour** : en voie A, une partie légale mal isolée lierait chaque version à une nouvelle attribution
  de valeur (6.3).
- **Montant faux** : une erreur sur le poids, le prix ou l'arrondi touche de l'argent réel et une donnée signée, qui ne se
  répare que par un remboursement. D'où entiers, contrôle croisé et table d'audit.
- **Double remboursement** : l'arrondi de `already_refunded_quantities`, sans effet sur des quantités entières, laisserait
  rembourser plusieurs fois une même pesée. Le test s'écrit avant la fonction.
- **Ticket** : `format_qty` imprimerait « 0,28 » pour 0,278 kg, soit un poids faux sur un document fiscal.
- **Panne de balance en service** : les ventes à la pièce continuent, les produits au poids ne se vendent plus (sauf voie C).
  Le restaurant doit le savoir avant de composer sa carte.
- **Port** : un seul RJ11 et un brochage inconnu ; un mauvais câble peut abîmer le matériel. Aucun branchement avant mesure.

## 11. Questions ouvertes

Toutes **non vérifiées** à ce jour, et aucune n'a été posée.

### Propriétaire

- P1. Quelle date limite pour la réponse de métrologie, et faute de réponse, attendre (a) ou soumettre avec la liaison éteinte (b) ?
  Recommandation : (b) si F6 l'admet (6.4).
- P2. Un premier restaurant est-il prévu, et à quelle date ? Cela fixe la date limite de P1.
- P3. Le remboursement partiel d'une pesée est-il utile, ou la pesée entière suffit-elle ?
- P4. Le contenant : tare fixe envoyée à la balance, ou tare posée par l'opérateur sur la balance ?

### SPF Économie, service Métrologie

- E1. Dans une vente directe au public avec NAWI et caisse raccordée, la caisse est-elle légalement pertinente ? Comment sa somme
  de contrôle et sa valeur de correction sont-elles attribuées et enregistrées en Belgique, et par qui (fabricant de la balance,
  éditeur, organisme notifié, SPF) ? Que déclenche une mise à jour de la caisse ?
- E2. Un protocole sans somme de contrôle est-il acceptable lorsque la balance affiche elle-même poids, prix unitaire et prix
  sur son écran client ?
- E3. Une balance autonome vérifiée, dont l'opérateur saisit le poids dans la caisse, est-elle admise pour la vente directe au public ?
- E4. Que doit imprimer le ticket de caisse pour une ligne pesée : poids, unité, prix unitaire, prix, tare, mention, référence de
  la balance ? Quelles sont les règles d'arrondi ?
- E5. Une tare envoyée par la caisse (enregistrements 03 et 05) est-elle admise, et doit-elle figurer sur le ticket ? Une remise
  pour contenant (`priceChange`, FAQ Q201 c) pose-t-elle un problème métrologique ?
- E6. Quels guides sont suivis (WELMEC 2.2, 2.5, logiciel 2.3, 7.2 ou 7.5) ? Quelle est la vérification périodique de la balance,
  et à qui incombe-t-elle ?
- E7. Une correction ou un remboursement d'une pesée a-t-il une exigence propre ?

### Dibal ou son revendeur belge

- D1. Algorithme exact de l'enregistrement 10, signification de `Z`, nombre de sommes, format de la valeur de correction.
- D2. Comment pili-pos reçoit-il sa valeur : quelle procédure, quels documents et quel coût ? Une somme sur une bibliothèque séparée
  est-elle acceptée ? Que se passe-t-il à une mise à jour, et le réglage de la balance change-t-il ?
- D3. La DPOS-400 proposée en Belgique est-elle livrée vérifiée avec le protocole 24, et qui le règle (menu de test, scellé) ?
- D4. Existe-t-il un protocole sans contrôle autorisé en Belgique parmi les 40 de la balance, et comment renvoie-t-il le poids et le prix ?
- D5. Règle d'arrondi du prix de vente ; plage, précision, cadrage du poids et du prix ; 5 ou 6 chiffres ; poids minimal.
- D6. Le port USB transporte-t-il le protocole 24 ? Quelle classe USB et quelle puce ? Quel courant demande un câble unique ?
- D7. Y a-t-il des cadences, délais et accusés imposés ? Existe-t-il un simulateur officiel ou un outil de validation pour éditeur ?
- D8. La balance a-t-elle des exigences sur le terminal (certificat de compatibilité, liste d'éditeurs) ?

### SPF Finances (mailbox producteurs)

- F1. Déclarer la balance change-t-il autre chose que les lignes « Balance » et « Prix au poids » du manuel, et un rejeu de UC4112 suffit-il ?
- F2. Le FDM accepte-t-il un `vat.price` différent d'un centime de `quantity x unitPrice`, quand l'instrument légal impose son prix ?
  À essayer d'abord sur le simulateur module2.
- F3. Une ligne pesée peut-elle se rembourser seulement entière ?
- F4. Un bol vendu à prix fixe sans pesée (commande en ligne) peut-il être signé en `PIECE`, alors que FAQ Q201 e) réserve ce type
  aux articles pré-emballés à prix fixe ?
- F5. La machine de test peut-elle présenter une balance simulée, comme le FDM et l'imprimante, ou faut-il une vraie DPOS-400 à
  l'admission ?
- F6. La certification v1 peut-elle être déposée avec la balance déclarée alors que la réponse de métrologie n'est pas encore là,
  la liaison restant éteinte chez les clients ? Une preuve métrologique est-elle attendue dans le dossier ?

## Sources consultées le 2026-10-30

Non relues en entier ; à vérifier avant toute décision.

- Fiche DPOS-400 : <https://tamtrongroup.com/wp-content/uploads/2023/02/DPOS-400.pdf>
- Fil B4X sur Dialog 06, avec un lien vers une spécification PDF : <https://www.b4x.com/android/forum/threads/scale-interface-checkout-dialog-06.157367/?amp=1>
- Manuel Dibal, table des matières vue seulement : <https://www.scribd.com/document/637101537/Untitled>
- Question CodeProject sur une somme de type CRC16 : <https://static.codeproject.com/Questions/286726/answer>
- WELMEC 2.2 : <https://welmec-upgrade.bs.ptb.de/welmec/documents/guides/2.2/WELMEC_Guide_2.2_v2007_Issue_3_FR.pdf>
- WELMEC 2.5 : <https://www.welmec.org/welmec/documents/guides/2.5/2-5.pdf>
- Guides WELMEC : <https://www.welmec.org/guides-and-publications/guides/>
- Enquête WELMEC NAWI et POS, 2019 : <https://welmec-upgrade.bs.ptb.de/welmec/documents/informative_documents/INFO_05-001_Screening_project_NAWI_with_POS_2019.pdf>
