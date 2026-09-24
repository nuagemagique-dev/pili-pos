# Garantie de build, procédure de mise à jour et politique de langue

**Date :** 2026-05-14
**Sections couvertes :** §II.D Garantie du produit, §II.E Mises à jour, §VI Règles de langue du manuel de certification.

---

## 1. Garantie d'identité machine-test <-> déploiement client (§II.D)

**Question du manuel :** "Quelles sont les garanties que vous avez mis en place afin de s'assurer que le logiciel de la machine-test et celui du SCE installé chez le client sont identiques ?"

**Réponse :**

La correspondance exacte entre le build de certification soumis au SPF Finances et le build déployé chez chaque client est garantie par quatre mécanismes complémentaires :

### 1.1 Build reproductible

Le dépôt source contient :
- `Cargo.lock` commité - fige les versions exactes de toutes les dépendances Rust transitives.
- `rust-toolchain.toml` épingle la version exacte du compilateur Rust (canal `1.96.0`).
- `package-lock.json` (UI Vue) - fige les versions exactes des dépendances Node.

La commande de build cert est documentée et figée :
```bash
cargo build --release --no-default-features --features cert
```

Deux compilations successives sur des machines différentes avec ce même `Cargo.lock` + même toolchain produisent un binaire identique octet par octet (modulo les timestamps embarqués, qui sont contrôlés par `SOURCE_DATE_EPOCH`).

### 1.2 Empreinte SHA-1 du binaire

Le script `scripts/build_and_hash.sh` :
1. Compile le build de certification.
2. Vérifie l'absence de symboles d'intégrations tierces (vérification anti-régression).
3. Vérifie l'absence de `DELETE FROM fiscal_events` ou `UPDATE fiscal_events` dans le source.
4. Calcule SHA-1, SHA-256 et MD5 du binaire.
5. Produit un rapport `build_info_YYYYMMDD-HHMMSS.txt`.

La valeur SHA-1 est déclarée au formulaire §I.D et au tableau §VII.F. Toute installation cliente est vérifiable via :
```bash
sha1sum /opt/moyun-pos/moyun-pos
# doit correspondre à la valeur déclarée au SPF
```

### 1.3 Environnement kiosk scellé

Le déploiement consiste à copier l'exécutable Tauri `moyun-pos` sur la machine cible via `deploy/install.sh` (premier déploiement) ou `deploy/update.sh` (mise à jour). Il n'y a pas d'image disque (.iso) : le binaire est transféré directement par `scp` et activé via systemd.

L'OS hôte (Debian 12 minimal) est configuré lors de l'installation initiale pour constituer un environnement scellé :

- Système de fichiers racine en lecture seule (option `ro` dans `/etc/fstab`).
- Aucun serveur SSH installé (`openssh-server` exclu du paquet de base).
- Aucun gestionnaire de paquets actif (apt désactivé via systemd-mask).
- Seul `/var/lib/moyun-pos` est en lecture/écriture (pour la DB et les backups).
- Cage Wayland compositor lance directement `moyun-pos` en plein écran ; pas de bureau, pas de terminal accessible.

Une fois l'installation initiale effectuée, toute modification du binaire passe obligatoirement par `deploy/update.sh` avec les droits root.

### 1.4 Tag git immutable et signé

Le commit du build de certification est tagué `vX.Y.Z` et signé GPG par le producteur. Le SHA git de ce tag est consigné dans le rapport de build et permet à tout moment de reproduire le build à l'identique à partir du dépôt source.

Toute mise à jour ultérieure (correction de bug, nouvelle fonctionnalité) passe par :
- Un nouveau tag `vX.Y.Z+1` signé.
- Une nouvelle build cert (et donc nouveau SHA-1).
- Une nouvelle demande de certification SPF Finances pour la mise à jour (procédure §II.E ci-dessous + Article 109 du Manuel).

---

## 2. Procédure de mise à jour (§II.E)

**Question du manuel :** "Pour chaque modification sur le POS, une demande de certification de mise à jour est nécessaire. Ces mises à jour sont testées sur la machine de test. Comment implémentez-vous les mises à jour sur la machine de test ?"

**Réponse :**

### 2.1 Mises à jour entre cycles de certification

Aucune mise à jour automatique en arrière-plan n'est implémentée. C'est une décision explicite de scellement de l'appliance fiscale : un POS certifié ne doit pas pouvoir voir son code changé sans une nouvelle validation SPF Finances.

La procédure de mise à jour est la suivante :

1. **Côté producteur :**
   - Implémentation des modifications dans une branche dédiée.
   - Tests automatisés (`cargo test --workspace`) passent à 100%.
   - Anti-régression : `cargo build --release --no-default-features --features cert` + `nm moyun-pos | grep -c pos_hubrise == 0`.
   - Nouveau tag git signé `vX.Y.Z+1`.
   - Production du nouvel exécutable Tauri (`moyun-pos`) via la commande de build de certification.
   - Calcul du SHA-1 du nouveau build.
   - Soumission d'une nouvelle demande de certification (mise à jour) au SPF Finances.

2. **Sur la machine de test SPF (pendant le cycle de mise à jour) :**
   - Le producteur transfère le nouveau binaire `moyun-pos` sur la machine de test (via `scp` ou remise physique sur clé USB).
   - Le binaire est installé via `deploy/update.sh` (arrêt du service, remplacement du binaire, redémarrage). Cf. *MANUEL_INSTALLATION_FR.md*.
   - Tests de validation effectués par l'agent SPF.

3. **Côté client (après acceptation SPF de la mise à jour) :**
   - Mise à jour du binaire client par le distributeur via `deploy/update.sh` lors d'une intervention sur site (ou via SSH si accès à distance temporaire accordé).
   - Sauvegarde préalable de la DB `pos.db` (cf. backups automatiques nocturnes).
   - Vérification post-update du SHA-1.

### 2.2 Accès à distance

Le manuel recommande l'usage d'une solution d'accès à distance (TeamViewer, AnyDesk, RustDesk, VNC, Zoho Assist, Splashtop) pour faciliter les mises à jour et l'assistance technique.

**Position de moyun-pos :** aucune solution d'accès à distance permanente n'est installée sur la machine de test livrée à l'Intake ni sur la production client. C'est une décision délibérée :

- **Raison principale :** ouvrir un canal d'administration distante sur un terminal de caisse fiscale augmente la surface d'attaque sans bénéfice opérationnel suffisant. Un attaquant qui compromet l'accès distant peut potentiellement contourner les protections fiscales.
- **Raison secondaire :** la spec SCE 2.0 demande un appliance scellé. Un agent SSH ou un client TeamViewer permanent contredit l'esprit du scellement.

**Compromis pratique :** pour les interventions techniques planifiées (mise à jour, debug d'un problème reporté par le client) :
- L'opérateur peut démarrer ponctuellement un client RustDesk portable (open source, binaire autonome, sans installation système) à la demande du producteur.
- Une fois la session terminée, le binaire est effacé.
- Aucun accès permanent.

**Au rendez-vous d'admission SPF Finances :**

- Accès console physique direct par l'agent (clavier USB branché pour les vérifications).
- Compte `spf` pré-créé avec mot de passe communiqué sur place.
- Aucun besoin d'accès à distance pendant l'Intake.

---

## 3. Politique de langue des tickets (§VI)

**Question du manuel :** règles de langue selon §VI - le ticket doit être disponible en français, néerlandais, allemand selon le besoin (anglais en zones touristiques).

**Réponse :**

### 3.1 Langues supportées par le code source

Le code de `moyun-pos` supporte le rendu multilingue des tickets via deux mécanismes complémentaires :

- **Libellés produits** : table `translations` polymorphe (cf. *SCHEMA_BD_FR.md*) - couvre catégories, articles, modificateurs. Les lignes du ticket portent le libellé dénormalisé dans la langue active au moment de la vente.
- **Mentions fiscales imprimées** : jeux de libellés FR/NL/DE embarqués dans le code (`crates/pos-fiscal/src/ticket.rs` : `ReceiptLanguage`, `ticket_type_label`, `invalid_disclaimer` ; `crates/pos-print/src/receipt.rs` : libellés fixes Date/Ticket/TOTAL/TVA). La langue est déterminée par la clé `language` du fichier de configuration `/etc/moyun-pos/pos.json`, mappée sur une langue nationale : `fr`/`nl`/`de` directement, toute autre valeur (en, zh) retombe sur le FR.

Langues des mentions fiscales supportées par le code :
- `fr` (français) - **complète** (libellés issus de la description détaillée POS v2.2 et du manuel §VI)
- `nl` (néerlandais) - **complète** (libellés issus des circulaires GKS)
- `de` (allemand) - **complète** (traduction de style officiel, translittération ASCII, à confirmer sur la version DE de la spec)

Langues d'affichage de l'interface (indépendantes du ticket) : `fr`, `en`, `zh`.

### 3.2 Règle appliquée : langue de la région linguistique de l'établissement

Le manuel (§VI, p. 99) dispose : *« En Belgique, le ticket doit être émis en français, en néerlandais ou en allemand, selon la langue officielle de la région linguistique où il est émis. Dans les lieux considérés comme des zones touristiques, le ticket peut également être émis en anglais. »*

La langue des mentions fiscales est donc fixée **par installation**, lors de la mise en service, d'après l'adresse de l'établissement où le POS est exploité (et non d'après le siège du producteur ni la langue de la clientèle) :

| Région linguistique de l'établissement | Valeur de `pos.json::language` |
|---|---|
| Région de langue française | `fr` |
| Région de langue néerlandaise | `nl` |
| Région de langue allemande | `de` |
| Région bilingue de Bruxelles-Capitale | `fr` ou `nl` (les deux langues sont officielles) |

L'anglais (zones touristiques) n'est pas proposé comme langue de ticket : toute valeur autre que `fr`/`nl`/`de` retombe sur le FR (cf. §3.1).

Le paramètre fait partie de la checklist de mise en service (`MANUEL_INSTALLATION_FR.md`) et figure sur chaque installation déclarée dans `LISTE_DISTRIBUTEURS_FR.md`.

**Premier client - Tokyo Sushi Bar :** établissement situé Rue de la Cathédrale 59, 4000 Liège, en **région de langue française** → `language = "fr"`, tickets imprimés en français.

**Pour basculer la langue des mentions fiscales (FR → NL ou DE)**, par exemple pour une installation dans une autre région linguistique, la procédure est une mise à jour de configuration (pas du code) :
1. Compléter les traductions manquantes dans la table `translations` pour les articles.
2. Éditer la clé `language` de `/etc/moyun-pos/pos.json` (`fr` / `nl` / `de`).
3. Redémarrer le service `moyun-pos` (la configuration est lue au boot).

Les trois jeux de mentions FR/NL/DE étant déjà compilés dans le binaire, aucune modification du binaire `moyun-pos` n'est nécessaire pour changer de langue : c'est de la **configuration**, pas du code. Donc pas de nouveau cycle de certification.

### 3.3 Tickets multilingues pour la machine de test SPF

Pour la machine de test livrée à l'Intake, le seed `pos.db` sera configuré avec les traductions FR/NL/DE/EN des produits du manuel (Dry Martini, Burger of the Chef, etc.). L'agent SPF pourra basculer la langue des mentions fiscales en éditant la clé `language` de `/etc/moyun-pos/pos.json` (procédure §3.2) et constater le changement sur le ticket suivant.

---

## 4. Récapitulatif des engagements

| Engagement | Mécanisme | Vérifiabilité |
|---|---|---|
| Build cert reproductible | `Cargo.lock` + `rust-toolchain.toml` épinglés | `cargo build --release` identique sur 2 machines |
| Build = déploiement client | Binaire Tauri scellé + SHA-1 vérifiable | `sha1sum /opt/moyun-pos/moyun-pos` |
| Pas de modification post-cert | Système de fichiers racine en lecture seule + tag git signé | Vérification au boot, lecture du tag dans `/etc/moyun-pos/version` |
| Nouvelle MAJ = nouveau cert | Procédure §II.E + Article 109 du manuel | Chaque tag `vX.Y.Z+1` ouvre une nouvelle demande SPF |
| Pas d'accès distant permanent | Pas de SSH, aucun client distant installé | Vérification au scan réseau du POS (aucun port ouvert) |
| Ticket dans la langue de la région linguistique de l'établissement | `translations` (libellés produits) + `pos.json::language` (mentions fiscales FR/NL/DE) | Inspection visuelle du ticket, paramétrage modifiable sans changer le binaire |
