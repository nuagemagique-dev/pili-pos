# Opérations à distance - pili-pos

Guide de dépannage et de mise à jour d'un terminal pili-pos sans être
sur place. Le terminal est une appliance scellée : **pas de serveur
SSH, aucun port en écoute, aucun accès distant permanent**
(`GARANTIE_MAJ_LANGUES_FR.md` §2.2). Le seul canal distant est une
session **RustDesk** ouverte à la demande, le temps de l'intervention.

## Session RustDesk à la demande

- Client : RustDesk portable (AppImage), version épinglée dans
  `deploy/provision/provision.sh` (`RUSTDESK_VERSION` + SHA-256 -
  `deploy/image/build.sh` garde une copie synchronisée à la main pour
  télécharger/vérifier l'AppImage une seule fois puis la réutiliser
  pour les deux profils d'image). Vérifiée (SHA-256) puis extraite
  avec `--appimage-extract` (jamais montée en FUSE - fonctionne sous
  `NoNewPrivileges=yes` et dans le chroot de build) dans
  `/opt/pili-pos/rustdesk/`, lancée via
  `/opt/pili-pos/rustdesk/AppRun`. Aucun service, jamais lancé au
  boot.
- Ouverture : depuis la caisse, par un Gérant connecté. "Config",
  section "Assistance à distance", bouton "Démarrer", puis son PIN
  (`src-tauri/src/commands/support.rs::do_start_remote_support`, rôle
  `admin` et PIN obligatoires). RustDesk s'ouvre et affiche un
  identifiant et un mot de passe à usage unique, que le Gérant lit au
  technicien par téléphone.
- Pendant la session, **chaque écran** de la caisse affiche le bandeau
  "Assistance à distance en cours (démarrée par <nom>)" : le personnel
  sait que quelqu'un peut voir ou agir sur le terminal.
- Fin : "Arrêter la session" dans la même section, la fermeture de
  RustDesk, ou le redémarrage du terminal. Une seule session à la
  fois. Le démarrage et l'arrêt (qui, quand) sont écrits dans le
  journal applicatif (`pos.log`).
- RustDesk reçoit un `HOME` privé sous le répertoire d'exécution du
  service (`RuntimeDirectory=pili-pos`, effacé à l'arrêt) : rien de ce
  qu'il écrit ne survit à un redémarrage.
- Machine de test SPF : sur demande du SPF Finances, une session de
  test est ouverte de la même façon et ses identifiants fournis
  (manuel §II.E).

Ce que permet la session : voir l'écran de la caisse et guider le
Gérant (ou agir sur l'écran si la saisie distante fonctionne, voir
ci-dessous). Elle ne donne **pas** de shell : la console de maintenance
(`Ctrl+Alt+F2`, login `root`) n'est accessible que sur place, au
clavier. Une mise à jour du binaire (`deploy/update.sh --sha1`) se fait
donc toujours sur place, depuis une clé USB.

> **Point restant à vérifier sur le terminal (image d'admission sous
> QEMU puis matériel J6412) :** la capture d'écran sous Cage via
> `xdg-desktop-portal-wlr` + PipeWire et la saisie clavier/tactile
> distante (problèmes connus de RustDesk sur les compositeurs
> wlroots). Si la saisie distante ne fonctionne pas, la session reste
> en **lecture seule** : le technicien voit l'écran et guide le Gérant
> par téléphone. Le déclenchement, le bandeau et l'arrêt décrits
> ci-dessus sont implémentés et testés
> (`src-tauri/tests/remote_support.rs`, `ui/e2e/support.spec.ts`).

## Emplacements

| Quoi | Où |
|---|---|
| Base de données | `/var/lib/pili-pos/pos.db` |
| Logs applicatifs (rotation quotidienne) | `/var/lib/pili-pos/logs/pos.log.AAAA-MM-JJ` |
| Sauvegardes (auto après Z, manuelles, pré-MAJ) | `/var/lib/pili-pos/backups/` |
| Configuration | `/etc/pili-pos/pos.json` |
| Binaire | `/opt/pili-pos/pili-pos` |
| Service | `pili-pos.service` (Cage sur tty1) |

## Lecture des logs

```bash
# Journal systemd du kiosque
journalctl -u pili-pos --since "1 hour ago" --no-pager

# Logs applicatifs
tail -50 /var/lib/pili-pos/logs/pos.log.$(date +%F)
grep -i "error\|warn\|failed" /var/lib/pili-pos/logs/pos.log.* | tail -20
```

**Messages courants** :
| Message | Signification |
|---|---|
| `receipt print failed` | L'imprimante ne répond pas (papier ? câble ?) |
| `boot: recovered orphaned checkout` | Un crash en plein encaissement a été récupéré au démarrage |
| `post-Z backup failed` | La sauvegarde automatique après le rapport Z a échoué (espace disque ?) |
| `no config file found` | `/etc/pili-pos/pos.json` absent : le POS tourne avec une identité de TEST |

## État de la base de données (lecture seule)

Sur l'image d'admission, le compte `spf` (tty2, mot de passe affiché
une seule fois au premier démarrage - `MANUEL_INSTALLATION_FR.md`
§3.4) dispose du wrapper `pos-db-ro`, qui ouvre `pos.db` en lecture
seule sans jamais nécessiter d'accès écriture au fichier (voir
`SECURITE_BD_FR.md` §3.2 et `FEUILLE_ACQUISITION_DONNEES_FR.md` §2.2) :

```bash
pos-db-ro          # shell interactif
pos-db-ro "SELECT id, event_type, sequence_number FROM fiscal_events ORDER BY id DESC LIMIT 10;"
```

Depuis la console de maintenance root (`Ctrl+Alt+F2`), `sqlite3` brut
reste disponible :

```bash
sqlite3 -readonly /var/lib/pili-pos/pos.db

-- Commandes ouvertes
SELECT id, table_number, order_type, total_cents, created_at
FROM orders WHERE status = 'open';

-- Derniers événements fiscaux
SELECT id, event_type, sequence_number, event_label, created_at
FROM fiscal_events ORDER BY id DESC LIMIT 10;

-- Dernier rapport Z
SELECT sequence_number, report_date, created_at
FROM z_reports ORDER BY sequence_number DESC LIMIT 1;
```

Ne jamais écrire dans `fiscal_events` : l'archive fiscale est
append-only par la loi.

## Redémarrage du service

```bash
systemctl restart pili-pos
systemctl status pili-pos
```

## Mise à jour du binaire

Toute modification du POS fait l'objet d'une demande de certification
de mise à jour auprès du SPF Finances (manuel §II.E) ; seul un binaire
dont le SHA-1 correspond à la valeur déclarée peut être installé.

1. Transférer le nouveau binaire sur le terminal (transfert de fichiers
   RustDesk, ou clé USB sur place) dans `/root/`.
2. Sur la console de maintenance :
   ```bash
   /opt/pili-pos/deploy/update.sh /root/pili-pos --sha1 <SHA-1 déclaré>
   ```
   Le script vérifie le SHA-1, sauvegarde le binaire et la base,
   applique les migrations puis redémarre le service.
3. Vérifier : `sha1sum /opt/pili-pos/pili-pos` et un ticket de test.

### Rollback

```bash
systemctl stop pili-pos
cp /opt/pili-pos/pili-pos.bak /opt/pili-pos/pili-pos
systemctl start pili-pos
```

Un rollback ne réinstalle que le binaire précédent (version déjà
certifiée). **Ne jamais restaurer une sauvegarde de base par-dessus
`pos.db`** : les événements fiscaux enregistrés depuis la sauvegarde
seraient perdus, ce qui est interdit.

## Sauvegarde manuelle

```bash
sqlite3 /var/lib/pili-pos/pos.db \
  ".backup /var/lib/pili-pos/backups/manual-$(date +%Y%m%d-%H%M%S).db"
ls -lh /var/lib/pili-pos/backups/
```

## Dépannage courant

### « Le POS est gelé / ne répond pas »

```bash
pgrep -a pili-pos
systemctl restart pili-pos
journalctl -u pili-pos --since "10 minutes ago" --no-pager | tail -30
```

### « L'imprimante n'imprime plus »

```bash
grep printer_address /etc/pili-pos/pos.json
nc -z -w 2 <printer-ip> 9100 && echo "OK" || echo "UNREACHABLE"
```

### « Le FDM ne répond pas »

Le POS bloque toute transaction tant que le FDM est injoignable
(comportement obligatoire). Vérifier le lien :

```bash
FDM_URL=$(sed -n 's/.*"fdm_url": *"\([^"]*\)".*/\1/p' /etc/pili-pos/pos.json)
curl -sf "$FDM_URL" -X POST \
  -H "Content-Type: application/json" \
  -d '{"query":"{ Status(language: FR) { device { fdmId } } }"}'
# Timeout ou erreur → vérifier le câble réseau / l'alimentation du FDM
```
