# Manuel d'installation et de configuration - moyun-pos v1.0.0

Ce manuel s'adresse au technicien installateur (= distributeur,
nuagemagique.dev). Il décrit la mise en place complète d'un kiosque
moyun-pos chez un client HoReCa.

## 1. Matériel requis

| Élément | Modèle / spécifications | Qté |
|---|---|---|
| Terminal POS | Penetek A3216 M82 ou Advantech USC-360J (Intel J6412, 8 Go RAM, 128 Go SSD, écran 15,6" 1920×1080 PCAP, IP65) | 1 |
| Boîtier FDM | Checkbox CBX01 (TAIGER BV, via Eutronix réf. TAFIP0002.00) ou équivalent agréé SPF Finances | 1 |
| Imprimante thermique | Epson TM-m30III (USB ou LAN) | 1 |
| Tiroir-caisse | Standard, connexion RJ11 vers l'imprimante | 1 |
| Câble Ethernet | RJ45 Cat-5e ou supérieur | 2 (POS + FDM) |
| Switch / box internet | 4 ports minimum | 1 |
| Clé USB d'installation | 8 Go minimum, formatée FAT32 | 1 |
| Onduleur (recommandé) | 600 VA minimum | 1 |

## 2. Préparation de la clé USB d'installation

### 2.1 Télécharger l'image kiosque

L'image est fournie par le distributeur sous forme d'un fichier
`moyun-pos-kiosk-vX.Y.Z.img.xz`. Sur la machine du distributeur :

```bash
xz -d moyun-pos-kiosk-vX.Y.Z.img.xz
```

### 2.2 Écrire l'image sur la clé USB

Attention : **L'opération efface toute la clé USB.**

Sur Linux/macOS :

```bash
# Identifier la clé (souvent /dev/sdX ou /dev/diskN)
lsblk        # Linux
diskutil list # macOS

# Écrire l'image (remplacer /dev/sdX par le bon device)
sudo dd if=moyun-pos-kiosk-vX.Y.Z.img of=/dev/sdX bs=4M status=progress conv=fsync
```

Sur Windows : utiliser **Rufus** ou **Balena Etcher** avec l'image.

## 3. Installation sur le terminal POS

### 3.1 Configuration BIOS

À la première mise sous tension du terminal :

1. Entrer dans le BIOS (touche `Del` ou `F2` selon le modèle).
2. Activer **Boot from USB**.
3. Désactiver **Secure Boot** si activé.
4. **Définir un mot de passe BIOS** (à conserver précieusement -
   le client n'en a pas besoin).
5. Sauvegarder et redémarrer.

### 3.2 Boot sur la clé USB

1. Brancher la clé USB préparée.
2. Démarrer le terminal - il doit booter sur la clé.
3. Choisir "Install moyun-pos kiosk" dans le menu de la clé.

### 3.3 Procédure d'installation

L'installeur est entièrement automatique :
1. Détection du disque interne.
2. Effacement complet du disque interne.
3. Copie de l'image (Debian 12 minimal + moyun-pos + Cage + outils).
4. Génération automatique d'un mot de passe `root` aléatoire,
   affiché à l'écran à la fin.
5. **Inscrire ce mot de passe** dans le carnet du distributeur
   (jamais communiqué au client).

Durée approximative : 8 à 12 minutes.

### 3.4 Premier démarrage

Retirer la clé USB et redémarrer. Le terminal doit :
1. Démarrer sur le disque interne.
2. Lancer automatiquement Cage Wayland en plein écran.
3. Démarrer moyun-pos en mode kiosque.
4. Afficher l'écran de connexion (avec un message rouge "POS NON
   CONFIGURÉ" tant que `pos.json` n'est pas paramétré).

## 4. Configuration initiale

### 4.1 Accès maintenance

Le terminal n'a **pas de TTY accessible** par défaut. Pour la
maintenance initiale, le distributeur :

1. Branche un clavier USB sur le terminal.
2. Presse `Ctrl + Alt + F2` (séquence cachée non accessible à
   l'opérateur).
3. Se connecte en `root` avec le mot de passe noté à l'installation.

### 4.2 Fichier de configuration `/etc/moyun-pos/pos.json`

Éditer avec `vi /etc/moyun-pos/pos.json` :

```json
{
  "pos_id": "CFOD0061234567",
  "terminal_id": "POS1",
  "device_id": "AA:BB:CC:DD:EE:01",
  "restaurant_name": "Restaurant Exemple",
  "restaurant_address": "Rue de l'Exemple 1, 4000 Liège",
  "restaurant_vat": "BE0499999960",
  "restaurant_est_no": "1234567890",
  "fdm_url": "http://192.168.1.20:8080/graphql",
  "printer_address": "192.168.1.30:9100",
  "training_mode": false,
  "language": "fr",
  "backup_dir": "/var/lib/moyun-pos/backups"
}
```

Champs :

| Champ | Description | Source |
|---|---|---|
| `pos_id` | Identifiant POS 14 caractères (commence par `C`). Attribué par le SPF Finances à la certification. | SPF Finances |
| `terminal_id` | ID du terminal physique. Libre, en général `POS1`, `POS2`... | Distributeur |
| `device_id` | Adresse MAC du terminal, format `AA:BB:CC:DD:EE:FF` (imprimée sur le ticket, manuel §VI ligne VII - cf. `crates/pos-fiscal/src/ticket.rs`). | Distributeur |
| `restaurant_name` | Nom commercial du restaurant tel qu'apparaît sur le ticket | Client |
| `restaurant_address` | Adresse complète | Client |
| `restaurant_vat` | Numéro de TVA belge format `BE0XXXXXXXXX` | Client |
| `restaurant_est_no` | Numéro d'établissement (10 chiffres) | BCE / client |
| `fdm_url` | URL GraphQL du FDM sur le LAN (port selon le modèle de FDM ; à confirmer sur le Checkbox CBX01) | Configuration FDM |
| `printer_address` | IP:port de l'imprimante (ou chemin série/USB) | Configuration imprimante |
| `training_mode` | `false` en production. À NE JAMAIS laisser `true` après installation. | Distributeur |
| `language` | Langue par défaut de l'interface : `fr` / `nl` / `de` / `en` / `zh`. Les mentions fiscales imprimées suivent cette clé, restreinte aux langues nationales (fr/nl/de) — toute autre valeur émet le ticket en FR. | Client |
| `backup_dir` | Répertoire des sauvegardes (défaut : `/var/lib/moyun-pos/backups`) | Distributeur |

#### Langue du ticket

La clé `language` détermine la langue des mentions fiscales imprimées.
Elle est fixée à l'installation d'après la **région linguistique de
l'établissement** où le POS est exploité (manuel de certification §VI,
p. 99) — ni d'après le siège du distributeur, ni d'après la langue de
l'interface souhaitée par le personnel :

| Région linguistique de l'établissement | `language` |
|---|---|
| Région de langue française | `fr` |
| Région de langue néerlandaise | `nl` |
| Région de langue allemande | `de` |
| Région bilingue de Bruxelles-Capitale | `fr` ou `nl` |

Exemple : un établissement à 4000 Liège (région de langue française)
→ `"language": "fr"`. Voir
`GARANTIE_MAJ_LANGUES_FR.md` §3.2.

### 4.3 Configuration réseau LAN

Le POS et le FDM doivent être sur le **même LAN privé**. Schéma type :

```
        Box internet
              │
              ▼
        ┌─────────┐
        │ Switch  │
        │  4 prt  │
        └─┬───┬─┬─┘
          │   │ │
   ┌──────┘   │ └─────────┐
   │          │            │
   ▼          ▼            ▼
┌────┐    ┌─────┐     ┌────────┐
│POS │    │ FDM │     │Printer │
│ .10│    │ .20 │     │ .30    │
└────┘    └─────┘     └────────┘
```

Adresses IP statiques recommandées (subnet `192.168.1.0/24`) :
- POS : `192.168.1.10`
- FDM : `192.168.1.20:8080` (port §4.2 ci-dessus)
- Imprimante : `192.168.1.30:9100`

(Réservations DHCP sur le routeur, cf. `SCHEMA_ASSEMBLAGE_FR.md` §4.)
- Box : `192.168.1.1`

### 4.4 Configuration utilisateurs

Créer les utilisateurs initiaux dans la base SQLite :

```bash
sqlite3 /var/lib/moyun-pos/pos.db
```

```sql
-- Gérant
INSERT INTO users (insz, name, role, active)
VALUES ('XXXXXXXXXXX', 'Bruno Moyse', 'admin', 1);

-- Serveur
INSERT INTO users (insz, name, role, active)
VALUES ('YYYYYYYYYYY', 'Marie Dupont', 'cashier', 1);

-- Apprenti
INSERT INTO users (insz, name, role, active)
VALUES ('ZZZZZZZZZZZ', 'Pierre Tran', 'waiter', 1);
```

(Voir `MANUEL_PROGRAMMATION_FR.md` pour la création/modification
de menus, modificateurs etc.)

### 4.5 Synchronisation horaire

Vérifier que `chrony` est actif :

```bash
systemctl status chrony
chronyc tracking
```

L'horloge **doit être à la seconde près**. Une dérive crée des
ennuis à la signature FDM (qui exige un `posDateTime` cohérent).

### 4.6 Test du flux complet

Avant de quitter, effectuer un test de bout en bout :

1. Redémarrer le terminal (`reboot`).
2. Se connecter avec un INSZ valide.
3. Vérifier que le pointage S signe correctement.
4. Ouvrir une table, ajouter un item.
5. Encaisser en espèces - le ticket doit s'imprimer avec QR code et
   le tiroir s'ouvrir.
6. Inspecter `fiscal_events` :

```sql
SELECT id, event_type, sequence_number, short_signature
FROM fiscal_events ORDER BY id DESC LIMIT 5;
```

Trois lignes minimum : un S (work-in), un N (la vente), idéalement
un P si la commande a été envoyée en cuisine.

7. Générer un rapport X pour validation finale.

## 5. Vérification de l'intégrité du binaire

Le hash SHA-1 du binaire `moyun-pos` certifié est consigné dans le
formulaire SPF Finances. Vérifier :

```bash
sha1sum /opt/moyun-pos/moyun-pos
```

La valeur doit correspondre exactement à celle déclarée dans le
formulaire. Si différente : binaire non-certifié, **NE PAS METTRE EN
PRODUCTION**.

## 6. Mise à jour (procédure post-cert)

Une mise à jour suit la procédure officielle SPF Finances
(`Manuel_Intro_ToRead_FR.md` §C) :

1. Le distributeur déclare la modification au SPF Finances :
   - Changelog signé
   - Affirmation qu'aucune autre modification n'a été apportée
   - Nouveau hash SHA-1
2. Délai de traitement SPF : 15 jours.
3. Après accord, le distributeur peut déployer sur les sites
   clients.

Procédure technique de déploiement :

1. Remettre le binaire `vX.Y.Z+1` sur le terminal : clé USB sur site,
   ou transfert de fichiers pendant une session RustDesk ouverte à la
   demande par un responsable sur place (`deploy/REMOTE-OPS.md`).
2. Accès maintenance (`Ctrl+Alt+F2`, login `root`), puis :
   ```bash
   /opt/moyun-pos/deploy/update.sh /root/moyun-pos --sha1 <SHA-1 déclaré>
   ```
   Le script refuse tout binaire dont le SHA-1 diffère de la valeur
   déclarée. Les étapes 3 à 7 ci-dessous décrivent la variante
   « réinstallation complète de l'image » depuis une clé USB.
3. Sauvegarder la base courante :
   ```bash
   sqlite3 /var/lib/moyun-pos/pos.db ".backup /tmp/pos-before-update.db"
   ```
4. Booter sur la clé USB en mode "Update" (sélection menu).
5. L'updater **conserve** le contenu de `/var/lib/moyun-pos/` (base +
   sauvegardes + logs) et remplace uniquement le binaire et les
   ressources.
6. Redémarrer.
7. Vérifier le nouveau hash SHA-1 et que `fiscal_events` est
   intacte.

## 7. Annexes

### 7.1 Liste des numéros INSZ de test

Les INSZ de test SPF (depuis `UC_TestSetup_20240806.md`) :

| Rôle | INSZ |
|---|---|
| Gérant | 00000000196 |
| Serveur | 00000000295 |
| Apprenti | 00000000394 |
| Robot (automate) | 00000000029 |

À utiliser exclusivement lors de la phase de test pré-soumission.

### 7.2 Numéros pos_id de test

`CFOD0061234567` - réservé tests SPF Finances.

### 7.3 Fournisseurs FDM agréés

- **Checkbox CBX01** : TAIGER BV, distribution via Eutronix
  (réf. TAFIP0002.00) - ~€195 HTVA, achat unique, LAN+WiFi+USB,
  alimentation incluse. **Référence retenue.**
- **BMC Light** : boxapos.com / zwartedoos.be - €12,95/mois
  (location), LAN+WiFi. Alternative.
- **Émulateur Checkbox** : inclus dans le kit de développement
  TAIGER (€380 HTVA, avec FDM physique de développement) - pour la
  phase de tests pré-certification.

### 7.4 Contact distributeur

Bruno Moyse / nuagemagique.dev
- Email : cloud@nuagemagique.dev
- Bruxelles, Belgique
