# Preuves §V - comportement du POS aux réponses du FDM

Générées par `scripts/capture_fdm_error_evidence.sh` contre le mock
FDM (`crates/pos-fdm-mock`), démarré une fois, chaque code étant
activé par son API de contrôle (`POST /mock/control`). Les erreurs
suivent le format de la spec POS v2.2 : message lisible, code dans
`extensions {category, code, showPos}`. Le payload signSale est la fixture officielle
M110 (Basic Set v1.7).

Par code, 4 fichiers :

- `<CODE>_signSale_request.json` - la requête GraphQL envoyée par le
  POS au moment du checkout.
- `<CODE>_signSale_response.json` - la réponse du FDM (enveloppe
  d'erreur GraphQL pour les erreurs critiques ; réponse signée
  portant l'avertissement dans `warnings` pour les simples
  avertissements).
- `<CODE>_status_request.json` / `<CODE>_status_response.json` - le
  sondage de statut que le POS effectue toutes les 30 s et
  la réponse contenant le message (catégorie + `showPos` conformes au
  tableau §V du manuel).

Codes couverts : les 4 erreurs à affichage obligatoire (BUFFER_FULL,
FDM_LOCKED, FDM_NOT_OPERATIONAL, UNKNOWN_POS), les 2 avertissements à
affichage obligatoire (CLIENT_CERT_EXPIRED, BUFFER_NEAR_FULL) et
2 codes optionnels (UNAUTHORIZED, RTC_SYNC_FAILED).

**Provenance mock :** ces captures documentent le comportement du POS.
Elles seront re-capturées contre le simulateur en ligne module2
(erreurs déclenchées depuis son onglet « Developer ») lors de la répétition
pré-soumission (`JOURNAL_TESTS_PRE_SOUMISSION_FR.md`) pour les codes
que l'émulateur permet de provoquer. Les captures
d'écran UI correspondantes sont dans `CAPTURES_ECRAN_FR.pdf`.
