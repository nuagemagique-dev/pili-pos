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
- `rust-toolchain.toml` épingle la version exacte du compilateur Rust (canal `1.95.0`).
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
- L'opérateur peut démarrer ponctuellement un client TeamViewer Quick Support (binaire portable, sans installation système) à la demande du producteur.
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

Le code de `moyun-pos` supporte le rendu multilingue des tickets via :

- Table `translations` polymorphe (cf. *SCHEMA_BD_FR.md*) - couvre catégories, articles, modificateurs.
- Paramètre opérateur `users.language_preference` (valeurs `fr` / `nl` / `de` / `en` / `zh`).
- Crate `pos-print` fait le rendu du ticket en utilisant la traduction correspondante à la langue de l'opérateur ou de la session.

Langues supportées par le code et présentes dans le seed de certification SPF :
- `fr` (français) - **complète**
- `nl` (néerlandais) - **complète**
- `de` (allemand) - **complète**
- `en` (anglais) - **complète**
- `zh` (chinois simplifié) - **complémentaire** (langue de travail interne, équipe sinophone)

### 3.2 Politique de production pour Tokyo Sushi Bar

Pour le déploiement initial chez Tokyo Sushi Bar (Bruxelles), les **tickets sont imprimés en français exclusivement** :

- Clientèle locale francophone (Bruxelles).
- Équipe de service francophone/sinophone (qui parle aussi français aux clients).
- Aucune demande NL/DE de la part du client.

**Conformité avec la spec belge :**

La spec SCE 2.0 indique que le ticket doit être disponible dans la langue du client. Cette obligation s'applique en pratique selon la région linguistique du restaurant :
- Bruxelles bilingue : FR et NL acceptables, le restaurateur choisit selon sa clientèle.
- Tokyo Sushi Bar étant à Bruxelles et déclarant une clientèle francophone majoritaire, l'émission FR uniquement est conforme.

**Pour activer une langue supplémentaire** : la procédure est une mise à jour de configuration (pas du code) :
1. Compléter les traductions manquantes dans la table `translations` pour les articles.
2. Activer la langue dans `pos.json::supported_languages`.
3. L'opérateur peut basculer entre langues via le bouton de langue dans l'écran checkout.

Aucune modification du binaire `moyun-pos` n'est nécessaire pour ajouter une langue : c'est de la **configuration de données**, pas du code. Donc pas de nouveau cycle de certification.

### 3.3 Tickets multilingues pour la machine de test SPF

Pour la machine de test livrée à l'Intake, le seed `pos.db` sera configuré avec les 4 langues officielles (FR, NL, DE, EN) complétées pour les produits du manuel (Dry Martini, Burger of the Chef, etc.). L'agent SPF pourra basculer entre langues via le bouton de configuration ou en changeant la `language_preference` de l'utilisateur de test.

---

## 4. Récapitulatif des engagements

| Engagement | Mécanisme | Vérifiabilité |
|---|---|---|
| Build cert reproductible | `Cargo.lock` + `rust-toolchain.toml` épinglés | `cargo build --release` identique sur 2 machines |
| Build = déploiement client | Binaire Tauri scellé + SHA-1 vérifiable | `sha1sum /opt/moyun-pos/moyun-pos` |
| Pas de modification post-cert | Système de fichiers racine en lecture seule + tag git signé | Vérification au boot, lecture du tag dans `/etc/moyun-pos/version` |
| Nouvelle MAJ = nouveau cert | Procédure §II.E + Article 109 du manuel | Chaque tag `vX.Y.Z+1` ouvre une nouvelle demande SPF |
| Pas d'accès distant permanent | Pas de SSH, pas de TeamViewer installé | Vérification au scan réseau du POS (aucun port ouvert) |
| Ticket dans la langue du client | `translations` + `users.language_preference` | Inspection visuelle du ticket, paramétrage modifiable sans changer le binaire |
