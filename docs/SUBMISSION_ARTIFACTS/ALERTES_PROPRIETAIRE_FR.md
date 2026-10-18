# Alertes du propriétaire sur téléphone

Fonction non fiscale de la caisse, comprise dans le binaire certifié et désactivée par défaut.

## 1. Objet et statut

Quand la caisse tombe en panne en dehors des heures de présence du propriétaire (module
fiscal muet, imprimante en défaut, terminal de carte hors ligne, Z oublié, commande en
ligne qui attend), le propriétaire veut le savoir sur son téléphone. La caisse envoie
alors un court message Telegram à la conversation que le propriétaire a choisie.

Points qui comptent pour la certification :

- **la fonction est désactivée par défaut** : tant que le propriétaire ne l'active pas
  dans Réglages, la caisse n'ouvre aucune connexion vers Telegram ;
- **elle ne touche aucune donnée fiscale** : elle ne signe rien, n'écrit aucun événement
  N, P, T, C, I, F, S ou R, ne lit ni les tickets ni les rapports Z ;
- **elle ne bloque jamais la caisse** : un message qui n'arrive pas, un téléphone éteint
  ou un service injoignable n'a aucun effet sur une vente, un ticket ou un Z ; un test
  automatique mesure qu'une vente et un Z restent rapides pendant qu'un message est
  bloqué sur un serveur qui ne répond pas ;
- **elle n'envoie aucune donnée personnelle** (voir la liste complète au point 3).

La fonction est dans le binaire certifié, sans option de compilation : tant que le propriétaire ne
l'active pas, la caisse se comporte comme si elle n'existait pas (aucune connexion sortante).

## 2. Ce qui déclenche un message

Cinq situations, chacune activable séparément par le propriétaire.

| Situation | Quand le message part | Remarque |
| --- | --- | --- |
| Module fiscal (FDM) | le FDM ne répond plus depuis 2 minutes | jugé seulement pendant une session ouverte (le contrôle n'a lieu qu'alors) |
| Imprimante | l'imprimante des tickets ou celle de la cuisine ne répond pas, ou signale plus de papier ou couvercle ouvert, à deux contrôles de suite (espacés de 5 minutes) | le contrôle est ignoré pendant une vente ou une impression en cours |
| Terminal de carte | le terminal lié ne répond pas à deux contrôles de suite | seulement si un terminal est configuré, jamais pendant un paiement |
| Z oublié | la période comptable ouverte n'est pas clôturée à l'heure limite (le lendemain à l'heure choisie, par défaut une heure après la fin de journée, heure de Bruxelles) | ignoré en mode formation |
| Commande en ligne en attente | une commande en ligne attend depuis plus de N minutes (5 par défaut) | seulement si l'intégration des commandes en ligne est activée |

Pour chaque situation : **un seul message d'alerte** à l'ouverture, **un rappel** toutes
les 60 minutes (réglable de 1 à 1440, 0 = jamais) tant que la situation dure, et **un
message « résolu »** quand elle disparaît (envoyé seulement si le propriétaire avait été
prévenu). Une limite globale de **20 messages par heure** protège le téléphone d'une
tempête de messages ; au-delà, les messages sont écartés et notés « retenu » dans le
journal. Au redémarrage de la caisse, l'état est relu dans le journal : une alerte
déjà annoncée n'est pas annoncée une seconde fois.

## 3. Données envoyées (liste complète)

Un message contient uniquement :

1. un mot d'en-tête : ALERTE, RAPPEL, RÉSOLU ou TEST (ou l'équivalent en anglais ou en
   chinois selon la langue réglée) ;
2. le **nom de l'installation** choisi par le propriétaire (par défaut l'identifiant du
   terminal), entre crochets ;
3. une phrase décrivant la situation, avec l'**heure locale de début** (heure de Bruxelles) ;
4. selon la situation : le nom de l'imprimante concernée (tickets ou cuisine) et la nature
   du défaut (injoignable, plus de papier, couvercle ouvert) ; la **date comptable** et
   l'heure limite d'un Z oublié ; le **nombre** de commandes en ligne en attente, le délai
   et le **nom des canaux** (plateforme de livraison ou boutique en ligne).

Ne sont **jamais** envoyés : numéro national (INSZ) ou nom d'un employé ou d'un client,
adresse, téléphone, courriel, montant, numéro de ticket ou de commande, contenu d'une
commande, mode de paiement, identifiant du FDM. Le code qui compose les messages
(`crates/pos-engine/src/alerts/text.rs`) n'a pas de champ pour ces informations, et un
test automatique, dans les trois langues, vérifie qu'une commande en ligne portant un nom
de client, une adresse et un montant ne laisse aucune de ces valeurs dans le message ni
dans le journal.

Les noms de canaux sont ramenés à des lettres, chiffres, espaces et quelques signes
(30 caractères au plus), pour qu'un nom de canal ne puisse pas transporter autre chose.

## 4. Secret et configuration

- **Le jeton du robot Telegram** est un secret. Il est dans un **fichier** nommé par la
  configuration de la caisse (`alerts.telegram_bot_token_file` dans `pos.json`, chemin
  relatif au fichier de configuration comme le jeton du FDM), jamais dans la base de
  données, jamais dans un journal, jamais renvoyé à l'écran : Réglages affiche
  seulement « Robot Telegram configuré » ou « Robot Telegram absent ». Il suit la configuration de la caisse.
- La vérification de configuration (`--check-config`) signale un fichier de jeton
  illisible ou une adresse invalide (section `alerts`), sans jamais citer le jeton.
- Le jeton figure dans l'adresse des requêtes Telegram. Les erreurs sont donc écrites
  sans adresse (le texte d'une erreur réseau est nettoyé et toute portion de la forme
  `/bot…/` est remplacée par `/bot***/`). Un test automatique cherche le jeton dans les
  journaux, les erreurs renvoyées à l'écran et les lignes du journal d'alertes : il n'y est pas.
- Le reste se règle dans Réglages (gérants seulement) : interrupteur général, conversation
  Telegram (identifiant numérique, négatif pour un groupe), nom dans les messages, langue
  (français, chinois, anglais), une case par situation, heure limite du Z, délai des
  commandes en ligne, intervalle des rappels. Chaque enregistrement est écrit dans un
  journal des réglages qui ne s'efface pas.
- Le bouton « Envoyer un message test » envoie un seul message TEST avec les réglages
  enregistrés et dit pourquoi si Telegram refuse.

## 5. Fonctionnement et robustesse

- Une boucle propre au moteur, indépendante des commandes de la caisse, relit les
  réglages toutes les 30 secondes. Elle n'attend jamais le verrou de signature ni
  celui de la base pendant qu'une requête réseau est en cours.
- Les contrôles d'imprimante et de terminal n'ont lieu que pendant une session ouverte, que
  la signature est libre, et que le terminal de carte n'est pas occupé par un paiement.
- Le contrôle du terminal de carte tient la ligne du terminal au plus 12 secondes (plus que le pire cas du
  contrôle du terminal Worldline, deux requêtes de 5 secondes) : un terminal qui ne répond pas est
  abandonné après ce délai et la ligne est libérée. Un paiement qui démarre pendant un contrôle attend donc
  au plus ce délai, et le contrôle ne tient jamais la ligne pendant un paiement.
- Chaque requête a un délai de 10 secondes et trois essais (attente de 2 puis 4 secondes) ;
  aucune nouvelle tentative pour un refus que répéter ne guérit pas (jeton invalide,
  conversation inconnue). Un message abandonné est noté « échec » dans le journal.
- Les seules données qui sortent de la caisse sont les messages ci-dessus, vers le service
  Telegram (HTTPS). Le service est un tiers : le propriétaire choisit son robot et sa
  conversation, et peut retirer la fonction en la désactivant ou en supprimant le fichier du jeton.

## 5 bis. Droits

Les quatre commandes (lire et régler les alertes, envoyer un message test, lire les derniers messages) sont
réservées au profil Gérant. Les autres profils ne voient pas le panneau et le moteur refuse la commande (`forbidden`).

## 6. Traces conservées

| Table | Contenu | Effacement |
| --- | --- | --- |
| `owner_settings` | la valeur courante des réglages (JSON) | remplacée à chaque changement |
| `owner_settings_log` | chaque valeur enregistrée, qui, quand | impossible (déclencheurs) |
| `owner_alert_log` | chaque message : situation, transition, détail (compteurs, date, noms de canaux), résultat (envoyé, échec, retenu), code HTTP, heure | impossible (déclencheurs) |

Le journal d'alertes n'est pas purgé : il reste petit (une ligne par message, et au plus 20 messages envoyés par heure).
Il ne contient aucune donnée personnelle ni le jeton.

## 7. Tests

- Règles pures (ouverture, rappel, résolution, limite horaire, reprise après redémarrage)
  avec une horloge fixe : `crates/pos-engine/src/alerts/rules.rs`.
- Moteur contre un faux service Telegram : fonction éteinte ou sans jeton, délais de
  confirmation du FDM, rappel, résolution, absence de session, reprise après redémarrage,
  limite de 20 par heure, trois essais et pas de nouvel essai sur un refus, Z oublié,
  imprimante (contrôle ignoré pendant une vente), terminal (ignoré pendant un paiement),
  commandes en ligne sans donnée personnelle, message test, accès réservé aux gérants,
  jeton absent des journaux, des erreurs et des lignes, un service qui ne répond pas
  qui ne retarde ni une vente ni un Z, arrêt propre de la boucle
  (`crates/pos-engine/tests/owner_alerts.rs`).
- Migration 60 : tables et journaux qui ne s'effacent pas, valeurs contrôlées
  (`crates/pos-db/src/migrate.rs`).
- L'interface du panneau « Alertes sur téléphone » (éteint sans jeton, allumé avec jeton, message test, derniers
  messages), textes en français, chinois et anglais, mêmes règles de saisie : `ui/e2e/owner-alerts.spec.ts`.

## 8. Cas d'usage officiels

Aucun cas d'usage SPF n'est modifié ni rejoué : la fonction ne produit aucun événement
fiscal et ne change aucun ticket, rapport ou paiement.

## Captures d'écran

L'écran de cette fonction est la capture 81 (panneau « Alertes sur téléphone ») de *CAPTURES_ECRAN_FR.pdf*.
