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
colonne par type de ticket émis par pili-pos.

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
| XI.2. Modifications de prix sur la ligne | ☑ (commandes en ligne : remise imprimée sous sa ligne) | ☑ (remise reprise en négatif) | N/A | ☑ (copie d'une vente en ligne) | N/A | - |
| XI.3. GTIN (optionnel) | N/A | N/A | N/A | N/A | N/A | - |
| XII. Total à payer TVA comprise | ☑ | ☑ (négatif) | ☑ | ☑ | ☑ | - |
| XIII. Base imposable par taux de TVA (calculée par le FDM) | ☑ | ☑ | - (jamais sur une addition) | ☑ | ☑ | - |
| XIV. Montant TVA par taux (calculé par le FDM) | ☑ | ☑ | - | ☑ | ☑ | - |
| XV. Modifications de prix hors lignes produits | N/A (les remises d'une commande en ligne sont réparties sur les lignes et imprimées sous chacune, voir XI.2) | N/A | N/A | N/A | N/A | - |
| XVI.I. Pied de page configuré par SPF Finances | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| XVI.II. fdmId | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| XVI.III. fdmDateTime | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| XVI.IV. eventLabel | ☑ (N) | ☑ (N) | ☑ (P) | ☑ (C) | ☑ (T) | - |
| XVI.V. eventCounter | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| XVI.VI. totalCounter | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| XVI.VII. shortSignature (renvoyée par le FDM pour les events N uniquement) | ☑ | ☑ | - (non renvoyée pour P) | ☑ (celle de l'original, sous « Informations FDM d'origine ») | - (non renvoyée pour T) | - |
| XVII. Lignes de paiement (arrondi, tips, remboursements) | ☑ | ☑ (négatif) | - (pas encore payé) | ☑ (reprises de l'original) | ☑ | - |
| XVIII. QR Code URL du FDM (renvoyée par le FDM pour les events N uniquement) | ☑ (Model 2, 29×29) | ☑ | - (non renvoyée pour P) | ☑ (QR de l'original, spec POS v2.2 §6.5) | - (non renvoyée pour T) | - |

**Mentions spécifiques cochées :**

| Mention | N | N Refund | P | C | T | Autres |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| Mention "PRO FORMA" et "Addition Provisoire" | - | - | ☑ | - | - | - |
| Mention "REMBOURSEMENT" (sous le titre, une fois) | - | ☑ | - | - | - | - |
| Mention "COPIE" (sous le titre) et date de la copie | - | - | - | ☑ | - | - |
| Bloc "Informations FDM d'origine" (fdmRef de l'original) | - | ☑ | - | ☑ | - | - |
| Mention "CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE" | - | - | ☑ | - (copie d'un ticket TVA, spec §1.8 ; ☑ pour la copie d'un autre événement) | ☑ | ☑ (bon bar / cuisine) |
| Bandeau "TRAINING" début et fin | - | - | - | - | ☑ | - |

Le remboursement et la copie référencent le ticket original dans l'événement envoyé au FDM (champ `fdmRef` : fdmId, fdmDateTime, eventLabel, compteurs de l'original) et l'impriment sous « Informations FDM d'origine », comme les tickets d'exemple du SPF du 24/09/2026. La copie reproduit le ticket original (tous les champs, données de contrôle comprises) et réimprime le QR code de l'événement N original (spec POS v2.2 §1.8, §6.5). La mise en page suit les tickets d'exemple du SPF ; `crates/pos-print/tests/official_examples.rs` imprime les données de chacun des six exemples et vérifie chaque mention, dans l'ordre de l'exemple.

**Règles de langue :** les mentions fiscales sont imprimées dans la langue officielle de la région linguistique de l'établissement (FR/NL/DE, paramètre `pos.json::language`, cf. *GARANTIE_MAJ_LANGUES_FR.md* §3.2). Les exemples de tickets (sections 1 à 5) sont présentés en français ; le rapport Z (section 7) est aussi présenté en néerlandais et en allemand pour illustrer la règle, les rapports X/Z, les rapports utilisateur (M182/M183/M184) et les noms de moyen de paiement imprimés (Especes/Contant/Bar, Carte/Kaart/Karte) suivant tous la même langue de ticket que les reçus (`crates/pos-print/src/labels.rs`). La langue des tickets et celle de l'interface sont toutes deux fixées par installation, par le paramètre `pos.json::language` (`fr`, `nl`, `de`, `en` ou `zh` pour l'écran ; les tickets suivent cette clé restreinte à FR/NL/DE, toute autre valeur imprime en FR) : il n'y a pas de langue par utilisateur. Les désignations de produits sont rendues dans cette langue à partir de la table `translations`.

---


## 1. Vente normale (M110 - Événement N)

Scénario (produits du manuel, M110) : 2 × Dry Martini (12,00 €, TVA A)
+ 1 × Burger du chef (28,00 €, TVA B), payé en espèces.

```
            TICKET DE CAISSE T.V.A.
Restaurant Exemple
Unite d'etablissement 2123456791
03.07.2026                                 12:34
Utilisateur: Marie Dupont
Terminal: POS1
Ticketnr.: 000015
POS Serial: CFOD0061234567
Input device MAC: 00:1b:44:11:3a:b7
Logiciel de caisse v.: pili-pos/1.0.0
................................................
2 Dry Martini                           24,00  A
1 Burger du chef                        28,00  B
TOTAL :                                   €52,00
................................................
Mode de paiement :
Espèces                                    52,00
% TVA   CODE          HT       TVA      Brut
21,00%  A          19,83      4,17     24,00
12,00%  B          25,00      3,00     28,00
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: FDM02000425;
fdmDateTime: 2026-07-03T12:34:57+02:00;
Event: N; eventCounter: 15;
totalCounter: 59; shortSignature:
a3f9c2b1e8d4750616fb92c3a4e5d6f701234567;

[QR: https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug]


Denomination legale : Societe Exemple SRL
Rue de l'Exemple 1, 4000 Liège
TVA: BE0499999960
      www.systemedecaisseenregistreuse.be




✂-----------------------------------------------
```

## 2. Addition, pré-ticket (M123 - Événement P)

Scénario : même commande qu'au M110, juste avant l'encaissement.
Mention PRO FORMA en tête ; ni QR code ni shortSignature - le FDM ne
les renvoie que pour une vente (N, voir `signPreBill_Response.json`
de l'UC M123, qui n'a ni `verificationUrl` ni `shortSignature`) ;
mention d'invalidité en bas.

```
                   PRO FORMA
              ADDITION PROVISOIRE
Restaurant Exemple
Unite d'etablissement 2123456791
03.07.2026                                 12:34
Utilisateur: Marie Dupont
Terminal: POS1
Ticketnr.: 000004
POS Serial: CFOD0061234567
Input device MAC: 00:1b:44:11:3a:b7
Logiciel de caisse v.: pili-pos/1.0.0
................................................
2 Dry Martini                           24,00  A
1 Burger du chef                        28,00  B
TOTAL :                                   €52,00
................................................
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: FDM02000425;
fdmDateTime: 2026-07-03T12:34:57+02:00;
Event: P; eventCounter: 4;
totalCounter: 61;

Denomination legale : Societe Exemple SRL
Rue de l'Exemple 1, 4000 Liège
TVA: BE0499999960
      www.systemedecaisseenregistreuse.be

   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

## 3. Remboursement complet (M111 - Événement N négatif)

Scénario : remboursement intégral du M110 ci-dessus (quantités et
montants négatifs, titre TICKET DE CAISSE TVA et mention
REMBOURSEMENT, données de contrôle de la vente d'origine sous
« Informations FDM d'origine »).

```
            TICKET DE CAISSE T.V.A.
                 REMBOURSEMENT
Restaurant Exemple
Unite d'etablissement 2123456791
03.07.2026                                 12:34
Utilisateur: Marie Dupont
Terminal: POS1
Ticketnr.: 000016
POS Serial: CFOD0061234567
Input device MAC: 00:1b:44:11:3a:b7
Logiciel de caisse v.: pili-pos/1.0.0
................................................
-2 Dry Martini                         -24,00  A
-1 Burger du chef                      -28,00  B
TOTAL :                                  €-52,00
................................................
Mode de paiement :
Espèces                                   -52,00
% TVA   CODE          HT       TVA      Brut
21,00%  A         -19,83     -4,17    -24,00
12,00%  B         -25,00     -3,00    -28,00
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: FDM02000425;
fdmDateTime: 2026-07-03T12:34:57+02:00;
Event: N; eventCounter: 16;
totalCounter: 60; shortSignature:
a3f9c2b1e8d4750616fb92c3a4e5d6f701234567;

[QR: https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug]

Informations FDM d'origine :
fdmId: FDM02000425;
fdmDateTime: 2026-07-03T12:34:57+02:00;
Event: N; eventCounter: 15;
totalCounter: 59; shortSignature:
a3f9c2b1e8d4750616fb92c3a4e5d6f701234567;

Denomination legale : Societe Exemple SRL
Rue de l'Exemple 1, 4000 Liège
TVA: BE0499999960
      www.systemedecaisseenregistreuse.be




✂-----------------------------------------------
```

## 4. Copie de ticket (M160 - Événement C)

Scénario : copie du ticket M110. Mention COPIE, pas de
shortSignature réimprimée, mention d'invalidité en bas.

```
            TICKET DE CAISSE T.V.A.
                     COPIE
Restaurant Exemple
Unite d'etablissement 2123456791
03.07.2026                                 12:34
Utilisateur: Marie Dupont
Terminal: POS1
Ticketnr.: 000015
POS Serial: CFOD0061234567
Input device MAC: 00:1b:44:11:3a:b7
Logiciel de caisse v.: pili-pos/1.0.0
................................................
2 Dry Martini                           24,00  A
1 Burger du chef                        28,00  B
TOTAL :                                   €52,00
................................................
Mode de paiement :
Espèces                                    52,00
% TVA   CODE          HT       TVA      Brut
21,00%  A          19,83      4,17     24,00
12,00%  B          25,00      3,00     28,00
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: FDM02000425;
fdmDateTime: 2026-07-03T12:34:57+02:00;
Event: C; eventCounter: 2;
totalCounter: 62;

[QR: https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug]

Informations FDM d'origine :
fdmId: FDM02000425;
fdmDateTime: 2026-07-03T12:34:57+02:00;
Event: N; eventCounter: 15;
totalCounter: 59; shortSignature:
a3f9c2b1e8d4750616fb92c3a4e5d6f701234567;
Copie etablie le 03.07.2026 12:41

Denomination legale : Societe Exemple SRL
Rue de l'Exemple 1, 4000 Liège
TVA: BE0499999960
      www.systemedecaisseenregistreuse.be




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
Unite d'etablissement 2123456791
03.07.2026                                 12:34
Utilisateur: Marie Dupont
Terminal: POS1
Ticketnr.: 000007
POS Serial: CFOD0061234567
Input device MAC: 00:1b:44:11:3a:b7
Logiciel de caisse v.: pili-pos/1.0.0
................................................
2 Dry Martini                           24,00  A
1 Burger du chef                        28,00  B
TOTAL :                                   €52,00
................................................
Mode de paiement :
Espèces                                    52,00
% TVA   CODE          HT       TVA      Brut
21,00%  A          19,83      4,17     24,00
12,00%  B          25,00      3,00     28,00
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: FDM02000425;
fdmDateTime: 2026-07-03T12:34:57+02:00;
Event: T; eventCounter: 7;
totalCounter: 63;

Denomination legale : Societe Exemple SRL
Rue de l'Exemple 1, 4000 Liège
TVA: BE0499999960
      www.systemedecaisseenregistreuse.be

   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

## 6. Rapport X (M180 - Événement R, intermédiaire)

Scénario : rapport X en cours de service (42 events N, 152,35 € de
chiffre, 3 ouvertures de tiroir).

```
              X CHIFFRE D'AFFAIRES
               Restaurant Exemple
         Rue de l'Exemple 1, 4000 Liège
               TVA: BE0499999960
        Unite d'etablissement 2123456791
------------------------------------------------
RAPPORT X                      03/07/26 23:05:41
Ticket:                                   000003
Employe:                             85073003328
------------------------------------------------
              Periode d'ouverture
Date:                                 2026-07-03
BookingPeriodId:
3f6c2a4e-8b1d-4c7a-9e25-7d0b6a1f4c83
Debut                           2026-07-03 11:02
Fin                             2026-07-03 22:41
POSID                             CFOD0061234567
TerminalId                                  POS1
FDMID                                FDM02000425
Premier                      41 2026-07-03 11:02
Dernier                      63 2026-07-03 22:48
------------------------------------------------
Event N Ventes 42                        1523,50
Event P Pro forma 12                        0,00
------------------------------------------------
                   Apercu TVA
Taux                HT          TVA          TTC
21%             317,78        66,72       384,50
12%            1016,96       122,04      1139,00
6%                   -            -            -
0%                   -            -            -
Total          1334,74       188,76      1523,50
------------------------------------------------
       Departements du chiffre d'affaires
Boissons                                  384,50
Plats                                    1139,00
------------------------------------------------
                  Corrections
Corrections                               -12,00
------------------------------------------------
                 Remboursements
Remboursements 1                          -52,00
------------------------------------------------
                 Tiroir-caisse
Tiroir-caisse 3
------------------------------------------------
                   Formation
Tickets de formation 1                     52,00
------------------------------------------------
                   Reductions
Reductions de lignes 0                      0,00
Reductions d'evenements 0                   0,00
------------------------------------------------
                    FACTURE
Factures 0
------------------------------------------------
             PAIEMENTS ENREGISTRES
Especes                                   612,00
Carte                                     911,50
Total des paiements                      1523,50
Arrondis                                    0,02
------------------------------------------------
    fdmDateTime: 2026-07-03T22:42:00+02:00;
================================================
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

## 7. Rapport Z (M181 - Événement R, clôture)

Scénario : clôture journalière (Z n°1) sur les mêmes totaux.

```
              Z CHIFFRE D'AFFAIRES
               Restaurant Exemple
         Rue de l'Exemple 1, 4000 Liège
               TVA: BE0499999960
        Unite d'etablissement 2123456791
------------------------------------------------
RAPPORT Z                      03/07/26 23:05:41
Rapport nr. 1
Ticket:                                   000003
Employe:                             85073003328
------------------------------------------------
              Periode d'ouverture
Date:                                 2026-07-03
BookingPeriodId:
3f6c2a4e-8b1d-4c7a-9e25-7d0b6a1f4c83
Debut                           2026-07-03 11:02
Fin                             2026-07-03 22:41
POSID                             CFOD0061234567
TerminalId                                  POS1
FDMID                                FDM02000425
Premier                      41 2026-07-03 11:02
Dernier                      63 2026-07-03 22:48
------------------------------------------------
Event N Ventes 42                        1523,50
Event P Pro forma 12                        0,00
------------------------------------------------
                   Apercu TVA
Taux                HT          TVA          TTC
21%             317,78        66,72       384,50
12%            1016,96       122,04      1139,00
6%                   -            -            -
0%                   -            -            -
Total          1334,74       188,76      1523,50
------------------------------------------------
       Departements du chiffre d'affaires
Boissons                                  384,50
Plats                                    1139,00
------------------------------------------------
                  Corrections
Corrections                               -12,00
------------------------------------------------
                 Remboursements
Remboursements 1                          -52,00
------------------------------------------------
                 Tiroir-caisse
Tiroir-caisse 3
------------------------------------------------
                   Formation
Tickets de formation 1                     52,00
------------------------------------------------
                   Reductions
Reductions de lignes 0                      0,00
Reductions d'evenements 0                   0,00
------------------------------------------------
                    FACTURE
Factures 0
------------------------------------------------
             PAIEMENTS ENREGISTRES
Especes                                   612,00
Carte                                     911,50
Total des paiements                      1523,50
Arrondis                                    0,02
------------------------------------------------
    fdmDateTime: 2026-07-03T22:42:00+02:00;
================================================
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

### Rapport Z en néerlandais (NL)

Même scénario que la section 7, imprimé dans la langue configurée
pour le ticket (`pos.json::language = "nl"`, cf.
*GARANTIE_MAJ_LANGUES_FR.md* §3). Le rapport X, le rapport
utilisateur (M182/M183/M184) et les noms de moyen de paiement suivent
la même règle : le rendu ci-dessous est généré par le même code que
la section 7, seule la langue change.

```
                    Z OMZET
               Restaurant Exemple
         Rue de l'Exemple 1, 4000 Liège
               BTW: BE0499999960
          Vestigingseenheid 2123456791
------------------------------------------------
RAPPORT Z                      03/07/26 23:05:41
Rapport nr. 1
Ticket:                                   000003
Werknemer:                           85073003328
------------------------------------------------
                Openingsperiode
Datum:                                2026-07-03
BookingPeriodId:
3f6c2a4e-8b1d-4c7a-9e25-7d0b6a1f4c83
Begin                           2026-07-03 11:02
Einde                           2026-07-03 22:41
POSID                             CFOD0061234567
TerminalId                                  POS1
FDMID                                FDM02000425
Eerste                       41 2026-07-03 11:02
Laatste                      63 2026-07-03 22:48
------------------------------------------------
Event N Verkoop 42                       1523,50
Event P Pro forma 12                        0,00
------------------------------------------------
                 BTW-overzicht
Tarief           Excl.          BTW        Incl.
21%             317,78        66,72       384,50
12%            1016,96       122,04      1139,00
6%                   -            -            -
0%                   -            -            -
Totaal         1334,74       188,76      1523,50
------------------------------------------------
               Omzet per afdeling
Boissons                                  384,50
Plats                                    1139,00
------------------------------------------------
                   Correcties
Correcties                                -12,00
------------------------------------------------
                Terugbetalingen
Terugbetalingen 1                         -52,00
------------------------------------------------
                   Kassalade
Kassalade 3
------------------------------------------------
                   Opleiding
Opleidingstickets 1                        52,00
------------------------------------------------
                   Kortingen
Lijnkortingen 0                             0,00
Ticketkortingen 0                           0,00
------------------------------------------------
                    FACTUUR
Facturen 0
------------------------------------------------
           GEREGISTREERDE BETALINGEN
Contant                                   612,00
Kaart                                     911,50
Totaal betalingen                        1523,50
Afrondingen                                 0,02
------------------------------------------------
    fdmDateTime: 2026-07-03T22:42:00+02:00;
================================================
        DIT IS GEEN GELDIG BTW-KASTICKET




✂-----------------------------------------------
```

### Rapport Z en allemand (DE)

Même scénario, imprimé en allemand (`pos.json::language = "de"`).

```
                    Z UMSATZ
               Restaurant Exemple
         Rue de l'Exemple 1, 4000 Liège
               MwSt: BE0499999960
        Niederlassungseinheit 2123456791
------------------------------------------------
BERICHT Z                      03/07/26 23:05:41
Bericht Nr. 1
Bon:                                      000003
Mitarbeiter:                         85073003328
------------------------------------------------
               Oeffnungszeitraum
Datum:                                2026-07-03
BookingPeriodId:
3f6c2a4e-8b1d-4c7a-9e25-7d0b6a1f4c83
Beginn                          2026-07-03 11:02
Ende                            2026-07-03 22:41
POSID                             CFOD0061234567
TerminalId                                  POS1
FDMID                                FDM02000425
Erstes                       41 2026-07-03 11:02
Letztes                      63 2026-07-03 22:48
------------------------------------------------
Event N Umsaetze 42                      1523,50
Event P Pro forma 12                        0,00
------------------------------------------------
                MwSt-Uebersicht
Satz             Netto         MwSt       Brutto
21%             317,78        66,72       384,50
12%            1016,96       122,04      1139,00
6%                   -            -            -
0%                   -            -            -
Gesamt         1334,74       188,76      1523,50
------------------------------------------------
             Umsatz nach Abteilung
Boissons                                  384,50
Plats                                    1139,00
------------------------------------------------
                  Korrekturen
Korrekturen                               -12,00
------------------------------------------------
               Rueckerstattungen
Rueckerstattungen 1                       -52,00
------------------------------------------------
                Kassenschublade
Kassenschublade 3
------------------------------------------------
                    Schulung
Schulungsbons 1                            52,00
------------------------------------------------
                    Rabatte
Zeilenrabatte 0                             0,00
Bonrabatte 0                                0,00
------------------------------------------------
                    RECHNUNG
Rechnungen 0
------------------------------------------------
               ERFASSTE ZAHLUNGEN
Bar                                       612,00
Karte                                     911,50
Summe Zahlungen                          1523,50
Rundungen                                   0,02
------------------------------------------------
    fdmDateTime: 2026-07-03T22:42:00+02:00;
================================================
   DIES IST KEIN GUELTIGER MWST.-KASSENZETTEL




✂-----------------------------------------------
```

## 8. Bon de commande bar / cuisine (sans prix)

Scénario : les lignes d'une table envoyées au FDM (événement P) ou
d'une vente directe (événement N) sortent aussi sur le bon du bar ou
de la cuisine (`pos.json::kitchen_printer_address`, à défaut
l'imprimante des tickets), comme le ticket bar d'exemple du SPF du
24/09/2026 : jamais de prix, et la mention d'invalidité en bas. Seules
les lignes nouvellement envoyées y figurent, corrections comprises
(quantité négative), pour les produits marqués « cuisine ».

```
                BON DE COMMANDE
================================================
Date:                        03/07/2026 12:31:05
Operateur:                          Marie Dupont
Table 4
------------------------------------------------
Qte   Article
2 x Dry Martini
1 x Burger du chef
------------------------------------------------
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

## 9. Ticket numérique par e-mail (exemple)

Scénario : la même vente M110 qu'à la section 1, envoyée par e-mail
("E-mail" ou "Les deux" choisi avant l'encaissement, cf.
`TICKET_NUMERIQUE_FR.md` §2-3 pour le parcours et §5 pour le contenu
détaillé de l'e-mail).

**Sujet :** `Votre ticket - Restaurant Exemple`

**Corps texte :** la transcription de la section 1 ci-dessus,
**identique ligne pour ligne** (`pos_mail::render::render_ticket_text`
appelle le même `format_receipt` que le papier), suivie d'une ligne
vide puis de l'URL de vérification seule :

```
            TICKET DE CAISSE T.V.A.
Restaurant Exemple
Unite d'etablissement 2123456791
03.07.2026                                 12:34
Utilisateur: Marie Dupont
Terminal: POS1
Ticketnr.: 000015
POS Serial: CFOD0061234567
Input device MAC: 00:1b:44:11:3a:b7
Logiciel de caisse v.: pili-pos/1.0.0
................................................
2 Dry Martini                           24,00  A
1 Burger du chef                        28,00  B
TOTAL :                                   €52,00
................................................
Mode de paiement :
Espèces                                    52,00
% TVA   CODE          HT       TVA      Brut
21,00%  A          19,83      4,17     24,00
12,00%  B          25,00      3,00     28,00
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: FDM02000425;
fdmDateTime: 2026-07-03T12:34:57+02:00;
Event: N; eventCounter: 15;
totalCounter: 59; shortSignature:
a3f9c2b1e8d4750616fb92c3a4e5d6f701234567;

[QR: https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug]


Denomination legale : Societe Exemple SRL
Rue de l'Exemple 1, 4000 Liège
TVA: BE0499999960
      www.systemedecaisseenregistreuse.be




✂-----------------------------------------------

https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug
```

**Version HTML :** le même texte dans un bloc `<pre>` (échappé), plus
le lien de vérification repris comme ancre cliquable
(`<a href="https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug">...</a>`).

**Pièce jointe :** `ticket-qr.png` (PNG, QR Code Model 2, ECC M,
module 8 px, encodant la même URL de vérification). Aucune pièce
jointe PDF.

Une **copie (C)** envoyée par e-mail reprend l'URL et le QR code du
ticket original (spec POS v2.2 §6.5). Un **ticket de formation (T)**
suit le même gabarit, sans la ligne d'URL ni la pièce jointe QR : le
FDM ne renvoie une URL de vérification que pour les événements N
(POS spec v2.2 §2.4) - le corps de l'e-mail est alors la transcription
du ticket seule (cf. section 5 ci-dessus pour le texte imprimé
équivalent).

## 10. Paiement en plusieurs moyens (ROUND5 - Événement N)

Scénario (test d'arrondi 3 du manuel) : Dame Blanche (11,52 €) +
Tapas Variation (3,34 €), TVA B, total 14,86 €, payé 3,13 € en
chèques-repas, 3,97 € par carte et le reste, 7,76 €, en espèces :
encaissé 7,75 €. Chaque moyen de paiement a sa ligne ; seul le reste en
espèces est arrondi, sur la ligne "Arrondi".

```
            TICKET DE CAISSE T.V.A.
Restaurant Exemple
Unite d'etablissement 2123456791
03.07.2026                                 12:34
Utilisateur: Marie Dupont
Terminal: POS1
Ticketnr.: 000015
POS Serial: CFOD0061234567
Input device MAC: 00:1b:44:11:3a:b7
Logiciel de caisse v.: pili-pos/1.0.0
................................................
1 Dame Blanche                          11,52  B
1 Tapas Variation                        3,34  B
TOTAL :                                   €14,86
................................................
Mode de paiement :
Cheques-repas                               3,13
Carte                                       3,97
Especes                                     7,76
Arrondi                                    -0,01
% TVA   CODE          HT       TVA      Brut
12,00%  B          13,27      1,59     14,86
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: FDM02000425;
fdmDateTime: 2026-07-03T12:34:57+02:00;
Event: N; eventCounter: 15;
totalCounter: 59; shortSignature:
a3f9c2b1e8d4750616fb92c3a4e5d6f701234567;

[QR: https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug]


Denomination legale : Societe Exemple SRL
Rue de l'Exemple 1, 4000 Liège
TVA: BE0499999960
      www.systemedecaisseenregistreuse.be




✂-----------------------------------------------
```

## 11. Vente sans paiement (UC4109 - Événement N)

Scénario : la vente M110 de la section 1, dont le client est parti sans
payer. Le Gérant la clôture avec son PIN : le FDM signe la vente sans
ligne de paiement (`financials: []`) et le ticket de caisse TVA porte
"Non paye" à la place des paiements.

```
            TICKET DE CAISSE T.V.A.
Restaurant Exemple
Unite d'etablissement 2123456791
03.07.2026                                 12:34
Utilisateur: Marie Dupont
Terminal: POS1
Ticketnr.: 000015
POS Serial: CFOD0061234567
Input device MAC: 00:1b:44:11:3a:b7
Logiciel de caisse v.: pili-pos/1.0.0
................................................
2 Dry Martini                           24,00  A
1 Burger du chef                        28,00  B
TOTAL :                                   €52,00
................................................
Mode de paiement :
Non paye
% TVA   CODE          HT       TVA      Brut
21,00%  A          19,83      4,17     24,00
12,00%  B          25,00      3,00     28,00
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: FDM02000425;
fdmDateTime: 2026-07-03T12:34:57+02:00;
Event: N; eventCounter: 15;
totalCounter: 59; shortSignature:
a3f9c2b1e8d4750616fb92c3a4e5d6f701234567;

[QR: https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug]


Denomination legale : Societe Exemple SRL
Rue de l'Exemple 1, 4000 Liège
TVA: BE0499999960
      www.systemedecaisseenregistreuse.be




✂-----------------------------------------------
```

## 11 bis. Autres documents imprimés (sans transcription ici)

Trois autres documents sortent de l'imprimante des tickets. Leur
transcription n'est pas produite par la suite
`crates/pos-print/tests/ticket_examples.rs` et n'est donc pas
reproduite dans ce fichier :

- **Ticket cuisine d'une commande en ligne** :
  `pos_print::kitchen_ticket::format_kitchen_ticket`, envoyé par
  `commands::print::print_webshop_kitchen_ticket` (à l'acceptation de
  la commande). Il n'est utilisé que pour les commandes en ligne ; les
  lignes envoyées d'une table ou d'une vente directe sortent sur le bon
  du §8 (`bar_ticket::format_bar_ticket`).
- **Ticket client d'une commande en ligne** (non fiscal, avec la
  mention « Ce ticket n'est pas un recu TVA valide ») :
  `pos_print::customer_slip::format_customer_slip`, envoyé par
  `commands::print::do_print_webshop_customer`. Il porte le nom, le
  téléphone et l'adresse du client (voir `SECURITE_BD_FR.md` §6).
- **Rapports X et Z par utilisateur** (M182, M183, M184) :
  `pos_print::report::format_user_report`, imprimés avec le rapport
  chiffre d'affaires correspondant (`commands::report::print_user_report`).

## 12. Note sur la reproduction

Les rendus ci-dessus sont générés par le pipeline d'impression réel
(`crates/pos-print/src/receipt.rs` et `report.rs`) - ils ne peuvent
pas diverger de la sortie papier, contrairement à des transcriptions
manuelles. Toute modification du layout d'impression invalide et
régénère ces exemples via `scripts/generate_ticket_examples.sh`.

Lors du rendez-vous d'admission SPF Finances, le distributeur
apportera la machine test connectée à l'imprimante réelle pour
démontrer la sortie physique de chaque type de ticket. Les
exemplaires physiques de ces tickets seront archivés avec le
dossier final.
