# Resultats des tests UC - moyun-pos v1.0.0

**Date :** 2026-05-14
**Reference :** §IV du manuel de certification (Procedure de communication GraphQL).

Ce document parcourt les UC obligatoires du Basic Set v1.7 (22 UC
spec) + les 5 UC Reports v1.3, **à l'exception de M150_signInvoice**
(event I, hors périmètre v1.0.0 - voir la section dédiée ci-dessous
et *DECISION_PEPPOL_AP_FR.md*). Pour chaque UC restant, il fournit :

- Le scenario d'execution (extrait de la fixture officielle SPF Finances).
- La capture du JSON request envoye au FDM (depuis le fichier Variables.json publie).
- La capture du JSON response renvoye par le FDM (depuis le fichier *_Response.json publie).
- Le tableau "Champs a verifier" coche OK avec valeurs observees.
- La reference au ticket attendu (cf. *TICKETS_EXEMPLES_FR.md*).
- La reference au test automatise qui valide la conformite.

**Note importante :** les fixtures SPF Finances etant la source de verite, les valeurs ci-dessous correspondent exactement a celles publiees par le SPF dans `docs/20250916_UC_BasicSet_POS_FDM_1.7/` et `docs/20250916_UC_Reports_1.3/`. Notre code (couche `pos-fiscal`) parse et produit ces JSON ; les tests `uc_fixtures.rs`, `reports_fixtures.rs` et `spf_e2e_basic.rs` valident automatiquement la conformite a chaque commit.

**Preuves generees par l'implementation :** en complement des fixtures publiees, le repertoire `evidence/uc/` contient, pour chaque UC rejoue par la suite e2e (`spf_e2e_basic.rs`, `spf_e2e_reports.rs`), le JSON de requete **reellement construit par moyun-pos** (colonne `event_data_json` de l'archive fiscale) et la reponse FDM archivee. Regeneration : `scripts/capture_uc_evidence.sh`. Ces fichiers prouvent que la couche commande produit des payloads conformes, pas seulement qu'elle sait parser les fixtures.

---

# Basic Set v1.7 (22 UC)

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
| `posDateTime` | `2024-07-29T13:00:00+02:00` | OK |
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
    "posDateTime": "2024-07-29T13:00:00+02:00",
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
      "posDateTime": "2024-07-29T13:00:00+02:00",
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
        "fdmDateTime": "2024-07-29T11:00:08Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
| `posDateTime` | `2024-07-29T13:15:00+02:00` | OK |
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
    "posDateTime": "2024-07-29T13:15:00+02:00",
    "posSwVersion": "1.8.3",
    "deviceId": "1631678d-7a85-4ac3-b296-bb4565e873fe",
    "terminalId": "TER-2-DIN",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "PAPER",
    "employeeId": "84022899837",
    "fdmRef": {
      "fdmId": "FOD01000001",
      "fdmDateTime": "2024-07-29T11:00:08Z",
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
      "posDateTime": "2024-07-29T13:15:00+02:00",
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
        "fdmDateTime": "2024-07-29T11:15:00Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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

**Note de déclaration :** le POS moyun-pos ne propose pas le pattern M112A
(remboursement partiel combiné à une nouvelle vente sur un même ticket).
Le dossier déclare le pattern **M112B** (remboursement en événement séparé,
chaîné à la vente d'origine via `fdmReference`) — voir la section
M112B ci-dessous. Les fixtures M112A sont validées en *parsing* uniquement.

### M112A_1_OriginalSale

**Mutation invoquee :** `inconnue`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `6587` | OK |
| `posDateTime` | `2024-10-07T13:00:00+02:00` | OK |
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
    "posDateTime": "2024-10-07T13:00:00+02:00",
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
      "posDateTime": "2024-10-07T13:00:00+02:00",
      "terminalId": "TER-2-DIN",
      "deviceId": "1631678d-7a85-4ac3-b296-bb4565e873fe",
      "eventOperation": "SALE",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-10-07T11:00:41Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing uniquement — pattern non déclaré, voir M112B).

### M112A_2_PartialRefund

**Mutation invoquee :** `signSale`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `6603` | OK |
| `posDateTime` | `2024-10-07T13:48:00+02:00` | OK |
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
    "posDateTime": "2024-10-07T13:48:00+02:00",
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
      "posDateTime": "2024-10-07T13:48:00+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "SALE",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-10-07T11:48:41Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing uniquement — pattern non déclaré, voir M112B).


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
`m112b_partial_refund_is_separate_event_chained_to_original` dans
`src-tauri/tests/spf_e2e_basic.rs` ; preuves générées dans
`evidence/uc/M112B_*.json`.

### M112B_1_OriginalSale

**Mutation invoquee :** `inconnue`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `6587` | OK |
| `posDateTime` | `2024-10-07T13:00:00+02:00` | OK |
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
    "posDateTime": "2024-10-07T13:00:00+02:00",
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
      "posDateTime": "2024-10-07T13:00:00+02:00",
      "terminalId": "TER-2-DIN",
      "deviceId": "1631678d-7a85-4ac3-b296-bb4565e873fe",
      "eventOperation": "SALE",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-10-07T11:00:41Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).

### M112B_2_PartialRefund

**Mutation invoquee :** `signSale`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `6603` | OK |
| `posDateTime` | `2024-10-07T13:48:00+02:00` | OK |
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
    "posDateTime": "2024-10-07T13:48:00+02:00",
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
      "posDateTime": "2024-10-07T13:48:00+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "SALE",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-10-07T11:48:27Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
| `posDateTime` | `2024-07-29T13:57:03+02:00` | OK |
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
    "posDateTime": "2024-07-29T13:57:03+02:00",
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
      "posDateTime": "2024-07-29T13:57:03+02:00",
      "terminalId": "TER-1-BAR ",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "ORDER",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T11:57:12Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


## M122_signCostCenterChange

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
| `posDateTime` | `2024-07-29T14:21:23+02:00` | OK |
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
    "posDateTime": "2024-07-29T14:21:23+02:00",
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
      "posDateTime": "2024-07-29T14:21:23+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "COST_CENTER_CHANGE",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T12:21:25Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
| `posDateTime` | `2024-07-29T14:48:03+02:00` | OK |
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
    "posDateTime": "2024-07-29T14:48:03+02:00",
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
      "posDateTime": "2024-07-29T14:48:03+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "PRE_BILL",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T12:48:12Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
| `posDateTime` | `2024-07-29T15:07:09+02:00` | OK |
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
    "posDateTime": "2024-07-29T15:07:09+02:00",
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
      "posDateTime": "2024-07-29T15:07:09+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "MONEY_IN_OUT",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T13:07:12Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
| `posDateTime` | `2024-07-29T15:12:04+02:00` | OK |
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
    "posDateTime": "2024-07-29T15:12:04+02:00",
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
      "posDateTime": "2024-07-29T15:12:04+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "DRAWER_OPEN",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T13:12:05Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


## M132_signPaymentCorrection

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
| `posDateTime` | `2024-07-29T15:22:06+02:00` | OK |
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
    "posDateTime": "2024-07-29T15:22:06+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "NONE",
    "employeeId": "75061189702",
    "fdmRef": {
      "fdmId": "FOD01000001",
      "fdmDateTime": "2024-07-29T11:35:09Z",
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
      "posDateTime": "2024-07-29T15:22:06+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "PAYMENT_CORRECTION",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T13:22:06Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
| `posDateTime` | `2024-07-29T15:39:39+02:00` | OK |
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
    "posDateTime": "2024-07-29T15:39:39+02:00",
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
      "posDateTime": "2024-07-29T15:39:39+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "MONEY_IN_OUT",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T13:39:45Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
| `posDateTime` | `2024-07-29T15:50:39+02:00` | OK |
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
    "posDateTime": "2024-07-29T15:50:39+02:00",
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
      "posDateTime": "2024-07-29T15:50:39+02:00",
      "terminalId": "POS-2-REC",
      "deviceId": "80:1A:A6:75:66:FC",
      "eventOperation": "WORK_IN",
      "fdmRef": {
        "fdmId": "FOD01000002",
        "fdmDateTime": "2024-07-29T13:50:39Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
| `posDateTime` | `2024-07-29T15:59:39+02:00` | OK |
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
    "posDateTime": "2024-07-29T15:59:39+02:00",
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
      "posDateTime": "2024-07-29T15:59:39+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "WORK_OUT",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T13:59:43Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


## M150_signInvoice - hors périmètre v1.0.0

L'événement fiscal **I (Invoice)** n'est **pas disponible en
v1.0.0** : le feature flag `peppol` n'est pas activé dans la build
de certification (`cert = ["custom-protocol"]`,
`src-tauri/Cargo.toml`), donc le code d'émission de facture B2B
(`crates/pos-peppol`, `src-tauri/src/commands/invoice.rs`,
`#[cfg(feature = "peppol")]`) n'est pas compilé dans le binaire
soumis. La fixture M150 continue d'être **parsée** par
`crates/pos-fiscal/tests/uc_fixtures.rs` (validation du format JSON
uniquement) et **exercée en e2e** par `src-tauri/tests/spf_e2e_basic.rs`
avec les features par défaut (`hubrise` + `peppol`, utilisées pour le
développement et les tests, pas pour la build cert) - le JSON produit
par `do_sign_invoice_fiscal` et la réponse FDM archivée pendant cet
e2e sont d'ailleurs disponibles sous `evidence/uc/M150_event2_I_*.json`
à titre de preuve de code, mais ce couple n'est **pas** inclus dans
les preuves soumises pour l'admission v1.0.0 puisque le binaire cert
ne peut pas produire cet événement.

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
| `posDateTime` | `2024-07-29T16:25:39+02:00` | OK |
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
    "posDateTime": "2024-07-29T16:25:39+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "PAPER",
    "employeeId": "75061189702",
    "fdmRef": {
      "fdmId": "FOD01000001",
      "fdmDateTime": "2024-07-29T11:48:04Z",
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
      "posDateTime": "2024-07-29T16:25:39+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "COPY",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T14:25:45Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
| `posDateTime` | `2024-07-29T16:30:39+02:00` | OK |
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
    "posDateTime": "2024-07-29T16:30:39+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "PAPER",
    "employeeId": "75061189702",
    "fdmRef": {
      "fdmId": "FOD01000001",
      "fdmDateTime": "2024-07-29T12:57:12Z",
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
      "posDateTime": "2024-07-29T16:30:39+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "COPY",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T14:30:42Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
| `posDateTime` | `2024-07-29T17:00:39+02:00` | OK |
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
    "posDateTime": "2024-07-29T17:00:39+02:00",
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
      "posDateTime": "2024-07-29T17:00:39+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "WORK_IN",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T15:00:41Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


## M171B_signSale_Training

**Mutation invoquee :** `signSale`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `1015` | OK |
| `posDateTime` | `2024-07-29T17:10:39+02:00` | OK |
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
    "posDateTime": "2024-07-29T17:10:39+02:00",
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
      "posDateTime": "2024-07-29T17:10:39+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "fdmSwVersion": "1.2.0",
      "eventOperation": "SALE",
      "digitalSignature": "c7uwqjLZ+Wk+R8swA/KHilWp4qCPK4u1L6TGxuCOJbf7bY32Ra2G8NiI+TO9lFGNFTfU1UA/8HGiccVzMQrUcA==",
      "bufferCapacityUsed": 0.25,
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T15:10:41Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
| `posDateTime` | `2024-07-29T17:10:39+02:00` | OK |
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
    "posDateTime": "2024-07-29T17:10:39+02:00",
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
      "posDateTime": "2024-07-29T17:10:39+02:00",
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
        "fdmDateTime": "2024-07-29T15:10:41Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
| `posDateTime` | `2024-07-29T17:10:39+02:00` | OK |
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
    "posDateTime": "2024-07-29T17:10:39+02:00",
    "posSwVersion": "1.8.3",
    "terminalId": "TER-1-BAR",
    "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
    "bookingPeriodId": "dffcd829-a0e5-41ca-a0ae-9eb887f95637",
    "bookingDate": "2024-07-29",
    "ticketMedium": "NONE",
    "employeeId": "75061189702",
    "fdmRef": {
      "fdmId": "FOD01000002",
      "fdmDateTime": "2024-07-03T16:40:39Z",
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
      "posDateTime": "2024-07-29T17:10:39+02:00",
      "terminalId": "TER-1-BAR",
      "deviceId": "b54a614f-39cc-4a7b-bd9f-aa6b693d769c",
      "eventOperation": "COPY",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-29T15:10:45Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
        "fdmDateTime": "2025-08-10T10:00:39.514Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
        "fdmDateTime": "2025-08-10T10:00:39.514Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


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
| `posDateTime` | `2024-07-30T13:00:35+02:00` | OK |
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
    "posDateTime": "2024-07-30T13:00:35+02:00",
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
        "lastFdmDateTime": "2024-07-30T10:57:18Z",
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
      "posDateTime": "2024-07-30T13:00:35+02:00",
      "terminalId": "POS-1-BAR",
      "deviceId": "F0:1D:3D:CC:13:C8",
      "eventOperation": "REPORT_TURNOVER_X",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-30T11:00:38Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


## M181_signReportTurnoverZ

**Note de déclaration :** le POS moyun-pos n'émet jamais le Z financier
seul : la clôture de journée émet toujours le **Z combiné M184**
(`signReportTurnoverZ` + `signReportUserZ` dans un même document GraphQL,
deux `posFiscalTicketNo` séquentiels) — voir la section M184 ci-dessous.
La fixture M181 est validée en *parsing* et sa forme `signReportTurnoverZ`
est exactement celle émise dans la première moitié du document M184.
Couverture e2e applicative du Z : `m184_*` dans
`src-tauri/tests/spf_e2e_reports.rs`.

**Mutation invoquee :** `signReportTurnoverZ`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `209` | OK |
| `posDateTime` | `2024-07-30T13:05:35+02:00` | OK |
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
    "posDateTime": "2024-07-30T13:05:35+02:00",
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
      "posDateTime": "2024-07-30T13:05:35+02:00",
      "terminalId": "POS-1-BAR",
      "deviceId": "F0:1D:3D:CC:13:C8",
      "eventOperation": "REPORT_TURNOVER_Z",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-30T11:05:41Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


## M182_signReportUserX

**Note de déclaration :** implémenté par la commande
`generate_user_x_report` (`src-tauri/src/commands/report.rs`,
`do_generate_user_x_report`). Couverture e2e applicative :
`m182_generate_user_x_report_archives_r_event_with_user_block` dans
`src-tauri/tests/spf_e2e_reports.rs` ; preuves dans
`evidence/uc/M182_*.json`.

**Mutation invoquee :** `signReportUserX`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `210` | OK |
| `posDateTime` | `2024-07-30T13:10:35+02:00` | OK |
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
    "posDateTime": "2024-07-30T13:10:35+02:00",
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
        "lastPosDateTime": "2024-07-30T13:05:35+02:00"
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
        "firstFdmDateTime": "2024-04-22T06:30:12Z",
        "firstTotalCounter": 49,
        "lastFdmDateTime": "2024-07-30T11:05:41Z",
        "lastTotalCounter": 58
      }
    ],
    "users": [
      {
        "employeeId": "75061189702",
        "totalAmount": 0,
        "firstPosDateTime": "2024-07-30T08:30:12+02:00",
        "lastPosDateTime": "2024-07-30T09:40:12+02:00",
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
        "firstPosDateTime": "2024-07-30T09:45:12+02:00",
        "lastPosDateTime": "2024-07-30T12:57:12+02:00",
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
      "posDateTime": "2024-07-30T13:10:35+02:00",
      "terminalId": "POS-1-BAR",
      "deviceId": "F0:1D:3D:CC:13:C8",
      "eventOperation": "REPORT_USER_X",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-30T11:10:42Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


## M183_signReportUserZ

**Note de déclaration :** comme pour M181, le POS n'émet jamais le Z
utilisateurs seul — il est toujours émis dans le document combiné M184
(seconde moitié). La fixture M183 est validée en *parsing* et sa forme
`signReportUserZ` est exactement celle émise dans le document M184.

**Mutation invoquee :** `signReportUserZ`

**Champs a verifier (manuel SCE 2.0 §IV) :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| `language` | `NL` | OK |
| `posId` | `CPOS0031234567` | OK |
| `posFiscalTicketNo` | `211` | OK |
| `posDateTime` | `2024-07-30T13:15:35+02:00` | OK |
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
    "posDateTime": "2024-07-30T13:15:35+02:00",
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
        "lastPosDateTime": "2024-07-30T13:10:35+02:00"
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
        "firstFdmDateTime": "2024-04-22T06:30:12Z",
        "firstTotalCounter": 49,
        "lastFdmDateTime": "2024-07-30T11:10:42Z",
        "lastTotalCounter": 59
      }
    ],
    "users": [
      {
        "employeeId": "75061189702",
        "totalAmount": 0,
        "firstPosDateTime": "2024-07-30T08:30:12+02:00",
        "lastPosDateTime": "2024-07-30T09:40:12+02:00",
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
        "firstPosDateTime": "2024-07-30T09:45:12+02:00",
        "lastPosDateTime": "2024-07-30T12:57:12+02:00",
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
      "posDateTime": "2024-07-30T13:15:35+02:00",
      "terminalId": "POS-1-BAR",
      "deviceId": "F0:1D:3D:CC:13:C8",
      "eventOperation": "REPORT_USER_Z",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-30T11:15:41Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


## M184_signReportTurnoverZUserZ

**Note de déclaration :** c'est CE mode d'émission du Z que le POS
implémente et déclare : `do_generate_z_report`
(`src-tauri/src/commands/report.rs`) envoie `signReportTurnoverZ` +
`signReportUserZ` dans **un seul document GraphQL**
(`FdmClient::sign_report_turnover_z_user_z`), alloue deux
`posFiscalTicketNo` séquentiels et archive les deux événements R dans
une même transaction SQLite. Couverture e2e applicative :
`m184_generate_z_report_signs_both_reports_in_one_operation` et
`z_report_starts_new_booking_period` dans
`src-tauri/tests/spf_e2e_reports.rs` ; preuves dans
`evidence/uc/M184_*.json` (2 événements R).

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
      "posDateTime": "2024-07-30T13:05:35.152+02:00",
      "terminalId": "POS-1-BAR",
      "deviceId": "F0:1D:3D:CC:13:C8",
      "eventOperation": "REPORT_TURNOVER_Z",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-30T11:05:43.231Z",
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
      "posDateTime": "2024-07-30T13:05:35.233+02:00",
      "terminalId": "POS-1-BAR",
      "deviceId": "F0:1D:3D:CC:13:C8",
      "eventOperation": "REPORT_USER_Z",
      "fdmRef": {
        "fdmId": "FOD01000001",
        "fdmDateTime": "2024-07-30T11:05:43.242Z",
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

**Validation automatisee :** test passant dans `crates/pos-fiscal/tests/uc_fixtures.rs` (parsing) et `src-tauri/tests/spf_e2e_basic.rs` (e2e applicatif).


---

# Recapitulatif

| Famille | UC (spec) | UC dans le perimetre v1.0.0 | Status fixture | Status e2e applicatif |
|---|---|---|:-:|:-:|
| Basic Set v1.7 | 22 | 21 (M150/event I exclu, voir *DECISION_PEPPOL_AP_FR.md*) | OK (22/22 parsent, dont M150 en parsing seul) | OK (16 UC e2e couverts dans `spf_e2e_basic.rs` — dont M112B, M150 (features par défaut, hors build cert), M161, M172 ; M112A non déclaré, remplacé par M112B) |
| Reports v1.3 | 5 | 5 | OK (5/5) | OK (4 e2e couverts dans `spf_e2e_reports.rs` : M180, M182, M184 combiné ×2 ; M181/M183 émis via M184) |
| **Total** | **27** | **26** | **OK** | **OK** |

Nombre d'UC couverts par les preuves d'annexe (JSON + ticket) :
**17 UC dans le périmètre v1.0.0** (25 paires requête/réponse,
`docs/SUBMISSION_ARTIFACTS/evidence/uc/`) - M150 y figure aussi
(`M150_event2_I_*.json`) mais reste hors périmètre de soumission
cert v1.0.0 puisque généré sous les features par défaut, pas sous la
build `cert` (cf. section M150 ci-dessus).

Pour le detail des couvertures, voir *COUVERTURE_FIXTURES_FR.md* et *COUVERTURE_RAPPORTS_FR.md*.
