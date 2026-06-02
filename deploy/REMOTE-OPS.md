# Remote Operations — moyun-pos

Guide pour le dépannage, la lecture des logs et la mise à jour du
POS à distance sans être physiquement au restaurant.

## Prérequis

- Le terminal POS est connecté au réseau du restaurant
- Cloudflare Tunnel (ou VPN) expose le SSH du terminal
- Tu as la clé SSH privée pour le user `tsb` sur le terminal

## Connexion SSH

```bash
ssh tsb@<terminal-hostname>
# ou via Cloudflare Tunnel :
cloudflared access ssh --hostname pos.tokyosushibarliege.be
```

## Healthcheck rapide

Le POS expose un endpoint HTTP local sur le port 9999 :

```bash
curl -s http://127.0.0.1:9999/health | jq
```

Retourne :
```json
{
  "status": "ok",
  "uptime_secs": 43200,
  "outbox_pending": 0
}
```

**Depuis le dashboard / un cron externe** :

```bash
curl -sf https://pos.tokyosushibarliege.be:9999/health
# (si exposé via le tunnel — recommandé pour le monitoring)
```

**À surveiller** :
- `outbox_pending > 10` → HubRise est probablement down, les
  orders s'empilent. Pas d'action immédiate — le worker retry.
  Si > 100, investiguer la connectivité.
- `status != "ok"` → le process a un souci. Restart via systemd.

## Lecture des logs

Les logs structurés (`tracing`) sont écrits dans :

```
/home/tsb/.local/share/moyun-pos/logs/pos.log
```

Rotation quotidienne. Rétention : dépend du cron de cleanup (à configurer).

```bash
# Dernières 50 lignes
tail -50 /home/tsb/.local/share/moyun-pos/logs/pos.log

# Chercher les erreurs
grep -i "error\|warn\|failed" /home/tsb/.local/share/moyun-pos/logs/pos.log | tail -20

# Suivre en temps réel
tail -f /home/tsb/.local/share/moyun-pos/logs/pos.log

# Chercher un order HubRise spécifique
grep "abc12345" /home/tsb/.local/share/moyun-pos/logs/pos.log
```

**Patterns courants dans les logs** :
| Pattern | Signification |
|---|---|
| `hubrise poll error` | Le poller n'arrive pas à contacter HubRise (réseau?) |
| `hubrise poll: unauthorized` | Le token a expiré → reconnecter via Settings |
| `outbox: push failed` | Un order n'a pas pu être envoyé à HubRise (retry auto) |
| `outbox: expired old rows` | Le TTL de 7j a expiré des orders non-pushées |
| `receipt print failed` | L'imprimante ne répond pas (papier? câble?) |
| `boot: recovered orphaned checkout` | Un crash mid-checkout a été récupéré au démarrage |
| `post-Z backup failed` | Le backup auto après Z-report a échoué (espace disque?) |

## État de la base de données

```bash
sqlite3 /home/tsb/.local/share/moyun-pos/pos.db

-- Commandes ouvertes
SELECT id, table_number, order_type, total_cents, created_at
FROM orders WHERE status = 'open';

-- Outbox HubRise en attente
SELECT id, order_id, attempts, error_msg, created_at
FROM hubrise_outbox WHERE sent_at IS NULL;

-- Dernier Z-report
SELECT sequence_number, report_date, created_at
FROM z_reports ORDER BY sequence_number DESC LIMIT 1;

-- Connexion HubRise active
SELECT account_name, location_name, last_poll_at
FROM hubrise_connection;

-- Backups effectués
ls -la /home/tsb/.local/share/moyun-pos/backups/
```

## Restart du service

```bash
sudo systemctl restart moyun-pos
sudo systemctl status moyun-pos

# Voir les logs systemd (journal)
sudo journalctl -u moyun-pos --since "1 hour ago" --no-pager
```

## Mise à jour du binaire

### Méthode manuelle (SSH)

```bash
# 1. Arrêter le service
sudo systemctl stop moyun-pos

# 2. Backup du binaire actuel
cp /opt/moyun-pos/moyun-pos /opt/moyun-pos/moyun-pos.bak

# 3. Copier le nouveau binaire
scp moyun-pos tsb@<hostname>:/opt/moyun-pos/moyun-pos
# ou via un download :
curl -L https://releases.tsb.nuagemagique.dev/pos/latest/moyun-pos -o /opt/moyun-pos/moyun-pos
chmod +x /opt/moyun-pos/moyun-pos

# 4. Redémarrer
sudo systemctl start moyun-pos

# 5. Vérifier le health
sleep 3 && curl -s http://127.0.0.1:9999/health | jq
```

### Rollback

```bash
sudo systemctl stop moyun-pos
cp /opt/moyun-pos/moyun-pos.bak /opt/moyun-pos/moyun-pos
sudo systemctl start moyun-pos
```

### Méthode depuis le dashboard (futur)

Quand `deploy/update.sh` est configuré en cron :

```bash
# Depuis n'importe où avec SSH :
ssh tsb@<hostname> '/opt/moyun-pos/deploy/update.sh'
```

Ou via un bouton dans tsb-dashboard qui exécute la commande par SSH
(même pattern que les workflows CI existants pour tsb-service).

## Backup manuel

```bash
# Backup immédiat (sans attendre un Z-report)
sqlite3 /home/tsb/.local/share/moyun-pos/pos.db ".backup /home/tsb/.local/share/moyun-pos/backups/manual-$(date +%Y%m%d-%H%M%S).db"

# Vérifier les backups
ls -lh /home/tsb/.local/share/moyun-pos/backups/
```

## Dépannage courant

### "Le POS est gelé / ne répond pas"

```bash
# 1. Check si le process tourne
pgrep -a moyun-pos

# 2. Si oui mais gelé → kill + restart
sudo systemctl restart moyun-pos

# 3. Si non → check les logs du crash
sudo journalctl -u moyun-pos --since "10 minutes ago" --no-pager | tail -30
```

### "Les commandes en ligne n'arrivent plus"

```bash
# 1. Check la connexion HubRise
sqlite3 /home/tsb/.local/share/moyun-pos/pos.db "SELECT last_poll_at FROM hubrise_connection;"
# Si last_poll_at > 5 min → le poller est bloqué

# 2. Check les erreurs de poll
grep "hubrise poll" /home/tsb/.local/share/moyun-pos/logs/pos.log | tail -5

# 3. Si "unauthorized" → reconnecter depuis Config > HubRise
# 4. Si "error" → vérifier la connectivité internet du terminal
ping -c 3 api.hubrise.com
```

### "L'imprimante n'imprime plus"

```bash
# 1. Check la config imprimante
sqlite3 /home/tsb/.local/share/moyun-pos/pos.db "SELECT printer_address FROM pos_config;" 2>/dev/null

# 2. Ping l'imprimante
nc -z -w 2 <printer-ip> 9100 && echo "OK" || echo "UNREACHABLE"

# 3. Test d'impression basique (envoie un ESC/POS init + text + cut)
echo -ne '\x1b@Hello POS\n\x1dV\x42\x03' | nc <printer-ip> 9100
```

### "Le FDM ne répond pas"

```bash
# Le timeout est de 10s. Si ça échoue :
curl -sf http://<fdm-ip>:8080/graphql -X POST \
  -H "Content-Type: application/json" \
  -d '{"query":"{ Status(language: FR) { device { fdmId } } }"}' \
  | jq
# Si timeout ou erreur → vérifier le câble réseau vers le FDM
```

## Monitoring (recommandé)

Configurer un cron sur le serveur OVH (ou un Uptime Robot) qui
ping le healthcheck toutes les 5 min et alerte par email/Slack
si le POS ne répond pas :

```bash
# /etc/cron.d/moyun-pos-monitor (sur le serveur OVH)
*/5 * * * * root curl -sf --max-time 10 https://pos.tokyosushibarliege.be:9999/health > /dev/null || echo "POS DOWN" | mail -s "ALERT: POS offline" bruno@nuagemagique.dev
```
