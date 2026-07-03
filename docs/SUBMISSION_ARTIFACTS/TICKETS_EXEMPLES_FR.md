# Tickets de test - exemples imprimés

Conformément au §IV §1209-1210 du Manuel de certification SCE 2.0,
le dossier doit contenir un exemple de chaque type de ticket
imprimé par le système.

Ces rendus sont **générés automatiquement depuis le code de
production** : la suite `crates/pos-print/tests/ticket_examples.rs`
construit chaque ticket, le passe dans `format_receipt` /
`format_report` (les mêmes fonctions que la caisse en service) puis
transcrit le flux ESC/POS en texte via `pos_print::render`.
Régénération : `scripts/generate_ticket_examples.sh`. La largeur
est de 48 caractères (police A 12×24, papier 80 mm, Epson
TM-m30III, code page CP858 pour les accents). `[QR …]` et `✂`
matérialisent le QR code et la coupe papier.

## Matrice de conformité du contenu (§VI du manuel)

Réponse au tableau "Contenu du ticket" du manuel de certification §VI.
Une ligne par élément obligatoire de la spec POS v2.2 §III.3, une
colonne par type de ticket émis par moyun-pos.

| Élément | N | N Refund | P (Pro Forma) | C (Copy) | T (Training) | Autres (cuisine) |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| I. Dénomination complète du prestataire | OK | OK | OK | OK | OK | - |
| II. Identification (nom, adresse, n° TVA) | OK | OK | OK | OK | OK | - |
| III. Dénomination commerciale | OK | OK | OK | OK | OK | - |
| IV. N° unité d'établissement (EST) | OK | OK | OK | OK | OK | - |
| V. N° fabrication POS (CPOS) + n° version logiciel | OK | OK | OK | OK | OK | - |
| VI. Identification terminal (terminalId) | OK | OK | OK | OK | OK | - |
| VII. Identification dispositif d'entrée (MAC / SN) | OK | OK | OK | OK | OK | - |
| VIII. Identification utilisateur (INSZ / employeeId) | OK | OK | OK | OK | OK | - |
| IX. Date et heure de délivrance | OK | OK | OK | OK | OK | OK |
| X. N° de ticket continu (série ininterrompue) | OK | OK | OK | OK | OK | - |
| XI.1. Lignes produits : qté, désignation, prix, code TVA | OK | OK (négatif) | OK | OK | OK | OK (désignation seule, pas de prix) |
| XI.2. Modifications de prix sur la ligne | N/A (MVP) | N/A | N/A | N/A | N/A | - |
| XI.3. GTIN (optionnel) | N/A | N/A | N/A | N/A | N/A | - |
| XII. Total à payer TVA comprise | OK | OK (négatif) | À PAYER (estimation) | OK | OK | - |
| XIII. Base imposable par taux de TVA (calculée par le FDM) | OK | OK | - (pas avant paiement) | OK | OK | - |
| XIV. Montant TVA par taux (calculé par le FDM) | OK | OK | - | OK | OK | - |
| XV. Modifications de prix hors lignes produits | N/A (MVP) | N/A | N/A | N/A | N/A | - |
| XVI.I. Pied de page configuré par SPF Finances | OK | OK | OK | OK | OK | - |
| XVI.II. fdmId | OK | OK | OK | OK | OK | - |
| XVI.III. fdmDateTime | OK | OK | OK | OK | OK | - |
| XVI.IV. eventLabel | OK (N) | OK (N) | OK (P) | OK (C) | OK (T) | - |
| XVI.V. eventCounter | OK | OK | OK | OK | OK | - |
| XVI.VI. totalCounter | OK | OK | OK | OK | OK | - |
| XVI.VII. shortSignature (interdit sur C et T) | OK | OK | OK (pour P : pas de short en v1.7) | INTERDIT (non imprimé) | INTERDIT (non imprimé) | - |
| XVII. Lignes de paiement (arrondi, tips, remboursements) | OK | OK (négatif) | - (pas encore payé) | - (copie sans paiement) | OK (mention FICTIF) | - |
| XVIII. QR Code URL du FDM | OK (25×25 Type 2) | OK | OK | OK | OK | - |

**Mentions spécifiques cochées :**

| Mention | N | N Refund | P | C | T | Autres |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| Mention "PRO FORMA - ne tient pas lieu de ticket TVA valable" | - | - | OK | - | - | - |
| Mention "REMBOURSEMENT" | - | OK | - | - | - | - |
| Mention "COPIE - pas un original" | - | - | - | OK | - | - |
| Mention "CECI N'EST PAS UN TICKET DE CAISSE DE TVA VALABLE" | - | - | OK (équivalent) | OK | OK | OK |
| Bandeau "TRAINING" début et fin | - | - | - | - | OK | - |
| Lien fdmRef vers le ticket original | - | OK | - | OK | - | - |

**Règles de langue :** les tickets sont imprimés en français pour le déploiement Tokyo Sushi Bar (langue de la clientèle locale). La spec belge demande FR/NL/DE selon le client ; pour TSB, FR uniquement. Le code source supporte le rendu multilingue (table `translations` + paramètre opérateur `users.language_preference`), à activer client par client.

---


## 1. Vente normale (M110 - Événement N)

Scénario : Saumon roll x8 (8,50 €, TVA B) + Coca 33cl (3,50 €, TVA A),
payé en espèces.

```
              TICKET DE CAISSE TVA

                Tokyo Sushi Bar
           Rue Grétry 42, 4000 Liège
               TVA: BE0499999960
                Etab: 2123456789

Date:                           2026-07-03 12:34
Ticket:                                      #15
Terminal:                                   POS1
------------------------------------------------
Saumon roll x8                            8.50 B
Coca 33cl                                 3.50 A
------------------------------------------------
TOTAL                                      12.00
------------------------------------------------
Espèces                                    12.00

TVA    Taux    Base HT    TVA
 A      21%      2.89      0.61
 B      12%      7.59      0.91
================================================
                FDM: CBX01000042
           2026-07-03T12:34:57+02:00
                    N 15/59
Sig: a3f9c2b1e8d4750616fb92c3a4e5d6f70123456789

[QR 25x25: https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug]

      www.systemedecaisseenregistreuse.be




✂-----------------------------------------------
```

## 2. Pré-ticket (M123 - Événement P)

Scénario : même commande qu'au M110, juste avant l'encaissement.
Mention PRO FORMA en tête, pas de shortSignature, mention
d'invalidité en bas.

```
                   PRO FORMA

                Tokyo Sushi Bar
           Rue Grétry 42, 4000 Liège
               TVA: BE0499999960
                Etab: 2123456789

Date:                           2026-07-03 12:34
Ticket:                                       #4
Terminal:                                   POS1
------------------------------------------------
Saumon roll x8                            8.50 B
Coca 33cl                                 3.50 A
------------------------------------------------
TOTAL                                      12.00
------------------------------------------------
================================================
                FDM: CBX01000042
           2026-07-03T12:34:57+02:00
                     P 4/61

[QR 25x25: https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug]

      www.systemedecaisseenregistreuse.be

 CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE




✂-----------------------------------------------
```

## 3. Remboursement complet (M111 - Événement N négatif)

Scénario : remboursement intégral du M110 ci-dessus (quantités et
montants négatifs, en-tête REMBOURSEMENT DU TICKET DE CAISSE TVA).

```
     REMBOURSEMENT DU TICKET DE CAISSE TVA

                Tokyo Sushi Bar
           Rue Grétry 42, 4000 Liège
               TVA: BE0499999960
                Etab: 2123456789

Date:                           2026-07-03 12:34
Ticket:                                      #16
Terminal:                                   POS1
------------------------------------------------
-1x Saumon roll x8                       -8.50 B
-1x Coca 33cl                            -3.50 A
------------------------------------------------
TOTAL                                     -12.00
------------------------------------------------
Espèces                                   -12.00

TVA    Taux    Base HT    TVA
 A      21%      2.89      0.61
 B      12%      7.59      0.91
================================================
                FDM: CBX01000042
           2026-07-03T12:34:57+02:00
                    N 16/60
Sig: a3f9c2b1e8d4750616fb92c3a4e5d6f70123456789

[QR 25x25: https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug]

      www.systemedecaisseenregistreuse.be

                 REMBOURSEMENT




✂-----------------------------------------------
```

## 4. Copie de ticket (M160 - Événement C)

Scénario : copie du ticket M110. Mention COPIE, pas de
shortSignature réimprimée, mention d'invalidité en bas.

```
                     COPIE

                Tokyo Sushi Bar
           Rue Grétry 42, 4000 Liège
               TVA: BE0499999960
                Etab: 2123456789

Date:                           2026-07-03 12:34
Ticket:                                       #2
Terminal:                                   POS1
------------------------------------------------
Saumon roll x8                            8.50 B
Coca 33cl                                 3.50 A
------------------------------------------------
TOTAL                                      12.00
------------------------------------------------
Espèces                                    12.00

TVA    Taux    Base HT    TVA
 A      21%      2.89      0.61
 B      12%      7.59      0.91
================================================
                FDM: CBX01000042
           2026-07-03T12:34:57+02:00
                     C 2/62

[QR 25x25: https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug]

      www.systemedecaisseenregistreuse.be

 CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE




✂-----------------------------------------------
```

## 5. Vente en formation (M171 - Événement T)

Scénario : même commande que M110 mais en mode formation. Bandeau
TRAINING (double largeur) en tête, mention d'invalidité en bas.

```
        TRAINING
================================================
                    TRAINING

                Tokyo Sushi Bar
           Rue Grétry 42, 4000 Liège
               TVA: BE0499999960
                Etab: 2123456789

Date:                           2026-07-03 12:34
Ticket:                                       #7
Terminal:                                   POS1
------------------------------------------------
Saumon roll x8                            8.50 B
Coca 33cl                                 3.50 A
------------------------------------------------
TOTAL                                      12.00
------------------------------------------------
Espèces                                    12.00

TVA    Taux    Base HT    TVA
 A      21%      2.89      0.61
 B      12%      7.59      0.91
================================================
                FDM: CBX01000042
           2026-07-03T12:34:57+02:00
                     T 7/63

[QR 25x25: https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug]

      www.systemedecaisseenregistreuse.be

 CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE




✂-----------------------------------------------
```

## 6. Rapport X (M180 - Événement R, intermédiaire)

Scénario : rapport X en cours de service (42 events N, 152,35 € de
chiffre, 3 ouvertures de tiroir).

```
       RAPPORT X

                Tokyo Sushi Bar
           Rue Grétry 42, 4000 Liège
               TVA: BE0499999960

Date:                                 2026-07-03
Debut:                          2026-07-03 11:02
Fin:                            2026-07-03 22:41
Terminal:                                   POS1
Employe:                             12345678901
Ticket:                                       #3
================================================
TRANSACTIONS
------------------------------------------------
Ventes (42)                              1523.50
Pro forma (12)                              0.00
------------------------------------------------
Total (54)                               1523.50

TVA
------------------------------------------------
Code   Taux     Base HT      TVA
 A      21%     317.78      66.72
 B      12%    1016.96     122.04

PAIEMENTS
------------------------------------------------
Espèces (18x)                             612.00
Carte (24x)                               911.50

Ouvertures caisse:                             3
================================================
TOTAL                                    1523.50
================================================
                FDM: CBX01000042
           2026-07-03T22:42:00+02:00
Sig: 7c2e91d4a5b3f60718c9e2d3a4b5c6d7e8f90a1b2c




✂-----------------------------------------------
```

## 7. Rapport Z (M181 - Événement R, clôture)

Scénario : clôture journalière (Z n°1) sur les mêmes totaux.

```
      RAPPORT Z #1

                Tokyo Sushi Bar
           Rue Grétry 42, 4000 Liège
               TVA: BE0499999960

Date:                                 2026-07-03
Debut:                          2026-07-03 11:02
Fin:                            2026-07-03 22:41
Terminal:                                   POS1
Employe:                             12345678901
Ticket:                                       #3
================================================
TRANSACTIONS
------------------------------------------------
Ventes (42)                              1523.50
Pro forma (12)                              0.00
------------------------------------------------
Total (54)                               1523.50

TVA
------------------------------------------------
Code   Taux     Base HT      TVA
 A      21%     317.78      66.72
 B      12%    1016.96     122.04

PAIEMENTS
------------------------------------------------
Espèces (18x)                             612.00
Carte (24x)                               911.50

Ouvertures caisse:                             3
================================================
TOTAL                                    1523.50
================================================
                FDM: CBX01000042
           2026-07-03T22:42:00+02:00
Sig: 7c2e91d4a5b3f60718c9e2d3a4b5c6d7e8f90a1b2c

                 FIN DE JOURNEE
             Compteurs remis a zero




✂-----------------------------------------------
```

## 8. Note sur la reproduction

Les rendus ci-dessus sont générés par le pipeline d'impression réel
(`crates/pos-print/src/receipt.rs` et `report.rs`) — ils ne peuvent
pas diverger de la sortie papier, contrairement à des transcriptions
manuelles. Toute modification du layout d'impression invalide et
régénère ces exemples via `scripts/generate_ticket_examples.sh`.

Lors du rendez-vous d'admission SPF Finances, le distributeur
apportera la machine test connectée à l'imprimante réelle pour
démontrer la sortie physique de chaque type de ticket. Les
exemplaires physiques de ces tickets seront archivés avec le
dossier final.
