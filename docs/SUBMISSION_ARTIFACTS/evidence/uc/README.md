# Preuves §IV — JSON générés par moyun-pos par UC

Générées par `scripts/capture_uc_evidence.sh`, qui rejoue les suites
e2e (`src-tauri/tests/spf_e2e_basic.rs`, `spf_e2e_reports.rs`) avec
`UC_EVIDENCE_DIR` positionné.

Par UC et par event fiscal archivé pendant le scénario :

- `<UC>_event<N>_<label>_request.json` — le payload d'event
  **réellement construit par la couche commande de moyun-pos**
  (colonne `event_data_json` de la table `fiscal_events`, c.-à-d.
  exactement ce qui part vers le FDM).
- `<UC>_event<N>_<label>_response.json` — la réponse FDM archivée
  (fixture officielle SPF rejouée par wiremock).

Exemple : M111 (remboursement total) archive 2 events — `event1_N`
(la vente originale) puis `event2_N` (le remboursement avec `fdmRef`
vers l'original).

Ces fichiers complètent `RESULTATS_TESTS_UC_FR.md` : le document cite
les fixtures publiées (référence), ce répertoire prouve que notre
implémentation produit des payloads conformes. Lors de la répétition
zwartedoos.be, les mêmes flux seront rejoués contre le FDM virtuel et
les réponses réelles (signatures ECDSA authentiques) archivées dans
`JOURNAL_TESTS_PRE_SOUMISSION_FR.md`.
