# pos-fdm-mock

Simulateur HTTP/GraphQL minimaliste du FDM Belge SCE 2.0 utilisé pour
les développements locaux et pour les captures de preuves de
certification.

## Démarrage

```bash
cargo run --package pos-fdm-mock
# ou avec un port spécifique
PORT=18099 cargo run --package pos-fdm-mock
```

Par défaut, le mock écoute sur `http://localhost:8081/graphql` et
retourne :
- `Status` : initialized=true, aucun warning, aucun erreur.
- `signSale` / `signOrder` / etc. : `SignResult` valide avec
  compteurs incrémentés, signature mock et URL de vérification.

## Comportement

Le mock suit le format d'erreur de la spec POS v2.2 (et du simulateur
module2) :

```json
{
  "data": null,
  "errors": [{
    "message": "Le buffer du FDM est plein",
    "extensions": {"category": "SPF_FOD", "code": "BUFFER_FULL", "showPos": "MANDATORY"}
  }]
}
```

- **Compteurs par `posId`** : chaque POS a ses propres compteurs
  (`totalCounter` et compteur par label), comme un vrai FDM partagé.
- **Jeton** : si `MOCK_FDM_TOKEN` est défini, une requête sans
  `Authorization: Bearer <jeton>` (ou avec un autre jeton) reçoit
  `UNAUTHORIZED`.
- **POS autorisés** : si `MOCK_FDM_ALLOWED_POS` est défini (liste
  séparée par virgules), tout autre `posId` reçoit `UNKNOWN_POS` sur
  les mutations `sign*`.
- **Warnings** : les codes de `warnings` apparaissent dans `Status` et
  dans chaque réponse `sign*` signée ; `bufferCapacityUsed` suit le
  réglage du buffer.
- **Erreurs critiques** : `Status` les liste, `initialized` passe à
  `false`, et les mutations `sign*` renvoient l'enveloppe d'erreur
  ci-dessus. Le POS doit alors afficher la modale bloquante, passer la
  pilule `FdmStatus` au rouge et refuser toute transaction tant que la
  condition n'est pas levée.

## API de contrôle (comme l'onglet Developer de module2)

Les réglages changent sans redémarrer le mock :

```bash
# lire les réglages (le jeton est masqué)
curl -s localhost:8081/mock/control
# FDM verrouillé
curl -s localhost:8081/mock/control -H 'content-type: application/json' \
  -d '{"errors":["FDM_LOCKED"]}'
# buffer presque plein
curl -s localhost:8081/mock/control -H 'content-type: application/json' \
  -d '{"warnings":["BUFFER_NEAR_FULL"],"bufferPct":72.5}'
# retour à la normale
curl -s localhost:8081/mock/control -H 'content-type: application/json' -d '{}'
```

Un `POST` remplace tous les réglages. Champs : `errors`, `warnings`,
`bufferPct`, `initialized`, `token`, `allowedPos`,
`placeholderSignatures`.

Journal des appels (les 100 derniers, comme l'onglet Logs de module2) :

```bash
curl -s localhost:8081/mock/requests      # requête + réponse de chaque appel
curl -s -X DELETE localhost:8081/mock/requests
```

## Variables d'environnement

Elles fixent les réglages au démarrage (l'API de contrôle les remplace
ensuite).

| Variable | Effet |
|---|---|
| `PORT` | port d'écoute (8081 par défaut) |
| `MOCK_FDM_ERROR` | codes d'erreur, séparés par virgules |
| `MOCK_FDM_WARNING` | codes de warning, séparés par virgules |
| `MOCK_FDM_BUFFER_PCT` | `bufferCapacityUsed` (100 si `BUFFER_FULL`, 82.53 si `BUFFER_NEAR_FULL`, sinon 1.2) |
| `MOCK_FDM_INITIALIZED` | `false` pour un FDM non initialisé (automatique avec une erreur) |
| `MOCK_FDM_TOKEN` | jeton Bearer exigé |
| `MOCK_FDM_ALLOWED_POS` | `posId` acceptés, séparés par virgules |
| `MOCK_FDM_PLACEHOLDER_SIGNATURES` | `1` : signe comme le simulateur module2 (`dummy_...`, shortSignature non dérivée) |

Warnings (§V du manuel) : `CLIENT_CERT_NEAR_EXPIRATION`,
`CLIENT_CERT_EXPIRED`, `RTC_SYNC_FAILED`, `UPDATE_URLS_FAILED`,
`UPDATE_TRUST_CERT_FAILED`, `UPDATE_TASK_LIST_FAILED`,
`TASK_FEEDBACK_FAILED`, `NOP_FAILED`, `BUFFER_NEAR_FULL`,
`SERVER_CERT_RESOLVE_FAILED`, `TRANSACTION_UPLOAD_FAILED`,
`INITIALIZATION_FAILED`, `UPDATE_POS_VATNO`, `UPDATE_POS_ESTNO`.

Erreurs : `BUFFER_FULL`, `FDM_LOCKED`, `FDM_NOT_OPERATIONAL`,
`UNKNOWN_POS`, `UNAUTHORIZED`, `INVALID_REQUEST`, `INTERNAL_ERROR`,
`UNDEFINED_ERROR`, `UNDEFINED_OTHER`.

Exemple :
```bash
MOCK_FDM_TOKEN=dev MOCK_FDM_ALLOWED_POS=CPOS0031234567 cargo run --package pos-fdm-mock
```

## Email API (Scaleway TEM)

The same process also serves the Scaleway Transactional Email API used
by the digital ticket, so email works with no internet (admission
image, screenshots, demos). Point `pos.json` at it:

```json
"digital_ticket": {
  "project_id": "admission",
  "from_email": "tickets@admission.invalid",
  "from_name": "Restaurant",
  "api_base": "http://127.0.0.1:8081",
  "api_key_file": "/etc/moyun-pos/secrets/tem.key",
  "allow_digital_only": true
}
```

- `POST /transactional-email/v1alpha1/regions/{region}/emails` writes
  `<id>.json` (the request) and `<id>.txt` (the ticket text) into
  `MOCK_TEM_OUTBOX` (default `./mock-tem-outbox`).
- `GET` on the same path is the POS's reachability probe.
- `MOCK_TEM_FAIL=1` answers 503: the POS prints the paper fallback.
- A request without an `X-Auth-Token` header gets 401 (any non-empty
  key is accepted).

```bash
MOCK_TEM_OUTBOX=/tmp/tem ./pos-fdm-mock        # emails land in /tmp/tem
MOCK_TEM_FAIL=1 ./pos-fdm-mock                 # → capture: paper fallback
```

## Captures cert §V

Les paires requête/réponse de chaque code sont produites par
`scripts/capture_fdm_error_evidence.sh` (un seul démarrage du mock,
un `POST /mock/control` par code) dans
`docs/SUBMISSION_ARTIFACTS/evidence/fdm_errors/`.

Pour les captures d'écran, un code à la fois :
1. Démarrer le mock, puis le POS (`cargo tauri dev`) qui l'appelle.
2. Activer le code via `POST /mock/control`.
3. Capturer la pilule `FdmStatus` rouge/jaune et la modale bloquante
   (si applicable au checkout) dans `docs/SUBMISSION_ARTIFACTS/screenshots/`.
4. Passer au code suivant (`POST /mock/control` suivant).
