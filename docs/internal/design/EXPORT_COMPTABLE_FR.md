# Export comptable : outil externe au binaire certifié

Statut : document de conception de l'outil séparé `pili-accounting`, hors du binaire certifié et hors des dossiers de certification (les dossiers le décrivent en quelques lignes, voir `LISTE_FONCTIONNALITES_FR.md`).
Décisions : V12-D5 à V12-D10 dans `docs/android/DECISIONS.md`.

## 1. Objet et statut

Le propriétaire veut remettre à son comptable, chaque mois, des fichiers qui se
rapprochent du rapport Z et qu'un logiciel comptable belge peut importer. L'outil
`pili-accounting` (crate `crates/pos-accounting`) le fait **en dehors de la caisse** :

- ce n'est pas une fonction du binaire certifié : aucun écran, aucune commande moteur,
  aucun point d'entrée dans l'application Linux ni dans l'application Android ;
- aucune crate livrée (`crates/pos-engine`, `src-tauri`, `android/rust/pos-ffi`) ne
  dépend de lui, et il ne dépend d'aucune crate de la caisse à l'exécution ; un test
  automatique (`crates/pos-accounting/tests/isolation.rs`) échoue si l'une de ces
  conditions change ;
- il ne modifie jamais la source : il n'écrit rien dans la base lue ;
- il s'exécute sur un ordinateur du propriétaire ou du comptable, jamais sur la caisse.

Raison du choix : un écran dans la caisse agrandirait le binaire certifié et demanderait
une mise à jour déclarée au SPF pour chaque ajustement de format demandé par un comptable.
Les données sources sont déjà l'archive signée que la caisse conserve et exporte.

## 2. Données lues (archive signée uniquement)

| Source dans la base | Contenu retenu |
| --- | --- |
| `fiscal_events` avec une réponse du FDM, étiquettes N | tickets de vente et de remboursement : lignes, paiements, centre de coût, calcul de TVA du FDM |
| `fiscal_events`, étiquettes I | factures (forme M150) : numéro, TVA et nom du client, tickets référencés |
| `fiscal_events`, étiquettes F | mouvements d'espèces mis en caisse ou retirés |
| `z_reports` et `booking_periods` | rapport Z qui clôture chaque période comptable |

Ne sont jamais lus : les commandes et leurs lignes (`orders`, `order_lines`), la table
`payments`, la carte, les utilisateurs. Ne sont jamais comptés : les ventes de formation
(étiquette T), les copies (C), les tickets provisoires (P), les présences du personnel
(S), les rapports X, les commandes ouvertes non signées. Les montants sont ceux de la
requête signée (lignes, paiements) et du calcul de TVA du FDM (`vatCalc`) ; rien n'est
recalculé à partir de la carte.

Canal de vente, lu dans le centre de coût signé : `TABLE` = sur place, aucun centre de
coût = à emporter ou vente directe, tout autre type = en ligne. Un remboursement sans
centre de coût propre prend le canal du ticket auquel il se rapporte.

Remboursement : une vente qui se rapporte à un ticket antérieur (`fdmRef` dans la requête)
ou dont le total est négatif (même règle que l'export statistique nocturne).

## 3. Entrée de l'outil

`--db` désigne soit un fichier `pos.db`, soit un dossier d'export de la caisse (celui que
l'export écrit sur la clé ou sur le stockage choisi) :

- le fichier est ouvert en lecture seule et immuable (aucun journal, aucun fichier WAL
  n'est créé à côté) ;
- si le dossier contient `MANIFEST.sha256`, chaque fichier listé est vérifié ; un écart
  arrête l'outil (`manifest_mismatch`) ;
- un fichier `pos.db-wal` non vide à côté signale une copie de la base en service :
  l'outil refuse (`db_unreadable`), car il ignorerait ce que le WAL contient encore ;
- une base sans la migration 4 (archive en ajout seul) est refusée (`not_a_pili_db`) ;
- l'empreinte SHA-256 du fichier lu est inscrite dans `LISEZMOI.txt`.

La sauvegarde chiffrée nocturne du propriétaire peut aussi servir, une fois déchiffrée
par lui.

## 4. Période

`--month AAAA-MM` ou `--from AAAA-MM-JJ --to AAAA-MM-JJ` (dates comptables, bornes
incluses). Seules les périodes clôturées par un vrai rapport Z sont exportées. Une période
encore ouverte dans l'intervalle est laissée de côté et nommée dans `LISEZMOI.txt`. Si
aucune période n'est clôturée, l'outil s'arrête (`period_open`, ou `empty_range` si
l'intervalle ne contient aucune période).

## 5. Rapprochement avec le rapport Z

Pour chaque période, avant toute écriture, les totaux des tickets sont comparés au Z signé :

1. nombre de tickets N (remboursements compris) ;
2. chiffre d'affaires (somme des lignes après modifications de prix) ;
3. TVA de chaque code : base, TVA, total ;
4. paiements de chaque type de paiement et de chaque type de montant (paiement, arrondi,
   pourboire).

Au moindre écart, l'outil s'arrête avec le code de sortie 2, imprime une ligne
`RECONCILIATION_FAILED` qui nomme la date, le numéro de Z et le champ, et n'écrit aucun
fichier. Un fichier modifié après coup est donc détecté.

## 6. Fichiers produits

Dossier `export-comptable-<du>-<au>/` créé dans `--out` (jamais d'écrasement : si le
dossier existe, `out_exists`). Il est d'abord écrit sous un nom provisoire puis renommé.

Format commun : UTF-8 avec BOM, séparateur point-virgule, fins de ligne CRLF, montants avec
la virgule décimale (option `--decimal point`), et une colonne `_centimes` entière à côté de
chaque montant. Un texte libre venu de la caisse qui commence par `=`, `+`, `-` ou `@` est
précédé d'une apostrophe (protection contre les formules des tableurs).

| Fichier | Une ligne par | Colonnes principales |
| --- | --- | --- |
| `jours.csv` | période (Z) | date, rapport_z, tickets, remboursements, total |
| `tva.csv` | période, canal, code TVA | taux, base, tva, total |
| `paiements.csv` | période, moyen | paiements, arrondis, pourboires, total |
| `remboursements.csv` | ticket de remboursement | ticket, date_heure, canal, moyen, total, ticket_origine |
| `factures.csv` | facture (étiquette I) | numero_facture, tva_client, client, base et tva par code, total, tickets |
| `bons_cadeaux.csv` | vente ou utilisation de bon | type, ticket, montant |
| `mouvements_caisse.csv` | mouvement d'espèces | sens, moyen, libelle, montant |
| `rapports_z.csv` | rapport Z | ticket_pos, date_heure_fdm, tickets, total, total et tva par code |
| `journal.csv` | ligne d'écriture | voir le paragraphe 7 |
| `LISEZMOI.txt` | | source, empreinte, périodes incluses et laissées de côté, conventions |
| `MANIFEST.sha256` | fichier | empreinte SHA-256 de chaque autre fichier (format sha256sum) |

Moyens de paiement : espèces, carte, chèques-repas, bon cadeau (type VOUCHER_STORE),
application, en ligne, autre. Les factures sont informatives : leurs tickets sont déjà
dans les ventes de leur propre jour.

Bons cadeaux : une vente de bon est une ligne signée de code TVA X dans le rayon `GV` ;
une utilisation est un paiement de type VOUCHER_STORE. L'outil les liste ; le traitement
comptable d'un bon non utilisé à l'échéance appartient au comptable.

Données personnelles : aucun numéro national, aucun nom d'employé, aucune adresse
e-mail. Le nom et le numéro de TVA d'un client n'apparaissent que dans `factures.csv`.
Le libellé d'un mouvement d'espèces est le texte saisi à la caisse.

## 7. Journal comptable

Une écriture équilibrée par période comptable (numéro de pièce = numéro du Z). Débit :
encaissements par moyen (paiements, arrondis, pourboires) et ventes non encaissées.
Crédit : ventes par code TVA, TVA collectée par taux, bons cadeaux vendus (dette, pas
chiffre d'affaires), arrondis de caisse, pourboires à reverser. Un bon cadeau utilisé est
un débit du compte des bons en circulation. Un remboursement porte le signe contraire.
La base vient du calcul du FDM ; la TVA est le total moins la base, donc l'écriture est
équilibrée dès que la journée se rapproche du Z. Un déséquilibre interne arrête l'outil
(`journal_unbalanced`).

Comptes par défaut (plan comptable minimum normalisé), modifiables avec `--accounts` :

| Poste | Compte |
| --- | --- |
| Ventes code A, B, C, D, X (hors bons) | 700000, 700100, 700200, 700300, 700400 |
| TVA collectée 21 %, 12 %, 6 % | 451021, 451012, 451006 |
| Espèces, carte, chèques-repas, application, en ligne, autre | 570000, 580000, 580100, 580200, 580300, 580900 |
| Bons cadeaux en circulation | 489000 |
| Arrondis (charge, produit) | 657000, 757000 |
| Pourboires à reverser | 489100 |
| Ventes non encaissées | 400000 |
| Code du journal | VTE |

Le fichier de comptes est un JSON (clés `journal`, `sales`, `vat`, `payments`,
`gift_vouchers`, `rounding_cost`, `rounding_income`, `tips`, `unpaid`) ; une clé inconnue
ou un numéro de compte invalide est refusé (`accounts_invalid`).

## 8. Profils

| Profil | Colonnes | Dates | Décimales |
| --- | --- | --- | --- |
| `generic` (référence) | date, journal, document, compte, libelle, debit, credit, code_tva | AAAA-MM-JJ | point |
| `winbooks` | DBKCODE, DBKTYPE (PRI), DOCNUMBER, DOCORDER, ACCOUNTGL, BOOKYEAR, PERIOD, DATE, COMMENT, AMOUNTEUR (débit positif, crédit négatif), VATCODE | AAAA/MM/JJ | point |
| `yuki` | Date, Journal, Reference, Account, Description, Debit, Credit, VATCode | JJ-MM-AAAA | virgule |

Le profil générique est la référence : un journal équilibré de colonnes simples est accepté
par les trois logiciels courants (Winbooks, Yuki, Exact Online) par leur import avec
association de colonnes. Les deux profils natifs suivent les descriptions publiques
d'import et doivent être confirmés par un import d'essai chez le comptable avant usage.
Exact Online n'a pas de disposition CSV de journal stable et publique : utiliser le profil
générique avec l'association de colonnes.

## 9. Utilisation

```
pili-accounting --db <pos.db ou dossier d'export> --month 2026-10 --out <dossier>
pili-accounting --db <...> --from 2026-10-01 --to 2026-10-15 --profile winbooks --accounts comptes.json
```

Codes de sortie : 0 terminé, 1 erreur (`db_unreadable`, `not_a_pili_db`, `manifest_mismatch`,
`period_invalid`, `period_open`, `empty_range`, `accounts_invalid`, `out_exists`,
`event_unreadable`, `journal_unbalanced`, `io_error`), 2 écart avec le Z
(`RECONCILIATION_FAILED`).

## 10. Tests

- rapprochement d'un jour réel construit par le moteur : le scénario Reports 1.3
  (T1 à T8 et le Z M184) ; chaque total du jour égale le Z signé au centime ;
- une journée mêlant remboursement, chèques-repas avec arrondi, vente à emporter, vente
  de formation (absente), ticket provisoire (absent), commande ouverte (absente),
  entrée et sortie d'espèces, période sans Z (laissée de côté et nommée) ;
- fichiers de référence octet pour octet (BOM, CRLF), équilibre du journal pour chaque
  jour, profils Winbooks et Yuki, fichier de comptes ;
- détection d'altération : un montant, un prix de ligne ou un événement supprimé dans une
  copie de la base fait échouer le rapprochement, sans aucun fichier écrit ;
- preuve de lecture seule : empreinte et date de modification de la source inchangées,
  aucun fichier créé à côté, fichier en lecture seule accepté ;
- dossier d'export avec `MANIFEST.sha256` (écart, fichier manquant), base ancienne,
  fichier qui n'est pas une base, copie de base en service refusée ;
- ligne de bon cadeau (rayon `GV`) et paiement VOUCHER_STORE synthétiques comptés dans
  `bons_cadeaux.csv` ; facture de la fixture M150 listée dans `factures.csv` ;
- isolation : aucune crate livrée ne nomme `pos-accounting`.

## 11. Points à confirmer par le propriétaire et son comptable

- disposition d'import réelle de chaque logiciel (un import d'essai par logiciel) ;
- numéros de compte du plan comptable de l'entreprise (fichier `--accounts`) ;
- traitement comptable des bons cadeaux non utilisés à l'échéance.
