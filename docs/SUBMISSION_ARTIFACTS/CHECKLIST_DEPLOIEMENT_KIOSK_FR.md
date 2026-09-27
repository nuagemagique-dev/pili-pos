# Checklist de déploiement kiosk

Ce que le relecteur d'admission cert doit s'attendre à trouver sur la
machine de test, de la clé USB au premier démarrage. Reflète l'image
kiosque mkosi (`deploy/image/`) et le flux de provisioning en trois
phases (`deploy/provision/provision.sh`), pas l'ancien installeur
monolithique.

## Couche matériel

- [ ] Penetek A3216 M82 ou Advantech USC-360J (ou terminal x86_64
      fanless équivalent avec écran tactile PCAP 15.6" 1920×1080).
- [ ] **TPM 2.0 activé en BIOS** (fTPM/Intel PTT ou puce discrète) -
      requis pour le chiffrement LUKS2 de `/var` au premier démarrage
      (`SECURITE_BD_FR.md` §3.6). Sans cela, `moyun-provision.service`
      échoue et le terminal ne dépasse jamais l'écran de mots de passe.
- [ ] Secure Boot **désactivé** (aucune infrastructure de signature de
      l'UKI n'existe encore - l'activer empêche le boot).
- [ ] Imprimante thermique Epson TM-m30III raccordée au LAN (TCP 9100 ; l'USB n'est pas pris en charge).
- [ ] Tiroir-caisse RJ11 raccordé à l'imprimante.
- [ ] URL du FDM module2 (boîtier sur le LAN, ou simulateur en ligne
      `https://fdm.module2.be/graphql/` pour la machine de test)
      configurée dans `/etc/moyun-pos/pos.json::fdm_url` (en
      `https://`), son jeton Bearer dans le fichier désigné par
      `fdm_token_file`, et les certificats module2 (`ca.crt`,
      `client.crt`, `client.key` en droits 600) dans
      `/etc/moyun-pos/fdm/`, désignés par `fdm_ca_file`,
      `fdm_client_cert_file`, `fdm_client_key_file`.

## Couche OS (image mkosi, `deploy/image/`)

- [ ] Debian 13 "trixie" minimal, construit par mkosi
      (`deploy/image/mkosi.conf`, profils `production`/`admission`
      sous `mkosi.profiles/`) - sans environnement de bureau.
- [ ] Disque GPT : ESP (512 M) + racine ext4 **lecture seule** (4 G,
      systemd-boot + UKI) + partition `moyun-data` (tout l'espace
      restant, créée par `systemd-repart` au premier démarrage).
- [ ] `moyun-data` chiffrée **LUKS2**, déverrouillage automatique via
      **TPM2** (PCR 7) + clé de récupération de secours, mise en place
      par `moyun-provision.service` au tout premier démarrage
      uniquement (`SECURITE_BD_FR.md` §3.6).
- [ ] Mot de passe `root` aléatoire (et `spf` sur l'image d'admission)
      généré et affiché **une seule fois** à la fin du premier
      démarrage, avec la clé de récupération LUKS - à vérifier que le
      relecteur/distributeur les a bien notés
      (`MANUEL_INSTALLATION_FR.md` §3.4).
- [ ] Compositeur Cage Wayland - lance automatiquement `moyun-pos` en
      plein écran au boot, sur `tty1` exclusivement
      (`Conflicts=getty@tty1.service`).
- [ ] Service systemd `moyun-pos.service` activé, tourne sous
      l'utilisateur non-root `pos` (`SECURITE_BD_FR.md` §3.2).
- [ ] Heure système synchronisée via chrony (les horodatages fiscaux
      corrects sont une exigence cert).
- [ ] RustDesk pinné (SHA-256 vérifié), extrait avec
      `--appimage-extract` dans `/opt/moyun-pos/rustdesk/`
      (`AppRun`) - aucun service, jamais lancé au boot
      (`deploy/REMOTE-OPS.md`).
- [ ] Pas de serveur SSH (appliance scellée), Bluetooth/CUPS/Avahi/
      ModemManager masqués. Accès console maintenance uniquement sur
      `tty2` (`Ctrl+Alt+F2`), terminal physique, login `root`.
- [ ] USB installer/updater : le même artefact `.img.xz` propose au
      boot les entrées systemd-boot **"Install"** et **"Update"**
      (`moyun.installer=install|update`) - "Install" réplique la table
      de partitions (`sgdisk --replicate`) et écrase le disque interne
      ; "Update" ne recopie que ESP+racine (`dd` partition à
      partition) et **préserve** `moyun-data` (donc `pos.db`).

## Couche application

- [ ] Build cert de `moyun-pos` installée dans `/opt/moyun-pos/`. Produite
      par `deploy/build-release.sh --docker` (build Linux x86_64,
      `--no-default-features --features cert`, UI embarquée) - le
      SHA-1 déclaré est vérifié par `.github/workflows/image.yml`
      (job `repro`, build reproductible x2, puis job `image`, hash de
      `/opt/moyun-pos/moyun-pos` **à l'intérieur** de l'image finie,
      partition racine montée en lecture seule).
- [ ] Binaire `pos-fdm-mock` dans `/opt/moyun-pos/bin/pos-fdm-mock`
      (utilisé pendant la démo d'admission si le FDM virtuel est
      injoignable) - image d'admission uniquement.
- [ ] Base de données de seed dans `/var/lib/moyun-pos/pos.db`,
      initialisée à la construction de l'image (dans
      `/usr/share/moyun-pos/factory/` puis copiée sur `/var` déchiffré
      au premier démarrage) avec `src-tauri/seed_spf_test.sql`
      (image d'admission) :
      - Lignes de produits prédéfinies du manuel §III.N (Dry Martini,
        Burger of the Chef, …) avec leurs productId et codes TVA,
        requises pour rejouer les UC tels que définis.
      - Utilisateurs de test correspondant à la liste INSZ dans
        `docs/20250916_UC_BasicSet_POS_FDM_1.7/UC_TestSetup_20240806.md`.
      - L'image de production (restaurant) utilise à la place
        le catalogue propre à l'établissement ; le binaire
        `moyun-pos` est identique dans les deux images.
- [ ] `pos.json::device_id` renseigné automatiquement par
      `moyun-provision` (adresse MAC du terminal, pas
      `CHANGE_ME_MAC`) - voir `MANUEL_INSTALLATION_FR.md` §4.2.
- [ ] IP LAN de l'imprimante configurée dans `pos.json::printer_address`.
- [ ] Sur l'image d'admission : compte `spf` (groupe `pos`, `tty2`,
      mot de passe affiché une seule fois au premier démarrage) peut
      exécuter `pos-db-ro` pour une lecture seule de `pos.db` sans
      jamais avoir besoin d'un accès écriture
      (`FEUILLE_ACQUISITION_DONNEES_FR.md` §2.2).

## Documentation dans la clé USB du dossier

- [ ] `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md`
- [ ] `FLUX_DONNEES_FR.md`
- [ ] `MAPPING_CONFORMITE_FR.md`
- [ ] `STRATEGIE_COMPTEURS_FR.md`
- [ ] `COUVERTURE_FIXTURES_FR.md` + `COUVERTURE_RAPPORTS_FR.md`
      + `COUVERTURE_ADVANCED_SET_FR.md`
- [ ] `FLUX_REMBOURSEMENT_FR.md`
- [ ] `DECISION_PEPPOL_AP_FR.md`
- [ ] `LIGNES_PRODUITS_FR.md`
- [ ] `LISTE_DISTRIBUTEURS_FR.md`
- [ ] `RAPPORT_TESTS_BASELINE_FR.md`
- [ ] `MANUEL_INSTALLATION_FR.md`, `SECURITE_BD_FR.md`,
      `FEUILLE_ACQUISITION_DONNEES_FR.md` (à jour de l'image mkosi)
- [ ] Formulaire de demande de certification signé (PDF + copie
      scannée signée).
- [ ] `JOURNAL_TESTS_PRE_SOUMISSION_FR.md` (rempli après la répétition
      de Phase 11).

## Commandes de vérification (sur la machine de test)

L'image kiosk de production embarque **sans code source ni toolchain
Rust** - uniquement le binaire signé, la base SQLite et le service
systemd. Les commandes de vérification ci-dessous s'exécutent sur la
**machine de build du développeur** (un poste de travail séparé),
contre l'arbre source qui a produit le binaire, pas sur le kiosk.

```bash
# Sur la MACHINE DE BUILD (arbre source présent), avant la production
# de l'image kiosk :
cd ~/moyun-pos                          # le dépôt source

# 1) Build cert reproductible (linux/amd64, Docker) + rapport SHA-1.
./deploy/build-release.sh --docker
# → deploy/dist/moyun-pos, deploy/dist/build_info_*.txt

# 2) Confirmer que le binaire cert ne contient aucune intégration
#    tierce désactivée (symboles résiduels attendus = 0).
nm target/linux-amd64/release/moyun-pos | grep -c pos_hubrise   # → 0

# 3) Confirmer que fiscal_events est append-only (pas de DELETE/UPDATE
#    dans le code).
grep -r "DELETE FROM fiscal_events\|UPDATE fiscal_events" \
  src-tauri/ crates/                                            # → aucun match

# 4) Lancer la suite de tests hors-ligne.
cargo test --workspace --no-fail-fast                           # → tout vert

# 5) Journée d'exploitation contre le simulateur module2 (URL et jeton
#    dans .env.live, cf. docs/internal/LIVE_FDM_MODULE2.md).
scripts/live_fdm.sh
# → tous les UC passent ; chaque réponse passe le contrôle de cohérence

# 6) Construire l'image kiosque (les deux profils) - root Linux x86_64
#    réel, ou conteneur linux/amd64 privilégié depuis macOS.
sudo deploy/image/build.sh production admission
# → deploy/image/mkosi.output/<profil>/moyun-pos-kiosk-<profil>.img.xz
```

Sur le **kiosk lui-même**, une fois démarré, le relecteur peut
vérifier que la version en cours d'exécution correspond au hash du
dossier :

```bash
sha1sum /opt/moyun-pos/moyun-pos
# → doit correspondre à la valeur déclarée dans le formulaire cert
#   et dans deploy/dist/build_info_*.txt
```

`.github/workflows/image.yml` automatise les étapes 1 et 6 en CI :
job `repro` (build #1/#2, compare les SHA-1), job `image` (build les
deux profils, vérifie sur la partition racine montée en lecture seule
que le binaire embarqué dans chaque `.img.xz` a bien le SHA-1 certifié), job
`boot-smoke` (premier démarrage complet de l'image d'admission sous
QEMU+KVM+swtpm : provisionnement LUKS2 + TPM2, écran des mots de passe
validé par Entrée, puis kiosque ; échoue si un service a échoué, si un
cycle d'ordonnancement a été rompu ou si `moyun-pos.service` ne reste
pas démarré, d'après le journal du système invité copié sur son port
série ; capture d'écran finale et journal publiés en artefact CI `boot-smoke-screenshot`).

## Script de démonstration pour le rendez-vous d'admission

0. **Avant le rendez-vous** : vérifier que le premier démarrage a déjà
   été fait sur CE terminal (LUKS2/TPM2 provisionné, mots de passe
   notés - §3.4 du manuel d'installation) ; le rendez-vous d'admission
   ne doit pas être le moment où l'assistant découvre l'écran de mots
   de passe à usage unique.
1. Boot du kiosk ; montrer Cage qui lance moyun-pos en plein écran.
2. Login avec INSZ ; montrer l'événement fiscal **S** work-in archivé.
3. Ouvrir Table 4 ; ajouter trois articles ; envoyer en cuisine -
   événement **P** + ticket cuisine.
4. Split de la table 2/1 ; payer un côté en espèces, l'autre en
   carte - **N** ×2.
5. Réimprimer le ticket espèces - événement **C** avec `fdmRef`.
6. Déclencher un rapport X - événement **R**.
7. Déclencher la clôture journalière Z - événements **R**+**R**,
   tiroir ouvert.
8. Pointage sortie - événement **S**.
9. Inspecter `fiscal_events` en lecture seule (`pos-db-ro` sur le
   compte `spf`, ou `sqlite3 -readonly` sur la console `root`) -
   compteurs strictement monotones par label ; chaque ligne a un
   `fdm_response_json` non-null.
10. Démontrer le blocage en cas de FDM déconnecté en tuant le mock
    FDM en plein milieu d'un flux.
