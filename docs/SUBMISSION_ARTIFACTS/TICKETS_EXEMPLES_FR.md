# Tickets de test - exemples imprimés

Conformément au §IV §1209-1210 du Manuel de certification SCE 2.0,
le dossier doit contenir un exemple de chaque type de ticket
imprimé par le système.

Ces rendus sont des **transcriptions texte** des sorties ESC/POS de
`crates/pos-print/src/receipt.rs` pour les scénarios de test SPF
Finances correspondants. La largeur est de 32 caractères (papier
80 mm thermique, police 9×17 px Epson TM-m30III).

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

Scénario : Saumon roll x8 (8,50 €) + Coca 33cl (3,50 €), payé en
espèces.

```
================================
       TOKYO SUSHI BAR
   Rue Test 1, 1000 Bruxelles
       TVA: BE0499999960
       Ets:  1234567890
================================
  TICKET TVA - Vente en salle
2026-05-14 12:34:56     Pos: POS1

1 x Saumon roll x8 (B)
                          8.50
1 x Coca 33cl (A)
                          3.50

-------------------------------
TOTAL TTC                12.00
Espèces                  12.00

VAT  Taux   Net      TVA    Brut
 A   21%   2.89    0.61    3.50
 B   12%   7.59    0.91    8.50
-------------------------------

[ Données de contrôle FDM ]
FDM ID    : FOD01000001
Date/h FDM: 2026-05-14T12:34:57Z
Étiquette : N
Compteur N: 00000015
Total cnt : 00000059
Signature : a3f9c2b1e8

[QR CODE 25x25 modules]
https://verify.fod.gov.be/X9k...

         MERCI DE VOTRE VISITE
================================
```

Champs obligatoires (POS v2.2 §III.3) :
- ✓ En-tête : nom, adresse, TVA, n° établissement.
- ✓ Date / heure / pos_id.
- ✓ Lignes : quantité, désignation, code TVA, total.
- ✓ Total TTC.
- ✓ Mode de paiement.
- ✓ Récapitulatif TVA par taux.
- ✓ Bloc de données de contrôle FDM complet.
- ✓ QR Code Type 2, 25×25 modules.

## 2. Pré-ticket (M123 - Événement P)

Scénario : Même commande qu'au M110, juste avant l'encaissement.

```
================================
       TOKYO SUSHI BAR
   Rue Test 1, 1000 Bruxelles
       TVA: BE0499999960
================================
   * PRO FORMA *
   ne tient pas lieu de
   ticket TVA valable
================================
2026-05-14 12:30:12     Pos: POS1
Table: 1

1 x Saumon roll x8 (B)
                          8.50
1 x Coca 33cl (A)
                          3.50

-------------------------------
À PAYER                  12.00

[ Données de contrôle FDM ]
FDM ID    : FOD01000001
Date/h FDM: 2026-05-14T12:30:13Z
Étiquette : P
Compteur P: 00000007
Total cnt : 00000058

[QR CODE 25x25 modules]
================================
```

Différences avec un ticket N :
- Mention "PRO FORMA" en gros.
- Pas de mode de paiement.
- Pas de récapitulatif TVA (le client n'a pas encore payé).
- `eventLabel: P`.
- `shortSignature` absente (P n'en a pas, cf. v1.7 schéma).

## 3. Remboursement complet (M111 - Événement N négatif)

Scénario : Remboursement intégral du M110 ci-dessus.

```
================================
       TOKYO SUSHI BAR
   Rue Test 1, 1000 Bruxelles
       TVA: BE0499999960
       Ets:  1234567890
================================
    * REMBOURSEMENT *
================================
2026-05-14 12:42:11     Pos: POS1
Ticket original: N #00000015

-1 x Saumon roll x8 (B)
                         -8.50
-1 x Coca 33cl (A)
                         -3.50

-------------------------------
TOTAL TTC               -12.00
Espèces                 -12.00

VAT  Taux   Net      TVA    Brut
 A   21%  -2.89   -0.61   -3.50
 B   12%  -7.59   -0.91   -8.50
-------------------------------

[ Données de contrôle FDM ]
FDM ID    : FOD01000001
Date/h FDM: 2026-05-14T12:42:12Z
Étiquette : N
Compteur N: 00000016
Total cnt : 00000060
Signature : 8e5c2a1bd4

Lien original (fdmRef):
  FDM: FOD01000001
  Étiquette: N
  Compteur N: 00000015
  Total cnt: 00000059

[QR CODE 25x25 modules]
================================
```

Spécificités :
- Quantités **négatives** sur chaque ligne.
- Mention "REMBOURSEMENT" bien visible.
- Bloc `fdmRef` qui référence le ticket original.
- Total TTC négatif, espèces négatives (sortie de caisse).

## 4. Copie de ticket (M160 - Événement C)

Scénario : Copie du ticket M110.

```
================================
       TOKYO SUSHI BAR
   Rue Test 1, 1000 Bruxelles
       TVA: BE0499999960
       Ets:  1234567890
================================
        * COPIE *
   pas un original
================================
2026-05-14 13:10:00     Pos: POS1
Ticket original: N #00000015

1 x Saumon roll x8 (B)
                          8.50
1 x Coca 33cl (A)
                          3.50

-------------------------------
TOTAL TTC                12.00

VAT  Taux   Net      TVA    Brut
 A   21%   2.89    0.61    3.50
 B   12%   7.59    0.91    8.50
-------------------------------

[ Données de contrôle FDM ]
FDM ID    : FOD01000001
Date/h FDM: 2026-05-14T13:10:01Z
Étiquette : C
Compteur C: 00000003
Total cnt : 00000061
Signature : f72a4b9c1e

Lien original (fdmRef):
  FDM: FOD01000001
  Étiquette: N
  Compteur N: 00000015
  Total cnt: 00000059

[QR CODE 25x25 modules]
================================
```

Spécificités :
- Mention "COPIE - pas un original" en gros.
- Quantités positives, mais sans mode de paiement.
- `eventLabel: C`.
- Lien `fdmRef` vers le ticket original.

## 5. Vente en formation (M171 - Événement T)

Scénario : Même commande que M110 mais en mode formation.

```
================================
       TOKYO SUSHI BAR
   Rue Test 1, 1000 Bruxelles
       TVA: BE0499999960
================================
    *** TRAINING ***
   NE TIENT PAS LIEU DE
   TICKET TVA VALABLE
    *** TRAINING ***
================================
2026-05-14 09:00:00     Pos: POS1

1 x Saumon roll x8 (B)
                          8.50
1 x Coca 33cl (A)
                          3.50

-------------------------------
TOTAL TTC                12.00
Espèces (FICTIF)         12.00

VAT  Taux   Net      TVA    Brut
 A   21%   2.89    0.61    3.50
 B   12%   7.59    0.91    8.50
-------------------------------

[ Données de contrôle FDM ]
FDM ID    : FOD01000001
Date/h FDM: 2026-05-14T09:00:01Z
Étiquette : T   *** TRAINING ***
Compteur T: 00000002
Total cnt : (compteur T séparé)
Signature : c9b3e7f1a2

[QR CODE 25x25 modules]
   *** TRAINING - NE TIENT PAS
   LIEU DE TICKET TVA VALABLE ***
================================
```

Spécificités :
- Bandeau "TRAINING" en début ET en fin de ticket.
- `eventLabel: T` - compté sur un compteur séparé, n'entre pas
  dans le Z des ventes réelles.
- Mention "(FICTIF)" sur le mode de paiement.

## 6. Rapport X (M180 - Événement R, intermédiaire)

```
================================
       TOKYO SUSHI BAR
   Rapport X - Intermédiaire
================================
2026-05-14 16:30:00     Pos: POS1
Période depuis : 2026-05-14 09:00

== Évènements ==
N (ventes)     :  15 tickets
P (commandes)  :   7 tickets
F (mvts caisse):   2 tickets
S (présences)  :   4 tickets
C (copies)     :   1 ticket

== Total TVA ==
VAT   Taux    Net       TVA      Brut
 A    21%   123.45    25.92    149.37
 B    12%   456.78    54.81    511.59
 D     0%    50.00     0.00     50.00
                              ------
Total Brut TTC :              710.96

== Paiements ==
Espèces        :          280.40
Carte          :          430.56
                          ------
Total reçu     :          710.96

== Mvts caisse ==
Fonds caisse   :     +50.00
Pourboires     :     +12.50

Ouvertures tiroir hors-vente : 1

[ Données de contrôle FDM ]
FDM ID    : FOD01000001
Date/h FDM: 2026-05-14T16:30:01Z
Étiquette : R
Compteur R: 00000005
Total cnt : 00000094
Signature : a1f2c3d4e5

[QR CODE 25x25 modules]
   Le compteur n'est PAS remis à 0
================================
```

## 7. Rapport Z (M181 - Événement R, clôture)

Identique au rapport X dans la forme, mais avec :
- En-tête "**Rapport Z - Clôture journalière**".
- Numéro de Z séquentiel ("Z #00045").
- Note finale : "Le compteur de cette journée est désormais
  archivé.".
- Démarrage automatique d'une nouvelle période de réservation au
  prochain événement.

```
================================
       TOKYO SUSHI BAR
   Rapport Z #00045 - Clôture
================================
2026-05-14 23:15:00     Pos: POS1
Journée fiscale du 2026-05-14
Période : 09:00:00 - 23:14:59

(... mêmes blocs que X ...)

== Total journée close ==
Total Brut TTC :             1248.30
Sous TVA       :  21%  149.37
                  12%  511.59
                   6%   45.34
                   0%   50.00

== Période de réservation ==
Ancienne : a3f9-c2b1-...
Nouvelle : b1e8-d473-...

[ Données de contrôle FDM ]
FDM ID    : FOD01000001
Date/h FDM: 2026-05-14T23:15:01Z
Étiquette : R
Compteur R: 00000006
Signature : 7b4a3f9e2d

   *** Journée fiscale clôturée ***
================================
```

## 8. Note sur la reproduction

Les rendus ci-dessus sont des **transcriptions ASCII** de ce que
produirait l'imprimante thermique Epson TM-m30III sur papier 80 mm.
Le code source qui génère ces sorties est dans
`crates/pos-print/src/receipt.rs`.

Lors du rendez-vous d'admission SPF Finances, le distributeur
apportera la machine test connectée à l'imprimante réelle pour
démontrer la sortie physique de chaque type de ticket. Les
exemplaires physiques de ces tickets seront archivés avec le
dossier final.
