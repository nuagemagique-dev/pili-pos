# Opérations à distance — moyun-pos

Guide de dépannage et de mise à jour d'un terminal moyun-pos sans être
sur place. Le terminal est une appliance scellée : **pas de serveur
SSH, aucun port en écoute, aucun accès distant permanent**
(`GARANTIE_MAJ_LANGUES_FR.md` §2.2). Le seul canal distant est une
session **RustDesk** ouverte à la demande, le temps de l'intervention.

## Session RustDesk à la demande

- Client : RustDesk portable (AppImage), version épinglée dans
  `deploy/provision/provision.sh` (`RUSTDESK_VERSION` + SHA-256 —
  `deploy/image/build.sh` garde une copie synchronisée à la main pour
  télécharger/vérifier l'AppImage une seule fois puis la réutiliser
  pour les deux profils d'image). Vérifiée (SHA-256) puis extraite
  avec `--appimage-extract` (jamais montée en FUSE — fonctionne sous
  `NoNewPrivileges=yes` et dans le chroot de build) dans
  `/opt/moyun-pos/rustdesk/`, lancée via
  `/opt/moyun-pos/rustdesk/AppRun`. Aucun service, jamais lancé au
  boot.
- Ouverture : un responsable présent sur place démarre la session à la
  demande du distributeur ; l'identifiant et le mot de passe à usage
  unique affichés sont communiqués par téléphone. La session se
  termine à la fermeture de RustDesk ou au redémarrage du terminal.
- Machine de test SPF : sur demande du SPF Finances, les identifiants
  d'une session de test sont fournis avec la machine (manuel §II.E).

> **À valider dans l'image kiosk (QEMU puis matériel) avant la
> soumission :** capture d'écran sous Cage via
> `xdg-desktop-portal-wlr` + PipeWire, saisie clavier distante
> (problèmes connus de RustDesk sur les compositeurs wlroots), et le
> point de déclenchement de la session depuis l'interface du POS
> (réservé au rôle administrateur). Ce document sera complété avec la
> procédure exacte une fois ces points vérifiés.

Pendant la session, le distributeur travaille sur la console de
maintenance (`Ctrl+Alt+F2`, login `root`).

## Emplacements

| Quoi | Où |
|---|---|
| Base de données | `/var/lib/moyun-pos/pos.db` |
| Logs applicatifs (rotation quotidienne) | `/var/lib/moyun-pos/logs/pos.log.AAAA-MM-JJ` |
| Sauvegardes (auto après Z, manuelles, pré-MAJ) | `/var/lib/moyun-pos/backups/` |
| Configuration | `/etc/moyun-pos/pos.json` |
| Binaire | `/opt/moyun-pos/moyun-pos` |
| Service | `moyun-pos.service` (Cage sur tty1) |

## Lecture des logs

```bash
# Journal systemd du kiosque
journalctl -u moyun-pos --since "1 hour ago" --no-pager

# Logs applicatifs
tail -50 /var/lib/moyun-pos/logs/pos.log.$(date +%F)
grep -i "error\|warn\|failed" /var/lib/moyun-pos/logs/pos.log.* | tail -20
```

**Messages courants** :
| Message | Signification |
|---|---|
| `receipt print failed` | L'imprimante ne répond pas (papier ? câble ?) |
| `boot: recovered orphaned checkout` | Un crash en plein encaissement a été récupéré au démarrage |
| `post-Z backup failed` | La sauvegarde automatique après le rapport Z a échoué (espace disque ?) |
| `no config file found` | `/etc/moyun-pos/pos.json` absent : le POS tourne avec une identité de TEST |

## État de la base de données (lecture seule)

Sur l'image d'admission, le compte `spf` (tty2, mot de passe affiché
une seule fois au premier démarrage — `MANUEL_INSTALLATION_FR.md`
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
sqlite3 -readonly /var/lib/moyun-pos/pos.db

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
systemctl restart moyun-pos
systemctl status moyun-pos
```

## Mise à jour du binaire

Toute modification du POS fait l'objet d'une demande de certification
de mise à jour auprès du SPF Finances (manuel §II.E) ; seul un binaire
dont le SHA-1 correspond à la valeur déclarée peut être installé.

1. Transférer le nouveau binaire sur le terminal (transfert de fichiers
   RustDesk, ou clé USB sur place) dans `/root/`.
2. Sur la console de maintenance :
   ```bash
   /opt/moyun-pos/deploy/update.sh /root/moyun-pos --sha1 <SHA-1 déclaré>
   ```
   Le script vérifie le SHA-1, sauvegarde le binaire et la base,
   applique les migrations puis redémarre le service.
3. Vérifier : `sha1sum /opt/moyun-pos/moyun-pos` et un ticket de test.

### Rollback

```bash
systemctl stop moyun-pos
cp /opt/moyun-pos/moyun-pos.bak /opt/moyun-pos/moyun-pos
systemctl start moyun-pos
```

Un rollback ne réinstalle que le binaire précédent (version déjà
certifiée). **Ne jamais restaurer une sauvegarde de base par-dessus
`pos.db`** : les événements fiscaux enregistrés depuis la sauvegarde
seraient perdus, ce qui est interdit.

## Sauvegarde manuelle

```bash
sqlite3 /var/lib/moyun-pos/pos.db \
  ".backup /var/lib/moyun-pos/backups/manual-$(date +%Y%m%d-%H%M%S).db"
ls -lh /var/lib/moyun-pos/backups/
```

## Dépannage courant

### « Le POS est gelé / ne répond pas »

```bash
pgrep -a moyun-pos
systemctl restart moyun-pos
journalctl -u moyun-pos --since "10 minutes ago" --no-pager | tail -30
```

### « L'imprimante n'imprime plus »

```bash
grep printer_address /etc/moyun-pos/pos.json
nc -z -w 2 <printer-ip> 9100 && echo "OK" || echo "UNREACHABLE"
```

### « Le FDM ne répond pas »

Le POS bloque toute transaction tant que le FDM est injoignable
(comportement obligatoire). Vérifier le lien :

```bash
FDM_URL=$(sed -n 's/.*"fdm_url": *"\([^"]*\)".*/\1/p' /etc/moyun-pos/pos.json)
curl -sf "$FDM_URL" -X POST \
  -H "Content-Type: application/json" \
  -d '{"query":"{ Status(language: FR) { device { fdmId } } }"}'
# Timeout ou erreur → vérifier le câble réseau / l'alimentation du FDM
```
