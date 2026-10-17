# Note de mise à jour v1.2

Brouillon pour la déclaration de mise à jour après certification (préavis d'un mois, au
plus douze mises à jour par an). Une section par fonction. Le certificat ne couvre que les
fonctions documentées : chaque section dit si la fonction touche le binaire certifié.

| Fonction | Dans le binaire certifié | Données fiscales | État par défaut |
| --- | --- | --- | --- |
| A. Export comptable | Non : outil séparé | lit les données signées, n'écrit rien | sans objet |
| B. Alertes sur téléphone | Oui, dans le moteur | aucune : ne signe rien, ne lit ni ticket ni rapport | désactivée |

## A. Export comptable (hors du binaire certifié)

**Ce qui change.** Un outil en ligne de commande, `pili-accounting`, produit pour le
comptable un dossier de fichiers CSV et une écriture de journal équilibrée par jour, à
partir d'une copie de la base `pos.db` (dossier d'export de la caisse ou sauvegarde
déchiffrée).

**Est-ce dans le binaire certifié ?** Non. Le code vit dans une crate à part
(`crates/pos-accounting`) dont aucune crate livrée ne dépend et qui ne dépend d'aucune
crate de la caisse à l'exécution ; un test automatique le vérifie. Le binaire Linux et
l'application Android ne changent pas à cause de cette fonction : pas d'écran, pas de
commande moteur, pas de migration de base. L'outil s'exécute hors de la caisse.

**Impact fiscal.** Aucun : l'outil lit uniquement l'archive signée (événements N, I, F, R
avec réponse du FDM, rapports Z), en lecture seule et immuable, et n'écrit jamais dans la
base. Il n'envoie rien au FDM ni au SPF.

**Garde-fous.** Chaque période comptable est comparée à son rapport Z signé (nombre de
tickets, chiffre d'affaires, TVA par code, paiements par type) ; au moindre écart, aucun
fichier n'est écrit. Seules les périodes clôturées par un Z sont exportées. Aucun numéro
national ni nom d'employé dans les fichiers.

**Tests.** Rapprochement d'un jour réel construit par le moteur (scénario Reports 1.3),
fichiers de référence octet pour octet, équilibre du journal, détection d'altération,
preuve de lecture seule, test d'isolation. Détail dans `EXPORT_COMPTABLE_FR.md`.

**Cas d'usage officiels.** Aucun cas d'usage SPF n'est rejoué : la fonction ne produit
pas d'événement.

**Écrans.** Aucun écran de la caisse. Sortie texte de l'outil et fichier `LISEZMOI.txt`
à joindre au dossier comme exemple.

## B. Alertes du propriétaire sur téléphone (dans le binaire certifié, désactivées par défaut)

**Ce qui change.** La caisse peut envoyer un court message Telegram au propriétaire quand
le module fiscal ne répond plus, qu'une imprimante ou le terminal de carte est en défaut,
que le Z du jour n'est pas fait à l'heure limite, ou qu'une commande en ligne attend trop
longtemps. Un panneau « Alertes sur téléphone » dans Réglages (gérants) règle la fonction,
envoie un message test et montre les derniers messages. Même panneau et mêmes textes sur
Linux (Vue) et sur Android (Compose).

**Est-ce dans le binaire certifié ?** Oui : moteur (`crates/pos-engine/src/alerts/`), quatre
commandes réservées aux gérants, migration 60 (trois tables : réglages, journal des
réglages, journal des alertes). Aucune option de compilation : toutes les installations
reçoivent le code, aucune ne l'utilise tant que le propriétaire ne l'active pas.

**Impact fiscal.** Aucun. La fonction ne signe rien, n'écrit aucun événement fiscal, ne lit
ni les tickets, ni les rapports, ni les paiements. Elle ne bloque jamais une vente, un
ticket ou un Z : les messages partent depuis une boucle séparée qui n'attend ni la base
ni le verrou de signature pendant l'envoi.

**Données qui quittent la caisse.** Uniquement les messages : nom de l'installation,
situation, heure locale de début, nom de l'imprimante concernée, date comptable d'un Z
oublié, nombre de commandes en ligne et noms de canaux. Jamais de donnée personnelle ni de
montant. Liste complète et justification dans `ALERTES_PROPRIETAIRE_FR.md`.

**Garde-fous.** Désactivée par défaut ; le jeton du robot est dans un fichier de
configuration, jamais en base ni dans un journal ni à l'écran ; un seul message par
situation, un rappel par heure, une limite de 20 messages par heure ; journaux qui ne
s'effacent pas.

**Tests.** Règles pures, moteur contre un faux service Telegram (27 scénarios dont la
non-fuite du jeton et l'absence de donnée personnelle dans les trois langues), migration,
accès, configuration, façade Android, interface Vue (Playwright) et Compose.
Détail dans `ALERTES_PROPRIETAIRE_FR.md`.

**Cas d'usage officiels.** Aucun cas d'usage SPF n'est rejoué : la fonction ne produit pas
d'événement fiscal.

**Écrans.** Réglages, panneau « Alertes sur téléphone » (éteint sans jeton, allumé avec
jeton, résultat du message test) : captures à reprendre après la fusion.
