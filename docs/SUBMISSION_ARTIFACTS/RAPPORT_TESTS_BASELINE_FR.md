# Rapport de tests baseline (couverture de la build cert)

**Date :** 2026-05-14
**Branche :** main
**Toolchain :** Rust stable (edition 2024), workspace Cargo, Tauri v2.
**Objet :** Photographier la posture de tests à la date de soumission,
afin que tout mouvement de tests ultérieur puisse être tracé et
attribué à des modifications futures.

---

## Vérification de build

| Configuration | Commande | Résultat |
|---|---|---|
| Par défaut (dev) | `cargo check --package moyun-pos` | OK vert |
| Certification | `cargo check --package moyun-pos --no-default-features --features cert` | OK vert |

**Vérification au niveau symboles** que le binaire cert n'embarque aucun
code d'intégrations tierces désactivées :

```
nm target/debug/moyun-pos | grep -c pos_hubrise
```

| Configuration | Symboles d'intégrations tierces |
|---|---|
| Par défaut | **493** |
| Cert (`--no-default-features --features cert`) | **0** |

Les colonnes de schéma liées aux intégrations webshop (noms de colonnes
du schéma SQL embarqué dans `pos-db`) restent présentes dans la
définition du schéma pour compatibilité de lecture avec des bases
créées par une build dev. Aucun chemin de code vivant n'écrit ces
colonnes dans la build cert.

---

## Totaux de tests (`cargo test --workspace --no-fail-fast`)

Totaux actuels à la dernière révision du dossier :

| Binaire de test | Couche | Passants | Échecs | Ignorés |
|---|---|---:|---:|---:|
| `pos_core` (unit) | types domaine, TVA | 6 | 0 | 0 |
| `pos_db` (unit) | schéma, requêtes, migrations | 29 | 0 | 0 |
| `pos_fiscal` (unit) | types d'événements, signature, compteur, JSON canonique | 50 | 0 | 0 |
| `pos_fiscal::full_service` (intégration) | workflow fiscal de bout en bout | 1 | 0 | 0 |
| `pos_fiscal::spf_basic_set` (intégration) | 10 UC obligatoires contre wiremock FDM | 10 | 0 | 0 |
| `pos_fiscal::uc_fixtures` (intégration) | parsing fixtures Basic Set v1.7 | 5 | 0 | 0 |
| `pos_fiscal::reports_fixtures` (intégration) | parsing fixtures Reports v1.3 | 8 | 0 | 0 |
| `pos_fiscal::advanced_set_fixtures` (intégration) | fixtures Advanced Set v1.5 (141 UC) | 1 | 0 | 0 |
| `pos_fiscal::e2e_staging` (intégration, opt-in) | contre FDM virtuel zwartedoos.be | 0 | 0 | 2 |
| intégration webshop (unit) | tests unitaires crate webshop (exclue de la build cert) | 19 | 0 | 0 |
| `pos_peppol` (unit) | sérialiseur UBL 2.1 BIS Billing 3.0 | 5 | 0 | 0 |
| `pos_print` (unit) | ESC/POS, layout ticket | 39 | 0 | 0 |
| `moyun-pos::spf_e2e_basic` (intégration) | 12 UC Basic Set end-to-end via commandes Tauri | 12 | 0 | 0 |
| `moyun-pos::spf_e2e_no_gap` (intégration) | invariant compteur sans trou sur échec FDM | 2 | 0 | 0 |
| `moyun-pos::spf_e2e_reports` (intégration) | M180, M181 + rotation période de réservation | 3 | 0 | 0 |
| `pos_fdm_mock`, `moyun-pos` main | - | 0 | 0 | 0 |
| **Total** | | **190** | **0** | **2** (staging, opt-in) |

Doc-tests : 0 dans toutes les crates (pas d'exemples `///` rédigés).

Les 2 tests ignorés dans `pos_fiscal::e2e_staging` constituent le
harness de répétition pré-soumission ; ils requièrent
`MOYUN_STAGING_FDM_URL` pointant vers un vrai FDM virtuel
zwartedoos.be, et sont exécutables via
`cargo test --package pos-fiscal --test e2e_staging -- --ignored`.
Ils ne font PAS partie de la barrière CI hors-ligne.

### Pyramide de tests par finalité

| Finalité | Binaires de test | Nombre |
|---|---|---:|
| Couverture domaine & unit du noyau fiscal | `pos_core`, `pos_db`, `pos_fiscal`, `pos_print`, `pos_peppol` | 129 |
| *Parsing* des fixtures SPF Finances (formes requête + réponse) | `uc_fixtures`, `reports_fixtures`, `advanced_set_fixtures` | 14 |
| Aller-retour FDM par mutation (wiremock au niveau `FdmClient`) | `spf_basic_set`, `full_service` | 11 |
| **Orchestration au niveau applicatif** (commande Tauri → tx → archive) | `spf_e2e_basic`, `spf_e2e_no_gap`, `spf_e2e_reports` | 17 |
| Intégrations webshop (exclues de la build cert) | tests crate webshop | 19 |
| **Total** | | **190** |

### Carte de couverture e2e applicatif

| Famille d'UC | Testé à travers | Nombre de tests |
|---|---|---:|
| Vente (M110) | `m110_sign_sale_archives_n_event_and_closes_order` | 1 |
| Remboursement complet / partiel (M111, M112A) | `m111_...`, `m112a_...` | 2 |
| Commande / Pré-ticket (M121, M123) | `m121_...`, `m123_...` | 2 |
| Mouvements caisse (M130, M131) | `m130_...`, `m131_...` | 2 |
| Social (M140, M141) | `m140_...`, `m141_...` | 2 |
| Copie (M160) | `m160_...` | 1 |
| Formation (M170, M171) | `m170_...`, `m171_...` | 2 |
| Rapports X / Z (M180, M181) + période de réservation | trois tests `spf_e2e_reports` | 3 |
| Invariant compteur sans trou (régression FDM-fail) | `checkout_fdm_failure_...`, `refund_fdm_failure_...` | 2 |
| **Total e2e applicatif** | | **17** |

---

## Rot de tests pré-existant corrigé lors du baselining

Les deux tests d'intégration pos-fiscal ne compilaient plus contre
l'API actuelle. Corrigé :

| Fichier | Sites | Raison |
|---|---:|---|
| `crates/pos-fiscal/tests/full_service.rs` | 5 | Champ `backup_dir` manquant sur `PosConfig` ; champ `rounding` manquant sur `TicketParams` (×2) ; 8e argument `Option<&FdmReference>` manquant sur `build_sale_input` (×2). |
| `crates/pos-fiscal/tests/spf_basic_set.rs` | 6 | `backup_dir` manquant sur `PosConfig` ; 8e argument manquant sur `build_sale_input` (×5). |

Toutes les corrections étaient mécaniques - `None` pour les nouveaux
champs/arguments optionnels. Aucun changement de sémantique de test.

---

## Mises en conformité lors de cette baseline

| Fichier | Changement |
|---|---|
| `src-tauri/Cargo.toml` | Intégrations tierces marquées `optional = true` ; feature `cert` (default-off) exclut ces intégrations du binaire cert. |
| `src-tauri/src/main.rs` | Commandes Tauri des intégrations tierces gardées derrière `#[cfg(feature = "hubrise")]`. |
| `src-tauri/src/commands/checkout.rs` | **Bug fix** : passait `restaurant_est_no: ""` à `TicketParams`, passe désormais `&config.restaurant_est_no`. |
| `ui/src/router/index.ts` | Vue de gestion webshop exclue du bundle cert via variable d'environnement Vite. |

---

## Limitations connues / suivis

- **Composants UI webshop** : la vue de gestion webshop est exclue du
  bundle cert via Vite. Le côté IPC est désactivé dans la build cert
  (aucune commande webshop enregistrée).
- **Endpoint healthcheck** (`127.0.0.1:9999`) : une variante
  liveness-only cert-friendly est planifiée pour un futur polissage UI.
- **Pas de run Playwright/E2E** dans cette baseline ; le workflow CI
  existant les lance séparément.

---

## Reproductibilité

```bash
cd moyun-pos

# Build dev (features par défaut)
cargo test --workspace --no-fail-fast

# Build certification
cargo build --package moyun-pos --no-default-features --features cert
nm target/debug/moyun-pos | grep -c pos_hubrise   # → 0

# Build UI certification (tree-shake WebshopOrdersView)
VITE_FEATURE_HUBRISE=false npm --prefix ui run build
```
