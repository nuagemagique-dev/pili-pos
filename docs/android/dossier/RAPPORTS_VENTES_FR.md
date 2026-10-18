# Rapports de ventes non fiscaux

Annexe du dossier de certification de la version 1.0.0. Elle répond à la question du manuel de
certification, section L « Rapports » : « Le POS dispose-t-il d'autres rapports que les rapports
obligatoires X et Z ? Si oui uniquement : veuillez les décrire et montrer comment ils correspondent
aux montants des rapports fiscaux. Veuillez également fournir les requêtes utilisées. » La réponse
est **OUI**, pour un seul rapport :
le **rapport de ventes**, un document de gestion **non fiscal**. Il répond aussi à la FAQ des
producteurs, question 508 : un rapport interne doit être cohérent avec les données des rapports Z
fiscaux.

Le rapport de ventes **ne signe rien** : aucun événement, aucun compteur, aucun message au module
de contrôle, aucune écriture dans une table. Il lit uniquement ce qui est déjà signé et archivé.

## 1. Description

| Élément | Contenu |
|---|---|
| Nom | « Rapport de ventes », sous-titre « DOCUMENT NON FISCAL ». Le titre ne contient jamais les mots X, Z ni « CHIFFRE D'AFFAIRES » (ces mots désignent les rapports fiscaux). |
| Période | Une ou plusieurs dates comptables (BookingDate) : aujourd'hui, hier, la semaine en cours (depuis le lundi), le mois en cours (depuis le 1er) ou une période saisie. Au plus 92 jours, bornes comprises ; au-delà, le moteur refuse avec le code `sales_report_range_invalid`. |
| Par produit | Une ligne par produit vendu : quantité et montant TTC. Un remboursement est une quantité et un montant négatifs : un produit vendu puis remboursé revient à zéro. Tri par montant ou par quantité. |
| Par heure | Les heures (heure de Bruxelles lue dans `posDateTime`) où il y a eu des ventes, avec le nombre de tickets, le montant et une barre proportionnelle. L'ordre commence à l'heure de fin de journée de l'installation (`day_end_hour`), puisque la journée commerciale commence là. |
| Concordance | Pour chaque période comptable de la plage : le numéro du rapport Z qui l'a clôturée, le montant des ventes recalculé, le chiffre d'affaires N que ce Z a imprimé, et une marque : « identique », « écart de x,xx € », ou « période en cours (pas encore de Z) ». |
| Totaux | Nombre de tickets (remboursements compris), total des ventes, dont remboursements ; modifications de prix de niveau ticket ; arrondi de caisse (affiché à part, **hors total**, voir plus bas). |
| Écran | Quatrième onglet « Ventes » de l'écran Rapports, identique sur la caisse Linux et sur la caisse portable Android, en français, néerlandais, anglais et chinois à l'écran. |
| Impression | Bouton « Imprimer » : même contenu sur l'imprimante de tickets, sous le titre RAPPORT DE VENTES et DOCUMENT NON FISCAL. Pieds de page : « CECI N'EST PAS UN RAPPORT FISCAL » et « CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE ». Aucun contrôle de signature, aucun QR code, aucun numéro de ticket fiscal. |
| Bons cadeaux | La vente d'un bon cadeau (ligne de T.V.A. X, rayon « Bons cadeaux ») est comptée dans les ventes comme dans le rapport Z, dont le total reste égal à celui du Z ; l'utilisation d'un bon est une ligne de paiement `VOUCHER_STORE`. Test `the_sales_report_counts_the_voucher_line_and_still_equals_the_z` de `crates/pos-engine/tests/lane_interactions.rs`. |
| Droits | Gérant seulement (comme les rapports X et Z). |

## 2. Correspondance avec les rapports Z

Le rapport de ventes et le rapport Z lisent la **même source** : la requête signée de chaque
événement N archivé (`fiscal_events.event_data_json`), c'est-à-dire exactement ce que le module de
contrôle a reçu. Le calcul de chaque ligne est celui du rapport Z (somme de `price` et des
`priceChanges.amount` de chaque `VatInput`).

| Rapport Z (champ `turnover`) | Rapport de ventes |
|---|---|
| `transactions` de l'étiquette N : nombre d'événements et somme des lignes | « Tickets » et « Total des ventes » de la période comptable, jamais calculés depuis les tables modifiables (`orders`, `order_lines`, `payments`) |
| Lignes de quantité négative | Montants négatifs, déjà compris dans les produits et dans le total |
| Tickets de remboursement (événements N dont la requête cite le ticket remboursé dans `fdmRef`) | Seuls comptés dans « dont remboursements » : somme de ces tickets, négative, déjà comprise dans le total. Une ligne négative d'un autre ticket (annulation de remise, correction dans la vente) n'est pas un remboursement : elle reste dans les produits et dans le total, jamais dans « dont remboursements » |
| Événements de formation (étiquette T), commandes de table (P), copies (C), caisse (F), factures (I), rapports (R) | Exclus : seule l'étiquette N est lue |
| `priceChanges` PUBLIC de portée LINE | Dans le montant de la ligne produit |
| `priceChanges` de portée EVENT | Ligne « Modifications de prix (tickets) » à part, comptée dans le total |
| Lignes de paiement ROUNDING (arrondi de caisse au 5 centimes) | **Hors total**, montrées à part : l'arrondi n'est pas dans `transactionTotal`, donc pas dans le chiffre d'affaires du Z |

Identité vérifiée par les tests : **Total des ventes = Σ des `transactionTotal` des tickets N =
chiffre d'affaires N du rapport Z** de la même période comptable. La ligne de concordance affiche
cette égalité ; si un jour elle ne tenait pas, elle l'afficherait (« écart de ... »), elle ne la
masquerait pas. Une date qui a eu deux rapports Z a deux lignes de concordance, une par période.

Le rapport Z lit le chiffre d'affaires N dans la requête du Z stockée à sa signature
(`z_reports.data_json`, `turnover.transactions`, étiquette N) ; c'est cette valeur, et non un
recalcul, qui est comparée.

## 3. Requêtes utilisées

La base de la caisse est lisible directement (réponse « SQLite lisible directement » de la
version 1.0.0). Le moteur exécute les trois lectures ci-dessous, en lecture seule.

Périodes comptables de la plage, avec le Z qui les a clôturées :

```sql
SELECT p.id, p.booking_date, p.closed_at IS NULL AS open,
       z.sequence_number, z.data_json
FROM booking_periods p
LEFT JOIN z_reports z ON z.fiscal_event_id = p.closing_event_id
WHERE p.booking_date >= :from AND p.booking_date <= :to
ORDER BY p.opened_at, p.id;
```

Événements archivés d'une période comptable (puis filtrés sur l'étiquette N) :

```sql
SELECT id, event_type, event_data_json, fdm_response_json
FROM fiscal_events
WHERE json_extract(event_data_json, '$.bookingPeriodId') = :period_id
  AND event_type = 'N'
ORDER BY id;
```

Contrôle indépendant du moteur, ticket par ticket (somme des lignes d'un ticket de vente) :

```sql
SELECT e.id,
       (SELECT SUM(json_extract(v.value, '$.price')
                   + COALESCE((SELECT SUM(json_extract(c.value, '$.amount'))
                               FROM json_each(v.value, '$.priceChanges') c), 0))
        FROM json_each(e.event_data_json, '$.transaction.transactionLines') l,
             json_each(l.value, '$.mainProduct.vats') v) AS ventes
FROM fiscal_events e
WHERE e.event_type = 'N'
  AND json_extract(e.event_data_json, '$.bookingPeriodId') = :period_id;
```

Le total des ventes d'une période est la somme de la colonne `ventes` ; il se compare au montant
de l'étiquette N dans `z_reports.data_json` (`$.turnover.transactions`). Le moteur remplace ce
dernier contrôle par la marque de concordance de chaque période.

## 4. Ce que le rapport ne fait pas

- Il n'exporte aucun fichier (ni CSV ni PDF) et n'est pas un rapport par utilisateur (le rapport
  utilisateurs X et Z fiscal existe pour cela).
- Il n'utilise aucune table modifiable pour ses chiffres : une commande, une ligne ou un paiement
  modifié ne change pas le rapport, seul un événement signé le ferait.
- Il n'est pas remis au client et ne peut pas être pris pour un ticket : il porte la mention
  « CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE » et ne porte aucune signature numérique
  (arrêté ministériel du 29 avril 2024, article 43 : pas de signature numérique sur une
  impression autre que le ticket TVA).

## 5. Tests

| Test | Ce qu'il démontre |
|---|---|
| `crates/pos-engine/tests/sales_report.rs`, `the_service_day_total_is_the_turnover_n_of_the_z` | La journée type des Rapports 1.3 (tickets T1 à T8) rejouée par les vraies commandes : le total des ventes est le chiffre d'affaires N publié dans le Z, la période est marquée « en cours » avant le Z et « identique » après |
| `reading_and_printing_the_report_signs_nothing` | Aucun événement ajouté, compteurs inchangés, aucune requête envoyée au module de contrôle |
| `a_training_sale_is_not_in_the_report`, `training_events_and_other_labels_are_left_out` | Formation, commandes de table, copies exclues |
| `refunds_are_negative_and_stay_inside_the_total` | Remboursements négatifs, dans le total |
| `a_negative_line_inside_a_sale_is_not_a_refund` | Une ligne négative hors ticket de remboursement n'entre pas dans « dont remboursements » |
| `price_changes_and_rounding_are_apart_and_the_total_is_the_sum_of_the_tickets` | Modifications de prix et arrondi à part |
| `hours_are_brussels_local_and_listed_from_the_day_end_hour` | Heures de Bruxelles (heure d'été comprise), ordre depuis l'heure de fin de journée |
| `a_gap_with_the_z_is_shown_and_two_zs_on_one_date_are_two_rows` | Écart affiché, deux Z le même jour = deux lignes |
| `the_range_is_limited_to_92_days`, `only_a_manager_may_read_or_print_it` | Plage et droits |
| `crates/pos-print/src/sales_report.rs` (tests) | Titre, deux mentions de pied, aucun mot des rapports fiscaux dans le titre |
| `ui/e2e/sales-report.spec.ts`, `SalesViewModelTest`, `SalesContentTest` | Les deux écrans : plages, tableaux, barres, concordance, impression |

## Captures d'écran

Les écrans de cette fonction sont les captures A169 (onglet « Ventes ») et A194 (période libre) de *CAPTURES_ECRAN_FR.pdf*.
