# Resultats des tests UC - pili-pos v1.0.0

**Date :** 2026-09-25
**Reference :** §IV du manuel de certification (Procedure de communication GraphQL).

Ce document parcourt les UC obligatoires du Basic Set v1.8 (22 UC
spec) + les 5 UC Reports v1.3 + les cinq situations d'arrondi du manuel (ROUND1 a ROUND5). Est hors périmètre v1.0.0, dans sa section : **M150_signInvoice** (event I, voir
*DECISION_PEPPOL_AP_FR.md*). **M122** (changement de centre de coût, transfert
de table) et **M132** (correction de paiement) sont dans le périmètre et rejoués, voir leur section ; **M112A** n'est pas un parcours
proposé par pili-pos (le remboursement partiel est toujours émis en
ticket séparé, parcours M112B). Pour chaque UC, il fournit :

- Le scenario d'execution (extrait de la fixture officielle SPF Finances).
- La requete et la reponse de reference de la fixture (fichiers Variables.json et *_Response.json publies).
- Le tableau "Champs a verifier" de la fixture.
- La reference au test automatise qui valide la conformite.
- Pour chaque UC rejoue : la requete **construite par pili-pos** et sa comparaison avec la fixture (bloc « Preuve pili-pos »).
- Les tickets correspondants sont transcrits dans *TICKETS_EXEMPLES_FR.md*.

**Note importante :** les fixtures SPF Finances etant la source de verite, les tableaux et JSON de reference ci-dessous correspondent exactement a ceux publies par le SPF dans `docs/20260821_UC_BasicSet_POS_FDM_1.8/` et `docs/20260821_UC_Reports_1.3/`. Les tests `crates/pos-fiscal/tests/uc_fixtures.rs` et `reports_fixtures.rs` verifient que pili-pos lit chaque fixture ; les rejeux applicatifs `crates/pos-engine/tests/spf_e2e_fixtures.rs` et `spf_e2e_report_fixtures.rs` verifient que pili-pos **produit** les memes requetes. Tous tournent a chaque commit (CI).

**Revision des jeux de tests :** Basic Set v1.8 (changelog : M121, reponse sans espace final dans `terminalId`), Advanced Set v1.6 (UC226, UC243, UC4119bis_2, UC4121 corriges, voir *COUVERTURE_ADVANCED_SET_FR.md*) et Reports v1.3 republie ; tous les horodatages des fixtures portent les millisecondes. Les JSON de reference ci-dessous sont ceux de ces dossiers.

**Preuves generees par l'implementation :** chaque UC rejoue se termine par un bloc « Preuve pili-pos (build `cert`) ». Le rejeu construit la commande a partir des produits, quantites et paiements de la fixture, la fait passer par les vraies commandes du binaire de certification (`--no-default-features --features cert`) et compare champ par champ la requete archivee a celle de la fixture : champs metier egaux, champs d'identification (posId, deviceId, numero de ticket, dates, periode de booking) controles sur leur format, ecarts assumes listes et motives. Le bloc reprend la requete **reellement construite par pili-pos** (colonne `event_data_json` de l'archive fiscale) et le resultat de la comparaison ; les fichiers sont dans `evidence/uc/` (`<UC>_event<N>_<label>_request.json`, `_response.json`, `<UC>_check.json`). Regeneration : `scripts/capture_uc_evidence.sh`. Le FDM y est un mock qui rejoue la reponse de la fixture ; une journee d'exploitation est signee par le simulateur en ligne module2 lors de la repetition (*JOURNAL_TESTS_PRE_SOUMISSION_FR.md*).

**Note, terminal de paiement relie :** les resultats ci-dessous sont ceux d'une caisse sans terminal de paiement relie (carte saisie par l'operateur, ligne de paiement `MANUAL`). Les lignes de paiement `AUTOMATIC` d'un terminal relie (type de carte fourni par le terminal, marque dans `provider`, identifiant de transaction dans `reference`) ne figurent pas dans les fixtures du SPF ; elles sont couvertes par les tests du moteur `crates/pos-engine/tests/eft_payments.rs` et `crates/pos-engine/tests/eft_signing.rs`. Aucun resultat enregistre ci-dessous n'est modifie.

---

# Basic Set v1.8 (22 UC)

## M110_signSale

**Mutation invoquee :** `signSale`

**Scenario :**

```
DINING ROOM:
Direct Sale of 
	- 2 Dry Martini (unit price 12 EUR, VAT code A) 
	- 1 Burger of the Chef (unit price 28 EUR, VAT code B).
Payment by CASH (52 EUR)
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1000` | OK |
| `posDateTime` | `2024-07-29T13:00:00.000+02:00` | OK |
| `terminalId` | `TER-2-DIN` | OK |
| `deviceId` | `1631678d-7a85-4ac3-b296-bb4565e873fe` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `84022899837` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `N` | OK |
| `fdmRef.eventCounter` | `15` | OK |
| `fdmRef.totalCounter` | `59` | OK |
| `shortSignature (presence)` | `40 caracteres hex` | OK |
| `vatCalc[] (calcul TVA par taux)` | `2 taux` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1000,
    "posDateTime": "2024-07-29T13:00:00.000+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "1631678d-7a85-4ac3-b296-bb4565e873fe",
    "terminalId": "TER-2-DIN",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "PAPER",
    "employeeId": "84022899837",
    "transaction": {
      "transactionLines": [
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "10006",
            "productName": "Dry Martini",
            "departmentId": "10",
            "departmentName": "Aperitifs",
            "quantity": 2,
            "quantityType": "PIECE",
            "unitPrice": 12,
            "vats": [
              {
                "label": "A",
                "price": 24
              }
            ]
          },
          "lineTotal": 24
        },
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "22001",
            "productName": "Burger of the Chef",
            "departmentId": "22",
            "departmentName": "Main Dishes",
            "quantity": 1,
            "quantityType": "PIECE",
            "unitPrice": 28,
            "vats": [
              {
                "label": "B",
                "price": 28
              }
            ]
          },
          "lineTotal": 28
        }
      ],
      "transactionTotal": 52
    },
    "financials": [
      {
        "id": "1",
        "name": "CONTANT",
        "type": "CASH",
        "inputMethod": "MANUAL",
        "amount": 52,
        "amountType": "PAYMENT",
        "drawer": {
          "id": "1",
          "name": "Drawer 1"
        }
      }
    ]
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signSale": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1000,
      "posDateTime": "2024-07-29T13:00:00.000+02:00",
      "terminalId": "TER-2-DIN",
      "deviceId": "1631678d-7a85-4ac3-b296-bb4565e873fe",
      "vatCalc": [
        {
          "label": "A",
          "rate": 21,
          "taxableAmount": 19.83,
          "vatAmount": 4.17,
          "totalAmount": 24.0,
          "outOfScope": false
        },
        {
          "label": "B",
          "rate": 12,
          "taxableAmount": 25.0,
          "vatAmount": 3.0,
          "totalAmount": 28.0,
          "outOfScope": false
        }
      ],
      "eventOperation": "SALE",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T11:00:08.000Z",
        "eventLabel": "N",
        "eventCounter": 15,
        "totalCounter": 59
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "shortSignature": "ca39a3ee5e6b4b0d3255bfef95601890afd80709",
      "verificationUrl": "https://gs2.be/",
      "warnings": [],
      "informations": [],
      "footer": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m110_sale` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m110_sale`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M110_event1_N_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "d24083f8-a621-4da2-b23f-ba853cc7f0bf",
  "costCenter": {
    "id": "T01",
    "reference": "O1",
    "type": "TABLE"
  },
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "84022899837",
  "estNo": "2000000042",
  "financials": [
    {
      "amount": 52.0,
      "amountType": "PAYMENT",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.535+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": 24.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "10",
          "departmentName": "Aperitifs",
          "productId": "10006",
          "productName": "Dry Martini",
          "quantity": 2.0,
          "quantityType": "PIECE",
          "unitPrice": 12.0,
          "vats": [
            {
              "label": "A",
              "price": 24.0
            }
          ]
        }
      },
      {
        "lineTotal": 28.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "22",
          "departmentName": "Main Dishes",
          "productId": "22001",
          "productName": "Burger of the CHEF",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 28.0,
          "vats": [
            {
              "label": "B",
              "price": 28.0
            }
          ]
        }
      }
    ],
    "transactionTotal": 52.0
  },
  "vatNo": "BE0000000097"
}
```

**Comparaison avec la fixture** (`evidence/uc/M110_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `costCenter.type` | - | `TABLE` | une vente en salle est encaissée depuis sa table et porte le centre de coût TABLE |

<!-- pili-pos-evidence:end -->

## M111_signSale_Refund

**Mutation invoquee :** `signSale`

**Scenario :**

```
Dining room:
Refund ticket (full refund):  
Products on original ticket N 1000/1200:
    - 2 Dry Martini (unit price 12 EUR, VAT code A) 
    - 1 Burger of the Chef (unit price 28 EUR, VAT code B)
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1001` | OK |
| `posDateTime` | `2024-07-29T13:15:00.000+02:00` | OK |
| `terminalId` | `TER-2-DIN` | OK |
| `deviceId` | `1631678d-7a85-4ac3-b296-bb4565e873fe` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `84022899837` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `N` | OK |
| `fdmRef.eventCounter` | `16` | OK |
| `fdmRef.totalCounter` | `60` | OK |
| `shortSignature (presence)` | `40 caracteres hex` | OK |
| `vatCalc[] (calcul TVA par taux)` | `2 taux` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1001,
    "posDateTime": "2024-07-29T13:15:00.000+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "1631678d-7a85-4ac3-b296-bb4565e873fe",
    "terminalId": "TER-2-DIN",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "PAPER",
    "employeeId": "84022899837",
    "fdmRef": {
      "fdmId": "FOD01000001",
      "fdmDateTime": "2024-07-29T11:00:08.000Z",
      "eventLabel": "N",
      "eventCounter": 15,
      "totalCounter": 59
    },
    "transaction": {
      "transactionLines": [
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "10006",
            "productName": "Dry Martini",
            "departmentId": "10",
            "departmentName": "Aperitifs",
            "quantity": -2,
            "quantityType": "PIECE",
            "negQuantityReason": "REFUND",
            "unitPrice": 12,
            "vats": [
              {
                "label": "A",
                "price": -24
              }
            ]
          },
          "lineTotal": -24
        },
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "22001",
            "productName": "Burger of the Chef",
            "departmentId": "22",
            "departmentName": "Main Dishes",
            "quantity": -1,
            "quantityType": "PIECE",
            "negQuantityReason": "REFUND",
            "unitPrice": 28,
            "vats": [
              {
                "label": "B",
                "price": -28
              }
            ]
          },
          "lineTotal": -28
        }
      ],
      "transactionTotal": -52
    },
    "financials": [
      {
        "id": "1",
        "name": "CONTANT",
        "type": "CASH",
        "inputMethod": "MANUAL",
        "amount": -52,
        "amountType": "PAYMENT",
        "drawer": {
          "id": "1",
          "name": "Drawer 1"
        }
      }
    ]
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signSale": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1001,
      "posDateTime": "2024-07-29T13:15:00.000+02:00",
      "terminalId": "TER-2-DIN",
      "deviceId": "1631678d-7a85-4ac3-b296-bb4565e873fe",
      "vatCalc": [
        {
          "label": "A",
          "rate": 21,
          "taxableAmount": -19.83,
          "vatAmount": -4.17,
          "totalAmount": -24.0,
          "outOfScope": false
        },
        {
          "label": "B",
          "rate": 12,
          "taxableAmount": -25.0,
          "vatAmount": -3.0,
          "totalAmount": -28.0,
          "outOfScope": false
        }
      ],
      "eventOperation": "SALE",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T11:15:00.000Z",
        "eventLabel": "N",
        "eventCounter": 16,
        "totalCounter": 60
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "shortSignature": "ca39a3ee5e6b4b0d3255bfef95601890afd80709",
      "verificationUrl": "https://gs2.be/",
      "warnings": [],
      "informations": null
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m111_full_refund` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m111_full_refund`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M111_event2_N_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "faa55f31-2953-40b6-a0b1-9778130fe731",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "84022899837",
  "estNo": "2000000042",
  "fdmRef": {
    "eventCounter": 15,
    "eventLabel": "N",
    "fdmDateTime": "2024-07-29T11:00:08.000Z",
    "fdmId": "FOD01000001",
    "totalCounter": 59
  },
  "financials": [
    {
      "amount": -52.0,
      "amountType": "PAYMENT",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.543+02:00",
  "posFiscalTicketNo": 2,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": -24.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "10",
          "departmentName": "Aperitifs",
          "negQuantityReason": "REFUND",
          "productId": "10006",
          "productName": "Dry Martini",
          "quantity": -2.0,
          "quantityType": "PIECE",
          "unitPrice": 12.0,
          "vats": [
            {
              "label": "A",
              "price": -24.0
            }
          ]
        }
      },
      {
        "lineTotal": -28.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "22",
          "departmentName": "Main Dishes",
          "negQuantityReason": "REFUND",
          "productId": "22001",
          "productName": "Burger of the CHEF",
          "quantity": -1.0,
          "quantityType": "PIECE",
          "unitPrice": 28.0,
          "vats": [
            {
              "label": "B",
              "price": -28.0
            }
          ]
        }
      }
    ],
    "transactionTotal": -52.0
  },
  "vatNo": "BE0000000097"
}
```

Events préalables du scénario, archivés de même : `evidence/uc/M111_event1_N_request.json`. Réponses FDM archivées : fichiers `*_response.json` correspondants.

**Comparaison avec la fixture** (`evidence/uc/M111_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Aucun écart.

<!-- pili-pos-evidence:end -->

## M112A_signSale_RefundPartial

**Scenario global :**
```
scenario M112A: 
ASSUMPTION: the cash register supports a partial refund in combination with a new sale

1. Original Ticket: direct sale of  
    * 2 dry Martini (unit price 12 EUR, VAT code A)
    * 1 burger of the chef (unit price 28 EUR, VAT code B)

2. Partial refund:
    * refund of 1 Dry Martini (unit price 12 EUR, VAT code A)
    * sale of 1 Coca Cola (unit price 3.04 EUR, VAT code A) & 1 crème brûlée (unit price 10 EUR, VAT code B)
    Payment by cash.
```

**Note de déclaration :** le POS pili-pos ne propose pas le pattern M112A
(remboursement partiel combiné à une nouvelle vente sur un même ticket).
Le dossier déclare le pattern **M112B** (remboursement en événement séparé,
chaîné à la vente d'origine via `fdmReference`) - voir la section
M112B ci-dessous. Les fixtures M112A sont validées en *parsing* uniquement.

### M112A_1_OriginalSale

**Mutation invoquee :** `inconnue`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `6587` | OK |
| `posDateTime` | `2024-10-07T13:00:00.000+02:00` | OK |
| `terminalId` | `TER-2-DIN` | OK |
| `deviceId` | `1631678d-7a85-4ac3-b296-bb4565e873fe` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-10-07` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `84022899837` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `N` | OK |
| `fdmRef.eventCounter` | `81958` | OK |
| `fdmRef.totalCounter` | `83876` | OK |
| `shortSignature (presence)` | `40 caracteres hex` | OK |
| `vatCalc[] (calcul TVA par taux)` | `2 taux` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 6587,
    "posDateTime": "2024-10-07T13:00:00.000+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "1631678d-7a85-4ac3-b296-bb4565e873fe",
    "terminalId": "TER-2-DIN",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-10-07",
    "ticketMedium": "PAPER",
    "employeeId": "84022899837",
    "transaction": {
      "transactionLines": [
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "10006",
            "productName": "Dry Martini",
            "departmentId": "10",
            "departmentName": "Aperitifs",
            "quantity": 2,
            "quantityType": "PIECE",
            "unitPrice": 12,
            "vats": [
              {
                "label": "A",
                "price": 24
              }
            ]
          },
          "lineTotal": 24
        },
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "22001",
            "productName": "Burger of the Chef",
            "departmentId": "22",
            "departmentName": "Main Dishes",
            "quantity": 1,
            "quantityType": "PIECE",
            "unitPrice": 28,
            "vats": [
              {
                "label": "B",
                "price": 28
              }
            ]
          },
          "lineTotal": 28
        }
      ],
      "transactionTotal": 52
    },
    "financials": [
      {
        "id": "1",
        "name": "CONTANT",
        "type": "CASH",
        "inputMethod": "MANUAL",
        "amount": 52,
        "amountType": "PAYMENT",
        "drawer": {
          "id": "1",
          "name": "Drawer 1"
        }
      }
    ]
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signSale": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 6587,
      "posDateTime": "2024-10-07T13:00:00.000+02:00",
      "terminalId": "TER-2-DIN",
      "deviceId": "1631678d-7a85-4ac3-b296-bb4565e873fe",
      "eventOperation": "SALE",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-10-07T11:00:41.000Z",
        "eventLabel": "N",
        "eventCounter": 81958,
        "totalCounter": 83876
      },
      "fdmSwVersion": "1.2.0",
      "digitalSignature": "tOKZG5v8RUeVPWu022lBIQf4gCVzAz9NlGr3JE3KyPj/e9K6fyHXRbZexqvgcSUfOlkrOScCpkmAesT9DQx4qA==",
      "shortSignature": "b6f7940dcc4694a14681471de6415defec44e02d",
      "verificationUrl": "https://gs2.be/xJYcHkRNO5UJS3hnXbo",
      "vatCalc": [
        {
          "label": "A",
          "rate": 21,
          "taxableAmount": 19.83,
          "vatAmount": 4.17,
          "totalAmount": 24,
          "outOfScope": false
        },
        {
          "label": "B",
          "rate": 12,
          "taxableAmount": 25.0,
          "vatAmount": 3.0,
          "totalAmount": 28,
          "outOfScope": false
        }
      ],
      "bufferCapacityUsed": 14.83,
      "warnings": null,
      "informations": null,
      "footer": []
    }
  }
}
```

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing uniquement - pattern non déclaré, voir M112B).

### M112A_2_PartialRefund

**Mutation invoquee :** `signSale`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `6603` | OK |
| `posDateTime` | `2024-10-07T13:48:00.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-10-07` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `84022899837` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `N` | OK |
| `fdmRef.eventCounter` | `81959` | OK |
| `fdmRef.totalCounter` | `83877` | OK |
| `shortSignature (presence)` | `40 caracteres hex` | OK |
| `vatCalc[] (calcul TVA par taux)` | `2 taux` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 6603,
    "posDateTime": "2024-10-07T13:48:00.000+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "terminalId": "TER-1-BAR",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-10-07",
    "ticketMedium": "PAPER",
    "employeeId": "84022899837",
    "transaction": {
      "transactionLines": [
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "10006",
            "productName": "Dry Martini",
            "departmentId": "10",
            "departmentName": "Aperitifs",
            "quantity": -1,
            "quantityType": "PIECE",
            "negQuantityReason": "REFUND",
            "unitPrice": 12,
            "vats": [
              {
                "label": "A",
                "price": -12
              }
            ]
          },
          "lineTotal": -12
        },
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "12009",
            "productName": "Coca Cola",
            "departmentId": "12",
            "departmentName": "Soft Drinks",
            "quantity": 1,
            "quantityType": "PIECE",
            "unitPrice": 3.04,
            "vats": [
              {
                "label": "A",
                "price": 3.04
              }
            ]
          },
          "lineTotal": 3.04
        },
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "25008",
            "productName": "crème brûlée",
            "departmentId": "25",
            "departmentName": "Desserts",
            "quantity": 1,
            "quantityType": "PIECE",
            "unitPrice": 10,
            "vats": [
              {
                "label": "B",
                "price": 10
              }
            ]
          },
          "lineTotal": 10
        }
      ],
      "transactionTotal": 1.04
    },
    "financials": [
      {
        "id": "1",
        "name": "CONTANT",
        "type": "CASH",
        "inputMethod": "MANUAL",
        "amount": 1.04,
        "amountType": "PAYMENT",
        "drawer": {
          "id": "1",
          "name": "Drawer 1"
        }
      },
      {
        "id": "1",
        "name": "CONTANT",
        "type": "CASH",
        "inputMethod": "MANUAL",
        "amount": 0.01,
        "amountType": "ROUNDING",
        "drawer": {
          "id": "1",
          "name": "Drawer 1"
        }
      }
    ]
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signSale": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 6603,
      "posDateTime": "2024-10-07T13:48:00.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "SALE",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-10-07T11:48:41.000Z",
        "eventLabel": "N",
        "eventCounter": 81959,
        "totalCounter": 83877
      },
      "fdmSwVersion": "1.2.0",
      "digitalSignature": "9UrNym6R2ky3eijjqBxtnuZpRlN6FJtGtJ+W9pcNVDkiS06blUMLS7yfGvQfBWSnrL+ZZSHkB0e7+sDWge5//A==",
      "shortSignature": "e0ed61dd6d3c46bf448c1991b49a3d39ca98ee15",
      "verificationUrl": "https://gs2.be/lLApSfduWYcqKx9QQ80",
      "vatCalc": [
        {
          "label": "A",
          "rate": 21,
          "taxableAmount": -7.4,
          "vatAmount": -1.56,
          "totalAmount": -8.96,
          "outOfScope": false
        },
        {
          "label": "B",
          "rate": 12,
          "taxableAmount": 8.93,
          "vatAmount": 1.07,
          "totalAmount": 10,
          "outOfScope": false
        }
      ],
      "bufferCapacityUsed": 14.83,
      "warnings": null,
      "informations": null,
      "footer": []
    }
  }
}
```

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing uniquement - pattern non déclaré, voir M112B).


## M112B_signSale_RefundPartial

**Scenario global :**
```
1. Original Ticket: direct sale of  
    * 2 dry Martini (unit price 12 EUR, VAT code A)
    * 1 burger of the chef (unit price 28 EUR, VAT code B)

2. Partial refund:
    * refund of 1 Dry Martini (unit price 12 EUR, VAT code A)
    * sale of 1 Coca Cola (unit price 3.04 EUR, VAT code A) & 1 crème brûlée (unit price 10 EUR, VAT code B)
    Payment by cash.
```

**Note de déclaration :** c'est CE pattern (remboursement partiel en
événement séparé, chaîné à la vente d'origine via `fdmReference`) que le
POS implémente et déclare. Couverture e2e applicative :
`m112b_partial_refund_as_a_separate_ticket` dans
`crates/pos-engine/tests/spf_e2e_fixtures.rs` (rejeu de la fixture, build
`cert`) et `m112b_partial_refund_is_separate_event_chained_to_original`
dans `crates/pos-engine/tests/spf_e2e_basic.rs` ; preuves générées dans
`evidence/uc/M112B_*.json` (voir « Preuve pili-pos » en fin de
section).

### M112B_1_OriginalSale

**Mutation invoquee :** `inconnue`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `6587` | OK |
| `posDateTime` | `2024-10-07T13:00:00.000+02:00` | OK |
| `terminalId` | `TER-2-DIN` | OK |
| `deviceId` | `1631678d-7a85-4ac3-b296-bb4565e873fe` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-10-07` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `84022899837` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `N` | OK |
| `fdmRef.eventCounter` | `81958` | OK |
| `fdmRef.totalCounter` | `83876` | OK |
| `shortSignature (presence)` | `40 caracteres hex` | OK |
| `vatCalc[] (calcul TVA par taux)` | `2 taux` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 6587,
    "posDateTime": "2024-10-07T13:00:00.000+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "1631678d-7a85-4ac3-b296-bb4565e873fe",
    "terminalId": "TER-2-DIN",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-10-07",
    "ticketMedium": "PAPER",
    "employeeId": "84022899837",
    "transaction": {
      "transactionLines": [
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "10006",
            "productName": "Dry Martini",
            "departmentId": "10",
            "departmentName": "Aperitifs",
            "quantity": 2,
            "quantityType": "PIECE",
            "unitPrice": 12,
            "vats": [
              {
                "label": "A",
                "price": 24
              }
            ]
          },
          "lineTotal": 24
        },
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "22001",
            "productName": "Burger of the Chef",
            "departmentId": "22",
            "departmentName": "Main Dishes",
            "quantity": 1,
            "quantityType": "PIECE",
            "unitPrice": 28,
            "vats": [
              {
                "label": "B",
                "price": 28
              }
            ]
          },
          "lineTotal": 28
        }
      ],
      "transactionTotal": 52
    },
    "financials": [
      {
        "id": "1",
        "name": "CONTANT",
        "type": "CASH",
        "inputMethod": "MANUAL",
        "amount": 52,
        "amountType": "PAYMENT",
        "drawer": {
          "id": "1",
          "name": "Drawer 1"
        }
      }
    ]
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signSale": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 6587,
      "posDateTime": "2024-10-07T13:00:00.000+02:00",
      "terminalId": "TER-2-DIN",
      "deviceId": "1631678d-7a85-4ac3-b296-bb4565e873fe",
      "eventOperation": "SALE",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-10-07T11:00:41.000Z",
        "eventLabel": "N",
        "eventCounter": 81958,
        "totalCounter": 83876
      },
      "fdmSwVersion": "1.2.0",
      "digitalSignature": "tOKZG5v8RUeVPWu022lBIQf4gCVzAz9NlGr3JE3KyPj/e9K6fyHXRbZexqvgcSUfOlkrOScCpkmAesT9DQx4qA==",
      "shortSignature": "b6f7940dcc4694a14681471de6415defec44e02d",
      "verificationUrl": "https://gs2.be/xJYcHkRNO5UJS3hnXbo",
      "vatCalc": [
        {
          "label": "A",
          "rate": 21,
          "taxableAmount": 19.83,
          "vatAmount": 4.17,
          "totalAmount": 24,
          "outOfScope": false
        },
        {
          "label": "B",
          "rate": 12,
          "taxableAmount": 25.0,
          "vatAmount": 3.0,
          "totalAmount": 28,
          "outOfScope": false
        }
      ],
      "bufferCapacityUsed": 14.83,
      "warnings": [],
      "informations": null,
      "footer": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m112b_partial_refund_as_a_separate_ticket` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

### M112B_2_PartialRefund

**Mutation invoquee :** `signSale`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `6603` | OK |
| `posDateTime` | `2024-10-07T13:48:00.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-10-07` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `84022899837` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `N` | OK |
| `fdmRef.eventCounter` | `76` | OK |
| `fdmRef.totalCounter` | `83885` | OK |
| `shortSignature (presence)` | `40 caracteres hex` | OK |
| `vatCalc[] (calcul TVA par taux)` | `1 taux` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 6603,
    "posDateTime": "2024-10-07T13:48:00.000+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "terminalId": "TER-1-BAR",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-10-07",
    "ticketMedium": "PAPER",
    "employeeId": "84022899837",
    "transaction": {
      "transactionLines": [
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "10006",
            "productName": "Dry Martini",
            "departmentId": "10",
            "departmentName": "Aperitifs",
            "quantity": -1,
            "quantityType": "PIECE",
            "negQuantityReason": "REFUND",
            "unitPrice": 12,
            "vats": [
              {
                "label": "A",
                "price": -12
              }
            ]
          },
          "lineTotal": -12
        }
      ],
      "transactionTotal": -12
    },
    "financials": [
      {
        "id": "1",
        "name": "CONTANT",
        "type": "CASH",
        "inputMethod": "MANUAL",
        "amount": -12,
        "amountType": "PAYMENT",
        "drawer": {
          "id": "1",
          "name": "Drawer 1"
        }
      }
    ]
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signSale": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 6603,
      "posDateTime": "2024-10-07T13:48:00.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "SALE",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-10-07T11:48:27.000Z",
        "eventLabel": "N",
        "eventCounter": 76,
        "totalCounter": 83885
      },
      "fdmSwVersion": "1.2.0",
      "digitalSignature": "lRvExSh9UUeyZRaCZyK54Tz+WiFHb25PvXldpSsYEPOC9JN2cFEXSr9/vOP0LKCVqFekl4IPXkm+CYLKM2ONTg==",
      "shortSignature": "84b433a643fc935474ee744678dcfe1042fbc3c2",
      "verificationUrl": "https://gs2.be/vKRnCKepX2Yy8WiDjvU",
      "vatCalc": [
        {
          "label": "A",
          "rate": 21,
          "taxableAmount": -9.92,
          "vatAmount": -2.08,
          "totalAmount": -12,
          "outOfScope": false
        }
      ],
      "bufferCapacityUsed": 14.83,
      "warnings": null,
      "informations": null,
      "footer": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m112b_partial_refund_as_a_separate_ticket` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m112b_partial_refund_as_a_separate_ticket`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M112B_event2_N_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "3a57f5c0-2941-49ad-9cf8-fe56e72e06ce",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "84022899837",
  "estNo": "2000000042",
  "fdmRef": {
    "eventCounter": 81958,
    "eventLabel": "N",
    "fdmDateTime": "2024-10-07T11:00:41.000Z",
    "fdmId": "FOD01000001",
    "totalCounter": 83876
  },
  "financials": [
    {
      "amount": -12.0,
      "amountType": "PAYMENT",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.542+02:00",
  "posFiscalTicketNo": 2,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": -12.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "10",
          "departmentName": "Aperitifs",
          "negQuantityReason": "REFUND",
          "productId": "10006",
          "productName": "Dry Martini",
          "quantity": -1.0,
          "quantityType": "PIECE",
          "unitPrice": 12.0,
          "vats": [
            {
              "label": "A",
              "price": -12.0
            }
          ]
        }
      }
    ],
    "transactionTotal": -12.0
  },
  "vatNo": "BE0000000097"
}
```

Events préalables du scénario, archivés de même : `evidence/uc/M112B_event1_N_request.json`. Réponses FDM archivées : fichiers `*_response.json` correspondants.

**Comparaison avec la fixture** (`evidence/uc/M112B_1_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `costCenter.type` | - | `TABLE` | une vente en salle est encaissée depuis sa table et porte le centre de coût TABLE |

**Comparaison avec la fixture** (`evidence/uc/M112B_2_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `fdmRef` | - | `{"eventCounter": 81958, "eventLabel": "N", "fdmDateTime": "2024-10-07T11:00:41.000Z", "fdmId": "FOD01000001", "totalCounter": 83876}` | pili-pos référence le ticket remboursé sur chaque remboursement (*FLUX_REMBOURSEMENT_FR.md*) ; le remboursement partiel de la fixture n'en porte pas |

<!-- pili-pos-evidence:end -->

## M121_signOrder

**Mutation invoquee :** `signOrder`

**Scenario :**

```
BAR: Client orders:
    2 Dry Martini (unit price 12 EUR, VAT code A)
    4 Tapas variation (unit price 3,34 EUR, VAT code B)
on table 1.
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1003` | OK |
| `posDateTime` | `2024-07-29T13:57:03.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `P` | OK |
| `fdmRef.eventCounter` | `22` | OK |
| `fdmRef.totalCounter` | `53` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1003,
    "posDateTime": "2024-07-29T13:57:03.000+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "terminalId": "TER-1-BAR",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "PAPER",
    "employeeId": "75061189702",
    "costCenter": {
      "id": "T1",
      "type": "TABLE",
      "reference": "O158"
    },
    "transaction": {
      "transactionLines": [
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "10006",
            "productName": "Dry Martini",
            "departmentId": "10",
            "departmentName": "Aperitifs",
            "quantity": 2,
            "quantityType": "PIECE",
            "unitPrice": 12,
            "vats": [
              {
                "label": "A",
                "price": 24
              }
            ]
          },
          "lineTotal": 24
        },
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "28007",
            "productName": "Tapas variation",
            "departmentId": "28",
            "departmentName": "Tapas",
            "quantity": 4,
            "quantityType": "PIECE",
            "unitPrice": 3.34,
            "vats": [
              {
                "label": "B",
                "price": 13.36
              }
            ]
          },
          "lineTotal": 13.36
        }
      ],
      "transactionTotal": 37.36
    }
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signOrder": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1003,
      "posDateTime": "2024-07-29T13:57:03.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "ORDER",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T11:57:12.000Z",
        "eventLabel": "P",
        "eventCounter": 22,
        "totalCounter": 53
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m121_table_order` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m121_table_order`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M121_event1_P_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "1c9ba09a-506e-4ca6-8c40-c9dc6e34fe48",
  "costCenter": {
    "id": "T01",
    "reference": "O1",
    "type": "TABLE"
  },
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.532+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "NONE",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": 24.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "10",
          "departmentName": "Aperitifs",
          "productId": "10006",
          "productName": "Dry Martini",
          "quantity": 2.0,
          "quantityType": "PIECE",
          "unitPrice": 12.0,
          "vats": [
            {
              "label": "A",
              "price": 24.0
            }
          ]
        }
      },
      {
        "lineTotal": 13.36,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "28",
          "departmentName": "Tapas",
          "productId": "28007",
          "productName": "Tapas variation",
          "quantity": 4.0,
          "quantityType": "PIECE",
          "unitPrice": 3.34,
          "vats": [
            {
              "label": "B",
              "price": 13.36
            }
          ]
        }
      }
    ],
    "transactionTotal": 37.36
  },
  "vatNo": "BE0000000097"
}
```

**Comparaison avec la fixture** (`evidence/uc/M121_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `ticketMedium` | `PAPER` | `NONE` | les commandes ne sont pas imprimées en v1.0.0 (pas de ticket cuisine) : le support est NONE |

<!-- pili-pos-evidence:end -->

## M122_signCostCenterChange

Le changement de centre de cout est disponible pour l'**attribution** : transfert d'une commande ouverte vers une table libre (*REPONSES_MANUEL_CERT_FR.md*, tableau des fonctionnalites). La division et la fusion de tables restent hors perimetre. Les valeurs ci-dessous sont celles de la fixture ; le rejeu applicatif la compare a la requete construite par pili-pos.

**Mutation invoquee :** `signCostCenterChange`

**Scenario :**

```
Order on T1 (BAR):
    1 Burger of the Chef (unit price 28 EUR, VAT code B)
    1 Dry Martini(unit price 12 EUR, VAT code A)    
is moved to T5 (Dining Room)
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1004` | OK |
| `posDateTime` | `2024-07-29T14:21:23.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `NONE` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `P` | OK |
| `fdmRef.eventCounter` | `23` | OK |
| `fdmRef.totalCounter` | `54` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1004,
    "posDateTime": "2024-07-29T14:21:23.000+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "terminalId": "TER-1-BAR",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "NONE",
    "employeeId": "75061189702",
    "transfer": {
      "from": [
        {
          "costCenter": {
            "id": "T1",
            "type": "TABLE",
            "reference": "O158"
          },
          "transaction": {
            "transactionLines": [
              {
                "lineType": "SINGLE_PRODUCT",
                "mainProduct": {
                  "productId": "22001",
                  "productName": "Burger of the Chef",
                  "departmentId": "22",
                  "departmentName": "Main Dishes",
                  "quantity": -1,
                  "negQuantityReason": "COST_CENTER_CHANGE",
                  "quantityType": "PIECE",
                  "unitPrice": 28,
                  "vats": [
                    {
                      "label": "B",
                      "price": -28
                    }
                  ]
                },
                "lineTotal": -28
              },
              {
                "lineType": "SINGLE_PRODUCT",
                "mainProduct": {
                  "productId": "10006",
                  "productName": "Dry Martini",
                  "departmentId": "10",
                  "departmentName": "Aperitifs",
                  "quantity": -1,
                  "negQuantityReason": "COST_CENTER_CHANGE",
                  "quantityType": "PIECE",
                  "unitPrice": 12,
                  "vats": [
                    {
                      "label": "A",
                      "price": -12
                    }
                  ]
                },
                "lineTotal": -12
              }
            ],
            "transactionTotal": -40
          }
        }
      ],
      "to": [
        {
          "costCenter": {
            "id": "T5",
            "type": "TABLE",
            "reference": "O158"
          },
          "transaction": {
            "transactionLines": [
              {
                "lineType": "SINGLE_PRODUCT",
                "mainProduct": {
                  "productId": "22001",
                  "productName": "Burger of the Chef",
                  "departmentId": "22",
                  "departmentName": "Main Dishes",
                  "quantity": 1,
                  "quantityType": "PIECE",
                  "unitPrice": 28,
                  "vats": [
                    {
                      "label": "B",
                      "price": 28
                    }
                  ]
                },
                "lineTotal": 28
              },
              {
                "lineType": "SINGLE_PRODUCT",
                "mainProduct": {
                  "productId": "10006",
                  "productName": "Dry Martini",
                  "departmentId": "10",
                  "departmentName": "Aperitifs",
                  "quantity": 1,
                  "quantityType": "PIECE",
                  "unitPrice": 12,
                  "vats": [
                    {
                      "label": "A",
                      "price": 12
                    }
                  ]
                },
                "lineTotal": 12
              }
            ],
            "transactionTotal": 40
          }
        }
      ]
    }
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signCostCenterChange": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1004,
      "posDateTime": "2024-07-29T14:21:23.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "COST_CENTER_CHANGE",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T12:21:25.000Z",
        "eventLabel": "P",
        "eventCounter": 23,
        "totalCounter": 54
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m122_cost_center_change` (rejeu applicatif, build `cert`, ecart nul ; la commande suivante sur T05 garde la meme reference).


## M123_signPrebill

**Mutation invoquee :** `signPreBill`

**Scenario :**

```
Prebill for the order:
    2 Dry Martini (unit price 12 EUR, VAT code A)
    4 Tapas variation (unit price 3,34 EUR, VAT code B)
on table 1 (BAR)  is printed
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1005` | OK |
| `posDateTime` | `2024-07-29T14:48:03.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `P` | OK |
| `fdmRef.eventCounter` | `24` | OK |
| `fdmRef.totalCounter` | `55` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1005,
    "posDateTime": "2024-07-29T14:48:03.000+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "terminalId": "TER-1-BAR",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "PAPER",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "employeeId": "75061189702",
    "costCenter": {
      "id": "T1",
      "type": "TABLE",
      "reference": "O158"
    },
    "transaction": {
      "transactionLines": [
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "10006",
            "productName": "Dry Martini",
            "departmentId": "10",
            "departmentName": "Aperitifs",
            "quantity": 2,
            "quantityType": "PIECE",
            "unitPrice": 12,
            "vats": [
              {
                "label": "A",
                "price": 24
              }
            ]
          },
          "lineTotal": 24
        },
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "28007",
            "productName": "Tapas variation",
            "departmentId": "28",
            "departmentName": "Tapas",
            "quantity": 4,
            "quantityType": "PIECE",
            "unitPrice": 3.34,
            "vats": [
              {
                "label": "B",
                "price": 13.36
              }
            ]
          },
          "lineTotal": 13.36
        }
      ],
      "transactionTotal": 37.36
    }
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signPreBill": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1005,
      "posDateTime": "2024-07-29T14:48:03.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "PRE_BILL",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T12:48:12.000Z",
        "eventLabel": "P",
        "eventCounter": 24,
        "totalCounter": 55
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m123_pre_bill` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m123_pre_bill`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M123_event2_P_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "897c862a-5288-414a-bb26-ad318c142b04",
  "costCenter": {
    "id": "T01",
    "reference": "O1",
    "type": "TABLE"
  },
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.539+02:00",
  "posFiscalTicketNo": 2,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": 24.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "10",
          "departmentName": "Aperitifs",
          "productId": "10006",
          "productName": "Dry Martini",
          "quantity": 2.0,
          "quantityType": "PIECE",
          "unitPrice": 12.0,
          "vats": [
            {
              "label": "A",
              "price": 24.0
            }
          ]
        }
      },
      {
        "lineTotal": 13.36,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "28",
          "departmentName": "Tapas",
          "productId": "28007",
          "productName": "Tapas variation",
          "quantity": 4.0,
          "quantityType": "PIECE",
          "unitPrice": 3.34,
          "vats": [
            {
              "label": "B",
              "price": 13.36
            }
          ]
        }
      }
    ],
    "transactionTotal": 37.36
  },
  "vatNo": "BE0000000097"
}
```

Events préalables du scénario, archivés de même : `evidence/uc/M123_event1_P_request.json`. Réponses FDM archivées : fichiers `*_response.json` correspondants.

**Comparaison avec la fixture** (`evidence/uc/M123_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Aucun écart.

<!-- pili-pos-evidence:end -->

## M130_signMoneyInOut

**Mutation invoquee :** `signMoneyInOut`

**Scenario :**

```
100 EUR cash is placed in drawer1
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1006` | OK |
| `posDateTime` | `2024-07-29T15:07:09.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `NONE` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `F` | OK |
| `fdmRef.eventCounter` | `6` | OK |
| `fdmRef.totalCounter` | `56` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1006,
    "posDateTime": "2024-07-29T15:07:09.000+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "NONE",
    "employeeId": "75061189702",
    "financials": [
      {
        "id": "1",
        "name": "CASH",
        "type": "CASH",
        "amount": 100,
        "inputMethod": "MANUAL",
        "amountType": "MONEY_IN_OUT",
        "drawer": {
          "id": "1",
          "name": "Drawer 1"
        }
      }
    ]
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signMoneyInOut": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1006,
      "posDateTime": "2024-07-29T15:07:09.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "MONEY_IN_OUT",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T13:07:12.000Z",
        "eventLabel": "F",
        "eventCounter": 6,
        "totalCounter": 56
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m130_money_in` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m130_money_in`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M130_event1_F_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "2e9a4ea3-343b-49de-a56a-9b58b24ff8dd",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "financials": [
    {
      "amount": 100.0,
      "amountType": "MONEY_IN_OUT",
      "id": "MOV1",
      "inputMethod": "MANUAL",
      "name": "Fond de caisse",
      "type": "CASH"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.529+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "NONE",
  "vatNo": "BE0000000097"
}
```

**Comparaison avec la fixture** (`evidence/uc/M130_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Aucun écart.

<!-- pili-pos-evidence:end -->

## M131_signDrawerOpen

**Mutation invoquee :** `signDrawerOpen`

**Scenario :**

```
Drawer 1 is opened without any action
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1007` | OK |
| `posDateTime` | `2024-07-29T15:12:04.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `NONE` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `F` | OK |
| `fdmRef.eventCounter` | `7` | OK |
| `fdmRef.totalCounter` | `57` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1007,
    "posDateTime": "2024-07-29T15:12:04.000+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "NONE",
    "employeeId": "75061189702",
    "drawer": {
      "id": "1",
      "name": "Drawer 1"
    }
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signDrawerOpen": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1007,
      "posDateTime": "2024-07-29T15:12:04.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "DRAWER_OPEN",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T13:12:05.000Z",
        "eventLabel": "F",
        "eventCounter": 7,
        "totalCounter": 57
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m131_drawer_open` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m131_drawer_open`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M131_event1_F_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "8c25cfc2-7326-49c4-8cb8-4dd825fef8cc",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "drawer": {
    "id": "DRAWER1",
    "name": "Tiroir principal"
  },
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.529+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "NONE",
  "vatNo": "BE0000000097"
}
```

**Comparaison avec la fixture** (`evidence/uc/M131_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Aucun écart.

<!-- pili-pos-evidence:end -->

## M132_signPaymentCorrection

La correction de paiement est disponible pour une ligne de paiement saisie a la main d'une vente de la journee en cours (*REPONSES_MANUEL_CERT_FR.md*, tableau des fonctionnalites). Les valeurs ci-dessous sont celles de la fixture ; le rejeu applicatif la compare a la requete construite par pili-pos. Un ecart declare : la fixture corrige vers une carte de credit (`CARD_CREDIT`), alors que, sans terminal relie, une carte saisie a la main est toujours `CARD_DEBIT` (*REPONSES_MANUEL_CERT_FR.md* §III.F). Deux differences ne sont pas comparees (la comparaison porte sur le type, le montant et le type de montant de chaque ligne) : la nouvelle ligne de carte part en saisie manuelle (`inputMethod` MANUAL, la fixture a AUTOMATIC) et la ligne d'especes n'indique pas de tiroir (champ optionnel, la fixture nomme le tiroir 1).

**Mutation invoquee :** `signPaymentCorrection`

**Scenario :**

```
PaymentCorrection
 
Payment of N14/50 (total to be paid 52 EUR) was made by CARD_CREDIT (Mastercard via EFT) and not in cash (as originally registered)
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1008` | OK |
| `posDateTime` | `2024-07-29T15:22:06.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `NONE` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `F` | OK |
| `fdmRef.eventCounter` | `8` | OK |
| `fdmRef.totalCounter` | `58` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1008,
    "posDateTime": "2024-07-29T15:22:06.000+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "NONE",
    "employeeId": "75061189702",
    "fdmRef": {
      "fdmId": "FOD01000001",
      "fdmDateTime": "2024-07-29T11:35:09.000Z",
      "eventLabel": "N",
      "eventCounter": 14,
      "totalCounter": 50
    },
    "financials": [
      {
        "id": "1",
        "name": "CONTANT",
        "type": "CASH",
        "inputMethod": "MANUAL",
        "amount": -52,
        "amountType": "PAYMENT",
        "drawer": {
          "id": "1",
          "name": "Drawer 1"
        }
      },
      {
        "id": "2",
        "name": "MASTERCARD",
        "type": "CARD_CREDIT",
        "inputMethod": "AUTOMATIC",
        "amount": 52,
        "amountType": "PAYMENT"
      }
    ]
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signPaymentCorrection": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1008,
      "posDateTime": "2024-07-29T15:22:06.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "PAYMENT_CORRECTION",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T13:22:06.000Z",
        "eventLabel": "F",
        "eventCounter": 8,
        "totalCounter": 58
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Ecarts declares du rejeu M132.** La comparaison automatique ne porte que sur le type, le montant et le type de montant (`amountType`) de chaque ligne de paiement. Un ecart est declare : la nouvelle ligne de carte est `CARD_DEBIT` (la fixture a `CARD_CREDIT`), car sans terminal de paiement relie l'operateur ne peut pas distinguer debit et credit. Deux differences ne sont pas comparees : la nouvelle ligne de carte part en saisie manuelle (`inputMethod` `MANUAL`, la fixture a `AUTOMATIC`, puisqu'elle suppose un terminal) et la ligne d'especes n'indique pas de tiroir (champ optionnel, la fixture nomme le tiroir 1).

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m132_payment_correction` (rejeu applicatif, build `cert`, un seul ecart : le type de la carte, `CARD_DEBIT` au lieu de `CARD_CREDIT`).


## M133_signMoneyInOut_DrawerDeclaration

**Mutation invoquee :** `signMoneyInOut`

**Scenario :**

```
Declaration of 1000 EUR cash in drawer 1
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1009` | OK |
| `posDateTime` | `2024-07-29T15:39:39.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `NONE` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `F` | OK |
| `fdmRef.eventCounter` | `9` | OK |
| `fdmRef.totalCounter` | `59` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1009,
    "posDateTime": "2024-07-29T15:39:39.000+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "NONE",
    "employeeId": "75061189702",
    "financials": [
      {
        "id": "1",
        "name": "CONTANT",
        "type": "CASH",
        "amount": 1000,
        "inputMethod": "MANUAL",
        "amountType": "DRAWER_DECLARATION",
        "drawer": {
          "id": "1",
          "name": "Drawer 1"
        }
      }
    ]
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signMoneyInOut": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1009,
      "posDateTime": "2024-07-29T15:39:39.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "MONEY_IN_OUT",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T13:39:45.000Z",
        "eventLabel": "F",
        "eventCounter": 9,
        "totalCounter": 59
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture)  et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m133_drawer_declaration` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m133_drawer_declaration`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M133_event1_F_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "f05ebc20-f531-4814-a8fa-36410a193df0",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "financials": [
    {
      "amount": 1000.0,
      "amountType": "DRAWER_DECLARATION",
      "id": "MOV1",
      "inputMethod": "MANUAL",
      "name": "Declaration tiroir",
      "type": "CASH"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.828+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "NONE",
  "vatNo": "BE0000000097"
}
```

**Comparaison avec la fixture** (`evidence/uc/M133_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Aucun écart.

<!-- pili-pos-evidence:end -->

## M140_signWorkIn

**Mutation invoquee :** `signWorkIn`

**Scenario :**

```
Registration of "Work In" of employee (employeeId 80113078968)
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234568` | OK |
| `posFiscalTicketNo` | `1010` | OK |
| `posDateTime` | `2024-07-29T15:50:39.000+02:00` | OK |
| `terminalId` | `POS-2-REC` | OK |
| `deviceId` | `80:1A:A6:75:66:FC` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `NONE` | OK |
| `employeeId` | `80113078968` | OK |
| `fdmRef.fdmId` | `FOD01000002` | OK |
| `fdmRef.eventLabel` | `S` | OK |
| `fdmRef.eventCounter` | `7` | OK |
| `fdmRef.totalCounter` | `30` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234568",
    "posFiscalTicketNo": 1010,
    "posDateTime": "2024-07-29T15:50:39.000+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "POS-2-REC",
    "deviceId": "80:1A:A6:75:66:FC",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "NONE",
    "employeeId": "80113078968"
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signWorkIn": {
      "posId": "CPOS0031234568",
      "posFiscalTicketNo": 1010,
      "posDateTime": "2024-07-29T15:50:39.000+02:00",
      "terminalId": "POS-2-REC",
      "deviceId": "80:1A:A6:75:66:FC",
      "eventOperation": "WORK_IN",
      "fdmRef": {
        "fdmId": "FOD01000002",
        "fdmDateTime": "2024-07-29T13:50:39.000Z",
        "eventLabel": "S",
        "eventCounter": 7,
        "totalCounter": 30
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m140_work_in` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m140_work_in`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M140_event1_S_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "94f666d6-cb6a-4136-83eb-8561bf1e58b6",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "80113078968",
  "estNo": "2000000042",
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.828+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "NONE",
  "vatNo": "BE0000000097"
}
```

**Comparaison avec la fixture** (`evidence/uc/M140_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Aucun écart.

<!-- pili-pos-evidence:end -->

## M141_signWorkOut

**Mutation invoquee :** `signWorkOut`

**Scenario :**

```
Registration of "Work Out" of employee (employeeId 84022899837)
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1011` | OK |
| `posDateTime` | `2024-07-29T15:59:39.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `NONE` | OK |
| `employeeId` | `84022899837` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `S` | OK |
| `fdmRef.eventCounter` | `8` | OK |
| `fdmRef.totalCounter` | `61` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1011,
    "posDateTime": "2024-07-29T15:59:39.000+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "NONE",
    "employeeId": "84022899837"
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signWorkOut": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1011,
      "posDateTime": "2024-07-29T15:59:39.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "WORK_OUT",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T13:59:43.000Z",
        "eventLabel": "S",
        "eventCounter": 8,
        "totalCounter": 61
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m141_work_out` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m141_work_out`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M141_event1_S_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "7e014ab6-cd80-44f1-ba15-7a62b080d74f",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "84022899837",
  "estNo": "2000000042",
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.836+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "NONE",
  "vatNo": "BE0000000097"
}
```

**Comparaison avec la fixture** (`evidence/uc/M141_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Aucun écart.

<!-- pili-pos-evidence:end -->

## M150_signInvoice - hors périmètre v1.0.0

L'événement fiscal **I (Invoice)** n'est **pas disponible en
v1.0.0** : le feature flag `peppol` n'est pas activé dans la build
de certification (`cert = ["custom-protocol"]`,
`src-tauri/Cargo.toml`), donc le code d'émission de facture B2B
(`crates/pos-peppol`, `crates/pos-engine/src/commands/invoice.rs`,
`#[cfg(feature = "peppol")]`) n'est pas compilé dans le binaire
soumis. La fixture M150 continue d'être **parsée** par
`crates/pos-fiscal/tests/uc_fixtures.rs` (validation du format JSON
uniquement) et **exercée en e2e** par `crates/pos-engine/tests/spf_e2e_basic.rs`
avec les features par défaut (`hubrise` + `peppol`, utilisées pour le
développement et les tests, pas pour la build cert). Les preuves de
`evidence/uc/` sont produites par la build cert, qui ne peut pas
émettre cet événement : M150 n'y figure donc pas.

Mise à jour certifiée prévue v1.1. *Voir : DECISION_PEPPOL_AP_FR.md*.

## M160_signCopy_Sale

**Mutation invoquee :** `signCopy`

**Scenario :**

```
Print of a copy of ticket (sale) N 16/52
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1013` | OK |
| `posDateTime` | `2024-07-29T16:25:39.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `C` | OK |
| `fdmRef.eventCounter` | `2` | OK |
| `fdmRef.totalCounter` | `63` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1013,
    "posDateTime": "2024-07-29T16:25:39.000+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "PAPER",
    "employeeId": "75061189702",
    "fdmRef": {
      "fdmId": "FOD01000001",
      "fdmDateTime": "2024-07-29T11:48:04.000Z",
      "eventLabel": "N",
      "eventCounter": 16,
      "totalCounter": 52
    }
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signCopy": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1013,
      "posDateTime": "2024-07-29T16:25:39.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "COPY",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T14:25:45.000Z",
        "eventLabel": "C",
        "eventCounter": 2,
        "totalCounter": 63
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m160_copy_of_a_sale` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m160_copy_of_a_sale`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M160_event2_C_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "8adb05fa-539d-4eb9-9708-24bfd4450c62",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "fdmRef": {
    "eventCounter": 16,
    "eventLabel": "N",
    "fdmDateTime": "2024-07-29T11:48:04.000Z",
    "fdmId": "FOD01000001",
    "totalCounter": 52
  },
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.842+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "vatNo": "BE0000000097"
}
```

Events préalables du scénario, archivés de même : `evidence/uc/M160_event1_N_request.json`. Réponses FDM archivées : fichiers `*_response.json` correspondants.

**Comparaison avec la fixture** (`evidence/uc/M160_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Aucun écart.

<!-- pili-pos-evidence:end -->

## M161_signCopy_Order

**Mutation invoquee :** `signCopy`

**Scenario :**

```
Print of a copy of order P 22/53
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1014` | OK |
| `posDateTime` | `2024-07-29T16:30:39.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `C` | OK |
| `fdmRef.eventCounter` | `3` | OK |
| `fdmRef.totalCounter` | `64` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1014,
    "posDateTime": "2024-07-29T16:30:39.000+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "PAPER",
    "employeeId": "75061189702",
    "fdmRef": {
      "fdmId": "FOD01000001",
      "fdmDateTime": "2024-07-29T12:57:12.000Z",
      "eventLabel": "P",
      "eventCounter": 22,
      "totalCounter": 53
    }
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signCopy": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1014,
      "posDateTime": "2024-07-29T16:30:39.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "COPY",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T14:30:42.000Z",
        "eventLabel": "C",
        "eventCounter": 3,
        "totalCounter": 64
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m161_copy_of_an_order` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m161_copy_of_an_order`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M161_event2_C_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "f5d7112a-082c-4438-a32b-17ea522fbcae",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "fdmRef": {
    "eventCounter": 22,
    "eventLabel": "P",
    "fdmDateTime": "2024-07-29T12:57:12.000Z",
    "fdmId": "FOD01000001",
    "totalCounter": 53
  },
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.853+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "vatNo": "BE0000000097"
}
```

Events préalables du scénario, archivés de même : `evidence/uc/M161_event1_P_request.json`. Réponses FDM archivées : fichiers `*_response.json` correspondants.

**Comparaison avec la fixture** (`evidence/uc/M161_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Aucun écart.

<!-- pili-pos-evidence:end -->

## M170_signWorkIn_Training

**Mutation invoquee :** `signWorkIn`

**Scenario :**

```
Registration of "Work In" of an employee (employeeId 75061189702) in training modus
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1014` | OK |
| `posDateTime` | `2024-07-29T17:00:39.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `NONE` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `T` | OK |
| `fdmRef.eventCounter` | `3` | OK |
| `fdmRef.totalCounter` | `65` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1014,
    "posDateTime": "2024-07-29T17:00:39.000+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "NONE",
    "employeeId": "75061189702"
  },
  "training": true
}
```

**Response FDM :**

```json
{
  "data": {
    "signWorkIn": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1014,
      "posDateTime": "2024-07-29T17:00:39.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "WORK_IN",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T15:00:41.000Z",
        "eventLabel": "T",
        "eventCounter": 3,
        "totalCounter": 65
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m170_work_in_in_training` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m170_work_in_in_training`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M170_event1_T_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "dd3a6701-b93c-418c-a11b-bfb482067f7f",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.837+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "NONE",
  "vatNo": "BE0000000097"
}
```

**Comparaison avec la fixture** (`evidence/uc/M170_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Aucun écart.

<!-- pili-pos-evidence:end -->

## M171B_signSale_Training

**Mutation invoquee :** `signSale`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1015` | OK |
| `posDateTime` | `2024-07-29T17:10:39.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `T` | OK |
| `fdmRef.eventCounter` | `4` | OK |
| `fdmRef.totalCounter` | `64` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1015,
    "posDateTime": "2024-07-29T17:10:39.000+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "PAPER",
    "employeeId": "75061189702",
    "transaction": {
      "transactionLines": [
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "12007",
            "productName": "Perrier",
            "departmentId": "12",
            "departmentName": "Soft Drinks",
            "quantity": 2,
            "quantityType": "PIECE",
            "unitPrice": 2.5,
            "vats": [
              {
                "label": "A",
                "price": 5
              }
            ]
          },
          "lineTotal": 5
        },
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "22001",
            "productName": "Burger of the Chef",
            "departmentId": "22",
            "departmentName": "Main Dishes",
            "quantity": 1,
            "quantityType": "PIECE",
            "unitPrice": 28,
            "vats": [
              {
                "label": "B",
                "price": 28
              }
            ]
          },
          "lineTotal": 28
        }
      ],
      "transactionTotal": 33
    },
    "financials": [
      {
        "id": "1",
        "name": "CONTANT",
        "type": "CASH",
        "inputMethod": "MANUAL",
        "amount": 33,
        "amountType": "PAYMENT",
        "drawer": {
          "id": "1",
          "name": "Drawer 1"
        }
      }
    ]
  },
  "training": true
}
```

**Response FDM :**

```json
{
  "data": {
    "signSale": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1015,
      "posDateTime": "2024-07-29T17:10:39.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "fdmSwVersion": "1.2.0",
      "eventOperation": "SALE",
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "bufferCapacityUsed": 0.25,
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T15:10:41.000Z",
        "eventLabel": "T",
        "eventCounter": 4,
        "totalCounter": 64
      },
      "warnings": null,
      "informations": null
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m171b_sale_in_training_with_skipped_fields` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m171b_sale_in_training_with_skipped_fields`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M171B_event1_T_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "41c5ec65-66fc-4f65-94f5-17840d7f365c",
  "costCenter": {
    "id": "T01",
    "reference": "O1",
    "type": "TABLE"
  },
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "financials": [
    {
      "amount": 33.0,
      "amountType": "PAYMENT",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.847+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": 5.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "12",
          "departmentName": "Soft Drinks",
          "productId": "12007",
          "productName": "Perrier",
          "quantity": 2.0,
          "quantityType": "PIECE",
          "unitPrice": 2.5,
          "vats": [
            {
              "label": "A",
              "price": 5.0
            }
          ]
        }
      },
      {
        "lineTotal": 28.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "22",
          "departmentName": "Main Dishes",
          "productId": "22001",
          "productName": "Burger of the CHEF",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 28.0,
          "vats": [
            {
              "label": "B",
              "price": 28.0
            }
          ]
        }
      }
    ],
    "transactionTotal": 33.0
  },
  "vatNo": "BE0000000097"
}
```

**Comparaison avec la fixture** (`evidence/uc/M171B_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `costCenter.type` | - | `TABLE` | une vente en salle est encaissée depuis sa table et porte le centre de coût TABLE |

<!-- pili-pos-evidence:end -->

## M171_signSale_Training

**Mutation invoquee :** `signSale`

**Scenario :**

```
Sale of
    2 Perrier (unit price 2.50 EUR, VAT code A)
    1 Burger of the Chef (unit price 28.00 EUR, VAT code B)
in training mode. Paymentmethod: CASH
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1015` | OK |
| `posDateTime` | `2024-07-29T17:10:39.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `T` | OK |
| `fdmRef.eventCounter` | `4` | OK |
| `fdmRef.totalCounter` | `64` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1015,
    "posDateTime": "2024-07-29T17:10:39.000+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "PAPER",
    "employeeId": "75061189702",
    "transaction": {
      "transactionLines": [
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "12007",
            "productName": "Perrier",
            "departmentId": "12",
            "departmentName": "Soft Drinks",
            "quantity": 2,
            "quantityType": "PIECE",
            "unitPrice": 2.5,
            "vats": [
              {
                "label": "A",
                "price": 5
              }
            ]
          },
          "lineTotal": 5
        },
        {
          "lineType": "SINGLE_PRODUCT",
          "mainProduct": {
            "productId": "22001",
            "productName": "Burger of the Chef",
            "departmentId": "22",
            "departmentName": "Main Dishes",
            "quantity": 1,
            "quantityType": "PIECE",
            "unitPrice": 28,
            "vats": [
              {
                "label": "B",
                "price": 28
              }
            ]
          },
          "lineTotal": 28
        }
      ],
      "transactionTotal": 33
    },
    "financials": [
      {
        "id": "1",
        "name": "CONTANT",
        "type": "CASH",
        "inputMethod": "MANUAL",
        "amount": 33,
        "amountType": "PAYMENT",
        "drawer": {
          "id": "1",
          "name": "Drawer 1"
        }
      }
    ]
  },
  "training": true
}
```

**Response FDM :**

```json
{
  "data": {
    "signSale": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1015,
      "posDateTime": "2024-07-29T17:10:39.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "fdmSwVersion": "1.2.0",
      "eventOperation": "SALE",
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "shortSignature": null,
      "verificationUrl": null,
      "bufferCapacityUsed": 0.25,
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T15:10:41.000Z",
        "eventLabel": "T",
        "eventCounter": 4,
        "totalCounter": 64
      },
      "vatCalc": null,
      "warnings": null,
      "informations": null
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m171_sale_in_training` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m171_sale_in_training`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M171_event1_T_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "b796c2e8-5232-4dbc-961d-915445cc1c56",
  "costCenter": {
    "id": "T01",
    "reference": "O1",
    "type": "TABLE"
  },
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "financials": [
    {
      "amount": 33.0,
      "amountType": "PAYMENT",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.874+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": 5.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "12",
          "departmentName": "Soft Drinks",
          "productId": "12007",
          "productName": "Perrier",
          "quantity": 2.0,
          "quantityType": "PIECE",
          "unitPrice": 2.5,
          "vats": [
            {
              "label": "A",
              "price": 5.0
            }
          ]
        }
      },
      {
        "lineTotal": 28.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "22",
          "departmentName": "Main Dishes",
          "productId": "22001",
          "productName": "Burger of the CHEF",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 28.0,
          "vats": [
            {
              "label": "B",
              "price": 28.0
            }
          ]
        }
      }
    ],
    "transactionTotal": 33.0
  },
  "vatNo": "BE0000000097"
}
```

**Comparaison avec la fixture** (`evidence/uc/M171_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `costCenter.type` | - | `TABLE` | une vente en salle est encaissée depuis sa table et porte le centre de coût TABLE |

<!-- pili-pos-evidence:end -->

## M172_signCopy_Training

**Mutation invoquee :** `signCopy`

**Scenario :**

```
In training mode: print of a copy of ticket (sale) N 35/73
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1016` | OK |
| `posDateTime` | `2024-07-29T17:10:39.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `NONE` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `T` | OK |
| `fdmRef.eventCounter` | `7` | OK |
| `fdmRef.totalCounter` | `67` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1016,
    "posDateTime": "2024-07-29T17:10:39.000+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "NONE",
    "employeeId": "75061189702",
    "fdmRef": {
      "fdmId": "FOD01000002",
      "fdmDateTime": "2024-07-03T16:40:39.000Z",
      "eventLabel": "N",
      "eventCounter": 35,
      "totalCounter": 73
    }
  },
  "training": true
}
```

**Response FDM :**

```json
{
  "data": {
    "signCopy": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1016,
      "posDateTime": "2024-07-29T17:10:39.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "COPY",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T15:10:45.000Z",
        "eventLabel": "T",
        "eventCounter": 7,
        "totalCounter": 67
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_fixtures.rs::m172_copy_in_training` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m172_copy_in_training`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M172_event2_T_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "3dacc358-0aaf-4a86-a18b-8518bb571efd",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "fdmRef": {
    "eventCounter": 35,
    "eventLabel": "N",
    "fdmDateTime": "2024-07-03T16:40:39.000Z",
    "fdmId": "FOD01000002",
    "totalCounter": 73
  },
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:41.987+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "vatNo": "BE0000000097"
}
```

Events préalables du scénario, archivés de même : `evidence/uc/M172_event1_N_request.json`. Réponses FDM archivées : fichiers `*_response.json` correspondants.

**Comparaison avec la fixture** (`evidence/uc/M172_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `ticketMedium` | `NONE` | `PAPER` | les copies sont imprimées aussi en mode formation : le support est PAPER |

<!-- pili-pos-evidence:end -->

## Q100_Status

**Mutation invoquee :** `inconnue`

*Aucun champ a auto-cocher : voir fixtures.*

*Request (Variables.json) : aucune fixture trouvee*

**Response FDM :**

```json
{
  "data": {
    "status": {
      "__typename": "FdmStatus",
      "initialized": true,
      "device": {
        "bufferCapacityUsed": 10.02,
        "fdmDateTime": "2025-08-10T10:00:39.000Z",
        "fdmId": "FOD01000001",
        "fdmSwVersion": "1.2.0"
      },
      "errors": [],
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-fiscal/tests/client_status.rs` (requete de statut du client FDM).


## Q110_Status_multi_lang_messages

**Mutation invoquee :** `inconnue`

*Aucun champ a auto-cocher : voir fixtures.*

*Request (Variables.json) : aucune fixture trouvee*

**Response FDM :**

```json
{
  "data": {
    "status": {
      "__typename": "FdmStatus",
      "initialized": true,
      "device": {
        "__typename": "Device",
        "bufferCapacityUsed": 10.02,
        "fdmDateTime": "2025-08-10T10:00:39.000Z",
        "fdmId": "FOD01000001",
        "fdmSwVersion": "1.2.0"
      },
      "errors": [],
      "warnings": [],
      "informations": []
    },
    "messagesFR": {
      "errors": [],
      "warnings": [],
      "informations": []
    },
    "messagesNL": {
      "errors": [],
      "warnings": [],
      "informations": []
    },
    "messagesDE": {
      "errors": [],
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/uc_fixtures.rs` (lecture de la fixture) et `crates/pos-fiscal/tests/client_status.rs` (requete de statut du client FDM).


---

# Reports v1.3 (5 UC)

## M180_signReportTurnoverX

**Mutation invoquee :** `signReportTurnoverX`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `208` | OK |
| `posDateTime` | `2024-07-30T13:00:35.000+02:00` | OK |
| `terminalId` | `POS-1-BAR` | OK |
| `deviceId` | `F0:1D:3D:CC:13:C8` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-30` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `R` | OK |
| `fdmRef.eventCounter` | `3` | OK |
| `fdmRef.totalCounter` | `57` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 208,
    "posDateTime": "2024-07-30T13:00:35.000+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "F0:1D:3D:CC:13:C8",
    "terminalId": "POS-1-BAR",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-30",
    "ticketMedium": "PAPER",
    "employeeId": "75061189702",
    "posDevices": [
      {
        "posId": "CPOS0031234567",
        "terminalId": "TER-1-BAR",
        "firstPosDateTime": "2024-07-30T08:30:12.000+02:00",
        "lastPosDateTime": "2024-07-30T08:40:12.000+02:00"
      },
      {
        "posId": "CPOS0031234567",
        "terminalId": "TER-2-DIN",
        "firstPosDateTime": "2024-07-30T11:45:12.000+02:00",
        "lastPosDateTime": "2024-07-30T12:57:12.000+02:00"
      },
      {
        "posId": "CPOS0031234568",
        "terminalId": "POS-2-REC",
        "firstPosDateTime": "2024-07-30T09:40:12.000+02:00",
        "lastPosDateTime": "2024-07-30T09:45:12.000+02:00"
      }
    ],
    "fdmDevices": [
      {
        "fdmId": "FOD01000001",
        "firstFdmDateTime": "2024-07-30T06:30:12.000Z",
        "firstTotalCounter": 49,
        "lastFdmDateTime": "2024-07-30T10:57:18.000Z",
        "lastTotalCounter": 56
      }
    ],
    "turnover": {
      "transactions": [
        {
          "eventLabel": "N",
          "ticketCount": 4,
          "amount": 102.04
        },
        {
          "eventLabel": "P",
          "ticketCount": 3,
          "amount": 198
        }
      ],
      "departments": [
        {
          "departmentId": "10",
          "departmentName": "Aperitifs",
          "amount": 24
        },
        {
          "departmentId": "12",
          "departmentName": "Soft Drinks",
          "amount": 3.04
        },
        {
          "departmentId": "15",
          "departmentName": "Wines",
          "amount": 35
        },
        {
          "departmentId": "22",
          "departmentName": "Main Dishes",
          "amount": 40
        }
      ],
      "vats": [
        {
          "label": "A",
          "rate": 21,
          "taxableAmount": 51.27,
          "vatAmount": 10.77,
          "totalAmount": 62.04
        },
        {
          "label": "B",
          "rate": 12,
          "taxableAmount": 35.71,
          "vatAmount": 4.29,
          "totalAmount": 40
        }
      ],
      "payments": [
        {
          "id": "1",
          "name": "Contant",
          "type": "CASH",
          "amount": [
            {
              "type": "PAYMENT",
              "normalAmount": 99,
              "negativeCorrections": 0,
              "positiveCorrections": 0,
              "correctionsCount": 0,
              "totalAmount": 99,
              "normalForeign": null,
              "correctionsForeign": null
            }
          ]
        },
        {
          "id": "2",
          "name": "Mastercard",
          "type": "CARD_CREDIT",
          "amount": [
            {
              "type": "PAYMENT",
              "normalAmount": 3.04,
              "negativeCorrections": 0,
              "positiveCorrections": 0,
              "correctionsCount": 0,
              "totalAmount": 3.04
            }
          ]
        }
      ],
      "drawersOpenCount": 0,
      "negQuantities": [
        {
          "negQuantityReason": "REFUND",
          "negQuantityCount": 2,
          "ticketCount": 1,
          "amount": -52
        }
      ],
      "priceChanges": [],
      "invoices": []
    }
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signReportTurnoverX": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 208,
      "posDateTime": "2024-07-30T13:00:35.000+02:00",
      "terminalId": "POS-1-BAR",
      "deviceId": "F0:1D:3D:CC:13:C8",
      "eventOperation": "REPORT_TURNOVER_X",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-30T11:00:38.000Z",
        "eventLabel": "R",
        "eventCounter": 3,
        "totalCounter": 57
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/reports_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_report_fixtures.rs::m180_turnover_x_of_the_scenario_day` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - la journée du scénario (InputReport T1–T8) est jouée par les vraies commandes, puis le rapport est construit par pili-pos à partir de son archive fiscale et comparé à la fixture par `crates/pos-engine/tests/spf_e2e_report_fixtures.rs::m180_turnover_x_of_the_scenario_day`.

Requête du rapport (`evidence/uc/M180_event9_R_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "cbfc8480-ece1-45a8-a2e5-edb0a3924432",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "fdmDevices": [
    {
      "fdmId": "FOD01000001",
      "firstFdmDateTime": "2024-07-30T06:30:19.000Z",
      "firstTotalCounter": 49,
      "lastFdmDateTime": "2024-07-30T10:57:18.000Z",
      "lastTotalCounter": 56
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:44.644+02:00",
  "posDevices": [
    {
      "firstPosDateTime": "2026-10-04T11:01:44.599+02:00",
      "lastPosDateTime": "2026-10-04T11:01:44.639+02:00",
      "posId": "CPOS0031234567",
      "terminalId": "POS-1-BAR"
    }
  ],
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "turnover": {
    "departments": [
      {
        "amount": 24.0,
        "departmentId": "10",
        "departmentName": "Aperitifs"
      },
      {
        "amount": 3.04,
        "departmentId": "12",
        "departmentName": "Soft Drinks"
      },
      {
        "amount": 35.0,
        "departmentId": "15",
        "departmentName": "Wines"
      },
      {
        "amount": 40.0,
        "departmentId": "22",
        "departmentName": "Main Dishes"
      }
    ],
    "drawersOpenCount": 0,
    "invoices": [],
    "negQuantities": [
      {
        "amount": -52.0,
        "negQuantityCount": 2,
        "negQuantityReason": "REFUND",
        "ticketCount": 1
      }
    ],
    "payments": [
      {
        "amount": [
          {
            "correctionsCount": 0,
            "negativeCorrections": 0.0,
            "normalAmount": 99.0,
            "positiveCorrections": 0.0,
            "totalAmount": 99.0,
            "type": "PAYMENT"
          }
        ],
        "id": "1",
        "name": "Cash",
        "type": "CASH"
      },
      {
        "amount": [
          {
            "correctionsCount": 0,
            "negativeCorrections": 0.0,
            "normalAmount": 3.04,
            "positiveCorrections": 0.0,
            "totalAmount": 3.04,
            "type": "PAYMENT"
          }
        ],
        "id": "2",
        "name": "Card",
        "type": "CARD_DEBIT"
      }
    ],
    "priceChanges": [],
    "transactions": [
      {
        "amount": 102.04,
        "eventLabel": "N",
        "ticketCount": 4
      },
      {
        "amount": 250.0,
        "eventLabel": "P",
        "ticketCount": 4
      }
    ],
    "vats": [
      {
        "label": "A",
        "rate": 21.0,
        "taxableAmount": 51.27,
        "totalAmount": 62.04,
        "vatAmount": 10.77
      },
      {
        "label": "B",
        "rate": 12.0,
        "taxableAmount": 35.71,
        "totalAmount": 40.0,
        "vatAmount": 4.29
      }
    ]
  },
  "vatNo": "BE0000000097"
}
```

Events de la journée, archivés de même : `evidence/uc/M180_event1_P_request.json`, `evidence/uc/M180_event2_N_request.json`, `evidence/uc/M180_event3_N_request.json`, `evidence/uc/M180_event4_N_request.json`, `evidence/uc/M180_event5_P_request.json`, `evidence/uc/M180_event6_P_request.json`, `evidence/uc/M180_event7_P_request.json`, `evidence/uc/M180_event8_N_request.json`.

Comparés exactement : nombre d'events et montants par label, départements, TVA (taux, base, TVA, total), paiements, quantités négatives, ouvertures de tiroir, totaux, paiements et présences par utilisateur. Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `transactions[P]` | 3 tickets, 198 € | 4 tickets, 250 € | chaque event P de la période est compté (§2.2.2 `ticketCount`) : T1, T5, T6 et T7 (AM du 29/04/2024, art. 29, t) |
| `payments[].id` / `name` | identifiants et libellés du scénario | identifiants et libellés de pili-pos | propres à chaque POS |
| `payments[].type` (carte) | `CARD_CREDIT` | `CARD_DEBIT` | l'opérateur ne distingue pas carte de débit et de crédit |
| `departments[].name` | libellés du scénario | libellés français du catalogue | propres au catalogue |
| `posDevices` / `fdmDevices` | appareils du scénario | ce terminal et le FDM de test | propres à l'installation |

<!-- pili-pos-evidence:end -->

## M181_signReportTurnoverZ

**Note de déclaration :** le POS pili-pos n'émet jamais le Z financier
seul : la clôture de journée émet toujours le **Z combiné M184**
(`signReportTurnoverZ` + `signReportUserZ` dans un même document GraphQL,
deux `posFiscalTicketNo` séquentiels) - voir la section M184 ci-dessous.
La fixture M181 est validée en *parsing* et sa forme `signReportTurnoverZ`
est exactement celle émise dans la première moitié du document M184.
Couverture e2e applicative du Z : `m184_combined_z_of_the_scenario_day`
dans `crates/pos-engine/tests/spf_e2e_report_fixtures.rs` (journée du scénario
comparée à la fixture) et `m184_*` dans
`crates/pos-engine/tests/spf_e2e_reports.rs`.

**Mutation invoquee :** `signReportTurnoverZ`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `209` | OK |
| `posDateTime` | `2024-07-30T13:05:35.000+02:00` | OK |
| `terminalId` | `POS-1-BAR` | OK |
| `deviceId` | `F0:1D:3D:CC:13:C8` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-30` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `R` | OK |
| `fdmRef.eventCounter` | `4` | OK |
| `fdmRef.totalCounter` | `58` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 209,
    "posDateTime": "2024-07-30T13:05:35.000+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "F0:1D:3D:CC:13:C8",
    "terminalId": "POS-1-BAR",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-30",
    "ticketMedium": "PAPER",
    "employeeId": "75061189702",
    "reportNo": 2,
    "reportBookingDate": "2024-07-30",
    "posDevices": [
      {
        "posId": "CPOS0031234567",
        "terminalId": "TER-1-BAR",
        "firstPosDateTime": "2024-07-30T08:30:12.000+02:00",
        "lastPosDateTime": "2024-07-30T08:40:12.000+02:00"
      },
      {
        "posId": "CPOS0031234567",
        "terminalId": "TER-2-DIN",
        "firstPosDateTime": "2024-07-30T11:45:12.000+02:00",
        "lastPosDateTime": "2024-07-30T12:57:12.000+02:00"
      },
      {
        "posId": "CPOS0031234567",
        "terminalId": "POS-1-BAR",
        "firstPosDateTime": "2024-07-30T13:00:35.000+02:00",
        "lastPosDateTime": "2024-07-30T13:00:35.000+02:00"
      },
      {
        "posId": "CPOS0031234568",
        "terminalId": "POS-2-REC",
        "firstPosDateTime": "2024-07-30T09:40:12.000+02:00",
        "lastPosDateTime": "2024-07-30T09:45:12.000+02:00"
      }
    ],
    "fdmDevices": [
      {
        "fdmId": "FOD01000001",
        "firstFdmDateTime": "2024-07-30T06:30:12.000Z",
        "firstTotalCounter": 49,
        "lastFdmDateTime": "2024-07-30T11:00:38.000Z",
        "lastTotalCounter": 57
      }
    ],
    "turnover": {
      "transactions": [
        {
          "eventLabel": "N",
          "ticketCount": 4,
          "amount": 102.04
        },
        {
          "eventLabel": "P",
          "ticketCount": 3,
          "amount": 198
        }
      ],
      "departments": [
        {
          "departmentId": "10",
          "departmentName": "Aperitifs",
          "amount": 24
        },
        {
          "departmentId": "12",
          "departmentName": "Soft Drinks",
          "amount": 3.04
        },
        {
          "departmentId": "15",
          "departmentName": "Wines",
          "amount": 35
        },
        {
          "departmentId": "22",
          "departmentName": "Main Dishes",
          "amount": 40
        }
      ],
      "vats": [
        {
          "label": "A",
          "rate": 21,
          "taxableAmount": 51.27,
          "vatAmount": 10.77,
          "totalAmount": 62.04
        },
        {
          "label": "B",
          "rate": 12,
          "taxableAmount": 35.71,
          "vatAmount": 4.29,
          "totalAmount": 40
        }
      ],
      "payments": [
        {
          "id": "1",
          "name": "Contant",
          "type": "CASH",
          "amount": [
            {
              "type": "PAYMENT",
              "normalAmount": 99,
              "negativeCorrections": 0,
              "positiveCorrections": 0,
              "correctionsCount": 0,
              "totalAmount": 99,
              "normalForeign": null,
              "correctionsForeign": null
            }
          ]
        },
        {
          "id": "2",
          "name": "Mastercard",
          "type": "CARD_CREDIT",
          "amount": [
            {
              "type": "PAYMENT",
              "normalAmount": 3.04,
              "negativeCorrections": 0,
              "positiveCorrections": 0,
              "correctionsCount": 0,
              "totalAmount": 3.04
            }
          ]
        }
      ],
      "drawersOpenCount": 0,
      "negQuantities": [
        {
          "negQuantityReason": "REFUND",
          "negQuantityCount": 2,
          "ticketCount": 1,
          "amount": -52
        }
      ],
      "priceChanges": [],
      "invoices": []
    }
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signReportTurnoverZ": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 209,
      "posDateTime": "2024-07-30T13:05:35.000+02:00",
      "terminalId": "POS-1-BAR",
      "deviceId": "F0:1D:3D:CC:13:C8",
      "eventOperation": "REPORT_TURNOVER_Z",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-30T11:05:41.000Z",
        "eventLabel": "R",
        "eventCounter": 4,
        "totalCounter": 58
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/reports_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_report_fixtures.rs::m184_combined_z_of_the_scenario_day` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - cet UC est signé par le Z combiné de M184 (mutation `signReportTurnoverZ`), rejoué par `crates/pos-engine/tests/spf_e2e_report_fixtures.rs::m184_combined_z_of_the_scenario_day` : voir `evidence/uc/M184_event9_R_request.json`, `evidence/uc/M184_event10_R_request.json` et la section M184.
<!-- pili-pos-evidence:end -->

## M182_signReportUserX

**Note de déclaration :** implémenté par la commande
`generate_user_x_report` (`crates/pos-engine/src/commands/report.rs`,
`do_generate_user_x_report`). Couverture e2e applicative :
`m182_user_x_of_the_scenario_day` dans
`crates/pos-engine/tests/spf_e2e_report_fixtures.rs` (journée du scénario
comparée à la fixture) et
`m182_generate_user_x_report_archives_r_event_with_user_block` dans
`crates/pos-engine/tests/spf_e2e_reports.rs` ; preuves dans
`evidence/uc/M182_*.json` (journée T1–T8 puis l'événement R).

**Mutation invoquee :** `signReportUserX`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `210` | OK |
| `posDateTime` | `2024-07-30T13:10:35.000+02:00` | OK |
| `terminalId` | `POS-1-BAR` | OK |
| `deviceId` | `F0:1D:3D:CC:13:C8` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-30` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `R` | OK |
| `fdmRef.eventCounter` | `5` | OK |
| `fdmRef.totalCounter` | `59` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 210,
    "posDateTime": "2024-07-30T13:10:35.000+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "F0:1D:3D:CC:13:C8",
    "terminalId": "POS-1-BAR",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-30",
    "ticketMedium": "PAPER",
    "employeeId": "75061189702",
    "posDevices": [
      {
        "posId": "CPOS0031234567",
        "terminalId": "TER-1-BAR",
        "firstPosDateTime": "2024-07-30T08:30:12.000+02:00",
        "lastPosDateTime": "2024-07-30T08:40:12.000+02:00"
      },
      {
        "posId": "CPOS0031234567",
        "terminalId": "TER-2-DIN",
        "firstPosDateTime": "2024-07-30T11:45:12.000+02:00",
        "lastPosDateTime": "2024-07-30T12:57:12.000+02:00"
      },
      {
        "posId": "CPOS0031234567",
        "terminalId": "POS-1-BAR",
        "firstPosDateTime": "2024-07-30T13:00:35.000+02:00",
        "lastPosDateTime": "2024-07-30T13:05:35.000+02:00"
      },
      {
        "posId": "CPOS0031234568",
        "terminalId": "POS-2-REC",
        "firstPosDateTime": "2024-07-30T09:40:12.000+02:00",
        "lastPosDateTime": "2024-07-30T09:45:12.000+02:00"
      }
    ],
    "fdmDevices": [
      {
        "fdmId": "FOD01000001",
        "firstFdmDateTime": "2024-04-22T06:30:12.000Z",
        "firstTotalCounter": 49,
        "lastFdmDateTime": "2024-07-30T11:05:41.000Z",
        "lastTotalCounter": 58
      }
    ],
    "users": [
      {
        "employeeId": "75061189702",
        "totalAmount": 0,
        "firstPosDateTime": "2024-07-30T08:30:12.000+02:00",
        "lastPosDateTime": "2024-07-30T09:40:12.000+02:00",
        "socialEvents": [],
        "payments": [
          {
            "id": "1",
            "name": "CONTANT",
            "type": "CASH",
            "amount": [
              {
                "type": "PAYMENT",
                "normalAmount": 0,
                "negativeCorrections": 0,
                "positiveCorrections": 0,
                "correctionsCount": 0,
                "totalAmount": 0
              }
            ]
          }
        ]
      },
      {
        "employeeId": "84022899837",
        "totalAmount": 102.04,
        "firstPosDateTime": "2024-07-30T09:45:12.000+02:00",
        "lastPosDateTime": "2024-07-30T12:57:12.000+02:00",
        "socialEvents": [],
        "payments": [
          {
            "id": "1",
            "name": "CONTANT",
            "type": "CASH",
            "amount": [
              {
                "type": "PAYMENT",
                "normalAmount": 99,
                "negativeCorrections": 0,
                "positiveCorrections": 0,
                "correctionsCount": 0,
                "totalAmount": 99
              }
            ]
          },
          {
            "id": "2",
            "name": "Master Card",
            "type": "CARD_CREDIT",
            "amount": [
              {
                "type": "PAYMENT",
                "normalAmount": 3.04,
                "negativeCorrections": 0,
                "positiveCorrections": 0,
                "correctionsCount": 0,
                "totalAmount": 3.04
              }
            ]
          }
        ]
      }
    ]
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signReportUserX": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 210,
      "posDateTime": "2024-07-30T13:10:35.000+02:00",
      "terminalId": "POS-1-BAR",
      "deviceId": "F0:1D:3D:CC:13:C8",
      "eventOperation": "REPORT_USER_X",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-30T11:10:42.000Z",
        "eventLabel": "R",
        "eventCounter": 5,
        "totalCounter": 59
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/reports_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_report_fixtures.rs::m182_user_x_of_the_scenario_day` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - la journée du scénario (InputReport T1–T8) est jouée par les vraies commandes, puis le rapport est construit par pili-pos à partir de son archive fiscale et comparé à la fixture par `crates/pos-engine/tests/spf_e2e_report_fixtures.rs::m182_user_x_of_the_scenario_day`.

Requête du rapport (`evidence/uc/M182_event9_R_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "147d7c32-2def-4a6e-8a8d-aa9a3c1f5c03",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "fdmDevices": [
    {
      "fdmId": "FOD01000001",
      "firstFdmDateTime": "2024-07-30T06:30:19.000Z",
      "firstTotalCounter": 49,
      "lastFdmDateTime": "2024-07-30T10:57:18.000Z",
      "lastTotalCounter": 56
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:44.650+02:00",
  "posDevices": [
    {
      "firstPosDateTime": "2026-10-04T11:01:44.600+02:00",
      "lastPosDateTime": "2026-10-04T11:01:44.641+02:00",
      "posId": "CPOS0031234567",
      "terminalId": "POS-1-BAR"
    }
  ],
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "users": [
    {
      "employeeId": "75061189702",
      "firstPosDateTime": "2026-10-04T11:01:44.600+02:00",
      "lastPosDateTime": "2026-10-04T11:01:44.612+02:00",
      "payments": [
        {
          "amount": [
            {
              "correctionsCount": 0,
              "negativeCorrections": 0.0,
              "normalAmount": 0.0,
              "positiveCorrections": 0.0,
              "totalAmount": 0.0,
              "type": "PAYMENT"
            }
          ],
          "id": "1",
          "name": "Cash",
          "type": "CASH"
        }
      ],
      "socialEvents": [],
      "totalAmount": 0.0
    },
    {
      "employeeId": "84022899837",
      "firstPosDateTime": "2026-10-04T11:01:44.622+02:00",
      "lastPosDateTime": "2026-10-04T11:01:44.641+02:00",
      "payments": [
        {
          "amount": [
            {
              "correctionsCount": 0,
              "negativeCorrections": 0.0,
              "normalAmount": 99.0,
              "positiveCorrections": 0.0,
              "totalAmount": 99.0,
              "type": "PAYMENT"
            }
          ],
          "id": "1",
          "name": "Cash",
          "type": "CASH"
        },
        {
          "amount": [
            {
              "correctionsCount": 0,
              "negativeCorrections": 0.0,
              "normalAmount": 3.04,
              "positiveCorrections": 0.0,
              "totalAmount": 3.04,
              "type": "PAYMENT"
            }
          ],
          "id": "2",
          "name": "Card",
          "type": "CARD_DEBIT"
        }
      ],
      "socialEvents": [],
      "totalAmount": 102.04
    }
  ],
  "vatNo": "BE0000000097"
}
```

Events de la journée, archivés de même : `evidence/uc/M182_event1_P_request.json`, `evidence/uc/M182_event2_N_request.json`, `evidence/uc/M182_event3_N_request.json`, `evidence/uc/M182_event4_N_request.json`, `evidence/uc/M182_event5_P_request.json`, `evidence/uc/M182_event6_P_request.json`, `evidence/uc/M182_event7_P_request.json`, `evidence/uc/M182_event8_N_request.json`.

Comparés exactement : nombre d'events et montants par label, départements, TVA (taux, base, TVA, total), paiements, quantités négatives, ouvertures de tiroir, totaux, paiements et présences par utilisateur. Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `transactions[P]` | 3 tickets, 198 € | 4 tickets, 250 € | chaque event P de la période est compté (§2.2.2 `ticketCount`) : T1, T5, T6 et T7 (AM du 29/04/2024, art. 29, t) |
| `payments[].id` / `name` | identifiants et libellés du scénario | identifiants et libellés de pili-pos | propres à chaque POS |
| `payments[].type` (carte) | `CARD_CREDIT` | `CARD_DEBIT` | l'opérateur ne distingue pas carte de débit et de crédit |
| `departments[].name` | libellés du scénario | libellés français du catalogue | propres au catalogue |
| `posDevices` / `fdmDevices` | appareils du scénario | ce terminal et le FDM de test | propres à l'installation |

<!-- pili-pos-evidence:end -->

## M183_signReportUserZ

**Note de déclaration :** comme pour M181, le POS n'émet jamais le Z
utilisateurs seul - il est toujours émis dans le document combiné M184
(seconde moitié). La fixture M183 est validée en *parsing* et sa forme
`signReportUserZ` est exactement celle émise dans le document M184.

**Mutation invoquee :** `signReportUserZ`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `211` | OK |
| `posDateTime` | `2024-07-30T13:15:35.000+02:00` | OK |
| `terminalId` | `POS-1-BAR` | OK |
| `deviceId` | `F0:1D:3D:CC:13:C8` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-30` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `75061189702` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `R` | OK |
| `fdmRef.eventCounter` | `6` | OK |
| `fdmRef.totalCounter` | `60` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 211,
    "posDateTime": "2024-07-30T13:15:35.000+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "F0:1D:3D:CC:13:C8",
    "terminalId": "POS-1-BAR",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-30",
    "ticketMedium": "PAPER",
    "employeeId": "75061189702",
    "reportBookingDate": "2024-07-30",
    "reportNo": 2,
    "posDevices": [
      {
        "posId": "CPOS0031234567",
        "terminalId": "TER-1-BAR",
        "firstPosDateTime": "2024-07-30T08:30:12.000+02:00",
        "lastPosDateTime": "2024-07-30T08:40:12.000+02:00"
      },
      {
        "posId": "CPOS0031234567",
        "terminalId": "TER-2-DIN",
        "firstPosDateTime": "2024-07-30T11:45:12.000+02:00",
        "lastPosDateTime": "2024-07-30T12:57:12.000+02:00"
      },
      {
        "posId": "CPOS0031234567",
        "terminalId": "POS-1-BAR",
        "firstPosDateTime": "2024-07-30T13:00:35.000+02:00",
        "lastPosDateTime": "2024-07-30T13:10:35.000+02:00"
      },
      {
        "posId": "CPOS0031234568",
        "terminalId": "POS-2-REC",
        "firstPosDateTime": "2024-07-30T09:40:12.000+02:00",
        "lastPosDateTime": "2024-07-30T09:45:12.000+02:00"
      }
    ],
    "fdmDevices": [
      {
        "fdmId": "FOD01000001",
        "firstFdmDateTime": "2024-04-22T06:30:12.000Z",
        "firstTotalCounter": 49,
        "lastFdmDateTime": "2024-07-30T11:10:42.000Z",
        "lastTotalCounter": 59
      }
    ],
    "users": [
      {
        "employeeId": "75061189702",
        "totalAmount": 0,
        "firstPosDateTime": "2024-07-30T08:30:12.000+02:00",
        "lastPosDateTime": "2024-07-30T09:40:12.000+02:00",
        "socialEvents": [],
        "payments": [
          {
            "id": "1",
            "name": "CONTANT",
            "type": "CASH",
            "amount": [
              {
                "type": "PAYMENT",
                "normalAmount": 0,
                "negativeCorrections": 0,
                "positiveCorrections": 0,
                "correctionsCount": 0,
                "totalAmount": 0
              }
            ]
          }
        ]
      },
      {
        "employeeId": "84022899837",
        "totalAmount": 102.04,
        "firstPosDateTime": "2024-07-30T09:45:12.000+02:00",
        "lastPosDateTime": "2024-07-30T12:57:12.000+02:00",
        "socialEvents": [],
        "payments": [
          {
            "id": "1",
            "name": "CONTANT",
            "type": "CASH",
            "amount": [
              {
                "type": "PAYMENT",
                "normalAmount": 99,
                "negativeCorrections": 0,
                "positiveCorrections": 0,
                "correctionsCount": 0,
                "totalAmount": 99
              }
            ]
          },
          {
            "id": "2",
            "name": "Master Card",
            "type": "CARD_CREDIT",
            "amount": [
              {
                "type": "PAYMENT",
                "normalAmount": 3.04,
                "negativeCorrections": 0,
                "positiveCorrections": 0,
                "correctionsCount": 0,
                "totalAmount": 3.04
              }
            ]
          }
        ]
      }
    ]
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signReportUserZ": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 211,
      "posDateTime": "2024-07-30T13:15:35.000+02:00",
      "terminalId": "POS-1-BAR",
      "deviceId": "F0:1D:3D:CC:13:C8",
      "eventOperation": "REPORT_USER_Z",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-30T11:15:41.000Z",
        "eventLabel": "R",
        "eventCounter": 6,
        "totalCounter": 60
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/reports_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_report_fixtures.rs::m184_combined_z_of_the_scenario_day` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - cet UC est signé par le Z combiné de M184 (mutation `signReportUserZ`), rejoué par `crates/pos-engine/tests/spf_e2e_report_fixtures.rs::m184_combined_z_of_the_scenario_day` : voir `evidence/uc/M184_event9_R_request.json`, `evidence/uc/M184_event10_R_request.json` et la section M184.
<!-- pili-pos-evidence:end -->

## M184_signReportTurnoverZUserZ

**Note de déclaration :** c'est CE mode d'émission du Z que le POS
implémente et déclare : `do_generate_z_report`
(`crates/pos-engine/src/commands/report.rs`) envoie `signReportTurnoverZ` +
`signReportUserZ` dans **un seul document GraphQL**
(`FdmClient::sign_report_turnover_z_user_z`), alloue deux
`posFiscalTicketNo` séquentiels et archive les deux événements R dans
une même transaction SQLite. Couverture e2e applicative :
`m184_generate_z_report_signs_both_reports_in_one_operation` et
`z_report_starts_new_booking_period` dans
`crates/pos-engine/tests/spf_e2e_reports.rs`, et
`m184_combined_z_of_the_scenario_day` dans
`crates/pos-engine/tests/spf_e2e_report_fixtures.rs` (journée du scénario
comparée à la fixture) ; preuves dans `evidence/uc/M184_*.json`
(journée T1–T8 puis les 2 événements R).

**Mutation invoquee :** `signReportTurnoverZ`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `R` | OK |
| `fdmRef.eventCounter` | `3` | OK |
| `fdmRef.totalCounter` | `47` | OK |

**Request (Variables.json) :**

```json
{
  "turnover": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 209,
    "posDateTime": "2024-07-30T13:05:35.521+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "F0:1D:3D:CC:13:C8",
    "terminalId": "POS-1-BAR",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-30",
    "ticketMedium": "PAPER",
    "employeeId": "75061189702",
    "reportNo": 2,
    "reportBookingDate": "2024-07-30",
    "posDevices": [
      {
        "posId": "CPOS0031234567",
        "terminalId": "TER-1-BAR",
        "firstPosDateTime": "2024-07-30T08:30:12+02:00",
        "lastPosDateTime": "2024-07-30T08:40:12+02:00"
      },
      {
        "posId": "CPOS0031234567",
        "terminalId": "TER-2-DIN",
        "firstPosDateTime": "2024-07-30T11:45:12+02:00",
        "lastPosDateTime": "2024-07-30T12:57:12+02:00"
      },
      {
        "posId": "CPOS0031234567",
        "terminalId": "POS-1-BAR",
        "firstPosDateTime": "2024-07-30T13:00:35+02:00",
        "lastPosDateTime": "2024-07-30T13:00:35+02:00"
      },
      {
        "posId": "CPOS0031234568",
        "terminalId": "POS-2-REC",
        "firstPosDateTime": "2024-07-30T09:40:12+02:00",
        "lastPosDateTime": "2024-07-30T09:45:12+02:00"
      }
    ],
    "fdmDevices": [
      {
        "fdmId": "FOD01000001",
        "firstFdmDateTime": "2024-07-30T06:30:12Z",
        "firstTotalCounter": 49,
        "lastFdmDateTime": "2024-07-30T11:00:38Z",
        "lastTotalCounter": 57
      }
    ],
    "turnover": {
      "transactions": [
        {
          "eventLabel": "N",
          "ticketCount": 4,
          "amount": 102.04
        },
        {
          "eventLabel": "P",
          "ticketCount": 3,
          "amount": 198
        }
      ],
      "departments": [
        {
          "departmentId": "10",
          "departmentName": "Aperitifs",
          "amount": 24
        },
        {
          "departmentId": "12",
          "departmentName": "Soft Drinks",
          "amount": 3.04
        },
        {
          "departmentId": "15",
          "departmentName": "Wines",
          "amount": 35
        },
        {
          "departmentId": "22",
          "departmentName": "Main Dishes",
          "amount": 40
        }
      ],
      "vats": [
        {
          "label": "A",
          "rate": 21,
          "taxableAmount": 51.27,
          "vatAmount": 10.77,
          "totalAmount": 62.04
        },
        {
          "label": "B",
          "rate": 12,
          "taxableAmount": 35.71,
          "vatAmount": 4.29,
          "totalAmount": 40
        }
      ],
      "payments": [
        {
          "id": "1",
          "name": "Contant",
          "type": "CASH",
          "amount": [
            {
              "type": "PAYMENT",
              "normalAmount": 99,
              "negativeCorrections": 0,
              "positiveCorrections": 0,
              "correctionsCount": 0,
              "totalAmount": 99,
              "normalForeign": null,
              "correctionsForeign": null
            }
          ]
        },
        {
          "id": "2",
          "name": "Mastercard",
          "type": "CARD_CREDIT",
          "amount": [
            {
              "type": "PAYMENT",
              "normalAmount": 3.04,
              "negativeCorrections": 0,
              "positiveCorrections": 0,
              "correctionsCount": 0,
              "totalAmount": 3.04
            }
          ]
        }
      ],
      "drawersOpenCount": 0,
      "negQuantities": [
        {
          "negQuantityReason": "REFUND",
          "negQuantityCount": 2,
          "ticketCount": 1,
          "amount": -52
        }
      ],
      "priceChanges": [],
      "invoices": []
    }
  },
  "user": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 210,
    "posDateTime": "202
... (tronque, voir fixture source)
```

**Response FDM :**

```json
{
  "data": {
    "signReportTurnoverZ": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 209,
      "posDateTime": "2024-07-30T13:05:35.000+02:00",
      "terminalId": "POS-1-BAR",
      "deviceId": "F0:1D:3D:CC:13:C8",
      "eventOperation": "REPORT_TURNOVER_Z",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-30T11:05:43.000Z",
        "eventLabel": "R",
        "eventCounter": 3,
        "totalCounter": 47
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    },
    "signReportUserZ": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 210,
      "posDateTime": "2024-07-30T13:05:35.000+02:00",
      "terminalId": "POS-1-BAR",
      "deviceId": "F0:1D:3D:CC:13:C8",
      "eventOperation": "REPORT_USER_Z",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-30T11:05:43.000Z",
        "eventLabel": "R",
        "eventCounter": 4,
        "totalCounter": 48
      },
      "fdmSwVersion": "1.2.0",
      "bufferCapacityUsed": 0.25,
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "warnings": [],
      "informations": []
    }
  }
}
```

**Validation automatisee :** `crates/pos-fiscal/tests/reports_fixtures.rs` (lecture de la fixture) et `crates/pos-engine/tests/spf_e2e_report_fixtures.rs::m184_combined_z_of_the_scenario_day` (rejeu applicatif, build `cert` ; voir « Preuve pili-pos » ci-dessous).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - la journée du scénario (InputReport T1–T8) est jouée par les vraies commandes, puis le rapport est construit par pili-pos à partir de son archive fiscale et comparé à la fixture par `crates/pos-engine/tests/spf_e2e_report_fixtures.rs::m184_combined_z_of_the_scenario_day`.

Requête du rapport (`evidence/uc/M184_event9_R_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "5f904cd4-0a5f-45db-b0b1-1bdfe2939c4f",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "fdmDevices": [
    {
      "fdmId": "FOD01000001",
      "firstFdmDateTime": "2024-07-30T06:30:19.000Z",
      "firstTotalCounter": 49,
      "lastFdmDateTime": "2024-07-30T10:57:18.000Z",
      "lastTotalCounter": 56
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:44.644+02:00",
  "posDevices": [
    {
      "firstPosDateTime": "2026-10-04T11:01:44.601+02:00",
      "lastPosDateTime": "2026-10-04T11:01:44.639+02:00",
      "posId": "CPOS0031234567",
      "terminalId": "POS-1-BAR"
    }
  ],
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "reportBookingDate": "2026-10-04",
  "reportNo": 1,
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "turnover": {
    "departments": [
      {
        "amount": 24.0,
        "departmentId": "10",
        "departmentName": "Aperitifs"
      },
      {
        "amount": 3.04,
        "departmentId": "12",
        "departmentName": "Soft Drinks"
      },
      {
        "amount": 35.0,
        "departmentId": "15",
        "departmentName": "Wines"
      },
      {
        "amount": 40.0,
        "departmentId": "22",
        "departmentName": "Main Dishes"
      }
    ],
    "drawersOpenCount": 0,
    "invoices": [],
    "negQuantities": [
      {
        "amount": -52.0,
        "negQuantityCount": 2,
        "negQuantityReason": "REFUND",
        "ticketCount": 1
      }
    ],
    "payments": [
      {
        "amount": [
          {
            "correctionsCount": 0,
            "negativeCorrections": 0.0,
            "normalAmount": 99.0,
            "positiveCorrections": 0.0,
            "totalAmount": 99.0,
            "type": "PAYMENT"
          }
        ],
        "id": "1",
        "name": "Cash",
        "type": "CASH"
      },
      {
        "amount": [
          {
            "correctionsCount": 0,
            "negativeCorrections": 0.0,
            "normalAmount": 3.04,
            "positiveCorrections": 0.0,
            "totalAmount": 3.04,
            "type": "PAYMENT"
          }
        ],
        "id": "2",
        "name": "Card",
        "type": "CARD_DEBIT"
      }
    ],
    "priceChanges": [],
    "transactions": [
      {
        "amount": 102.04,
        "eventLabel": "N",
        "ticketCount": 4
      },
      {
        "amount": 250.0,
        "eventLabel": "P",
        "ticketCount": 4
      }
    ],
    "vats": [
      {
        "label": "A",
        "rate": 21.0,
        "taxableAmount": 51.27,
        "totalAmount": 62.04,
        "vatAmount": 10.77
      },
      {
        "label": "B",
        "rate": 12.0,
        "taxableAmount": 35.71,
        "totalAmount": 40.0,
        "vatAmount": 4.29
      }
    ]
  },
  "vatNo": "BE0000000097"
}
```

Requête du rapport (`evidence/uc/M184_event10_R_request.json`) :

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "5f904cd4-0a5f-45db-b0b1-1bdfe2939c4f",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "fdmDevices": [
    {
      "fdmId": "FOD01000001",
      "firstFdmDateTime": "2024-07-30T06:30:19.000Z",
      "firstTotalCounter": 49,
      "lastFdmDateTime": "2024-07-30T10:57:18.000Z",
      "lastTotalCounter": 56
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:44.644+02:00",
  "posDevices": [
    {
      "firstPosDateTime": "2026-10-04T11:01:44.601+02:00",
      "lastPosDateTime": "2026-10-04T11:01:44.639+02:00",
      "posId": "CPOS0031234567",
      "terminalId": "POS-1-BAR"
    }
  ],
  "posFiscalTicketNo": 2,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "reportBookingDate": "2026-10-04",
  "reportNo": 1,
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "users": [
    {
      "employeeId": "75061189702",
      "firstPosDateTime": "2026-10-04T11:01:44.601+02:00",
      "lastPosDateTime": "2026-10-04T11:01:44.612+02:00",
      "payments": [
        {
          "amount": [
            {
              "correctionsCount": 0,
              "negativeCorrections": 0.0,
              "normalAmount": 0.0,
              "positiveCorrections": 0.0,
              "totalAmount": 0.0,
              "type": "PAYMENT"
            }
          ],
          "id": "1",
          "name": "Cash",
          "type": "CASH"
        }
      ],
      "socialEvents": [],
      "totalAmount": 0.0
    },
    {
      "employeeId": "84022899837",
      "firstPosDateTime": "2026-10-04T11:01:44.621+02:00",
      "lastPosDateTime": "2026-10-04T11:01:44.639+02:00",
      "payments": [
        {
          "amount": [
            {
              "correctionsCount": 0,
              "negativeCorrections": 0.0,
              "normalAmount": 99.0,
              "positiveCorrections": 0.0,
              "totalAmount": 99.0,
              "type": "PAYMENT"
            }
          ],
          "id": "1",
          "name": "Cash",
          "type": "CASH"
        },
        {
          "amount": [
            {
              "correctionsCount": 0,
              "negativeCorrections": 0.0,
              "normalAmount": 3.04,
              "positiveCorrections": 0.0,
              "totalAmount": 3.04,
              "type": "PAYMENT"
            }
          ],
          "id": "2",
          "name": "Card",
          "type": "CARD_DEBIT"
        }
      ],
      "socialEvents": [],
      "totalAmount": 102.04
    }
  ],
  "vatNo": "BE0000000097"
}
```

Events de la journée, archivés de même : `evidence/uc/M184_event1_P_request.json`, `evidence/uc/M184_event2_N_request.json`, `evidence/uc/M184_event3_N_request.json`, `evidence/uc/M184_event4_N_request.json`, `evidence/uc/M184_event5_P_request.json`, `evidence/uc/M184_event6_P_request.json`, `evidence/uc/M184_event7_P_request.json`, `evidence/uc/M184_event8_N_request.json`.

Comparés exactement : nombre d'events et montants par label, départements, TVA (taux, base, TVA, total), paiements, quantités négatives, ouvertures de tiroir, totaux, paiements et présences par utilisateur. Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `transactions[P]` | 3 tickets, 198 € | 4 tickets, 250 € | chaque event P de la période est compté (§2.2.2 `ticketCount`) : T1, T5, T6 et T7 (AM du 29/04/2024, art. 29, t) |
| `payments[].id` / `name` | identifiants et libellés du scénario | identifiants et libellés de pili-pos | propres à chaque POS |
| `payments[].type` (carte) | `CARD_CREDIT` | `CARD_DEBIT` | l'opérateur ne distingue pas carte de débit et de crédit |
| `departments[].name` | libellés du scénario | libellés français du catalogue | propres au catalogue |
| `posDevices` / `fdmDevices` | appareils du scénario | ce terminal et le FDM de test | propres à l'installation |

<!-- pili-pos-evidence:end -->

---

# Arrondissement des methodes de paiement (manuel §IV)

Le manuel demande les tests de base 1 et 2 dans tous les cas, le test 3
si le POS n'arrondit que les paiements en especes, les tests 4 et 5 si
tous les moyens de paiement sont arrondis. pili-pos n'arrondit **que
les especes** (`crates/pos-fiscal/src/payment.rs` : un paiement par
carte ou par cheque-repas est toujours exact, l'arrondi au 0,05 EUR
porte sur le solde paye en especes, sur sa propre ligne `ROUNDING`).
Sont donc presentees les cinq situations des tests de base 1 a 3 (les
tests 4 et 5 ne s'appliquent pas). Produits du manuel : Dame Blanche
11,52 EUR et Tapas Variation 3,34 EUR, TVA B (12 %). Les situations
sont rejouees par `crates/pos-engine/tests/spf_e2e_rounding.rs`, par les
commandes d'encaissement, contre le FDM simule ; le test compare les
lignes `financials` de la requete et le ticket imprime au tableau du
manuel.

## ROUND1_Arrondi_cash_Dame_Blanche

**Test de base 1, situation 1.** Vente directe de 1x Dame Blanche
(11,52 EUR), payee uniquement en especes. Attendu : a payer 11,50 EUR,
arrondi -0,02 EUR.

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - situation rejouée par `crates/pos-engine/tests/spf_e2e_rounding.rs::round1_dame_blanche_cash`, par la commande que l'écran d'encaissement appelle. Requête **construite et archivée par pili-pos** (`evidence/uc/ROUND1_event1_N_request.json`) ; réponse FDM archivée : `evidence/uc/ROUND1_event1_N_response.json`.

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "6f58677b-be75-4481-94be-a2c3c13e5463",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "12345678958",
  "estNo": "2000000042",
  "financials": [
    {
      "amount": 11.52,
      "amountType": "PAYMENT",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    },
    {
      "amount": -0.02,
      "amountType": "ROUNDING",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:45.493+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": 11.52,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "1",
          "departmentName": "Desserts",
          "productId": "25003",
          "productName": "Dame Blanche",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 11.52,
          "vats": [
            {
              "label": "B",
              "price": 11.52
            }
          ]
        }
      }
    ],
    "transactionTotal": 11.52
  },
  "vatNo": "BE0000000097"
}
```

Ticket imprimé (`evidence/tickets/ROUND1_ticket.txt`, rendu texte du flux ESC/POS) :

```text
            TICKET DE CAISSE T.V.A.
The FPS Finances Experience
Unite d'etablissement 2000000042
04.10.2026                                 11:01
Utilisateur: Test 12345678958
Terminal: POS-1-BAR
Ticketnr.: 000001
POS Serial: CPOS0031234567
Input device MAC: F0:1D:3D:CC:13:C8
Logiciel de caisse v.: pili-pos/1.0.0
................................................
1 Dame Blanche                          11,52  B
TOTAL :                                   €11,52
................................................
Mode de paiement :
Especes                                    11,52
Arrondi                                    -0,02
% TVA   CODE          HT       TVA      Brut
12,00%  B          10,29      1,23     11,52
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-04T09:01:45Z;
Event: N; eventCounter: 1;
totalCounter: 1; shortSignature:
c7474d7a65650e0cb1410ccd3ac08113cfa69212;

 [QR: https://www.gs2.be/M0000100001AbCdEfGhI]  


Denomination legale : The FPS Finances
Experience
Boulevard Albert II 33, 1030 Brussels
TVA: BE0000000097




✂-----------------------------------------------
```
<!-- pili-pos-evidence:end -->

## ROUND2_Arrondi_cash_Tapas_Variation

**Test de base 1, situation 2.** Vente directe de 1x Tapas Variation
(3,34 EUR), payee uniquement en especes. Attendu : a payer 3,35 EUR,
arrondi +0,01 EUR.

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - situation rejouée par `crates/pos-engine/tests/spf_e2e_rounding.rs::round2_tapas_variation_cash`, par la commande que l'écran d'encaissement appelle. Requête **construite et archivée par pili-pos** (`evidence/uc/ROUND2_event1_N_request.json`) ; réponse FDM archivée : `evidence/uc/ROUND2_event1_N_response.json`.

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "e366137d-6ae7-4782-82dc-db7c41bd7ae5",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "12345678958",
  "estNo": "2000000042",
  "financials": [
    {
      "amount": 3.34,
      "amountType": "PAYMENT",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    },
    {
      "amount": 0.01,
      "amountType": "ROUNDING",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:45.493+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": 3.34,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "1",
          "departmentName": "Desserts",
          "productId": "28007",
          "productName": "Tapas Variation",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 3.34,
          "vats": [
            {
              "label": "B",
              "price": 3.34
            }
          ]
        }
      }
    ],
    "transactionTotal": 3.34
  },
  "vatNo": "BE0000000097"
}
```

Ticket imprimé (`evidence/tickets/ROUND2_ticket.txt`, rendu texte du flux ESC/POS) :

```text
            TICKET DE CAISSE T.V.A.
The FPS Finances Experience
Unite d'etablissement 2000000042
04.10.2026                                 11:01
Utilisateur: Test 12345678958
Terminal: POS-1-BAR
Ticketnr.: 000001
POS Serial: CPOS0031234567
Input device MAC: F0:1D:3D:CC:13:C8
Logiciel de caisse v.: pili-pos/1.0.0
................................................
1 Tapas Variation                        3,34  B
TOTAL :                                    €3,34
................................................
Mode de paiement :
Especes                                     3,34
Arrondi                                     0,01
% TVA   CODE          HT       TVA      Brut
12,00%  B           2,98      0,36      3,34
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-04T09:01:45Z;
Event: N; eventCounter: 1;
totalCounter: 1; shortSignature:
c7474d7a65650e0cb1410ccd3ac08113cfa69212;

 [QR: https://www.gs2.be/M0000100001AbCdEfGhI]  


Denomination legale : The FPS Finances
Experience
Boulevard Albert II 33, 1030 Brussels
TVA: BE0000000097




✂-----------------------------------------------
```
<!-- pili-pos-evidence:end -->

## ROUND3_Arrondi_cash_vente_combinee

**Test de base 1, situation 3.** Vente directe de 1x Dame Blanche et
1x Tapas Variation (14,86 EUR), payee uniquement en especes. Attendu :
a payer 14,85 EUR, arrondi -0,01 EUR.

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - situation rejouée par `crates/pos-engine/tests/spf_e2e_rounding.rs::round3_both_desserts_cash`, par la commande que l'écran d'encaissement appelle. Requête **construite et archivée par pili-pos** (`evidence/uc/ROUND3_event1_N_request.json`) ; réponse FDM archivée : `evidence/uc/ROUND3_event1_N_response.json`.

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "8cbd27ef-4339-48a0-aaeb-639ed4dc46d0",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "12345678958",
  "estNo": "2000000042",
  "financials": [
    {
      "amount": 14.86,
      "amountType": "PAYMENT",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    },
    {
      "amount": -0.01,
      "amountType": "ROUNDING",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:45.493+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": 11.52,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "1",
          "departmentName": "Desserts",
          "productId": "25003",
          "productName": "Dame Blanche",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 11.52,
          "vats": [
            {
              "label": "B",
              "price": 11.52
            }
          ]
        }
      },
      {
        "lineTotal": 3.34,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "1",
          "departmentName": "Desserts",
          "productId": "28007",
          "productName": "Tapas Variation",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 3.34,
          "vats": [
            {
              "label": "B",
              "price": 3.34
            }
          ]
        }
      }
    ],
    "transactionTotal": 14.86
  },
  "vatNo": "BE0000000097"
}
```

Ticket imprimé (`evidence/tickets/ROUND3_ticket.txt`, rendu texte du flux ESC/POS) :

```text
            TICKET DE CAISSE T.V.A.
The FPS Finances Experience
Unite d'etablissement 2000000042
04.10.2026                                 11:01
Utilisateur: Test 12345678958
Terminal: POS-1-BAR
Ticketnr.: 000001
POS Serial: CPOS0031234567
Input device MAC: F0:1D:3D:CC:13:C8
Logiciel de caisse v.: pili-pos/1.0.0
................................................
1 Dame Blanche                          11,52  B
1 Tapas Variation                        3,34  B
TOTAL :                                   €14,86
................................................
Mode de paiement :
Especes                                    14,86
Arrondi                                    -0,01
% TVA   CODE          HT       TVA      Brut
12,00%  B          13,27      1,59     14,86
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-04T09:01:45Z;
Event: N; eventCounter: 1;
totalCounter: 1; shortSignature:
c7474d7a65650e0cb1410ccd3ac08113cfa69212;

 [QR: https://www.gs2.be/M0000100001AbCdEfGhI]  


Denomination legale : The FPS Finances
Experience
Boulevard Albert II 33, 1030 Brussels
TVA: BE0000000097




✂-----------------------------------------------
```
<!-- pili-pos-evidence:end -->

## ROUND4_Cheques_repas_puis_cash

**Test de base 2, situation 4.** Memes produits (14,86 EUR) ; 11,52 EUR
en cheques-repas (jamais arrondis), le solde 3,34 EUR en especes.
Attendu : a payer 3,35 EUR en especes, arrondi +0,01 EUR.

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - situation rejouée par `crates/pos-engine/tests/spf_e2e_rounding.rs::round4_meal_vouchers_then_cash`, par la commande que l'écran d'encaissement appelle. Requête **construite et archivée par pili-pos** (`evidence/uc/ROUND4_event1_N_request.json`) ; réponse FDM archivée : `evidence/uc/ROUND4_event1_N_response.json`.

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "f4d9b1fe-7d63-489f-a207-f42269cf52a8",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "12345678958",
  "estNo": "2000000042",
  "financials": [
    {
      "amount": 11.52,
      "amountType": "PAYMENT",
      "id": "3",
      "inputMethod": "MANUAL",
      "name": "Meal voucher",
      "type": "CHEQUE_MEAL"
    },
    {
      "amount": 3.34,
      "amountType": "PAYMENT",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    },
    {
      "amount": 0.01,
      "amountType": "ROUNDING",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:45.493+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": 11.52,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "1",
          "departmentName": "Desserts",
          "productId": "25003",
          "productName": "Dame Blanche",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 11.52,
          "vats": [
            {
              "label": "B",
              "price": 11.52
            }
          ]
        }
      },
      {
        "lineTotal": 3.34,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "1",
          "departmentName": "Desserts",
          "productId": "28007",
          "productName": "Tapas Variation",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 3.34,
          "vats": [
            {
              "label": "B",
              "price": 3.34
            }
          ]
        }
      }
    ],
    "transactionTotal": 14.86
  },
  "vatNo": "BE0000000097"
}
```

Ticket imprimé (`evidence/tickets/ROUND4_ticket.txt`, rendu texte du flux ESC/POS) :

```text
            TICKET DE CAISSE T.V.A.
The FPS Finances Experience
Unite d'etablissement 2000000042
04.10.2026                                 11:01
Utilisateur: Test 12345678958
Terminal: POS-1-BAR
Ticketnr.: 000001
POS Serial: CPOS0031234567
Input device MAC: F0:1D:3D:CC:13:C8
Logiciel de caisse v.: pili-pos/1.0.0
................................................
1 Dame Blanche                          11,52  B
1 Tapas Variation                        3,34  B
TOTAL :                                   €14,86
................................................
Mode de paiement :
Cheques-repas                              11,52
Especes                                     3,34
Arrondi                                     0,01
% TVA   CODE          HT       TVA      Brut
12,00%  B          13,27      1,59     14,86
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-04T09:01:45Z;
Event: N; eventCounter: 1;
totalCounter: 1; shortSignature:
c7474d7a65650e0cb1410ccd3ac08113cfa69212;

 [QR: https://www.gs2.be/M0000100001AbCdEfGhI]  


Denomination legale : The FPS Finances
Experience
Boulevard Albert II 33, 1030 Brussels
TVA: BE0000000097




✂-----------------------------------------------
```
<!-- pili-pos-evidence:end -->

## ROUND5_Multi_paiements

**Test de base 3, situation 5.** Memes produits (14,86 EUR) ; 3,13 EUR
en cheques-repas, 3,97 EUR par carte de debit, le solde 7,76 EUR en
especes. Attendu : 7,75 EUR en especes, arrondi -0,01 EUR ; seul le
paiement en especes est arrondi.

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - situation rejouée par `crates/pos-engine/tests/spf_e2e_rounding.rs::round5_meal_vouchers_card_and_cash`, par la commande que l'écran d'encaissement appelle. Requête **construite et archivée par pili-pos** (`evidence/uc/ROUND5_event1_N_request.json`) ; réponse FDM archivée : `evidence/uc/ROUND5_event1_N_response.json`.

```json
{
  "bookingDate": "2026-10-04",
  "bookingPeriodId": "44d6523c-39d1-420c-8031-df3bbcc334fa",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "12345678958",
  "estNo": "2000000042",
  "financials": [
    {
      "amount": 3.13,
      "amountType": "PAYMENT",
      "id": "3",
      "inputMethod": "MANUAL",
      "name": "Meal voucher",
      "type": "CHEQUE_MEAL"
    },
    {
      "amount": 3.97,
      "amountType": "PAYMENT",
      "id": "2",
      "inputMethod": "MANUAL",
      "name": "Card",
      "type": "CARD_DEBIT"
    },
    {
      "amount": 7.76,
      "amountType": "PAYMENT",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    },
    {
      "amount": -0.01,
      "amountType": "ROUNDING",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-04T11:01:45.493+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": 11.52,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "1",
          "departmentName": "Desserts",
          "productId": "25003",
          "productName": "Dame Blanche",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 11.52,
          "vats": [
            {
              "label": "B",
              "price": 11.52
            }
          ]
        }
      },
      {
        "lineTotal": 3.34,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "1",
          "departmentName": "Desserts",
          "productId": "28007",
          "productName": "Tapas Variation",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 3.34,
          "vats": [
            {
              "label": "B",
              "price": 3.34
            }
          ]
        }
      }
    ],
    "transactionTotal": 14.86
  },
  "vatNo": "BE0000000097"
}
```

Ticket imprimé (`evidence/tickets/ROUND5_ticket.txt`, rendu texte du flux ESC/POS) :

```text
            TICKET DE CAISSE T.V.A.
The FPS Finances Experience
Unite d'etablissement 2000000042
04.10.2026                                 11:01
Utilisateur: Test 12345678958
Terminal: POS-1-BAR
Ticketnr.: 000001
POS Serial: CPOS0031234567
Input device MAC: F0:1D:3D:CC:13:C8
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
fdmId: MCK00000001;
fdmDateTime: 2026-10-04T09:01:45Z;
Event: N; eventCounter: 1;
totalCounter: 1; shortSignature:
c7474d7a65650e0cb1410ccd3ac08113cfa69212;

 [QR: https://www.gs2.be/M0000100001AbCdEfGhI]  


Denomination legale : The FPS Finances
Experience
Boulevard Albert II 33, 1030 Brussels
TVA: BE0000000097




✂-----------------------------------------------
```
<!-- pili-pos-evidence:end -->

---

# Recapitulatif

| Famille | UC (spec) | UC dans le perimetre v1.0.0 | Lecture des fixtures | Rejeu applicatif (build `cert`) |
|---|---|---|:-:|:-:|
| Basic Set v1.8 | 22 | 20 (hors M112A et M150/event I) | OK (22/22) | OK : 18 UC rejoues et compares a la fixture dans `spf_e2e_fixtures.rs` (M110, M111, M112B, M121, M122, M123, M130, M131, M132, M133, M140, M141, M160, M161, M170, M171, M171B, M172) ; M112A n'est pas un parcours propose (remboursement partiel toujours en ticket separe, parcours M112B) ; Q100/Q110 : `client_status.rs` |
| Reports v1.3 | 5 | 5 | OK (5/5) | OK : journee T1–T8 jouee puis M180, M182 et M184 compares a la fixture dans `spf_e2e_report_fixtures.rs` ; M181/M183 signes par le Z combine de M184 |
| **Total** | **27** | **25** | **OK** | **OK** |

Preuves d'annexe : **55 paires requete/reponse** construites et
archivees par la build `cert` pour le Basic Set et les rapports
(`docs/SUBMISSION_ARTIFACTS/evidence/uc/`), plus un fichier
`<UC>_check.json` par UC du Basic Set rejoue (comparaison champ par
champ avec la fixture), **5 paires** pour les situations d'arrondi
(`ROUND1` a `ROUND5`, avec leur ticket dans `evidence/tickets/`) et 10
paires pour six UC de l'Advanced Set rejoues par le meme harnais
(UC223, UC225A, UC229B, UC230, UC4109, UC4119 ; voir *COUVERTURE_ADVANCED_SET_FR.md*). M150 n'y
figure pas : la build `cert` ne peut pas emettre d'event I.

Pour le detail des couvertures, voir *COUVERTURE_FIXTURES_FR.md* et *COUVERTURE_RAPPORTS_FR.md*.
