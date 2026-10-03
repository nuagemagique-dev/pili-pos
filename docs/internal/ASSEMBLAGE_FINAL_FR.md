# Assemblage final - ce que l'opérateur fait au moment de soumettre

Ce document est un runbook. Les étapes ci-dessous sont manuelles et ne peuvent pas être effectuées par
l'outil de développement - elles requièrent le distributeur humain
(Bruno Moyse) pour signer les documents, construire les artefacts et
envoyer l'email.

Référence : procédure officielle `docs/20260903_Intro_certification_procédure.md`
(7 étapes, section 3 « Documents requis »). Le binaire ne doit plus changer
après l'étape 3 : toute modification du code refait les étapes 1 à 3.

## Étape 1 - Vérification de sanité

```bash
cd pili-pos   # racine du dépôt

cargo fmt --all --check                   # doit être propre
cargo clippy --workspace -- -D warnings   # doit être propre
cargo test --workspace                    # doit être tout vert
scripts/generate_test_report.py           # totaux de RAPPORT_TESTS_BASELINE_FR.md, sur cette révision
```

## Étape 2 - Commit et tag du build cert

Tout le code est commité et l'arbre est propre. Le tag marque la révision
dont le binaire est tiré :

```bash
git tag -a v1.0.0 -m "pili-pos v1.0.0, build de certification SCE 2.0"
```

Tag annoté simple. Aucun script du dépôt ne produit de signature GPG, ni
du tag ni du binaire : n'en annoncer aucune. (Un `git tag -s` est possible
si une clé GPG existe, mais c'est facultatif et hors dossier.)

## Étape 3 - Build de référence et SHA-1

```bash
# Build cert Linux x86_64 dans un conteneur Debian 13 (toolchain
# épinglée par rust-toolchain.toml via rustup). Construit l'UI,
# compile avec --no-default-features --features cert, vérifie que
# l'intégration HubRise est présente (marqueur integration_switch_log,
# activée ou non par installation) et qu'aucun symbole pos_peppol n'y
# est (la facturation Peppol est hors v1.0.0), vérifie les assets
# embarqués, calcule les empreintes et écrit build_info_*.txt
# (scripts/build_and_hash.sh).
./deploy/build-release.sh --docker

# Reproductibilité : deux builds complètes, mêmes SHA-1.
scripts/check_repro.sh            # sortie : target/repro/{run1,run2,sha1}
```

Le SHA-1 à déclarer (formulaire §I.D et email) est celui du binaire
**Linux** de référence, relevé dans `build_info_*.txt` (copié dans
`deploy/dist/`), jamais celui d'une build de développement macOS.

Copier les deux rapports de `target/repro/run1/` et `run2/` dans
`docs/SUBMISSION_ARTIFACTS/evidence/build/` (le MANIFEST les reprend ;
`GARANTIE_MAJ_LANGUES_FR.md` §1.2 les annonce).

## Étape 4 - Construire l'image kiosk d'admission

Une install Debian 13 minimale avec :

- Binaire cert `pili-pos` (paquet `deploy/dist/` produit à l'étape 3).
- Binaire `pos-fdm-mock` (pour le fallback de démo d'admission).
- Compositeur Cage Wayland.
- Unité systemd `pili-pos.service` (autostart plein écran).
- `pos.db` initialisée avec `seed_spf_test.sql` (produits prédéfinis du
  manuel §III.N + utilisateurs de test SPF).
- `chrony` configuré pour NTP.
- Pas de serveur SSH ; RustDesk portable présent, jamais lancé au boot.

L'image est construite par `deploy/image/build.sh admission` (mkosi) et
vérifiée en CI (`.github/workflows/image.yml` : build reproductible, SHA-1
du binaire embarqué, démarrage sous QEMU). Selon le budget d'Actions,
construire et démarrer l'image en local (Docker + QEMU) suffit.

## Étape 5 - Répétition pré-soumission live (accord du propriétaire requis)

Contre le simulateur en ligne module2 (URL et jeton dans `.env.live`,
cf. `docs/internal/LIVE_FDM_MODULE2.md`), avec le binaire final :

```bash
scripts/live_fdm.sh
```

Elle écrit les paires requête/réponse dans
`docs/SUBMISSION_ARTIFACTS/Report_Testing/module2/` (remplace le run du
2026-09-27, antérieur au binaire final). Ajouter les photos du ticket
imprimé sur l'imprimante réelle (procédure 3.2). Reporter les résultats
dans `docs/SUBMISSION_ARTIFACTS/JOURNAL_TESTS_PRE_SOUMISSION_FR.md`,
remplir chaque « À effectuer ». S'arrêter en cas d'échec : corriger dans
le code, refaire les étapes 1 à 3, re-répéter.

Ensuite seulement, activer la ligne `Report_Testing/module2` de
`docs/SUBMISSION_ARTIFACTS/MANIFEST.txt`.

## Étape 6 - Remplir et signer le formulaire de demande

Ouvrir `docs/20250307_POS_CERTIFICATION_MANUEAL_FRV1.1/20250115_FORMULAIRE DEMANDE_CERTIF_POS-FDM.md`
(ou le PDF). Champs à remplir selon `DOSSIER_SOUMISSION_FR.md`.
Le formulaire dit : « Ce formulaire doit être signé numériquement et ne
nous sera envoyé que par e-mail via secr.gksce@minfin.fed.be. » Signer donc
avec une signature électronique qualifiée (pas de signature manuscrite
scannée), enregistrer sous
`20250115_FORMULAIRE_DEMANDE_CERTIF_POS-FDM_signe.pdf`.

## Étape 7 - Assembler le ZIP du dossier

```bash
scripts/make_submission_zip.sh            # --skip-pdfs pour réutiliser les PDF déjà construits
```

Le script régénère les PDF du dossier et des captures, lance
`scripts/check_dossier.py --final` (références de code, vocabulaire neutre,
placeholders, captures, fraîcheur des PDF, MANIFEST ; tout écart arrête),
puis zippe exactement ce que liste `docs/SUBMISSION_ARTIFACTS/MANIFEST.txt`.
Sortie : `target/submission/pili-pos-dossier-v1.0.0.zip` et son SHA-256
affiché. Les PDF d'annexes sont à la racine du ZIP, les preuves
(`evidence/uc`, `fdm_errors`, `tickets`) sous `Report_Testing/` (procédure
3.2).

Pièces exigées par la procédure (section 3.3) : formulaire signé, manuel de
certification complété (`REPONSES_MANUEL_CERT_FR`), aperçu des distributeurs
(`LISTE_DISTRIBUTEURS_FR`), manuels (`MANUEL_*_FR`), description du processus
de production (`DOCUMENT_PROCESSUS_TECHNIQUE_FR`), description de
l'intégration des prescriptions (`MAPPING_CONFORMITE_FR`), description des
données (`FLUX_DONNEES_FR`, `FEUILLE_ACQUISITION_DONNEES_FR`), résultats de
tests (`Report_Testing`). « Une demande n'est valable que lorsque
l'ensemble de ces éléments a été transmis intégralement. »

## Étape 8 - Archive du code source

Le code source est offert sur demande, non joint d'office :

```bash
git archive --format=zip --prefix=pili-pos-v1.0.0/ -o target/submission/pili-pos-source-v1.0.0.zip v1.0.0
```

## Étape 9 - Héberger l'image et envoyer l'email

L'artefact CI `kiosk-image-admission` expire après 14 jours
(`retention-days: 14` dans `.github/workflows/image.yml`) : ne pas s'en
servir comme lien. Héberger soi-même l'image `.img.xz` (stockage objet,
WeTransfer ou équivalent) avec un lien valable 30 jours, et noter sa date
d'expiration. Procédure 3.3 : si le ZIP est trop gros, contacter l'équipe
de certification, qui met une plateforme de dépôt à disposition.

Utiliser le gabarit de `DOSSIER_SOUMISSION_FR.md`. Joindre :
1. Formulaire de demande de certification signé numériquement (PDF).
2. ZIP du dossier (`pili-pos-dossier-v1.0.0.zip`).
3. Le lien de téléchargement de l'image kiosk d'admission (binaire
   `pili-pos` embarqué), et le SHA-1 du binaire dans le corps de l'email.

Envoyer à : `secr.gksce@minfin.fed.be`. Envoi par le propriétaire seul.

## Étape 10 - Après l'envoi

Procédure officielle, étapes 3 à 7 :

1. **Pré-intake virtuel** (étape 3) : « une réunion de pré-intake en ligne
   est organisée. [...] le producteur a l'occasion de présenter brièvement
   son système ». Retours possibles : « les tests modifiés devront être
   soumis à nouveau » ; un second pré-intake si le dossier n'est pas assez
   avancé.
2. **Sites clients de test** : « après approbation explicite du
   certificateur, installer au maximum cinq environnements de test chez des
   clients ». Aucune installation chez un client sans cet accord écrit.
   Décision déjà prise (plan, section 6) : en utiliser autant que permis.
3. **Intake** (étape 4) : la caisse est installée dans le laboratoire à
   Bruxelles ; « une alimentation électrique ainsi qu'une connexion
   Internet ouverte via Wi-Fi ou câble UTP sont mises à disposition ».
   Apporter : kiosk + imprimante, câbles Ethernet (UTP) de rechange, clé USB
   avec le dossier, l'image. Vérifier avant le jour J que le posId et le
   jeton autorisés du simulateur module2 fonctionnent depuis le réseau du
   laboratoire (pare-feu, port 80/443 vers `fdm.module2.be`), ou prévoir le
   FDM sur le LAN.
4. **Certification, tests de deuxième ligne, certificat** (étapes 5 à 7).
5. **Après la certification** : « les producteurs qui souhaitent distribuer
   eux-mêmes leur système certifié doivent s'enregistrer en tant que
   distributeur ». Toute mise à jour touchant le volet SCE doit être
   certifiée avant installation chez les clients (section 4 de la
   procédure).

SPF Finances accuse typiquement réception sous 5 jours ouvrables (délais
indicatifs, non fixés par la procédure 2026).

## Voie d'échec

La procédure 2026 ne dit rien d'un délai avant une nouvelle demande : elle
parle de « rapports de feedback » pour indiquer les adaptations nécessaires
(étape 5). Le délai de 3 mois (Art. 109) vient de l'ancienne introduction du
manuel ; à confirmer auprès du secrétariat avant de s'y fier. Utiliser le
temps pour corriger les écarts signalés.

## Ce qui n'a PAS été fait dans cette base de code

Les éléments suivants sont intrinsèquement côté opérateur et n'ont
pas été automatisés :

- Signature numérique du formulaire (requiert un certificat de signature
  électronique qualifiée).
- Envoi de l'email proprement dit.
- Hébergement de l'image kiosk (lien 30 jours).

Peppol (contrat Access Point) est hors v1.0.0 ; à traiter après la
certification, pour la v1.1 (`DECISION_PEPPOL_AP_FR.md`).

## Reste à faire (ordonné)

Légende : DEV = faisable dans le dépôt ; PROPRIÉTAIRE = décision ou action
humaine ; MATÉRIEL = kiosk, imprimante, réseau ; LIVE = appel au simulateur
module2, accord du propriétaire requis avant chaque run.

| # | Action | Type |
|---|---|---|
| 1 | Figer le code ; `cargo fmt --all --check`, clippy, tests (étape 1) | DEV |
| 2 | Commit + tag annoté `v1.0.0` (étape 2) | DEV |
| 3 | `deploy/build-release.sh --docker` + `scripts/check_repro.sh`, relever le SHA-1, copier les rapports dans `evidence/build/` (étape 3) | DEV |
| 4 | Construire l'image d'admission, la démarrer sous QEMU (étape 4) | DEV |
| 5 | Répétition `scripts/live_fdm.sh` avec le binaire final (étape 5) | LIVE |
| 6 | Photos des tickets sur l'imprimante réelle ; scan du QR (journal, sections manuelles) | MATÉRIEL |
| 7 | Reporter les résultats dans le journal ; ajouter `Report_Testing/module2` au MANIFEST | DEV |
| 8 | Vérifier le posId/jeton module2 depuis un réseau extérieur (proxy du labo : à reconfirmer sur place) | LIVE |
| 9 | Remplir et signer numériquement le formulaire (étape 6) | PROPRIÉTAIRE |
| 10 | `scripts/make_submission_zip.sh`, noter le SHA-256 (étape 7) | DEV |
| 11 | `git archive` du tag (étape 8) | DEV |
| 12 | Héberger l'image, lien 30 jours, date d'expiration (étape 9) | PROPRIÉTAIRE |
| 13 | Envoyer l'email à `secr.gksce@minfin.fed.be` (étape 9) | PROPRIÉTAIRE |
| 14 | Pré-intake virtuel ; resoumettre les tests modifiés si demandé | PROPRIÉTAIRE |
| 15 | Demander l'accord pour les sites clients de test (max 5) | PROPRIÉTAIRE |
| 16 | Intake au laboratoire : kiosk, câbles Ethernet, USB | MATÉRIEL |
| 17 | S'enregistrer comme distributeur après le certificat | PROPRIÉTAIRE |
