# Preuves §IV - JSON générés par moyun-pos par UC

Générées par `scripts/capture_uc_evidence.sh`, qui rejoue les fixtures
officielles avec la build de certification
(`--no-default-features --features cert`) et `UC_EVIDENCE_DIR`
positionné : `src-tauri/tests/spf_e2e_fixtures.rs` (Basic Set v1.7,
commandes construites à partir des produits de la fixture) et
`spf_e2e_report_fixtures.rs` (journée T1–T8 des Reports v1.3, puis
M180, M182 et M184). Le script les intègre ensuite à
`RESULTATS_TESTS_UC_FR.md` (`scripts/embed_uc_evidence.py`).

Par UC et par event fiscal archivé pendant le scénario :

- `<UC>_event<N>_<label>_request.json` - le payload d'event
  **réellement construit par la couche commande de moyun-pos**
  (colonne `event_data_json` de la table `fiscal_events`, c.-à-d.
  exactement ce qui part vers le FDM).
- `<UC>_event<N>_<label>_response.json` - la réponse FDM archivée
  (fixture officielle SPF rejouée par wiremock).
- `<UC>_check.json` - le résultat de la comparaison champ par champ
  avec la fixture : champs métier égaux, champs d'identification
  contrôlés sur leur format, écarts assumés et leur motif.

Exemple : M111 (remboursement total) archive 2 events - `event1_N`
(la vente originale) puis `event2_N` (le remboursement avec `fdmRef`
vers l'original).

Ces fichiers complètent `RESULTATS_TESTS_UC_FR.md` : le document cite
les fixtures publiées (référence), ce répertoire prouve que notre
implémentation produit des payloads conformes. Lors de la répétition
pré-soumission avec le kit de développement Checkbox (FDM de
développement / émulateur GraphQL), les mêmes flux seront rejoués
contre le FDM réel et les réponses authentiques (signées par le FDM,
contrôlées par le POS avant archivage) archivées dans `JOURNAL_TESTS_PRE_SOUMISSION_FR.md`.
