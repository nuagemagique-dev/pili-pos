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

## Variables d'environnement pour les captures cert

Pour produire les preuves §V du manuel de certification (Comportement
du POS aux réponses du FDM), le mock accepte trois variables
d'environnement qui injectent des warnings et erreurs dans la réponse
de `Status`. Quand un code "critique" est injecté, les mutations sign*
retournent également une enveloppe GraphQL d'erreur (`data: null,
errors: [...]`) pour que le POS soit forcé d'afficher le message à
l'utilisateur.

### `MOCK_FDM_WARNING`

Liste de codes de warning, séparés par virgules. Codes acceptés
(extraits du §V du manuel) :

- `CLIENT_CERT_NEAR_EXPIRATION`
- `CLIENT_CERT_EXPIRED`
- `RTC_SYNC_FAILED`
- `UPDATE_URLS_FAILED`
- `UPDATE_TRUST_CERT_FAILED`
- `UPDATE_TASK_LIST_FAILED`
- `TASK_FEEDBACK_FAILED`
- `NOP_FAILED`
- `BUFFER_NEAR_FULL`
- `SERVER_CERT_RESOLVE_FAILED`
- `TRANSACTION_UPLOAD_FAILED`
- `INITIALIZATION_FAILED`

Exemple :
```bash
MOCK_FDM_WARNING=BUFFER_NEAR_FULL cargo run --package pos-fdm-mock
```

Le `Status` retourne alors :
```json
{
  "data": {
    "Status": {
      "warnings": [{"code": "BUFFER_NEAR_FULL", "message": "..."}],
      "errors": null,
      ...
    }
  }
}
```

### `MOCK_FDM_ERROR`

Liste de codes d'erreur, séparés par virgules. Codes acceptés :

- `BUFFER_FULL`
- `FDM_LOCKED`
- `FDM_NOT_OPERATIONAL`
- `UNKNOWN_POS`
- `UNAUTHORIZED`
- `INVALID_REQUEST`
- `INTERNAL_ERROR`
- `UNDEFINED_ERROR`
- `UNDEFINED_OTHER`

Exemple :
```bash
MOCK_FDM_ERROR=FDM_LOCKED cargo run --package pos-fdm-mock
```

Quand un code "critique" est présent, les mutations `sign*` retournent :
```json
{
  "data": null,
  "errors": [{"message": "Le FDM est verrouillé", "extensions": {"code": "FDM_LOCKED"}}]
}
```

Le POS doit alors :
1. Afficher la modale bloquante de checkout.
2. Mettre à jour la pilule `FdmStatus` à rouge.
3. Refuser toute nouvelle transaction tant que la condition n'est pas levée.

### `MOCK_FDM_BUFFER_PCT`

Pourcentage d'utilisation du buffer FDM à reporter dans la réponse
`Status.device.bufferCapacityUsed`. Par défaut 1.2 (~1%).

Exemple pour démontrer `BUFFER_NEAR_FULL` (>70%) :
```bash
MOCK_FDM_WARNING=BUFFER_NEAR_FULL MOCK_FDM_BUFFER_PCT=72.5 cargo run --package pos-fdm-mock
```

### `MOCK_FDM_INITIALIZED`

Si défini à `false`, retourne `initialized: false` dans la réponse
`Status`. Sinon ou par défaut, retourne `true`. Quand des erreurs
critiques sont injectées via `MOCK_FDM_ERROR`, `initialized` passe
automatiquement à `false`.

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

## Recipes pour les captures cert §V

Pour produire un screenshot par code :

```bash
# 1. Mandatoires (M dans la spec)
MOCK_FDM_ERROR=BUFFER_FULL ./pos-fdm-mock                            # → capture: BUFFER_FULL
MOCK_FDM_ERROR=FDM_LOCKED ./pos-fdm-mock                             # → capture: FDM_LOCKED
MOCK_FDM_ERROR=FDM_NOT_OPERATIONAL ./pos-fdm-mock                    # → capture: FDM_NOT_OPERATIONAL
MOCK_FDM_ERROR=UNKNOWN_POS ./pos-fdm-mock                            # → capture: UNKNOWN_POS
MOCK_FDM_WARNING=BUFFER_NEAR_FULL MOCK_FDM_BUFFER_PCT=72.5 ./pos-fdm-mock  # → capture: BUFFER_NEAR_FULL
MOCK_FDM_WARNING=CLIENT_CERT_EXPIRED ./pos-fdm-mock                  # → capture: CLIENT_CERT_EXPIRED

# 2. Optionnels (O dans la spec)
MOCK_FDM_ERROR=UNAUTHORIZED ./pos-fdm-mock                            # → capture: UNAUTHORIZED
MOCK_FDM_ERROR=INVALID_REQUEST ./pos-fdm-mock                         # → capture: INVALID_REQUEST
MOCK_FDM_ERROR=INTERNAL_ERROR ./pos-fdm-mock                          # → capture: INTERNAL_ERROR
MOCK_FDM_WARNING=CLIENT_CERT_NEAR_EXPIRATION ./pos-fdm-mock           # → capture
```

Pour chaque exécution :
1. Démarrer le mock avec la variable d'environnement.
2. Démarrer le POS (`cargo tauri dev`) qui appelle le mock.
3. Naviguer dans l'UI et capturer la pilule `FdmStatus` rouge/jaune
   et la modale bloquante (si applicable au checkout).
4. Sauvegarder les captures dans `docs/SUBMISSION_ARTIFACTS/screenshots/`.
5. Stopper le mock.
6. Passer au code suivant.
