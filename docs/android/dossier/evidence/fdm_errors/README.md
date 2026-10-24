# Preuves §V - comportement du POS aux réponses du FDM

Dossier Android (pili-pos-android 1.0.0). Ces fichiers sont produits par
`scripts/capture_fdm_error_evidence.sh` contre le FDM simulé du dépôt
(`crates/pos-fdm-mock`), démarré une fois, chaque code étant activé par son
API de contrôle (`POST /mock/control`). Les erreurs suivent le format de la
spécification POS v2.2 : message lisible, code dans
`extensions {category, code, showPos}`. La requête signSale est la fixture
officielle M110 (Basic Set v1.8) et le sondage de statut est une requête de
statut postée au même simulateur, avec chaque code activé tour à tour.

Par code, 4 fichiers :

- `<CODE>_signSale_request.json` : la requête GraphQL de la fixture officielle
  M110, postée au simulateur.
- `<CODE>_signSale_response.json` : la réponse du simulateur (enveloppe
  d'erreur GraphQL pour les erreurs critiques ; réponse signée portant
  l'avertissement dans `warnings` pour les simples avertissements).
- `<CODE>_status_request.json` / `<CODE>_status_response.json` : la requête de
  statut (le moteur interroge le statut toutes les 30 s) et la réponse contenant le
  message (catégorie et `showPos` conformes au tableau §V du manuel).

Codes couverts : les 4 erreurs à affichage obligatoire (BUFFER_FULL,
FDM_LOCKED, FDM_NOT_OPERATIONAL, UNKNOWN_POS), les 2 avertissements à
affichage obligatoire (CLIENT_CERT_EXPIRED, BUFFER_NEAR_FULL) et
2 codes optionnels (UNAUTHORIZED, RTC_SYNC_FAILED).

**Provenance du simulateur :** ces fichiers documentent les réponses du FDM
simulé du dépôt et la forme des messages que le moteur doit afficher. Ils
seront re-capturés contre le simulateur en ligne module2 (erreurs déclenchées
depuis son onglet « Developer ») lors de la répétition pré-soumission
(`JOURNAL_TESTS_PRE_SOUMISSION_FR.md`) pour les codes que ce simulateur permet
de provoquer. Les captures d'écran correspondantes sont dans
`CAPTURES_ECRAN_FR.pdf`.
