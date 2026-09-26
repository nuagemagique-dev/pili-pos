# Schéma d'assemblage physique - moyun-pos

**Date :** 2026-05-14
**Référence :** réponse au §I.C "Schéma d'assemblage" et §II.G du manuel de certification.

Le présent document décrit l'assemblage physique du SCE (Système de
Caisse Enregistreuse) `moyun-pos` dans sa configuration de référence (un établissement
HoReCa), identique à la configuration livrée à l'Intake SPF Finances.

---

## 1. Vue d'ensemble

```
                    INTERNET
                       |
                       | (LAN routeur restaurant ou
                       |  routeur fourni Intake)
                       |
              +--------+--------+
              |    ROUTEUR      |
              |  IP : 192.168.1.1
              |  DHCP : actif    |
              +---+----------+---+
                  |          |
                  |          |
                  | RJ45     | RJ45
                  |          |
       +----------+--+    +--+---------------+
       |             |    |                  |
       |   FDM       |    |   IMPRIMANTE     |
       |  Checkbox   |    |   Epson TM-m30III|
       |  CBX01      |    |                  |
       | IP : 192.168.1.20|  IP : 192.168.1.30|
       | TCP 8080 GraphQL |  TCP 9100 ESC/POS|
       +------------------+  +-------+--------+
                                     |
                                     | RJ11
                                     | (séquence ESC/POS
                                     |  0x1B 0x70 0x00 0x19 0xFA)
                                     |
                                +----+-----+
                                |          |
                                | TIROIR-  |
                                | CAISSE   |
                                | physique |
                                |          |
                                +----------+


              +--------+--------+
              |                 |
              |   ROUTEUR       |
              |  IP : 192.168.1.1
              |                 |
              +--------+--------+
                       |
                       | RJ45
                       |
              +--------+--------+
              |                 |
              |   POS terminal  |
              |  Penetek A3216  |
              |  M82 (Intel J6412)
              |                 |
              |  IP : 192.168.1.10
              |  HTTP sortant    |
              |   vers 192.168.1.20:8080
              |   (FDM GraphQL)  |
              |                 |
              |  TCP sortant vers
              |   192.168.1.30:9100
              |   (Imprimante)   |
              |                 |
              |  HTTPS sortant (WAN), le cas
              |   echeant : sessions RustDesk
              |   d'assistance a distance a la
              |   demande, et (phase ulterieure,
              |   non active en v1.0.0) service
              |   d'envoi du ticket numerique par
              |   email - voir §4 "Pare-feu"|
              |                 |
              | Écran tactile   |
              | PCAP 15.6"      |
              | 1920×1080       |
              | (interface      |
              |  opérateur)     |
              +-----------------+
```

---

## 2. Composants et identification

| # | Composant | Marque/Modèle | Adressage | Connectivité |
|---|---|---|---|---|
| 1 | Terminal POS | Penetek A3216 M82 (Intel Celeron J6412, 8 Go RAM, 128 Go SSD, écran tactile PCAP 15.6" 1920×1080, fanless, IP65) | IP statique LAN `192.168.1.10` | RJ45 vers routeur (1×), USB-A (4×) disponibles |
| 2 | Routeur LAN | TP-Link Archer C6 (ou équivalent) - fourni avec la machine-test à l'Intake | IP gateway `192.168.1.1`, DHCP `192.168.1.100-200`, baux réservés pour FDM et imprimante | WAN RJ45 (Internet), LAN RJ45 (4×) |
| 3 | FDM | Checkbox CBX01 (TAIGER BV, via Eutronix réf. TAFIP0002.00) - **FDM physique de développement Checkbox pour l'Intake ; émulateur en secours (acceptable selon manuel §1202)** | IP statique LAN `192.168.1.20` (réservation DHCP). TCP port 8080 ouvert (GraphQL - cf. MANUEL_INSTALLATION_FR.md pour le statut de confirmation de ce port). | RJ45 vers routeur |
| 4 | Imprimante thermique | Epson TM-m30III (80 mm thermique) | IP statique LAN `192.168.1.30` (réservation DHCP). TCP port 9100 ouvert (ESC/POS raw). RJ11 sortant vers tiroir. | RJ45 vers routeur. USB et Wifi disponibles mais non utilisés. RJ11 vers tiroir-caisse. |
| 5 | Tiroir-caisse | Tiroir RJ11 standard (modèle générique compatible Epson) | N/A (passif) | RJ11 depuis l'imprimante. Ouverture par séquence ESC/POS `0x1B 0x70 0x00 0x19 0xFA` envoyée par le POS. |

---

## 3. Ports utilisés

### Sur le terminal POS

| Port physique | Utilisation |
|---|---|
| RJ45 #1 | Connexion LAN vers routeur (1) |
| USB-A #1 | (inutilisé) |
| USB-A #2 | (inutilisé) |
| USB-A #3 | Disponible pour clavier de configuration à l'installation (déconnecté en service normal) |
| USB-A #4 | Disponible pour clé USB de mise à jour (déconnecté en service normal) |
| HDMI | (inutilisé - écran intégré au POS) |
| 3.5 mm jack audio | (inutilisé) |

### Sur le routeur

| Port physique | Utilisation |
|---|---|
| WAN RJ45 | Connexion Internet (FAI du restaurant) |
| LAN RJ45 #1 | Vers POS (1) |
| LAN RJ45 #2 | Vers FDM (3) |
| LAN RJ45 #3 | Vers Imprimante (4) |
| LAN RJ45 #4 | Libre (extension futur) |

### Sur l'imprimante

| Port physique | Utilisation |
|---|---|
| RJ45 | Connexion LAN vers routeur |
| USB-B | (inutilisé) |
| RJ11 (DK port) | Vers tiroir-caisse (5) |
| Alimentation | Bloc 24V dédié |

---

## 4. Configuration réseau

### Plan d'adressage IP

| Appareil | Hostname | IP | MAC (à renseigner à l'install) | Service |
|---|---|---|---|---|
| Routeur | `gw` | `192.168.1.1` | - | DHCP, gateway, DNS |
| POS | `pos-01` | `192.168.1.10` | (à renseigner) | moyun-pos (Tauri app, kiosk) |
| FDM | `fdm` | `192.168.1.20` | (à renseigner) | Checkbox GraphQL endpoint |
| Imprimante | `printer` | `192.168.1.30` | (à renseigner) | ESC/POS raw TCP 9100 |

### Réservations DHCP

Configurées sur le routeur : 3 baux statiques pour les MAC du POS, du FDM et de l'imprimante, mappés sur leur IP respective. Aucune autre machine n'a accès au LAN du SCE en production (le SCE est isolé du réseau invité du restaurant).

### Pare-feu

Sur le routeur : politique par défaut bloquante en LAN <-> LAN. Règles autorisées :
- POS (.10) → FDM (.20) port TCP 8080 (GraphQL)
- POS (.10) → Imprimante (.30) port TCP 9100 (ESC/POS)
- FDM (.20) → Internet (WAN) sur les domaines/IP SPF Finances (gérés par le FDM lui-même via mTLS)
- POS (.10) → Internet (WAN) en HTTPS sortant uniquement : sessions RustDesk d'assistance à distance démarrées depuis la caisse par un Gérant, avec son PIN (jamais permanent), et - uniquement si `pos.json::digital_ticket` est configuré (§III.H) - l'envoi du ticket numérique par e-mail vers `api.scaleway.com` (port 443, cf. `TICKET_NUMERIQUE_FR.md` §9). Sans cette configuration, aucun trafic sortant n'est initié par le POS en dehors du FDM et de RustDesk.

Aucun port n'est ouvert depuis Internet vers le LAN (zone DMZ vide). Aucun port d'administration distante permanent (SSH, RDP, VNC) n'est ouvert sur le POS - pas de serveur SSH installé (cf. `deploy/install.sh`).

---

## 5. Procédure d'installation physique

À effectuer dans cet ordre :

1. Déballer et installer le routeur. Le brancher au modem Internet du restaurant. Vérifier l'accès Internet.
2. Configurer le routeur (login admin du routeur, IP statique gateway 192.168.1.1, DHCP 100-200, réservations DHCP pour les 3 appareils SCE).
3. Brancher le FDM au routeur via RJ45 (port LAN 2). Mettre sous tension. Vérifier que le FDM obtient son IP réservée 192.168.1.20.
4. Brancher l'imprimante au routeur via RJ45 (port LAN 3). Brancher l'alimentation. Vérifier IP 192.168.1.30. Brancher le RJ11 du tiroir-caisse au DK port de l'imprimante.
5. Brancher le POS au routeur via RJ45 (port LAN 1). Brancher l'alimentation. Mettre sous tension.
6. Au démarrage du POS, l'image kiosk lance automatiquement Cage compositor + moyun-pos en plein écran après quelques secondes.
7. Première connexion : INSZ technicien `00000000097` + PIN par défaut `0097`. L'écran "Configuration POS" s'affiche pour saisir/valider :
   - `fdm_url` : `http://192.168.1.20:8080/graphql`
   - `printer_address` : `192.168.1.30:9100`
   - `restaurant_est_no` : (n° d'établissement TVA assigné au restaurant)
   - `pos_id` : code 14 caractères assigné par SPF Finances au moment de l'enregistrement du POS
   - `terminal_id` : généré au premier boot et stocké
8. Le POS effectue un `signWorkIn` du technicien (event S) pour valider la chaîne complète. Si succès, le POS est opérationnel.

---

## 6. Procédure de réassemblage (pour audit futur)

En cas d'audit SPF Finances, le SCE peut être réassemblé à l'identique à partir du présent schéma et du manuel d'installation `MANUEL_INSTALLATION_FR.md`. Les éléments à conserver pour ré-installation :

- L'image kiosk de la build cert (fichier .iso).
- Le fichier `pos.json` de configuration (sauvegardé séparément).
- La base SQLite `pos.db` (snapshot quotidien dans `/var/lib/moyun-pos/backups/`).
- Le présent schéma + les n° de série des composants (à consigner au formulaire d'Intake).

---

## 7. Variante pour la machine de test à l'Intake

Pour la machine de test livrée au rendez-vous d'admission SPF Finances :

- Routeur fourni par le distributeur (mini-routeur de prêt).
- FDM physique de développement Checkbox utilisé pour l'Intake ; l'émulateur Checkbox reste utilisable en secours (acceptable selon manuel de certification §1202). Dans ce cas, l'URL FDM dans `pos.json` pointe vers l'endpoint de l'émulateur plutôt que vers une IP LAN.
- Tiroir-caisse RJ11 fourni avec l'imprimante.
- Compte SPF pré-créé sur la machine de test (login `spf`, mot de passe communiqué à l'Intake).

Le reste du schéma est identique à la production.
