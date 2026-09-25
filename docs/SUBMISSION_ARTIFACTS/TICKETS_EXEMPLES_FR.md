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
| I. Dénomination complète du prestataire | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| II. Identification (nom, adresse, n° TVA) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| III. Dénomination commerciale | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| IV. N° unité d'établissement (EST) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| V. N° fabrication POS (posId) + n° version logiciel | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| VI. Identification terminal (terminalId) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| VII. Identification dispositif d'entrée (adresse MAC, `deviceId`) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| VIII. Identification utilisateur (nom de l'opérateur ; l'INSZ n'est pas imprimé) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| IX. Date et heure de délivrance | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ |
| X. N° de ticket continu (série ininterrompue) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| XI.1. Lignes produits : qté, désignation, prix, code TVA | ☑ | ☑ (négatif) | ☑ | ☑ | ☑ | ☑ (désignation seule, pas de prix) |
| XI.2. Modifications de prix sur la ligne | N/A (MVP) | N/A | N/A | N/A | N/A | - |
| XI.3. GTIN (optionnel) | N/A | N/A | N/A | N/A | N/A | - |
| XII. Total à payer TVA comprise | ☑ | ☑ (négatif) | À PAYER (estimation) | ☑ | ☑ | - |
| XIII. Base imposable par taux de TVA (calculée par le FDM) | ☑ | ☑ | - (pas avant paiement) | ☑ | ☑ | - |
| XIV. Montant TVA par taux (calculé par le FDM) | ☑ | ☑ | - | ☑ | ☑ | - |
| XV. Modifications de prix hors lignes produits | N/A (MVP) | N/A | N/A | N/A | N/A | - |
| XVI.I. Pied de page configuré par SPF Finances | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| XVI.II. fdmId | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| XVI.III. fdmDateTime | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| XVI.IV. eventLabel | ☑ (N) | ☑ (N) | ☑ (P) | ☑ (C) | ☑ (T) | - |
| XVI.V. eventCounter | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| XVI.VI. totalCounter | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| XVI.VII. shortSignature (renvoyée par le FDM pour les events N uniquement) | ☑ | ☑ | - (non renvoyée pour P) | - (non réimprimée) | - (non renvoyée pour T) | - |
| XVII. Lignes de paiement (arrondi, tips, remboursements) | ☑ | ☑ (négatif) | - (pas encore payé) | ☑ (reprises de l'original) | ☑ | - |
| XVIII. QR Code URL du FDM (renvoyée par le FDM pour les events N uniquement) | ☑ (Model 2, 29×29) | ☑ | - (non renvoyée pour P) | - (non renvoyée pour C) | - (non renvoyée pour T) | - |

**Mentions spécifiques cochées :**

| Mention | N | N Refund | P | C | T | Autres |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| Mention "PRO FORMA - ne tient pas lieu de ticket TVA valable" | - | - | ☑ | - | - | - |
| Mention "REMBOURSEMENT" | - | ☑ | - | - | - | - |
| Mention "COPIE - pas un original" | - | - | - | ☑ | - | - |
| Mention "CECI N'EST PAS UN TICKET DE CAISSE DE TVA VALABLE" | - | - | ☑ (équivalent) | ☑ | ☑ | ☑ |
| Bandeau "TRAINING" début et fin | - | - | - | - | ☑ | - |

Le remboursement et la copie référencent le ticket original dans l'événement envoyé au FDM (champ `fdmRef` : fdmId, fdmDateTime, eventLabel, compteurs de l'original) ; cette référence n'est pas imprimée. La copie réimprime le QR code de l'événement N original (spec POS v2.2 §6.5).

**Règles de langue :** les mentions fiscales sont imprimées dans la langue officielle de la région linguistique de l'établissement (FR/NL/DE, paramètre `pos.json::language`, cf. *GARANTIE_MAJ_LANGUES_FR.md* §3.2). Les exemples ci-dessous sont en français. Le code source supporte le rendu multilingue (table `translations` + paramètre opérateur `users.language_preference`), à activer client par client.

---


## 1. Vente normale (M110 - Événement N)

Scénario (produits du manuel, M110) : 2 × Dry Martini (12,00 €, TVA A)
+ 1 × Burger du chef (28,00 €, TVA B), payé en espèces.

```
              TICKET DE CAISSE TVA

               Restaurant Exemple
         Rue de l'Exemple 1, 4000 Liège
               TVA: BE0499999960
                Etab: 2123456791

Date:                           2026-07-03 12:34
Ticket:                                      #15
POS:                              CFOD0061234567
Logiciel:                        moyun-pos/1.0.0
Terminal:                                   POS1
Appareil:                      00:1b:44:11:3a:b7
Utilisateur:                        Marie Dupont
------------------------------------------------
2x Dry Martini                           24.00 A
Burger du chef                           28.00 B
------------------------------------------------
TOTAL                                      52.00
------------------------------------------------
Espèces                                    52.00

TVA    Taux    Base HT    TVA
 A      21%     19.83      4.17
 B      12%     25.00      3.00
================================================
                FDM: CBX01000042
           2026-07-03T12:34:57+02:00
                    N 15/59
 Sig: a3f9c2b1e8d4750616fb92c3a4e5d6f701234567

[QR: https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug]

      www.systemedecaisseenregistreuse.be




✂-----------------------------------------------
```

## 2. Pré-ticket (M123 - Événement P)

Scénario : même commande qu'au M110, juste avant l'encaissement.
Mention PRO FORMA en tête, pas de shortSignature, mention
d'invalidité en bas.

```
                   PRO FORMA

               Restaurant Exemple
         Rue de l'Exemple 1, 4000 Liège
               TVA: BE0499999960
                Etab: 2123456791

Date:                           2026-07-03 12:34
Ticket:                                       #4
POS:                              CFOD0061234567
Logiciel:                        moyun-pos/1.0.0
Terminal:                                   POS1
Appareil:                      00:1b:44:11:3a:b7
Utilisateur:                        Marie Dupont
------------------------------------------------
2x Dry Martini                           24.00 A
Burger du chef                           28.00 B
------------------------------------------------
TOTAL                                      52.00
------------------------------------------------
================================================
                FDM: CBX01000042
           2026-07-03T12:34:57+02:00
                     P 4/61
      www.systemedecaisseenregistreuse.be

 CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE




✂-----------------------------------------------
```

## 3. Remboursement complet (M111 - Événement N négatif)

Scénario : remboursement intégral du M110 ci-dessus (quantités et
montants négatifs, en-tête REMBOURSEMENT DU TICKET DE CAISSE TVA).

```
     REMBOURSEMENT DU TICKET DE CAISSE TVA

               Restaurant Exemple
         Rue de l'Exemple 1, 4000 Liège
               TVA: BE0499999960
                Etab: 2123456791

Date:                           2026-07-03 12:34
Ticket:                                      #16
POS:                              CFOD0061234567
Logiciel:                        moyun-pos/1.0.0
Terminal:                                   POS1
Appareil:                      00:1b:44:11:3a:b7
Utilisateur:                        Marie Dupont
------------------------------------------------
-2x Dry Martini                         -24.00 A
-1x Burger du chef                      -28.00 B
------------------------------------------------
TOTAL                                     -52.00
------------------------------------------------
Espèces                                   -52.00

TVA    Taux    Base HT    TVA
 A      21%     19.83      4.17
 B      12%     25.00      3.00
================================================
                FDM: CBX01000042
           2026-07-03T12:34:57+02:00
                    N 16/60
 Sig: a3f9c2b1e8d4750616fb92c3a4e5d6f701234567

[QR: https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug]

      www.systemedecaisseenregistreuse.be

                 REMBOURSEMENT




✂-----------------------------------------------
```

## 4. Copie de ticket (M160 - Événement C)

Scénario : copie du ticket M110. Mention COPIE, pas de
shortSignature réimprimée, mention d'invalidité en bas.

```
                     COPIE

               Restaurant Exemple
         Rue de l'Exemple 1, 4000 Liège
               TVA: BE0499999960
                Etab: 2123456791

Date:                           2026-07-03 12:34
Ticket:                                       #2
POS:                              CFOD0061234567
Logiciel:                        moyun-pos/1.0.0
Terminal:                                   POS1
Appareil:                      00:1b:44:11:3a:b7
Utilisateur:                        Marie Dupont
------------------------------------------------
2x Dry Martini                           24.00 A
Burger du chef                           28.00 B
------------------------------------------------
TOTAL                                      52.00
------------------------------------------------
Espèces                                    52.00

TVA    Taux    Base HT    TVA
 A      21%     19.83      4.17
 B      12%     25.00      3.00
================================================
                FDM: CBX01000042
           2026-07-03T12:34:57+02:00
                     C 2/62
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

               Restaurant Exemple
         Rue de l'Exemple 1, 4000 Liège
               TVA: BE0499999960
                Etab: 2123456791

Date:                           2026-07-03 12:34
Ticket:                                       #7
POS:                              CFOD0061234567
Logiciel:                        moyun-pos/1.0.0
Terminal:                                   POS1
Appareil:                      00:1b:44:11:3a:b7
Utilisateur:                        Marie Dupont
------------------------------------------------
2x Dry Martini                           24.00 A
Burger du chef                           28.00 B
------------------------------------------------
TOTAL                                      52.00
------------------------------------------------
Espèces                                    52.00

TVA    Taux    Base HT    TVA
 A      21%     19.83      4.17
 B      12%     25.00      3.00
================================================
                FDM: CBX01000042
           2026-07-03T12:34:57+02:00
                     T 7/63
      www.systemedecaisseenregistreuse.be

 CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE




✂-----------------------------------------------
```

## 6. Rapport X (M180 - Événement R, intermédiaire)

Scénario : rapport X en cours de service (42 events N, 152,35 € de
chiffre, 3 ouvertures de tiroir).

```
       RAPPORT X

               Restaurant Exemple
         Rue de l'Exemple 1, 4000 Liège
               TVA: BE0499999960

Date:                                 2026-07-03
Debut:                          2026-07-03 11:02
Fin:                            2026-07-03 22:41
Terminal:                                   POS1
Employe:                             85073003328
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




✂-----------------------------------------------
```

## 7. Rapport Z (M181 - Événement R, clôture)

Scénario : clôture journalière (Z n°1) sur les mêmes totaux.

```
      RAPPORT Z #1

               Restaurant Exemple
         Rue de l'Exemple 1, 4000 Liège
               TVA: BE0499999960

Date:                                 2026-07-03
Debut:                          2026-07-03 11:02
Fin:                            2026-07-03 22:41
Terminal:                                   POS1
Employe:                             85073003328
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
