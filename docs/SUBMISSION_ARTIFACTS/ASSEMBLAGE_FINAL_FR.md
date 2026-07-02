# Assemblage final - ce que l'opérateur fait au moment de soumettre

Ce document est un runbook. Le travail de code est terminé ; les
étapes restantes sont manuelles et ne peuvent pas être effectuées par
l'outil de développement - elles requièrent le distributeur humain
(Bruno Moyse) pour signer les documents, construire les artefacts et
envoyer l'email.

## Étape 1 - Vérification de sanité de la build cert

```bash
cd /Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos

cargo test --workspace        # doit être tout vert
cargo clippy --workspace -- -D warnings   # doit être propre
cargo fmt --all --check       # doit être propre

cargo build --release --no-default-features --features cert
nm target/release/moyun-pos | grep -c pos_hubrise   # doit afficher 0
```

## Étape 2 - Exécuter la répétition pré-soumission

Une fois le compte FDM virtuel zwartedoos.be activé :

```bash
export MOYUN_STAGING_FDM_URL=https://...   # depuis le dashboard zwartedoos.be
cargo test --package pos-fiscal --test e2e_staging -- --ignored --nocapture \
  | tee -a docs/SUBMISSION_ARTIFACTS/JOURNAL_TESTS_PRE_SOUMISSION_FR.md
```

Parcourir les 10 sections de `JOURNAL_TESTS_PRE_SOUMISSION_FR.md`,
remplir chaque À effectuer au fur et à mesure. S'arrêter en cas d'échec -
corriger dans le code, rebuilder, re-répéter.

## Étape 3 - Construire l'image kiosk

Une install Debian 12 minimale avec :

- .deb de build cert de `moyun-pos` (construit à partir de
  `target/release/`).
- Binaire `pos-fdm-mock` (pour le fallback de démo d'admission).
- Compositeur Cage Wayland.
- Unité systemd `moyun-pos.service` (autostart plein écran).
- `pos.db` de seed avec menu Tokyo Sushi Bar + utilisateurs de test.
- `chrony` configuré pour NTP.
- Pas de serveur SSH.

Approche recommandée : un script packer / debootstrap dans une PR de
suivi. Pour la soumission, une install manuelle sur une image disque
USB-bootable est suffisante.

## Étape 4 - Remplir le formulaire de demande de certification

Ouvrir `docs/20250307_POS_CERTIFICATION_MANUEAL_FRV1.1/20250115_FORMULAIRE DEMANDE_CERTIF_POS-FDM.md`
(ou le PDF). Champs à remplir selon `DOSSIER_SOUMISSION_FR.md`.
Imprimer le PDF rempli, signer avec une signature électronique
qualifiée (ou une signature manuscrite + scan de carte d'identité),
enregistrer sous
`20250115_FORMULAIRE_DEMANDE_CERTIF_POS-FDM_signe.pdf`.

## Étape 5 - Assembler le ZIP du dossier

```bash
cd docs/SUBMISSION_ARTIFACTS
zip -r ../moyun-pos_dossier_cert_$(date +%Y-%m-%d).zip .
```

Déplacer le zip résultant vers un emplacement sécurisé (l'email de
soumission l'attache).

## Étape 6 - Envoyer l'email

Utiliser le gabarit de `DOSSIER_SOUMISSION_FR.md`. Joindre :
1. Formulaire de demande de certification signé (PDF).
2. ZIP du dossier.
3. Un lien de téléchargement (par ex. WeTransfer, expiration à
   30 jours) vers le binaire `moyun-pos` (build cert, signé GPG).

Envoyer à : `secr.gksce@minfin.fed.be`.

## Étape 7 - Attendre

SPF Finances accuse typiquement réception sous 5 jours ouvrables et
planifie un rendez-vous d'admission ~4 semaines plus tard. Pendant
le rendez-vous, apporter le matériel kiosk + le simulateur FDM + le
dossier sur une clé USB.

## Voie d'échec

Si la certification est refusée (Art. 109) : il y a un **délai de
3 mois** avant de re-soumettre. Utiliser ce temps pour corriger les
écarts signalés par SPF Finances. Le rapport d'admission devrait
arriver par email dans les 2 semaines suivant le rendez-vous.

## Ce qui n'a PAS été fait dans cette base de code

Les éléments suivants sont intrinsèquement côté opérateur et n'ont
pas été automatisés :

- Signature du formulaire (requiert un certificat de signature
  électronique qualifiée).
- Achat du FDM virtuel zwartedoos.be (requiert paiement + KYC).
- Construction de l'image OS (reportée à une PR de suivi avec un
  script packer).
- Achat d'un contrat Peppol Access Point (relation commerciale
  Storecove/Unifiedpost).
- Envoi de l'email proprement dit.

Le code, les tests et le dossier sont prêts. Les éléments restants
sont du travail de processus pour le soumetteur humain.
