# Note de mise à jour v1.2

Brouillon pour la déclaration de mise à jour après certification (préavis d'un mois, au
plus douze mises à jour par an). Une section par fonction. Le certificat ne couvre que les
fonctions documentées : chaque section dit si la fonction touche le binaire certifié.

| Fonction | Dans le binaire certifié | Données fiscales | État par défaut |
| --- | --- | --- | --- |
| A. Export comptable | Non : outil séparé | lit les données signées, n'écrit rien | sans objet |
| B. Alertes sur téléphone | Oui, dans le moteur | aucune : ne signe rien, ne lit ni ticket ni rapport | désactivée |
| E. Bons cadeaux à usage multiple | Oui, dans le moteur | vente T.V.A. X ; paiement `VOUCHER_STORE` ; cas officiels UC260 à UC262 rejoués | désactivée |

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

## C. Allergènes : les 14 allergènes à déclaration obligatoire (dans le binaire certifié)

**Ce qui change.** La caisse affiche, sur chaque produit et chaque option, les allergènes
contenus et les traces possibles (annexe II du règlement (UE) n° 1169/2011), sous forme de
pastilles numérotées, avec un mode « Allergènes » qui ouvre la fiche du produit, et imprime
une fiche d'allergènes pour un produit ou pour toute la carte. Les listes viennent du
catalogue HubRise (étiquettes `allergen:`, `traces:`, `allergen:none`). Même écran et mêmes
textes sur Linux (Vue) et sur Android (Compose).

**Est-ce dans le binaire certifié ?** Oui : modèle (`pos-core`), migration 61 (quatre
tables), import du catalogue, ticket `pos-print`, quatre commandes du moteur. Aucune option
de compilation.

**Impact fiscal.** Aucun. La fonction ne signe rien, n'écrit aucun événement fiscal et
aucun allergène n'entre dans une demande signée (test). Le ticket d'allergènes est un
document non fiscal, sans prix, qui porte la mention d'un document non valable pour la T.V.A.

**Données qui quittent la caisse.** Aucune.

**Garde-fous.** Une absence de déclaration s'affiche « information non disponible », jamais
« aucun allergène » ; un code d'allergène inconnu refuse tout le catalogue ; un catalogue
silencieux n'efface pas une liste saisie localement ; journal des changements inaltérable.

**Limite connue.** La saisie locale d'une liste par un gérant existe dans le moteur mais n'a
pas encore d'écran (éditeur de produit d'un autre chantier).

**Tests.** Voir `ALLERGENES_FR.md`, section 8.

**Cas d'usage officiels.** Aucun cas d'usage SPF n'est rejoué : la fonction ne produit pas
d'événement fiscal.

**Écrans.** Écran de commande avec pastilles, mode « Allergènes » et légende, fiche du
produit, sélecteur d'options, ticket imprimé : captures à reprendre après la fusion.

## D. Service en plusieurs temps : entrées, plats, desserts (dans le binaire certifié)

**Ce qui change.** Un gérant peut activer (Réglages, panneau « Service en plusieurs temps »)
l'envoi de la cuisine en plusieurs temps : chaque catégorie a un temps (Direct, Entrées,
Plats, Desserts). Le premier ticket de cuisine imprime le Direct et les entrées ; le bouton
« Envoyer la suite » imprime ensuite les plats, puis les desserts. Le panier est groupé par
temps, une pastille permet de changer le temps d'une ligne, le plan de salle indique « Suite :
Plats » et l'encaissement avertit si un temps n'a pas été envoyé. Même écran et mêmes textes
sur Linux (Vue) et sur Android (Compose).

**Est-ce dans le binaire certifié ?** Oui : migration 62 (trois tables), six commandes du moteur,
ticket `pos-print`. Aucune option de compilation. Éteint par défaut ; éteint, les tickets sont
identiques octet pour octet à ceux d'avant (test).

**Impact fiscal.** Aucun. Le temps est une donnée de cuisine : il n'entre dans aucune demande
signée (P, N, T, pré-addition), aucun nom de produit, aucun département ni le ticket de T.V.A.
(test sur tous les textes archivés). Les tickets de cuisine ne portent aucun montant et
gardent la mention d'un document non valable pour la T.V.A. Fondement : spécification
sections 8.1 et 8.2 (consolidation des imprimés, jamais des messages du module fiscal) et FAQ
Q210 b.

**Données qui quittent la caisse.** Aucune.

**Garde-fous.** Un ticket de cuisine ne suit qu'un P signé (module fiscal absent : « Envoyer la
suite » est refusé, rien n'est imprimé) ; une ligne n'est enregistrée comme imprimée que si le
ticket est réellement sorti ; une correction d'une ligne déjà imprimée sort tout de suite ; un
temps retenu n'est jamais imprimé par la caisse seule à l'encaissement.

**Tests.** Voir `SERVICES_CUISINE_FR.md`, section 7.

**Cas d'usage officiels.** Les cas d'usage SPF rejoués ne sont pas modifiés : éteinte, la
fonction ne change aucun octet imprimé ni signé.

**Écrans.** Réglages (panneau « Service en plusieurs temps »), panier groupé par temps avec
pastilles, sélecteur de temps et bouton « Envoyer la suite », plan de salle (« Suite : Plats »),
avertissement à l'encaissement, tickets imprimés (premier envoi, « SUITE : Plats », vente
directe avec en-têtes, DUPLICATA) : captures à reprendre après la fusion.

## E. Bons cadeaux à usage multiple (dans le binaire certifié)

**Ce qui change.** Un gérant peut activer (Réglages, panneau « Bons cadeaux ») la vente et
l'acceptation de bons cadeaux. À l'écran de commande, le bouton « Bon cadeau » demande un montant
et ajoute au panier un produit système (T.V.A. X, département GV). À l'encaissement, la méthode
« Bon cadeau » demande le code à 12 chiffres, montre le solde et la validité, et règle le plus
petit du solde et du reste à payer. Le bon vendu s'imprime après la vente ; un solde restant
s'imprime sur un petit ticket. L'onglet « Bons cadeaux » des rapports recherche un bon par son
code et, pour un gérant, liste les bons et réimprime un duplicata. Même écran et mêmes textes
sur Linux (Vue) et sur Android (Compose).

**Est-ce dans le binaire certifié ?** Oui : migration 63 (produit système et trois tables), six
commandes du moteur, tickets `pos-print`, un moyen de paiement de plus. Aucune option de
compilation. Éteint par défaut ; éteint, les ventes sont inchangées.

**Impact fiscal.** Vente d'un bon : ligne T.V.A. X (hors champ), aucune T.V.A. à la vente, la
T.V.A. est due sur les marchandises achetées avec le bon. Utilisation : un paiement de type
`VOUCHER_STORE` (identité 8, « Gift voucher », saisie `MANUAL`), montant exact, jamais arrondi.
Les cas officiels du jeu avancé UC260, UC261 et UC262 sont rejoués et conformes, avec des écarts
expliqués (voir `BONS_CADEAUX_FR.md`, section 9). Le code d'un bon n'entre dans aucune demande
signée ni aucun ticket de T.V.A.

**Données qui quittent la caisse.** Aucune. Une vente qui vend un bon n'est pas renvoyée à HubRise.

**Garde-fous.** Code à 12 chiffres dont 2 de contrôle ; recherches limitées à 5 échecs par
minute ; code masqué partout sauf sur le bon imprimé ; un bon utilisé ne peut pas être remboursé
(`gift_voucher_used`) ; un bon expiré, annulé, d'un autre mode (formation) ou insuffisant est
refusé avant l'envoi au module fiscal ; les trois tables sont en ajout seulement.

**Différé.** Bons à usage unique (UC270, UC271), conversion d'un bon en espèces (UC4105),
remboursement vers un bon : voir `BONS_CADEAUX_FR.md`, section 8.

**Tests.** Voir `BONS_CADEAUX_FR.md`, section 11.

**Écrans.** Réglages (panneau « Bons cadeaux »), commande (bouton, saisie du montant, ligne avec
pastille), encaissement (méthode, saisie du code, notes d'impression), rapports (onglet « Bons
cadeaux »), tickets imprimés (bon, solde, duplicata) : captures à reprendre après la fusion.
