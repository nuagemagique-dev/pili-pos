<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/RAPPORT_TESTS_BASELINE_FR.md @ 86088e4 -->
# Rapport de tests baseline (couverture de la build cert) - pili-pos-android

**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
**Date :** À effectuer (EV-18 : date de dépôt)
**Toolchain :** Rust 1.99.0 épinglée (`rust-toolchain.toml`), edition 2024, deux workspaces Cargo (la racine et `android/rust`), Kotlin et Gradle pour l'application, JDK 17, NDK 27.1.
**Objet :** photographier la posture de tests de la variante Android à la date de soumission, afin que tout mouvement de tests ultérieur puisse être tracé et attribué à des modifications futures.

Le moteur fiscal est le même code source que celui de `pili-pos` (voir `CADRE_DEMANDE_FR.md` §4) : ses suites de tests sont celles du rapport Linux, rejouées ici sur l'hôte de construction et sur le terminal émulé. Ce rapport ajoute ce qui est propre à Android : la couche de liaison, l'application Kotlin et les parcours de bout en bout sur le terminal émulé.

**Provenance des chiffres.** Chaque chiffre ci-dessous est repris d'un rapport de sortie de phase du dépôt, avec le commit auquel il a été mesuré. Aucun total n'est recopié à la main d'une exécution du commit de livraison : les totaux finaux de ce commit sont `À effectuer (EV-17 : comptes finaux au commit de livraison)`.

---

## Vérification de build

| Configuration | Commande | Résultat |
|---|---|---|
| Moteur pour Android | `cargo ndk` (arm64-v8a, API 33) sur `android/rust/pos-ffi`, qui dépend de `pos-engine` avec `--no-default-features --features cert` | OK : bibliothèque produite (74155856 octets relevés par la porte de contrôle, `X1-exit.md` à a364ad7) ; 4 segments LOAD alignés à 0x4000 (`P6-exit.md` à 4cfe0b3) |
| Application | `./gradlew lint ktlintCheck detekt test koverVerifyDebug assembleRelease` dans `android/` | OK : 291 tâches (`merge-lane-export-lane-ui-lane-plat.md` à 9a6d29e) |
| APK reproductible | `scripts/android/check_repro.sh` | OK : deux APK non signés identiques (`P6-exit.md`, à 986108b) ; empreintes `À effectuer (EV-05 : empreinte de l'image et SHA-1 des deux APK au commit de livraison)` |
| Gel du moteur | `scripts/android/check_linux_frozen.sh` | `À effectuer (EV-04 : commit de l'étiquette de gel linux-v1.0.0-rc2 et SHA-1 du binaire Linux du même moteur)` |

**Vérification du périmètre de la build cert.** La bibliothèque embarquée est construite comme le binaire cert Linux : HubRise présent, facturation B2B (événement I) et file Peppol présentes, sans fonction Cargo (`android/rust/pos-ffi/Cargo.toml`). Le contrôle de périmètre (`check_ipc_wrappers.py`) donne 128 commandes, chacune appelée une fois par son enveloppe. Les 128 commandes du jeu cert sont toutes des méthodes de l'interface Kotlin : parité 128/128, aucune manquante (`check_command_parity.py --strict`). Contrôle de l'arbre des dépendances au commit de livraison : `À effectuer (EV-17 : cargo tree de pos-ffi au commit de livraison)`.

---

## Totaux de tests

Les chiffres viennent des portes de contrôle des phases (`x1_gate.sh`, `p5_gate.sh`, `p6_gate.sh`) et du script d'inventaire. Les suites hors ligne tournent à chaque porte ; les tests marqués « ignorés » appellent un service en ligne et ne sont jamais lancés hors répétition.

### Moteur partagé, sur l'hôte de construction

| Suite | Résultat | Rapport et commit |
|---|---|---|
| Workspace racine (`cargo test --workspace`) | 1627 passants, 0 échec | `X1-exit.md`, a364ad7 ; confirmé au 9a6d29e (`merge-lane-export-lane-ui-lane-plat.md`) |
| Build cert, `pili-pos` | 8 passants, 0 échec | `X1-exit.md`, a364ad7 |
| Build cert, `pos-engine` | 591 passants, 0 échec | `X1-exit.md`, a364ad7 (rejeu après nettoyage du cache de compilation du banc de test, X1D29) |
| Inventaire (`scripts/android/test_inventory.sh --diff`) | aucun test disparu : 1640 tests du workspace, contre 1355 à la baseline | `X1-exit.md`, a364ad7 |

### Couche Android, sur l'hôte de construction

| Suite | Résultat | Rapport et commit |
|---|---|---|
| Tests Rust de la liaison (`android/rust`) | 242 passants | `X1-exit.md`, a364ad7 (236 à `P6-exit.md`, 4cfe0b3, plus 6 tests des méthodes d'export nocturne) |
| Parité des commandes moteur et interface Kotlin | 86/86, aucune manquante | `merge-lane-export-lane-ui-lane-plat.md`, 9a6d29e |
| Parité des écrans (`check_ui_parity.py --strict`) | 75 commandes appelées et 9 exclues sur 84, 0 en attente, 0 manquante | idem ; les 9 exclues sont les 4 commandes d'assistance à distance (absente d'Android) et 5 commandes sans appelant dans l'interface Linux (`docs/android/PARITY.md`) |
| Inventaire des tests Kotlin (`kotlin_test_inventory.py`) | 1957 à la tête de l'intégration, 0 supprimé | idem ; le commit 602544c n'ajoute que de la documentation à 9a6d29e |
| Tests unitaires du module `:app` | 1913 | idem (porte `p5_gate.sh --exit --parity-strict`, captures de référence vérifiées) |
| Couverture du module `:app` (seuils 80 % et 60 %) | lignes 97,4 %, branches 74,7 % | idem |

### Terminal émulé (profil Sunmi D3 Pro, Android 13)

| Parcours | Résultat | Rapport et commit |
|---|---|---|
| Tests du moteur sur l'émulateur (`scripts/android/device_engine_tests.sh --check`) | 1384 tests listés dans 88 binaires : 1178 passants, 13 ignorés (tests en ligne) ; 193 échecs, tous dus à la lecture de fixtures sur la machine de construction ; 0 instable ; 0 différence Android | `docs/android/DEVICE_TESTS.md`, `P3-exit.md`, script au commit 9dfb6d5, exécution du 2026-10-08 |
| Idem avec les fixtures poussées sur l'appareil (`--fixtures`) | 1371 passants, 0 échec, 13 ignorés | idem |
| Bout en bout de l'application (`app_e2e.sh`) | 11 contrôles sur 11, qui établissent la mise en service par paquet signé, le moteur prêt, le verrouillage kiosque, la connexion d'un gérant contre le FDM simulé et le retour prêt et verrouillé après un arrêt forcé | `P6-exit.md`, 4cfe0b3 ; `X1-exit.md`, a364ad7 |
| Bout en bout de la plateforme (`plat_e2e.sh`) | 19 contrôles sur 19 : 13 restrictions du propriétaire d'appareil, export vers le SPF avec vérification de `MANIFEST.sha256`, mise à jour par session d'installation signée avec refus des cas `not_newer` et `sha256_mismatch`, redémarrage par le chien de garde, journal d'audit | `P6-exit.md`, 4cfe0b3 |

Le profil émulé de ces mesures est `sunmi_d3_pro_api33`. La variante avec services Google (`sunmi_d3_pro_gms_api33`) a reçu le parcours de la plateforme (`scripts/android/plat_e2e.sh`, 19 contrôles, 0 échec, 15.10.2026) : EV-13, `evidence/gms/EV13_2026-10-15.md`.

### Totaux à la livraison

| Suite | Résultat |
|---|---|
| Workspace racine, build cert, `android/rust`, inventaire Kotlin, parcours UC | `À effectuer (EV-17 : comptes finaux au commit de livraison)` |
| Résultats des UC officiels sur le terminal émulé | `À effectuer (EV-06 : résultats des UC)`, voir `RESULTATS_TESTS_UC_FR.md` |

Les tests ignorés de `live_fdm`, `live_hubrise`, `live_combined` et `e2e_staging` constituent le harnais de répétition pré-soumission ; ils requièrent le simulateur en ligne module2 ou un compte HubRise de test. Ils ne font pas partie des portes hors ligne. La répétition sur le terminal Android est consignée dans `JOURNAL_TESTS_PRE_SOUMISSION_FR.md` (`À effectuer (EV-14 : répétition en ligne sur le simulateur)`).

### Familles de tests

| Finalité | Où elles tournent sur Android |
|---|---|
| Domaine et noyau fiscal (`pos-core`, `pos-db`, `pos-fiscal`, `pos-print`, `pos-mail`) | hôte et émulateur (liste des binaires dans `docs/android/DEVICE_TESTS.md`) |
| Rejeu des UC officiels par les commandes réelles | hôte (build cert) ; émulateur avec `--fixtures` ; UC joués par l'interface : `À effectuer (EV-06 : résultats des UC)` |
| Numérotation sans trou, pannes FDM, crash, concurrence | hôte et émulateur (`spf_e2e_*`) |
| Profils utilisateurs, validation des entrées, remboursements | hôte et émulateur |
| Ticket numérique, impression, tiroir | hôte et émulateur ; tickets imprimés sur l'imprimante simulée : `TICKETS_EXEMPLES_FR.md` |
| Liaison moteur-Kotlin (méthodes, flux d'événements, arrêt) | `android/rust` (242) et module `:pos-core` |
| Écrans Compose, navigation, kiosque, plateforme | tests unitaires du module `:app` (1913) et captures de référence |
| Mise en service, verrouillage, mise à jour, export vers le SPF | `app_e2e.sh` et `plat_e2e.sh` sur l'émulateur |

---

## Rot de tests pré-existant corrigé lors du baselining

Sans objet sur Android : ce chapitre du rapport Linux décrit des corrections de la baseline du moteur Linux. Le présent rapport ne reprend pas cet historique ; le moteur testé ici est celui du gel (`CADRE_DEMANDE_FR.md` §4).

## Mises en conformité lors de cette baseline

Sans objet sur Android : le périmètre cert du moteur est celui du rapport Linux. La seule particularité propre à Android est que l'interface Vue et la coque Tauri sont remplacées par l'application Kotlin et la liaison UniFFI, couvertes par les tableaux ci-dessus.

---

## Limitations connues / suivis

- **Terminal émulé.** Toutes les mesures ci-dessus viennent d'un profil émulé du Sunmi D3 Pro. Aucune n'établit un fait matériel (chrono, mémoire, carte SD, imprimante Epson réelle) : `À effectuer (EV-15 : constats sur un vrai terminal, ou mention que la machine de test est le profil émulé)`.
- **Moteur sur l'émulateur.** L'exécution du 2026-10-08 précède l'ajout de l'export nocturne au moteur ; elle est à refaire au commit de livraison (`EV-17`). Les 193 échecs de la lecture de fixtures disparaissent avec `--fixtures` et ne cachent aucune différence de comportement.
- **Test instable sous charge.** Un test de la liaison (`lifecycle`, arrêt annulé puis arrêt) a échoué une fois sous forte charge de la machine de construction et passe seul (`X1-exit.md`, X1D22).
- **Parcours de longue durée.** Marche de 24 h avec le service d'avant-plan : `À effectuer (EV-11 : marche de 24 h)`. Reprise après coupure de courant : `À effectuer (EV-12 : coupure en pleine transaction)`. Parcours du terminal complet sur les deux profils : `À effectuer (EV-10 : parcours D3 Pro)`.
- **Comparaison avec Linux.** Journaux et requêtes FDM comparés pour les mêmes scénarios : `À effectuer (EV-07 : comparaison Android et Linux)`.
- **Interface.** Les captures de référence des écrans Compose servent de garde contre les régressions visuelles ; les 122 captures du dossier (profil avec services Google, A01 à A122) sont dans `CAPTURES_ECRAN_FR.pdf`.

---

## Reproductibilité

```bash
cd pili-pos

# Moteur partagé, sur l'hôte
cargo test --workspace --no-fail-fast
cargo test -p pos-engine --no-default-features --features cert
bash scripts/android/test_inventory.sh --diff

# Liaison Rust et application, dans android/
(cd android/rust && cargo test)
(cd android && ./gradlew lint ktlintCheck detekt test koverVerifyDebug assembleRelease)

# Terminal émulé (profil sunmi_d3_pro_api33)
bash scripts/android/device_engine_tests.sh --check
bash scripts/android/app_e2e.sh
bash scripts/android/plat_e2e.sh
```
