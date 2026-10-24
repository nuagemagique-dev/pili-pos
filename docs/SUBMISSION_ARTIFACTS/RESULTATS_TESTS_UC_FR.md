# Resultats des tests UC - pili-pos v1.0.0

**Date :** 2026-09-25
**Reference :** §IV du manuel de certification (Procedure de communication GraphQL).

Ce document parcourt les UC obligatoires du Basic Set v1.8 (22 UC
spec) + les 5 UC Reports v1.3 + les cinq situations d'arrondi du manuel (ROUND1 a ROUND5). Aucun UC n'est hors périmètre : **M122** (changement de centre de coût, transfert
de table) et **M132** (correction de paiement) sont rejoués, voir leur section ; **M112A** n'est pas un parcours
proposé par pili-pos (le remboursement partiel est toujours émis en
ticket séparé, parcours M112B). La facture (**M150**, événement I) est
rejouée comme les autres UC ; les deux UC de facture de l'Advanced Set
(**UC4121** et **UC4122**) le sont aussi, dans la section qui suit les
rapports, ainsi que les trois UC de bon cadeau à usage multiple (**UC260**,
**UC261** et **UC262**) dans la section qui suit les factures. Pour chaque UC, il fournit :

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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "503094d8-8ae1-4156-9730-504a19c71ada",
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
  "posDateTime": "2026-10-18T12:59:03.716+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "d54b6426-56d9-46ad-a2e6-67ba958a1413",
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
  "posDateTime": "2026-10-18T12:59:03.725+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "ef06e8ae-2b82-4fe2-aa42-5bb12242d3aa",
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
  "posDateTime": "2026-10-18T12:59:03.725+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "c729c4bc-8856-41ef-8681-e7075da3e917",
  "costCenter": {
    "id": "T01",
    "reference": "O1",
    "type": "TABLE"
  },
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:03.713+02:00",
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

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m122_cost_center_change`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M122_event2_P_request.json`) :

```json
{
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "85daceb0-3d09-42e3-a221-289c7644ae00",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:03.724+02:00",
  "posFiscalTicketNo": 2,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "NONE",
  "transfer": {
    "from": [
      {
        "costCenter": {
          "id": "T01",
          "reference": "O1",
          "type": "TABLE"
        },
        "transaction": {
          "transactionLines": [
            {
              "lineTotal": -28.0,
              "lineType": "SINGLE_PRODUCT",
              "mainProduct": {
                "departmentId": "22",
                "departmentName": "Main Dishes",
                "negQuantityReason": "COST_CENTER_CHANGE",
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
            },
            {
              "lineTotal": -12.0,
              "lineType": "SINGLE_PRODUCT",
              "mainProduct": {
                "departmentId": "10",
                "departmentName": "Aperitifs",
                "negQuantityReason": "COST_CENTER_CHANGE",
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
          "transactionTotal": -40.0
        }
      }
    ],
    "to": [
      {
        "costCenter": {
          "id": "T05",
          "reference": "O1",
          "type": "TABLE"
        },
        "transaction": {
          "transactionLines": [
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
            },
            {
              "lineTotal": 12.0,
              "lineType": "SINGLE_PRODUCT",
              "mainProduct": {
                "departmentId": "10",
                "departmentName": "Aperitifs",
                "productId": "10006",
                "productName": "Dry Martini",
                "quantity": 1.0,
                "quantityType": "PIECE",
                "unitPrice": 12.0,
                "vats": [
                  {
                    "label": "A",
                    "price": 12.0
                  }
                ]
              }
            }
          ],
          "transactionTotal": 40.0
        }
      }
    ]
  },
  "vatNo": "BE0000000097"
}
```

Autres events du scénario, archivés de même : `evidence/uc/M122_event1_P_request.json`, `evidence/uc/M122_event3_N_request.json`. Réponses FDM archivées : fichiers `*_response.json` correspondants.

**Comparaison avec la fixture** (`evidence/uc/M122_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Aucun écart.

<!-- pili-pos-evidence:end -->

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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "01fe433a-e07c-4bb4-90cd-4c14a76443cb",
  "costCenter": {
    "id": "T01",
    "reference": "O1",
    "type": "TABLE"
  },
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:03.721+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "64320725-ea4f-41dc-bea4-dd7d0fb5d206",
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
  "posDateTime": "2026-10-18T12:59:03.707+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "d628b12c-9ed0-4c44-92c5-09631e8c2ec0",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "drawer": {
    "id": "DRAWER1",
    "name": "Tiroir principal"
  },
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:04.425+02:00",
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

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m132_payment_correction`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M132_event2_F_request.json`) :

```json
{
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "1f39b930-1b9e-42f7-9256-a8573dda4d83",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "fdmRef": {
    "eventCounter": 14,
    "eventLabel": "N",
    "fdmDateTime": "2024-07-29T11:35:09.000Z",
    "fdmId": "FOD01000001",
    "totalCounter": 50
  },
  "financials": [
    {
      "amount": -52.0,
      "amountType": "PAYMENT",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    },
    {
      "amount": 52.0,
      "amountType": "PAYMENT",
      "id": "2",
      "inputMethod": "MANUAL",
      "name": "Card",
      "type": "CARD_DEBIT"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:07.168+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "NONE",
  "vatNo": "BE0000000097"
}
```

Events préalables du scénario, archivés de même : `evidence/uc/M132_event1_N_request.json`. Réponses FDM archivées : fichiers `*_response.json` correspondants.

**Comparaison avec la fixture** (`evidence/uc/M132_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `financials[0].type` | `CARD_CREDIT` | `CARD_DEBIT` | sans terminal relié, l'opérateur ne distingue pas débit et crédit : une carte saisie à la main est CARD_DEBIT, MANUAL (*REPONSES_MANUEL_CERT_FR.md* §III.F) ; la comparaison ne porte que sur le type, le montant et le type de montant, la saisie MANUAL (AUTOMATIC dans la fixture) n'est donc pas comparée |

<!-- pili-pos-evidence:end -->

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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "a17d19d2-b177-4013-8c40-c5caf89a2dd0",
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
  "posDateTime": "2026-10-18T12:59:04.515+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "9932f046-cb68-4643-ba81-0af15085b5c1",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "80113078968",
  "estNo": "2000000042",
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:04.475+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "9bb4c309-2ccc-4874-84f3-519a90af7b38",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "84022899837",
  "estNo": "2000000042",
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:04.527+02:00",
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

## M150_signInvoice

**Mutation invoquee :** `signInvoice`

**Scenario :**

```
Invoice for customer (Id 100 - customerVatNo BE0308357159)
for tickets
    N 14/50
    N 16/52
```

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1012` | OK |
| `posDateTime` | `2024-07-29T16:22:39.000+02:00` | OK |
| `terminalId` | `TER-1-BAR` | OK |
| `deviceId` | `b54a614f-39cc-4a7b-bd9f-aa6b693d769c` | OK |
| `vatNo` | `BE0000000097` | OK |
| `estNo` | `2000000042` | OK |
| `posSwVersion` | `1.8.3` | OK |
| `bookingPeriodId` | `dffcd829-a0e5-41ca-a0ae-9eb887f95637` | OK |
| `bookingDate` | `2024-07-29` | OK |
| `ticketMedium` | `PAPER` | OK |
| `employeeId` | `75061189702` | OK |
| `customerVatNo` | `BE0308357159` | OK |
| `costCenter.id` | `100` | OK |
| `costCenter.type` | `CUSTOMER` | OK |
| `costCenter.reference` | `FIN` | OK |
| `invoiceNo` | `20240005` | OK |
| `fdmRefs[0].fdmId` | `FOD01000001` | OK |
| `fdmRefs[0].eventLabel` | `N` | OK |
| `fdmRefs[0].eventCounter` | `14` | OK |
| `fdmRefs[0].totalCounter` | `50` | OK |
| `fdmRefs[1].fdmId` | `FOD01000001` | OK |
| `fdmRefs[1].eventLabel` | `N` | OK |
| `fdmRefs[1].eventCounter` | `16` | OK |
| `fdmRefs[1].totalCounter` | `52` | OK |
| `fdmRef.fdmId` | `FOD01000001` | OK |
| `fdmRef.eventLabel` | `I` | OK |
| `fdmRef.eventCounter` | `1` | OK |
| `fdmRef.totalCounter` | `62` | OK |

**Request (Variables.json) :**

```json
{
  "data": {
    "language": "NL",
    "vatNo": "BE0000000097",
    "estNo": "2000000042",
    "posId": "CPOS0031234567",
    "posFiscalTicketNo": 1012,
    "posDateTime": "2024-07-29T16:22:39.000+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "PAPER",
    "employeeId": "75061189702",
    "customerVatNo": "BE0308357159",
    "costCenter": {
      "id": "100",
      "type": "CUSTOMER",
      "reference": "FIN"
    },
    "invoiceNo": "20240005",
    "fdmRefs": [
      {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-04-29T11:35:09.000Z",
        "eventLabel": "N",
        "eventCounter": 14,
        "totalCounter": 50
      },
      {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T11:48:04.000Z",
        "eventLabel": "N",
        "eventCounter": 16,
        "totalCounter": 52
      }
    ]
  }
}
```

**Response FDM :**

```json
{
  "data": {
    "signInvoice": {
      "posId": "CPOS0031234567",
      "posFiscalTicketNo": 1012,
      "posDateTime": "2024-07-29T16:22:39.000+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "INVOICE",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T14:22:41.000Z",
        "eventLabel": "I",
        "eventCounter": 1,
        "totalCounter": 62
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

**Rejeu applicatif :** `crates/pos-engine/tests/spf_e2e_fixtures.rs::m150_invoice` (build `cert`, comme tous les UC ci-dessus). Le rejeu fait deux ventes (événements N), établit la facture par la commande de l'écran de facturation et compare la requête archivée à la fixture. Champs métier égaux (`costCenter.type` et `costCenter.reference`, `customerVatNo`, libellés des références). Chaque `fdmRefs[i]` (`fdmId`, `fdmDateTime`, `eventLabel`, `eventCounter`, `totalCounter`) est comparé à la référence que le module de contrôle a répondue pour la vente correspondante, dans l'ordre. Différences voulues, propres à la session de test : identifiant du centre de coût (`C` suivi du numéro du client, contre `100` dans le scénario), numéro de facture (année et séquence continue, `20260001` à la place de `20240005`) et compteurs des références. Voir *FACTURATION_PEPPOL_FR.md*.

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_fixtures.rs::m150_invoice`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/M150_event3_I_request.json`) :

```json
{
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "c0d5f6d3-b4fb-4a73-9588-f2c10f9b544f",
  "costCenter": {
    "id": "C1",
    "reference": "FIN",
    "type": "CUSTOMER"
  },
  "customerVatNo": "BE0308357159",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "fdmRefs": [
    {
      "eventCounter": 15,
      "eventLabel": "N",
      "fdmDateTime": "2024-07-29T11:00:08.000Z",
      "fdmId": "FOD01000001",
      "totalCounter": 59
    },
    {
      "eventCounter": 15,
      "eventLabel": "N",
      "fdmDateTime": "2024-07-29T11:00:08.000Z",
      "fdmId": "FOD01000001",
      "totalCounter": 59
    }
  ],
  "invoiceNo": "20260001",
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:04.545+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "vatNo": "BE0000000097"
}
```

Events préalables du scénario, archivés de même : `evidence/uc/M150_event1_N_request.json`, `evidence/uc/M150_event2_N_request.json`. Réponses FDM archivées : fichiers `*_response.json` correspondants.

**Comparaison avec la fixture** (`evidence/uc/M150_check.json`) : champs métier identiques à la fixture ; champs d'identification (posId, deviceId, numéro de ticket, dates, période de booking) contrôlés sur leur format ; libellés présents.

Aucun écart.

<!-- pili-pos-evidence:end -->

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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "eaa005f6-28f7-4414-a176-a00f0ac602ac",
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
  "posDateTime": "2026-10-18T12:59:04.560+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "9ecc9945-7e21-4ecc-a5ce-0d83bdf9d46f",
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
  "posDateTime": "2026-10-18T12:59:04.514+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "5c7f031a-85b0-4720-b6dd-5b2935135869",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:05.042+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "94f382f2-ba80-416b-bd92-2be445fd3c8e",
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
  "posDateTime": "2026-10-18T12:59:05.233+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "a90825e4-5772-4447-81f1-4d59a4ba7873",
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
  "posDateTime": "2026-10-18T12:59:05.117+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "ed4acbef-a451-4d0e-b7a0-73a520f0fbde",
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
  "posDateTime": "2026-10-18T12:59:05.162+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "c9fda6c9-22d7-42f3-9bc1-f7a4283e8b5c",
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
  "posDateTime": "2026-10-18T12:59:09.242+02:00",
  "posDevices": [
    {
      "firstPosDateTime": "2026-10-18T12:59:09.186+02:00",
      "lastPosDateTime": "2026-10-18T12:59:09.233+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "99f4ffde-b402-4594-b755-d61ed60294ac",
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
  "posDateTime": "2026-10-18T12:59:09.240+02:00",
  "posDevices": [
    {
      "firstPosDateTime": "2026-10-18T12:59:09.183+02:00",
      "lastPosDateTime": "2026-10-18T12:59:09.232+02:00",
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
      "firstPosDateTime": "2026-10-18T12:59:09.183+02:00",
      "lastPosDateTime": "2026-10-18T12:59:09.201+02:00",
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
      "firstPosDateTime": "2026-10-18T12:59:09.211+02:00",
      "lastPosDateTime": "2026-10-18T12:59:09.232+02:00",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "1f951e29-3662-412c-82ec-9bb427e4c6f5",
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
  "posDateTime": "2026-10-18T12:59:09.236+02:00",
  "posDevices": [
    {
      "firstPosDateTime": "2026-10-18T12:59:09.187+02:00",
      "lastPosDateTime": "2026-10-18T12:59:09.232+02:00",
      "posId": "CPOS0031234567",
      "terminalId": "POS-1-BAR"
    }
  ],
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "reportBookingDate": "2026-10-18",
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "1f951e29-3662-412c-82ec-9bb427e4c6f5",
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
  "posDateTime": "2026-10-18T12:59:09.236+02:00",
  "posDevices": [
    {
      "firstPosDateTime": "2026-10-18T12:59:09.187+02:00",
      "lastPosDateTime": "2026-10-18T12:59:09.232+02:00",
      "posId": "CPOS0031234567",
      "terminalId": "POS-1-BAR"
    }
  ],
  "posFiscalTicketNo": 2,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "reportBookingDate": "2026-10-18",
  "reportNo": 1,
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "users": [
    {
      "employeeId": "75061189702",
      "firstPosDateTime": "2026-10-18T12:59:09.187+02:00",
      "lastPosDateTime": "2026-10-18T12:59:09.201+02:00",
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
      "firstPosDateTime": "2026-10-18T12:59:09.210+02:00",
      "lastPosDateTime": "2026-10-18T12:59:09.232+02:00",
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

# Factures de l'Advanced Set (UC4121, UC4122)

Les deux UC de facture de l'Advanced Set v1.6 sont rejoués par `crates/pos-engine/tests/invoice_advanced.rs`, par la commande de facturation de l'écran, contre le module de contrôle simulé. Le scénario paie les ventes en crédit client ; ce moyen de paiement n'existe pas dans le produit, les ventes sont donc encaissées au comptant (la demande de facture ne dépend pas du moyen de paiement des ventes). Le test compare la requête archivée à la fixture (champs métier égaux, références des ventes comparées valeur par valeur à celles que le module de contrôle a répondues) et vérifie le format du numéro de facture.

## UC4121_CustomerInvoice

**Mutation invoquee :** `signInvoice`

**Scenario (fixture, en anglais) :**

```
Sale of 2 carpaccio (unit price 20EUR) and 1 bottle of Chardonnay (unit price 40EUR), payment type CUSTOMER CREDIT
Sale of 2 dry Martini (unit price 12EUR), payment type CUSTOMER CREDIT
Sale of 2 dame blanche (unit price 11,52EUR), payment type CUSTOMER CREDIT
End of the month: invoice with reference to tickets
```

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - scénario rejoué par `crates/pos-engine/tests/invoice_advanced.rs::uc4121_an_invoice_of_three_sales` avec des ventes au comptant génériques à la place des étapes de vente de la fixture (le crédit client n'existe pas dans le produit ; la demande de facture n'en dépend pas). Requête **construite et archivée par pili-pos** (`evidence/uc/UC4121_event4_I_request.json`), réponse FDM archivée : `evidence/uc/UC4121_event4_I_response.json`.

```json
{
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "bb82341a-d358-457a-bc9b-813b5e93bc5f",
  "costCenter": {
    "id": "C1",
    "reference": "JANSSENS",
    "type": "CUSTOMER"
  },
  "customerVatNo": "BE0308357159",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "84022899837",
  "estNo": "2000000042",
  "fdmRefs": [
    {
      "eventCounter": 1,
      "eventLabel": "N",
      "fdmDateTime": "2026-10-18T10:59:02Z",
      "fdmId": "MCK00000001",
      "totalCounter": 1
    },
    {
      "eventCounter": 2,
      "eventLabel": "N",
      "fdmDateTime": "2026-10-18T10:59:02Z",
      "fdmId": "MCK00000001",
      "totalCounter": 2
    },
    {
      "eventCounter": 3,
      "eventLabel": "N",
      "fdmDateTime": "2026-10-18T10:59:02Z",
      "fdmId": "MCK00000001",
      "totalCounter": 3
    }
  ],
  "invoiceNo": "20260001",
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:02.906+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "vatNo": "BE0000000097"
}
```

Events préalables du scénario, archivés de même : `evidence/uc/UC4121_event1_N_request.json`, `evidence/uc/UC4121_event2_N_request.json`, `evidence/uc/UC4121_event3_N_request.json`.

Comparaison avec la fixture : champs métier identiques (`costCenter.type` et `costCenter.reference`, `customerVatNo`, libellés des `fdmRefs`) ; chaque `fdmRefs[i]` (`fdmId`, `fdmDateTime`, `eventLabel`, `eventCounter`, `totalCounter`) est comparé à la référence que le module de contrôle a répondue pour la vente correspondante, dans l'ordre ; numéro de facture contrôlé sur son format (année et séquence). Écarts voulus : numéro de facture, identifiant du centre de coût (`C` suivi du numéro du client) et compteurs, qui sont ceux de la session de test.
<!-- pili-pos-evidence:end -->

## UC4122_Creditnote

**Mutation invoquee :** `signInvoice`

**Scenario (fixture, en anglais) :**

```
Refund of ticket, sale of 4 dame blanche (unit price 11.52EUR) had already been invoiced
Sale 1 of 4 dry Martini (unit price:12EUR)
Sale 2 of 4 burger of the chef (unit price: 28 EUR) and 4 Coca-Cola (unit price 3.04EUR)
Invoice of refund and sale 1 & 2
```

Le rejeu respecte la précondition : la vente de 46,08 EUR est d'abord facturée (première facture), puis remboursée par un événement N négatif ; la seconde facture reprend ce remboursement et les deux ventes (total 46,08 EUR en négatif, 48,00 EUR et 120,00 EUR en positif). La seconde tentative de facturer la vente d'origine est refusée (une vente, une facture).

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - scénario rejoué par `crates/pos-engine/tests/invoice_advanced.rs::uc4122_an_invoice_of_a_refund_and_two_sales` avec des ventes au comptant génériques à la place des étapes de vente de la fixture (le crédit client n'existe pas dans le produit ; la demande de facture n'en dépend pas). Requête **construite et archivée par pili-pos** (`evidence/uc/UC4122_event6_I_request.json`), réponse FDM archivée : `evidence/uc/UC4122_event6_I_response.json`.

```json
{
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "b4ca254c-054a-4f78-ac48-a08bebf628a7",
  "costCenter": {
    "id": "C1",
    "reference": "AUDI",
    "type": "CUSTOMER"
  },
  "customerVatNo": "BE0308357159",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "84022899837",
  "estNo": "2000000042",
  "fdmRefs": [
    {
      "eventCounter": 2,
      "eventLabel": "N",
      "fdmDateTime": "2026-10-18T10:59:02Z",
      "fdmId": "MCK00000001",
      "totalCounter": 3
    },
    {
      "eventCounter": 3,
      "eventLabel": "N",
      "fdmDateTime": "2026-10-18T10:59:02Z",
      "fdmId": "MCK00000001",
      "totalCounter": 4
    },
    {
      "eventCounter": 4,
      "eventLabel": "N",
      "fdmDateTime": "2026-10-18T10:59:02Z",
      "fdmId": "MCK00000001",
      "totalCounter": 5
    }
  ],
  "invoiceNo": "20260002",
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:02.912+02:00",
  "posFiscalTicketNo": 2,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "vatNo": "BE0000000097"
}
```

Events préalables du scénario, archivés de même : `evidence/uc/UC4122_event1_N_request.json`, `evidence/uc/UC4122_event2_I_request.json`, `evidence/uc/UC4122_event3_N_request.json`, `evidence/uc/UC4122_event4_N_request.json`, `evidence/uc/UC4122_event5_N_request.json`.

Comparaison avec la fixture : champs métier identiques (`costCenter.type` et `costCenter.reference`, `customerVatNo`, libellés des `fdmRefs`) ; chaque `fdmRefs[i]` (`fdmId`, `fdmDateTime`, `eventLabel`, `eventCounter`, `totalCounter`) est comparé à la référence que le module de contrôle a répondue pour la vente correspondante, dans l'ordre ; numéro de facture contrôlé sur son format (année et séquence). Écarts voulus : numéro de facture, identifiant du centre de coût (`C` suivi du numéro du client) et compteurs, qui sont ceux de la session de test.
<!-- pili-pos-evidence:end -->

---

# Bons cadeaux à usage multiple de l'Advanced Set (UC260, UC261, UC262)

Les trois UC de bon à usage multiple (MPV) de l'Advanced Set v1.6 sont rejoués par `crates/pos-engine/tests/spf_e2e_gift_vouchers.rs`, par les commandes que l'écran appelle (ligne de bon cadeau, encaissement fractionné), contre le module de contrôle simulé qui rejoue la réponse de la fixture. Le test compare la requête archivée à celle de la fixture champ par champ : les lignes de paiement (`VOUCHER_STORE`, saisie, montants) doivent être identiques ; tout autre écart est listé avec son motif, et un écart non listé, ou listé et disparu, fait échouer le test. La fonction est décrite dans *BONS_CADEAUX_FR.md*. Les bons à usage unique (UC270, UC271) et la conversion d'un bon en espèces (UC4105) ne sont pas offerts en v1.0.0 (*MATRICE_TRACABILITE_FR.md*).

## UC260_MPV_Sale

**Mutation invoquee :** `signSale`

**Scenario (fixture, en anglais) :**

```
Purchase of MPV - 50 EUR - payment CARD
```

**Test automatise :** `crates/pos-engine/tests/spf_e2e_gift_vouchers.rs::uc260_mpv_sale`. La vente d'un bon de 50,00 EUR payée par carte produit une ligne T.V.A. X et une ligne de paiement ; le bon existe une fois la vente signée.

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_gift_vouchers.rs::uc260_mpv_sale`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/UC260_event1_N_request.json`) :

```json
{
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "4f228e06-e7f5-49dd-a7cd-59c7cbc2408d",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "financials": [
    {
      "amount": 50.0,
      "amountType": "PAYMENT",
      "id": "2",
      "inputMethod": "MANUAL",
      "name": "Card",
      "type": "CARD_DEBIT"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:08.330+02:00",
  "posFiscalTicketNo": 1,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": 50.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "GV",
          "departmentName": "Bons cadeaux",
          "productId": "990001",
          "productName": "Bon cadeau",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 50.0,
          "vats": [
            {
              "label": "X",
              "price": 50.0
            }
          ]
        }
      }
    ],
    "transactionTotal": 50.0
  },
  "vatNo": "BE0000000097"
}
```

**Comparaison avec la fixture** (`evidence/uc/UC260_check.json`) : champs métier identiques, dont les lignes de paiement `VOUCHER_STORE` (type, saisie, montant).

Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `transaction.transactionLines[0].departmentId` | `400` | `GV` | la ligne du bon est dans le département GV (Bons cadeaux), pas dans le département 400 de l'installation de test |
| `transaction.transactionLines[0].productId` | `40001` | `990001` | le bon est le produit système « Bon cadeau » (990001), pas le produit 40001 de l'installation de test |

<!-- pili-pos-evidence:end -->

## UC261_MPV_Use

**Mutation invoquee :** `signSale` (commande de table puis vente)

**Scenario (fixture, en anglais) :**

```
Client comes to diner: 2 business menu all-in (total price 116 EUR)
and pays with a Multi Purpose Voucher (value 50 EUR).
The remaining balance is paid in cash.

OPERATION 1: order on table 1: 2 business menu all-in
OPERATION 2: Finalise ticket, payment by MPV and cash
```

**Test automatise :** `crates/pos-engine/tests/spf_e2e_gift_vouchers.rs::uc261_mpv_use`. Le bon de 50,00 EUR sert pour une partie de la note de 116,00 EUR (paiement `VOUCHER_STORE`, identité « 8 », saisie `MANUAL`), le reste est réglé en espèces.

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_gift_vouchers.rs::uc261_mpv_use`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/UC261_event3_N_request.json`) :

```json
{
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "14adefba-6c34-470b-82c9-fd01ffd3ad44",
  "costCenter": {
    "id": "T01",
    "reference": "O2",
    "type": "TABLE"
  },
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "financials": [
    {
      "amount": 50.0,
      "amountType": "PAYMENT",
      "id": "8",
      "inputMethod": "MANUAL",
      "name": "Gift voucher",
      "type": "VOUCHER_STORE"
    },
    {
      "amount": 66.0,
      "amountType": "PAYMENT",
      "id": "1",
      "inputMethod": "MANUAL",
      "name": "Cash",
      "type": "CASH"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:08.337+02:00",
  "posFiscalTicketNo": 2,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": 116.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "30",
          "departmentName": "Menus",
          "productId": "30010",
          "productName": "Business menu all-in",
          "quantity": 2.0,
          "quantityType": "PIECE",
          "unitPrice": 58.0,
          "vats": [
            {
              "label": "B",
              "price": 116.0
            }
          ]
        }
      }
    ],
    "transactionTotal": 116.0
  },
  "vatNo": "BE0000000097"
}
```

Events préalables du scénario (vente du bon, ticket provisoire), archivés de même : `evidence/uc/UC261_event1_N_request.json`, `evidence/uc/UC261_event2_P_request.json`. Réponses FDM archivées : fichiers `*_response.json` correspondants.

**Comparaison avec la fixture** (`evidence/uc/UC261_1_check.json`) : champs métier identiques, dont les lignes de paiement `VOUCHER_STORE` (type, saisie, montant).

Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `transaction.transactionLines[0].lineType` | `COMPOSITE_PRODUCT` | `SINGLE_PRODUCT` | les produits composés ne sont pas pris en charge : le menu est un seul produit |
| `transaction.transactionLines[0].vats` | `[]` | `[{"label": "B", "price": 116.0, "priceChanges": null}]` | un produit simple porte sa propre ligne de T.V.A. (B 116,00) |

**Comparaison avec la fixture** (`evidence/uc/UC261_2_check.json`) : champs métier identiques, dont les lignes de paiement `VOUCHER_STORE` (type, saisie, montant).

Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `transaction.transactionLines[0].lineType` | `COMPOSITE_PRODUCT` | `SINGLE_PRODUCT` | les produits composés ne sont pas pris en charge : le menu est un seul produit |
| `transaction.transactionLines[0].vats` | `[]` | `[{"label": "B", "price": 116.0, "priceChanges": null}]` | un produit simple porte sa propre ligne de T.V.A. (B 116,00) |

<!-- pili-pos-evidence:end -->

## UC262_MPV_Use

**Mutation invoquee :** `signSale`

**Scenario (fixture, en anglais) :**

```
Sale of
    1 x Dry Martini (12 EUR, A)
    1 x Burger of the chef (28 EUR, B)
    1 x Perrier on take away (2.50 EUR, C)

    Payment method = MPV
```

**Test automatise :** `crates/pos-engine/tests/spf_e2e_gift_vouchers.rs::uc262_mpv_use`. La vente est réglée en entier par le bon.

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - rejeu de la fixture par `crates/pos-engine/tests/spf_e2e_gift_vouchers.rs::uc262_mpv_use`. Requête **construite et archivée par pili-pos** pour l'event de l'UC (`evidence/uc/UC262_event2_N_request.json`) :

```json
{
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "0309a86f-e8d8-4452-b702-2871e7c457a1",
  "deviceId": "F0:1D:3D:CC:13:C8",
  "employeeId": "75061189702",
  "estNo": "2000000042",
  "financials": [
    {
      "amount": 42.5,
      "amountType": "PAYMENT",
      "id": "8",
      "inputMethod": "MANUAL",
      "name": "Gift voucher",
      "type": "VOUCHER_STORE"
    }
  ],
  "language": "FR",
  "posDateTime": "2026-10-18T12:59:08.336+02:00",
  "posFiscalTicketNo": 2,
  "posId": "CPOS0031234567",
  "posSwVersion": "pili-pos/1.0.0",
  "terminalId": "POS-1-BAR",
  "ticketMedium": "PAPER",
  "transaction": {
    "transactionLines": [
      {
        "lineTotal": 12.0,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "10",
          "departmentName": "Aperitifs",
          "productId": "10006",
          "productName": "Dry Martini",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 12.0,
          "vats": [
            {
              "label": "A",
              "price": 12.0
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
              "label": "C",
              "price": 28.0
            }
          ]
        }
      },
      {
        "lineTotal": 2.5,
        "lineType": "SINGLE_PRODUCT",
        "mainProduct": {
          "departmentId": "12",
          "departmentName": "Soft Drinks",
          "productId": "12007",
          "productName": "Perrier",
          "quantity": 1.0,
          "quantityType": "PIECE",
          "unitPrice": 2.5,
          "vats": [
            {
              "label": "C",
              "price": 2.5
            }
          ]
        }
      }
    ],
    "transactionTotal": 42.5
  },
  "vatNo": "BE0000000097"
}
```

Events préalables du scénario (vente du bon, ticket provisoire), archivés de même : `evidence/uc/UC262_event1_N_request.json`. Réponses FDM archivées : fichiers `*_response.json` correspondants.

**Comparaison avec la fixture** (`evidence/uc/UC262_check.json`) : champs métier identiques, dont les lignes de paiement `VOUCHER_STORE` (type, saisie, montant).

Écarts assumés :

| Champ | Fixture | pili-pos | Motif |
|---|---|---|---|
| `transaction.transactionLines[1].vats[0].label` | `B` | `C` | une commande est sur place ou à emporter dans son ensemble : vendue à emporter, le hamburger prend le taux C (la fixture mélange un hamburger en salle B et une boisson à emporter C sur un même ticket) |

<!-- pili-pos-evidence:end -->

---

# Arrondissement des methodes de paiement (manuel §IV)

Le manuel demande les tests de base 1 et 2 dans tous les cas, le test 3
si le POS n'arrondit que les paiements en especes, les tests 4 et 5 si
tous les moyens de paiement sont arrondis. pili-pos n'arrondit **que
les especes** (`crates/pos-fiscal/src/payment.rs` : un paiement par
carte ou par chèque-repas est toujours exact, l'arrondi au 0,05 EUR
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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "94646cea-3b2c-4b6b-95a4-8212209a1fc5",
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
  "posDateTime": "2026-10-18T12:59:10.606+02:00",
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
18.10.2026                                 12:59
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
fdmDateTime: 2026-10-18T10:59:10Z;
Event: N; eventCounter: 1;
totalCounter: 1; shortSignature:
4dd6a0ef922c7b19b17cc5567c87e4f80f55c90a;

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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "2d5f332c-05ac-436d-a500-01a22bcc1e42",
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
  "posDateTime": "2026-10-18T12:59:10.606+02:00",
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
18.10.2026                                 12:59
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
fdmDateTime: 2026-10-18T10:59:10Z;
Event: N; eventCounter: 1;
totalCounter: 1; shortSignature:
4dd6a0ef922c7b19b17cc5567c87e4f80f55c90a;

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
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "f42c9a43-385c-4e71-a4b5-d47e33e40ad9",
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
  "posDateTime": "2026-10-18T12:59:10.607+02:00",
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
18.10.2026                                 12:59
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
fdmDateTime: 2026-10-18T10:59:10Z;
Event: N; eventCounter: 1;
totalCounter: 1; shortSignature:
4dd6a0ef922c7b19b17cc5567c87e4f80f55c90a;

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
en chèques-repas (jamais arrondis), le solde 3,34 EUR en especes.
Attendu : a payer 3,35 EUR en especes, arrondi +0,01 EUR.

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - situation rejouée par `crates/pos-engine/tests/spf_e2e_rounding.rs::round4_meal_vouchers_then_cash`, par la commande que l'écran d'encaissement appelle. Requête **construite et archivée par pili-pos** (`evidence/uc/ROUND4_event1_N_request.json`) ; réponse FDM archivée : `evidence/uc/ROUND4_event1_N_response.json`.

```json
{
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "f13c4965-6cfb-41a7-8f67-a5b62a19c7a0",
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
  "posDateTime": "2026-10-18T12:59:10.607+02:00",
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
18.10.2026                                 12:59
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
Chèques-repas                              11,52
Especes                                     3,34
Arrondi                                     0,01
% TVA   CODE          HT       TVA      Brut
12,00%  B          13,27      1,59     14,86
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-18T10:59:10Z;
Event: N; eventCounter: 1;
totalCounter: 1; shortSignature:
4dd6a0ef922c7b19b17cc5567c87e4f80f55c90a;

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
en chèques-repas, 3,97 EUR par carte de debit, le solde 7,76 EUR en
especes. Attendu : 7,75 EUR en especes, arrondi -0,01 EUR ; seul le
paiement en especes est arrondi.

<!-- pili-pos-evidence:start (généré par scripts/embed_uc_evidence.py) -->
**Preuve pili-pos (build `cert`)** - situation rejouée par `crates/pos-engine/tests/spf_e2e_rounding.rs::round5_meal_vouchers_card_and_cash`, par la commande que l'écran d'encaissement appelle. Requête **construite et archivée par pili-pos** (`evidence/uc/ROUND5_event1_N_request.json`) ; réponse FDM archivée : `evidence/uc/ROUND5_event1_N_response.json`.

```json
{
  "bookingDate": "2026-10-18",
  "bookingPeriodId": "48805d56-481b-4b2f-ba5c-db770a883713",
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
  "posDateTime": "2026-10-18T12:59:10.608+02:00",
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
18.10.2026                                 12:59
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
Chèques-repas                               3,13
Carte                                       3,97
Especes                                     7,76
Arrondi                                    -0,01
% TVA   CODE          HT       TVA      Brut
12,00%  B          13,27      1,59     14,86
------------------------------------------------
                INFORMATIONS FDM
------------------------------------------------
fdmId: MCK00000001;
fdmDateTime: 2026-10-18T10:59:10Z;
Event: N; eventCounter: 1;
totalCounter: 1; shortSignature:
4dd6a0ef922c7b19b17cc5567c87e4f80f55c90a;

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
| Basic Set v1.8 | 22 | 21 (hors M112A) | OK (22/22) | OK : 19 UC rejoues et compares a la fixture dans `spf_e2e_fixtures.rs` (M110, M111, M112B, M121, M122, M123, M130, M131, M132, M133, M140, M141, M150, M160, M161, M170, M171, M171B, M172) ; M112A n'est pas un parcours propose (remboursement partiel toujours en ticket separe, parcours M112B) ; Q100/Q110 : `client_status.rs` |
| Reports v1.3 | 5 | 5 | OK (5/5) | OK : journee T1–T8 jouee puis M180, M182 et M184 compares a la fixture dans `spf_e2e_report_fixtures.rs` ; M181/M183 signes par le Z combine de M184 |
| Advanced Set v1.6 (factures) | 2 | 2 | OK | OK : UC4121 et UC4122 rejoues dans `invoice_advanced.rs` |
| Advanced Set v1.6 (bons à usage multiple) | 3 | 3 | OK | OK : UC260, UC261 et UC262 rejoues et compares a la fixture dans `spf_e2e_gift_vouchers.rs` |
| **Total** | **32** | **31** | **OK** | **OK** |

Preuves d'annexe : **58 paires requete/reponse** construites et
archivees par la build `cert` pour le Basic Set (M150 compris) et les rapports
(`docs/SUBMISSION_ARTIFACTS/evidence/uc/`), plus un fichier
`<UC>_check.json` par UC du Basic Set rejoue (comparaison champ par
champ avec la fixture), **5 paires** pour les situations d'arrondi
(`ROUND1` a `ROUND5`, avec leur ticket dans `evidence/tickets/`) et 10
paires pour six UC de l'Advanced Set rejoues par le meme harnais
(UC223, UC225A, UC229B, UC230, UC4109, UC4119 ; voir *COUVERTURE_ADVANCED_SET_FR.md*), plus **10 paires** pour
les deux factures de l'Advanced Set (UC4121 : 4 ; UC4122 : 6, dont les deux factures et
le remboursement), plus **6 paires** pour les trois UC de bon cadeau
(UC260 : 1 ; UC261 : 3 ; UC262 : 2).

Pour le detail des couvertures, voir *COUVERTURE_FIXTURES_FR.md* et *COUVERTURE_RAPPORTS_FR.md*.
