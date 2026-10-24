# Essai du terminal CCV sur place (pos-eft-probe)

Un essai de 10 minutes au restaurant, avant le gel de la version à certifier.
Il confirme que la caisse parle bien au terminal CCV (A920) par le réseau
local (protocole OPI), et il enregistre une trace des échanges (numéros de
carte masqués) pour ajuster le pilote si besoin.

Rien n'est débité sans que vous tapiez `YES`.

## À préparer

- Le Mac sur le Wi-Fi du restaurant, le même réseau que le terminal.
- L'adresse IP du terminal. Elle se trouve dans les paramètres réseau du
  terminal, ou dans la liste des appareils du routeur. Une adresse fixe est
  préférable : sinon elle peut changer au prochain redémarrage.
- Une carte Bancontact (et si possible une Visa ou une Mastercard).
- La liaison caisse du terminal : chez certains revendeurs, CCV doit
  l'activer (« ECR » ou « OPI »). Si l'étape 1 échoue alors que l'adresse est
  bonne, c'est la première question à poser au support CCV.
- Une fois, avant de partir : `cargo build --release -p pos-eft-probe` dans le
  dépôt, pour ne rien compiler sur place. Le binaire est
  `target/release/pos-eft-probe`.

Dans les commandes ci-dessous, remplacez `192.168.1.50` par l'adresse du
terminal.

## 1. Le terminal répond-il ?

```bash
target/release/pos-eft-probe --kind ccv_opi --address 192.168.1.50:4100 check
```

Rien n'est envoyé au terminal à part l'ouverture d'une connexion. Attendu : une ligne
`Reachable: …`. Sinon : vérifier l'adresse, le Wi-Fi, puis la liaison caisse
chez CCV.

## 2. Un paiement de 0,01 € en Bancontact

```bash
target/release/pos-eft-probe --kind ccv_opi --address 192.168.1.50:4100 \
  --device-port 4102 pay --amount 0.01
```

- Le programme affiche la référence, le terminal et le montant, puis demande
  `YES`. Tapez `YES`.
- macOS peut demander d'autoriser les connexions entrantes (le terminal
  rappelle le Mac sur le port 4102 pour le ticket) : autorisez.
- Présentez la carte Bancontact sur le terminal.
- Attendu : une ligne `Outcome:` qui contient `approved` et la marque `Bancontact`.

Si la réponse se perd (coupure, délai), notez la référence affichée et
lancez :

```bash
target/release/pos-eft-probe --kind ccv_opi --address 192.168.1.50:4100 status <référence>
```

## 3. Si possible : une carte de crédit, puis une annulation

- Refaire l'étape 2 avec une Visa ou une Mastercard. Attendu : `approved`,
  avec la bonne marque.
- Refaire l'étape 2 et annuler sur le terminal au lieu de présenter la carte.
  Attendu : `cancelled` ou `declined`, et rien de débité.

## Après l'essai

- Les fichiers `eft-trace-*.log` sont dans le dossier courant. Les numéros de
  carte y sont masqués (seuls les 4 derniers chiffres restent), mais
  relisez-les avant de les partager.
- Envoyez-les à Claude, avec ce qui s'affichait sur le terminal à chaque
  étape. Ils servent à confirmer le libellé Bancontact renvoyé par le
  terminal et le format exact des échanges.
- Les paiements de 0,01 € sont de vrais paiements : ils apparaissent sur le
  relevé CCV. Il n'y a pas de remboursement depuis cet outil.
