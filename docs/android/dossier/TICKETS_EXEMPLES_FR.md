<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/TICKETS_EXEMPLES_FR.md @ 86088e4 -->
# Tickets de test - exemples imprimés - pili-pos-android

**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
**Date :** À effectuer (EV-18 : date de dépôt)
**Tickets relevés le :** 15.10.2026, sur l'AVD `sunmi_d3_pro_gms_api33` (identifiant d'appareil imprimé `EMULATOR37X2X12X0`)

Conformément au §IV §1209-1210 du Manuel de certification SCE 2.0, le dossier doit contenir un exemple de chaque type de ticket imprimé par le système.

Ces rendus ont été produits par le pipeline d'impression réel du moteur embarqué (mêmes fonctions `format_receipt` et `format_report` que la caisse en service), imprimés sur l'imprimante simulée `pos-printer-mock` puis transcrits en texte (page HTTP du simulateur, octets d'état DLE EOT retirés), avec l'identifiant du terminal de test sur chaque ticket. Largeur de 48 caractères, papier 80 mm, Epson TM-m30III, code page CP858. Aucun ticket du dossier Linux n'est repris ici : chaque transcription vient d'une vente faite à l'écran de la caisse Android contre les simulateurs de l'hôte (FDM, imprimante, imprimante cuisine, service d'e-mail, terminal de paiement), jamais d'un fournisseur réel. Les fichiers bruts sont dans `evidence/tickets/`. Impression sur l'Epson réelle : `À effectuer (EV-15 : imprimante réelle, ou mention du profil émulé)`. Identifiant d'appareil imprimé : `EMULATOR37X2X12X0` (EV-16, ligne « Input device MAC » ; sur Android c'est le numéro de série matériel de la caisse, ici celui de l'AVD).

## Matrice de conformité du contenu (§VI du manuel)

Réponse au tableau « Contenu du ticket » du manuel §VI. Une ligne par élément obligatoire de la spec POS v2.2 §III.3, une colonne par type de ticket. L'élément VII (identification du dispositif d'entrée, `deviceId`) est sur Android le numéro de série matériel de la caisse, non une adresse MAC (PLATEFORME_ANDROID_FR.md §4).

| Élément | N | N Refund | P (Pro Forma) | C (Copy) | T (Training) | Autres (cuisine) |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| I. Dénomination complète du prestataire | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| II. Identification (nom, adresse, n° TVA) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| III. Dénomination commerciale | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| IV. N° unité d'établissement (EST) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| V. N° fabrication POS (posId) + n° version logiciel | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| VI. Identification terminal (terminalId) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| VII. Identification dispositif d'entrée (`deviceId`, numéro de série matériel sur Android) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| VIII. Identification utilisateur (nom de l'opérateur ; l'INSZ n'est pas imprimé) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| IX. Date et heure de délivrance | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ |
| X. N° de ticket continu (série ininterrompue) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| XI.1. Lignes produits : qté, désignation, prix, code TVA | ☑ | ☑ (négatif) | ☑ | ☑ | ☑ | ☑ (désignation seule, pas de prix) |
| XI.2. Modifications de prix sur la ligne | ☑ (commandes en ligne et « Offrir » à la caisse : remise imprimée sous sa ligne) | ☑ (remise reprise en négatif) | N/A | ☑ (copie d'une vente en ligne) | N/A | - |
| XI.3. GTIN (optionnel) | N/A | N/A | N/A | N/A | N/A | - |
| XII. Total à payer TVA comprise | ☑ | ☑ (négatif) | ☑ | ☑ | ☑ | - |
| XIII. Base imposable par taux de TVA (calculée par le FDM) | ☑ | ☑ | - (jamais sur une addition) | ☑ | ☑ | - |
| XIV. Montant TVA par taux (calculé par le FDM) | ☑ | ☑ | - | ☑ | ☑ | - |
| XV. Modifications de prix hors lignes produits | ☑ (la remise d'une commande, saisie à la caisse par un Gérant ou venue d'une commande en ligne, est répartie sur les lignes et imprimée sous chacune, voir XI.2 ; « Offert » signe la vente à 0,00) | ☑ (remise reprise en négatif) | ☑ (l'addition imprime les remises) | N/A | N/A | - |
| XVI.I. Pied de page configuré par SPF Finances (imprimé quand le FDM en renvoie ; le simulateur du banc n'en renvoie pas, la zone reste vide) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
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

Cases relevées sur les transcriptions des sections 1 à 8 ci-dessous.

**Règles de langue :** les mentions fiscales sont imprimées dans la langue officielle de la région linguistique de l'établissement (FR/NL/DE), fixée par installation (GARANTIE_MAJ_LANGUES_FR.md §3.2). Les exemples sont présentés en français ; le rapport Z l'est aussi en néerlandais et en allemand (section 7), obtenus en réglant `language` du fichier `pos.json` de l'appareil de test puis en redémarrant l'application. Les libellés fixes (moyens de paiement, « Unite d'etablissement », « Denomination legale »...) sont imprimés sans accents, à dessein : `labels.rs` les garde en ASCII pour qu'ils s'impriment de la même façon quelle que soit la page de codes de l'imprimante (seuls les textes saisis, comme les produits et l'adresse, passent par CP858). Les exemples reprennent donc « Especes » et « Chèques-repas » tels qu'imprimés.

---

## 1. Vente normale (M110 - Événement N)
Scénario du manuel : 2 Dry Martini (12,00 €, TVA A) et 1 Burger du chef (28,00 €, TVA B), payé en espèces. 

```
            TICKET DE CAISSE T.V.A.
Restaurant Test
Unite d'etablissement 2000000042
15.10.2026                                 00:46
Utilisateur: Gerant Test
Terminal: POS-2-ANDROID
Ticketnr.: 000001
POS Serial: CPOS0031234567
Input device MAC: EMULATOR37X2X12X0
Logiciel de caisse v.: pili-pos-android/1.0.0
................................................
2 Dry Martini                           24,00  A
1 Burger of the CHEF                    28,00  B
TOTAL :                                   €52,00
................................................
Mode de paiement :
Especes                                    52,00
% TVA   CODE          HT       TVA      Brut
21,00%  A          19,83      4,17     24,00
12,00%  B          25,00      3,00     28,00
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-14T22:46:04Z;
Event: N; eventCounter: 1;
totalCounter: 4; shortSignature:
0bf0f4adefa87f77457a0995208ffca905958e31;

 [QR: https://www.gs2.be/M0000100004AbCdEfGhI]  


Denomination legale : Restaurant Test
1 Rue du Test, 4000 Liege
TVA: BE0000000097




✂-----------------------------------------------
```

## 2. Addition, pré-ticket (M123 - Événement P)
Même commande qu'au M110, juste avant l'encaissement. Ni QR code ni shortSignature : le FDM ne les renvoie que pour une vente (N).

```
                   PRO FORMA
              ADDITION PROVISOIRE
Restaurant Test
Unite d'etablissement 2000000042
15.10.2026                                 00:45
Utilisateur: Gerant Test
Terminal: POS-2-ANDROID
Ticketnr.: 000002
POS Serial: CPOS0031234567
Input device MAC: EMULATOR37X2X12X0
Logiciel de caisse v.: pili-pos-android/1.0.0
................................................
2 Dry Martini                           24,00  A
1 Burger of the CHEF                    28,00  B
TOTAL :                                   €52,00
................................................
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-14T22:45:58Z;
Event: P; eventCounter: 2;
totalCounter: 3;

Denomination legale : Restaurant Test
1 Rue du Test, 4000 Liege
TVA: BE0000000097

   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

## 3. Remboursement complet (M111 - Événement N négatif)
Remboursement intégral du M110. Remboursement en espèces par « Rapports », « Tickets », « Rembourser » sur le ticket de la section 1 ; le bloc « Informations FDM d'origine » reprend ce ticket.

```
            TICKET DE CAISSE T.V.A.
                 REMBOURSEMENT
Restaurant Test
Unite d'etablissement 2000000042
15.10.2026                                 00:47
Utilisateur: Gerant Test
Terminal: POS-2-ANDROID
Ticketnr.: 000002
POS Serial: CPOS0031234567
Input device MAC: EMULATOR37X2X12X0
Logiciel de caisse v.: pili-pos-android/1.0.0
................................................
-2 Dry Martini                         -24,00  A
-1 Burger of the CHEF                  -28,00  B
TOTAL :                                  €-52,00
................................................
Mode de paiement :
Especes                                   -52,00
% TVA   CODE          HT       TVA      Brut
12,00%  B         -25,00     -3,00    -28,00
21,00%  A         -19,83     -4,17    -24,00
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-14T22:47:29Z;
Event: N; eventCounter: 2;
totalCounter: 6; shortSignature:
848cd607e989bdfe53ae5a21bb9452c463aaaf62;

 [QR: https://www.gs2.be/M0000200006AbCdEfGhI]  

Informations FDM d'origine :
fdmId: MCK00000001;
fdmDateTime: 2026-10-14T22:46:04Z;
Event: N; eventCounter: 1;
totalCounter: 4; shortSignature:
0bf0f4adefa87f77457a0995208ffca905958e31;

Denomination legale : Restaurant Test
1 Rue du Test, 4000 Liege
TVA: BE0000000097




✂-----------------------------------------------
```

## 4. Copie de ticket (M160 - Événement C)
Copie du ticket M110. La copie reproduit le ticket de la section 1 et son QR code.

```
            TICKET DE CAISSE T.V.A.
                     COPIE
Restaurant Test
Unite d'etablissement 2000000042
15.10.2026                                 00:46
Utilisateur: Gerant Test
Terminal: POS-2-ANDROID
Ticketnr.: 000001
POS Serial: CPOS0031234567
Input device MAC: EMULATOR37X2X12X0
Logiciel de caisse v.: pili-pos-android/1.0.0
................................................
2 Dry Martini                           24,00  A
1 Burger of the CHEF                    28,00  B
TOTAL :                                   €52,00
................................................
Mode de paiement :
Especes                                    52,00
% TVA   CODE          HT       TVA      Brut
21,00%  A          19,83      4,17     24,00
12,00%  B          25,00      3,00     28,00
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-14T22:46:42Z;
Event: C; eventCounter: 1;
totalCounter: 5;

 [QR: https://www.gs2.be/M0000100004AbCdEfGhI]  

Informations FDM d'origine :
fdmId: MCK00000001;
fdmDateTime: 2026-10-14T22:46:04Z;
Event: N; eventCounter: 1;
totalCounter: 4; shortSignature:
0bf0f4adefa87f77457a0995208ffca905958e31;
Copie etablie le 15.10.2026 00:46

Denomination legale : Restaurant Test
1 Rue du Test, 4000 Liege
TVA: BE0000000097




✂-----------------------------------------------
```

## 5. Vente en formation (M171 - Événement T)
Même commande que M110 en mode formation. Mode formation actif dans Configuration ; ni QR code ni shortSignature.

```
        TRAINING
================================================
                    TRAINING
Restaurant Test
Unite d'etablissement 2000000042
15.10.2026                                 00:51
Utilisateur: Gerant Test
Terminal: POS-2-ANDROID
Ticketnr.: 000001
POS Serial: CPOS0031234567
Input device MAC: EMULATOR37X2X12X0
Logiciel de caisse v.: pili-pos-android/1.0.0
................................................
2 Dry Martini                           24,00  A
1 Burger of the CHEF                    28,00  B
TOTAL :                                   €52,00
................................................
Mode de paiement :
Especes                                    52,00
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-14T22:51:58Z;
Event: T; eventCounter: 1;
totalCounter: 12;

Denomination legale : Restaurant Test
1 Rue du Test, 4000 Liege
TVA: BE0000000097

   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

## 6. Rapport X (M180 - Événement R, intermédiaire)
Rapport X en cours de service. Tiré après les ventes des sections 1 à 4 et 10 à 11, avant la clôture.

```
              X CHIFFRE D'AFFAIRES
                Restaurant Test
           1 Rue du Test, 4000 Liege
               TVA: BE0000000097
        Unite d'etablissement 2000000042
------------------------------------------------
RAPPORT X                      15/10/26 00:50:38
Ticket:                                   000001
Employe:                             94060242576
------------------------------------------------
              Periode d'ouverture
Date:                                 2026-10-14
BookingPeriodId:
663a179a-4f60-4238-802a-dcdc466ce9f0
Debut                           2026-10-15 00:45
Fin                             2026-10-15 00:50
POSID                             CPOS0031234567
TerminalId                         POS-2-ANDROID
FDMID                                MCK00000001
Premier                       1 2026-10-14 22:45
Dernier                       8 2026-10-14 22:50
------------------------------------------------
Event N Ventes 4                           26,86
Event P Pro forma 2                       104,00
------------------------------------------------
                   Apercu TVA
Taux                HT          TVA          TTC
21%               9,92         2,08        12,00
12%              13,27         1,59        14,86
6%                   -            -            -
0%                   -            -            -
Total            23,19         3,67        26,86
------------------------------------------------
       Departements du chiffre d'affaires
Aperitifs                                  12,00
Main Dishes                                 0,00
Desserts                                   11,52
Tapas                                       3,34
------------------------------------------------
                  Corrections
Corrections                                 0,00
------------------------------------------------
                 Remboursements
Remboursements 1                          -52,00
------------------------------------------------
                 Tiroir-caisse
Tiroir-caisse 0
------------------------------------------------
                   Formation
Tickets de formation 0                      0,00
------------------------------------------------
                   Reductions
Reductions de lignes 0                      0,00
Reductions d'evenements 0                   0,00
------------------------------------------------
                    FACTURE
Factures 0
------------------------------------------------
             PAIEMENTS ENREGISTRES
Especes                                     2,46
Chèques-repas                               11,52
Carte                                       0,88
Total des paiements                        14,86
Arrondis                                   -0,01
------------------------------------------------
       fdmDateTime: 2026-10-14T22:50:37Z;
================================================
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

## 7. Rapport Z (M181 - Événement R, clôture)
Clôture journalière (Z n°1). Z n°1, tiré après le rapport X ; la clôture imprime aussi le rapport par utilisateur (`evidence/tickets/rapport_utilisateurs_Z.txt`).

```
              Z CHIFFRE D'AFFAIRES
                Restaurant Test
           1 Rue du Test, 4000 Liege
               TVA: BE0000000097
        Unite d'etablissement 2000000042
------------------------------------------------
RAPPORT Z                      15/10/26 00:51:02
Rapport nr. 1
Ticket:                                   000002
Employe:                             94060242576
------------------------------------------------
              Periode d'ouverture
Date:                                 2026-10-14
BookingPeriodId:
663a179a-4f60-4238-802a-dcdc466ce9f0
Debut                           2026-10-15 00:45
Fin                             2026-10-15 00:50
POSID                             CPOS0031234567
TerminalId                         POS-2-ANDROID
FDMID                                MCK00000001
Premier                       1 2026-10-14 22:45
Dernier                       9 2026-10-14 22:50
------------------------------------------------
Event N Ventes 4                           26,86
Event P Pro forma 2                       104,00
------------------------------------------------
                   Apercu TVA
Taux                HT          TVA          TTC
21%               9,92         2,08        12,00
12%              13,27         1,59        14,86
6%                   -            -            -
0%                   -            -            -
Total            23,19         3,67        26,86
------------------------------------------------
       Departements du chiffre d'affaires
Aperitifs                                  12,00
Main Dishes                                 0,00
Desserts                                   11,52
Tapas                                       3,34
------------------------------------------------
                  Corrections
Corrections                                 0,00
------------------------------------------------
                 Remboursements
Remboursements 1                          -52,00
------------------------------------------------
                 Tiroir-caisse
Tiroir-caisse 0
------------------------------------------------
                   Formation
Tickets de formation 0                      0,00
------------------------------------------------
                   Reductions
Reductions de lignes 0                      0,00
Reductions d'evenements 0                   0,00
------------------------------------------------
                    FACTURE
Factures 0
------------------------------------------------
             PAIEMENTS ENREGISTRES
Especes                                     2,46
Chèques-repas                               11,52
Carte                                       0,88
Total des paiements                        14,86
Arrondis                                   -0,01
------------------------------------------------
       fdmDateTime: 2026-10-14T22:51:02Z;
================================================
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

### Rapport Z en néerlandais (NL)
Même Z, ticket en néerlandais. Z n°2 de l'appareil de test (nouvelle journée, une vente) avec `language` réglé sur `nl` : le contenu diffère de la section 7 (autre journée), les libellés illustrent la règle de langue.

```
                    Z OMZET
                Restaurant Test
           1 Rue du Test, 4000 Liege
               BTW: BE0000000097
          Vestigingseenheid 2000000042
------------------------------------------------
RAPPORT Z                      15/10/26 00:53:30
Rapport nr. 2
Ticket:                                   000004
Werknemer:                           94060242576
------------------------------------------------
                Openingsperiode
Datum:                                2026-10-14
BookingPeriodId:
369a5777-2b22-4ba8-8321-01ea3bb0abd9
Begin                           2026-10-15 00:51
Einde                           2026-10-15 00:53
POSID                             CPOS0031234567
TerminalId                         POS-2-ANDROID
FDMID                                MCK00000001
Eerste                       12 2026-10-14 22:51
Laatste                      13 2026-10-14 22:53
------------------------------------------------
Event N Verkoop 1                          12,00
Event T Opleiding 1                        52,00
------------------------------------------------
                 BTW-overzicht
Tarief           Excl.          BTW        Incl.
21%               9,92         2,08        12,00
12%                  -            -            -
6%                   -            -            -
0%                   -            -            -
Totaal            9,92         2,08        12,00
------------------------------------------------
               Omzet per afdeling
Aperitifs                                  12,00
------------------------------------------------
                   Correcties
Correcties                                  0,00
------------------------------------------------
                Terugbetalingen
Terugbetalingen 0                           0,00
------------------------------------------------
                   Kassalade
Kassalade 0
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
Contant                                    12,00
Totaal betalingen                          12,00
Afrondingen                                 0,00
------------------------------------------------
       fdmDateTime: 2026-10-14T22:53:30Z;
================================================
        DIT IS GEEN GELDIG BTW-KASTICKET




✂-----------------------------------------------
```

### Rapport Z en allemand (DE)
Même Z, ticket en allemand. Z n°3, `language` réglé sur `de`, même remarque.

```
                    Z UMSATZ
                Restaurant Test
           1 Rue du Test, 4000 Liege
               MwSt: BE0000000097
        Niederlassungseinheit 2000000042
------------------------------------------------
BERICHT Z                      15/10/26 00:54:30
Bericht Nr. 3
Bon:                                      000006
Mitarbeiter:                         94060242576
------------------------------------------------
               Oeffnungszeitraum
Datum:                                2026-10-14
BookingPeriodId:
bc64d77a-1105-415b-821c-f3994cbc9eaf
Beginn                          2026-10-15 00:54
Ende                            2026-10-15 00:54
POSID                             CPOS0031234567
TerminalId                         POS-2-ANDROID
FDMID                                MCK00000001
Erstes                       16 2026-10-14 22:54
Letztes                      16 2026-10-14 22:54
------------------------------------------------
Event N Umsaetze 1                         12,00
------------------------------------------------
                MwSt-Uebersicht
Satz             Netto         MwSt       Brutto
21%               9,92         2,08        12,00
12%                  -            -            -
6%                   -            -            -
0%                   -            -            -
Gesamt            9,92         2,08        12,00
------------------------------------------------
             Umsatz nach Abteilung
Aperitifs                                  12,00
------------------------------------------------
                  Korrekturen
Korrekturen                                 0,00
------------------------------------------------
               Rueckerstattungen
Rueckerstattungen 0                         0,00
------------------------------------------------
                Kassenschublade
Kassenschublade 0
------------------------------------------------
                    Schulung
Schulungsbons 0                             0,00
------------------------------------------------
                    Rabatte
Zeilenrabatte 0                             0,00
Bonrabatte 0                                0,00
------------------------------------------------
                    RECHNUNG
Rechnungen 0
------------------------------------------------
               ERFASSTE ZAHLUNGEN
Bar                                        12,00
Summe Zahlungen                            12,00
Rundungen                                   0,00
------------------------------------------------
       fdmDateTime: 2026-10-14T22:54:29Z;
================================================
   DIES IST KEIN GUELTIGER MWST.-KASSENZETTEL




✂-----------------------------------------------
```

## 8. Bon de commande bar / cuisine (sans prix)
Lignes envoyées d'une table ou d'une vente directe, jamais de prix. « Envoyer en cuisine » sur la commande de la section 1 : seul l'article du bon cuisine est imprimé, sans prix.

```
                BON DE COMMANDE
================================================
Date:                        15/10/2026 00:45:53
Operateur:                           Gerant Test
Table 5
------------------------------------------------
Qte   Article
1 x Burger of the CHEF
------------------------------------------------
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

### Bon de commande avec note de cuisine

Scénario : la ligne « Burger du chef » porte la note « allergie arachide
- sans oignons ». La note s'imprime en retrait sous l'article, sans prix ;
elle n'a aucun effet sur le message envoyé au FDM.

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
   > allergie arachide - sans oignons
------------------------------------------------
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

### Bon de modification de note (note changée après l'envoi)

Scénario : la note du « Burger du chef », déjà en cuisine, devient « bien
cuit ». Le bon sort tout de suite, sans quantité ni prix, avec la mention
d'invalidité en bas ; ce n'est ni une commande ni un ticket TVA, et il
n'envoie rien au FDM. Une note effacée imprime « Note supprimee ».

```
                  MODIFICATION
================================================
Date:                        03/07/2026 12:38:12
Operateur:                          Marie Dupont
Table 4
------------------------------------------------
Burger du chef
Note : bien cuit
------------------------------------------------
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

### Bon de transfert de table (M122 - Événement P)

Scénario : une commande ouverte passe de la table 4 à la table 9
(`signCostCenterChange`). Le bon sort sur l'imprimante du bar, sans
aucun article ni prix, avec la mention d'invalidité en bas ; une
impression en échec n'annule jamais la signature.

```
               TRANSFERT DE TABLE
================================================
Date:                        03/07/2026 12:35:40
Operateur:                          Marie Dupont
Table 4 -> Table 9
------------------------------------------------
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

## 9. Ticket numérique par e-mail (exemple)
Même vente M110 envoyée par e-mail (TICKET_NUMERIQUE_FR.md §2-3 et §5). Vente en espèces (2 Dry Martini et 1 Burger of the CHEF, 52,00 €) avec le support « E-mail » ; corps texte reçu par le simulateur du service d'e-mail (sujet `Votre ticket - Restaurant Test`, pièce jointe `ticket-qr.png`), suivi de l'URL de vérification.

```
            TICKET DE CAISSE T.V.A.
Restaurant Test
Unite d'etablissement 2000000042
15.10.2026                                 00:56
Utilisateur: Gerant Test
Terminal: POS-2-ANDROID
Ticketnr.: 000007
POS Serial: CPOS0031234567
Input device MAC: EMULATOR37X2X12X0
Logiciel de caisse v.: pili-pos-android/1.0.0
................................................
2 Dry Martini                           24,00  A
1 Burger of the CHEF                    28,00  B
TOTAL :                                   €52,00
................................................
Mode de paiement :
Especes                                    52,00
% TVA   CODE          HT       TVA      Brut
21,00%  A          19,83      4,17     24,00
12,00%  B          25,00      3,00     28,00
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-14T22:56:16Z;
Event: N; eventCounter: 7;
totalCounter: 19; shortSignature:
42aa7c69dd1f3a68b8024d422af41d7f5fc5006a;

 [QR: https://www.gs2.be/M0000700019AbCdEfGhI]  


Denomination legale : Restaurant Test
1 Rue du Test, 4000 Liege
TVA: BE0000000097




✂-----------------------------------------------


https://www.gs2.be/M0000700019AbCdEfGhI
```

## 10. Paiement en plusieurs moyens (ROUND5 - Événement N)
Chèques-repas, carte et reste en espèces arrondi. Dame Blanche 11,52 € et Tapas variation 3,34 € : chèques-repas, carte (terminal relié, simulateur) et reste en espèces arrondi aux 5 cents.

```
            TICKET DE CAISSE T.V.A.
Restaurant Test
Unite d'etablissement 2000000042
15.10.2026                                 00:50
Utilisateur: Gerant Test
Terminal: POS-2-ANDROID
Ticketnr.: 000004
POS Serial: CPOS0031234567
Input device MAC: EMULATOR37X2X12X0
Logiciel de caisse v.: pili-pos-android/1.0.0
................................................
1 Dame Blanche                          11,52  B
1 Tapas variation                        3,34  B
TOTAL :                                   €14,86
................................................
Mode de paiement :
Chèques-repas                               11,52
Carte Visa                                  0,88
Especes                                     2,46
Arrondi                                    -0,01
% TVA   CODE          HT       TVA      Brut
12,00%  B          13,27      1,59     14,86
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-14T22:50:14Z;
Event: N; eventCounter: 4;
totalCounter: 8; shortSignature:
b9d7bbea8214876a11f8b2d3c78fb9c9a7f39854;

 [QR: https://www.gs2.be/M0000400008AbCdEfGhI]  


Denomination legale : Restaurant Test
1 Rue du Test, 4000 Liege
TVA: BE0000000097




✂-----------------------------------------------
```

## 11. Vente sans paiement (UC4109 - Événement N)
Client parti sans payer, clôturé par le gérant avec son PIN. « Client parti sans payer », confirmé par le PIN du gérant : ligne « Non paye ».

```
            TICKET DE CAISSE T.V.A.
Restaurant Test
Unite d'etablissement 2000000042
15.10.2026                                 00:48
Utilisateur: Gerant Test
Terminal: POS-2-ANDROID
Ticketnr.: 000003
POS Serial: CPOS0031234567
Input device MAC: EMULATOR37X2X12X0
Logiciel de caisse v.: pili-pos-android/1.0.0
................................................
1 Dry Martini                           12,00  A
TOTAL :                                   €12,00
................................................
Mode de paiement :
Non paye
% TVA   CODE          HT       TVA      Brut
21,00%  A           9,92      2,08     12,00
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-14T22:48:05Z;
Event: N; eventCounter: 3;
totalCounter: 7; shortSignature:
57a04b5e82231c24acdd4bef132a4e646dc89fc8;

 [QR: https://www.gs2.be/M0000300007AbCdEfGhI]  


Denomination legale : Restaurant Test
1 Rue du Test, 4000 Liege
TVA: BE0000000097




✂-----------------------------------------------
```

## 11 bis. Autres documents imprimés (sans transcription ici)
Ticket cuisine et ticket client d'une commande en ligne, rapports X et Z par utilisateur (M182, M183, M184) : le rapport par utilisateur de la clôture est joint (`evidence/tickets/rapport_utilisateurs_Z.txt`) ; les autres ne sont pas transcrits ici.
## 11 ter. Bons cadeaux, allergènes et service en plusieurs temps

Documents des fonctions décrites dans *BONS_CADEAUX_FR.md*, *ALLERGENES_FR.md* et *SERVICES_CUISINE_FR.md*,
produits par le pipeline d'impression réel du moteur (`crates/pos-print/tests/ticket_examples.rs`, les mêmes fonctions `format_receipt`, `format_gift_voucher`, `format_balance_slip`, `format_allergen_sheet` et `format_bar_ticket` que l'application). Ces rendus sont ceux du moteur exécuté sur l'hôte ; leur relevé sur l'imprimante simulée du terminal émulé : À effectuer (EV-08 : bon cadeau, solde du bon, fiche d'allergènes et bons de commande par temps). Le ticket de T.V.A. d'une
vente payée avec un bon est un ticket N ordinaire dont une ligne de paiement porte le nom du moyen ; les autres
documents sont non fiscaux, sans signature, et portent la mention d'un document qui n'est pas un ticket de caisse
T.V.A. valable.

### 11 ter.1 Vente payée avec un bon cadeau (UC261 - Événement N)

Deux menus d'affaires (116,00 €), payés 50,00 € avec un bon et 66,00 € en espèces. Le code du bon n'est jamais
imprimé sur un ticket de T.V.A.

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
2 Menu d'affaires                      116,00  B
TOTAL :                                  €116,00
................................................
Mode de paiement :
Bon cadeau                                 50,00
Especes                                    66,00
% TVA   CODE          HT       TVA      Brut
12,00%  B         103,57     12,43    116,00
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

### 11 ter.2 Bon cadeau (après la vente qui le vend)

```
                   BON CADEAU
          The FPS Finances Experience
             Boulevard Albert II 33
                 1030 Brussels
================================================
               Valeur: 50,00 EUR

                      Code
                 1234 5678 9095
               [QR: 123456789095]               

              Emis le: 03.07.2026
          Valable jusqu'au 03.07.2027
------------------------------------------------
Utilisable en une ou plusieurs fois, sans rendu
                   de monnaie
                  Ticket: N 12
------------------------------------------------
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

### 11 ter.3 Solde du bon (après un paiement qui laisse un solde)

Le code est masqué : seuls les quatre derniers chiffres sont imprimés.

```
                  SOLDE DU BON
          The FPS Finances Experience
------------------------------------------------
              Code: **** **** 9095
                Solde: 30,00 EUR
          Valable jusqu'au 03.07.2027
------------------------------------------------
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

### 11 ter.4 Fiche d'allergènes

Un produit à la liste déclarée, un produit déclaré sans allergène, un produit sans déclaration. Aucun prix.

```
                   ALLERGÈNES
          The FPS Finances Experience
                   03/07/2026
================================================
Maki saumon
  Contient: Poissons, Sésame
  Traces possibles: Mollusques
  Options:
  - Sauce soja
      Contient: Soja, Céréales contenant du
      gluten
------------------------------------------------
Riz nature
  Aucun allergène
------------------------------------------------
Dessert du jour
  Information non disponible, demandez au
  responsable
------------------------------------------------
Selon le règlement (UE) n° 1169/2011, annexe II.
    Informations fournies par l'exploitant.
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

### 11 ter.5 Premier bon de commande d'une table servie en plusieurs temps

Le Direct et les entrées partent, les plats et les desserts sont retenus.

```
                BON DE COMMANDE
================================================
Date:                        03/07/2026 12:31:05
Operateur:                          Marie Dupont
Table 4
------------------------------------------------
Qte   Article
-- Direct --
2 x Dry Martini
-- Entrees --
2 x Carpaccio
------------------------------------------------
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

### 11 ter.6 Bon de commande « Envoyer la suite »

```
                BON DE COMMANDE
================================================
Date:                        03/07/2026 12:58:40
Operateur:                          Marie Dupont
Table 4
SUITE : Plats
------------------------------------------------
Qte   Article
-- Plats --
1 x Burger du chef
1 x Tapas Variation
------------------------------------------------
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    VALABLE




✂-----------------------------------------------
```

## 12. Note sur la reproduction
Les rendus sont imprimés par le pipeline d'impression réel de l'application et ne peuvent pas diverger de la sortie papier. Les accents des libellés (« Especes », « Chèques-repas ») sont retirés à l'impression sur ce profil. Au rendez-vous d'admission, le distributeur apporte la machine test connectée à l'imprimante réelle (`À effectuer (EV-15 : imprimante réelle, ou mention du profil émulé)`) ; les exemplaires physiques sont archivés avec le dossier final.
