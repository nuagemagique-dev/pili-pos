# Manuel utilisateur - pili-pos v1.0.0

Ce manuel s'adresse au personnel de salle et de caisse qui utilise
`pili-pos` au quotidien : connexion, prise de commande, addition,
encaissement, copies, remboursements, rapports, mouvements de caisse et
mode formation. Les libellés cités entre guillemets sont ceux de
l'écran, en français (l'interface existe aussi en anglais et en
chinois, §12.2). Les écrans correspondants figurent dans
`CAPTURES_ECRAN_FR.pdf`.

## Sommaire

1. Profils : qui peut faire quoi
2. Démarrage, connexion et PIN
3. Pointage d'arrivée et de départ (événement S)
4. L'écran de commande
5. Tables, à emporter et prise de commande
6. Commande enregistrée et addition (événements P)
7. Encaissement (vente, événement N)
8. Copie de ticket (événement C)
9. Remboursement
9 bis. Corriger un moyen de paiement (événement F)
10. Rapports X et Z (événements R)
11. Mouvements de caisse, tiroir et comptage (événements F)
12. Configuration (Gérant)
13. Mode formation (événements T)
14. Que faire en cas de problème

---

## 1. Profils : qui peut faire quoi

Chaque employé a un profil, choisi par le Gérant à la création de son
compte (§12.3). Le POS n'affiche que les boutons autorisés, et la caisse
refuse de toute façon une action interdite au profil
(`PROFILS_UTILISATEURS_FR.md` §3).

| Action | Apprenti | Serveur | Gérant |
|---|:-:|:-:|:-:|
| Se connecter, pointer, prendre des commandes, "Envoyer en cuisine" | ☑ | ☑ | ☑ |
| Changer son propre PIN | ☑ | ☑ | ☑ |
| Commandes en ligne : "Préparer", "Prête", "Retarder" (§12.8) | ☑ | ☑ | ☑ |
| Changer une commande de table (§5.1 bis) | ☑ | ☑ | ☑ |
| Imprimer l'addition, encaisser, "Imprimer le ticket" (§7.3) | - | ☑ | ☑ |
| Commandes en ligne : "Accepter", "Refuser", "Terminer", "Encaisser espèces", "Encaisser carte", "Clôturer" | - | ☑ | ☑ |
| Rapports X / Z, copies, remboursements, corrections de paiement (§9 bis), mouvements de caisse, tiroir, comptage du tiroir | - | - | ☑ |
| Clôturer une vente sans paiement (client parti sans payer, §7.5, avec son PIN) | - | - | ☑ |
| Commandes en ligne : "Rembourser" | - | - | ☑ |
| Mode formation, employés (ajouter, "Désactiver", PIN des autres), HubRise (activer, catalogue, canaux), plan de salle (§12.9), configuration | - | - | ☑ |
| Démarrer l'assistance à distance (§12.6, avec son PIN) | - | - | ☑ |

## 2. Démarrage, connexion et PIN

Le terminal démarre seul sur pili-pos, en plein écran. Aucune
intervention n'est nécessaire.

### 2.1 Se connecter

L'écran de connexion est coupé en deux.

- À gauche, le panneau "Démarrage" donne l'état des appareils : "FDM"
  ("connecté", "avertissement" ou "hors ligne"), "Imprimante ticket"
  et "Imprimante cuisine" ("prête", "injoignable" ou "non configuré"),
  "Terminal de paiement" (seulement si un terminal est relié à la
  caisse : "connecté", "paiement en cours" ou "injoignable"),
  "HubRise" (seulement s'il est activé : "connecté" ou "non connecté")
  et "Email tickets" ("joignable", "injoignable" ou "non configuré"). Ces
  lignes informent seulement : un appareil qui ne répond pas n'empêche
  jamais de se connecter. Le nom de l'établissement et l'identifiant du terminal sont en bas, avec
  la version du logiciel ("Version pili-pos/<numéro>"). Capture
  `01a` de `CAPTURES_ECRAN_FR.pdf` (terminal de paiement relié et connecté).
- À droite, "Qui êtes-vous ?" affiche une tuile par employé (initiale,
  nom, profil) et son état : "en service · 11:42" ou "hors service".

```
┌─────────────────────────┬────────────────────────────────────┐
│ Démarrage               │ Qui êtes-vous ?                    │
│  ● FDM        connecté  │  ┌────────┐ ┌────────┐ ┌────────┐  │
│  ● Imprimante ticket    │  │ G      │ │ S      │ │ A      │  │
│    prête                │  │ Gérant │ │ Serveur│ │Apprenti│  │
│  ● Imprimante cuisine   │  │ en     │ │ hors   │ │ hors   │  │
│    prête                │  │ service│ │ service│ │ service│  │
│  ● Email tickets        │  └────────┘ └────────┘ └────────┘  │
│    non configuré        │  [Autre personne (INSZ)]           │
│ Restaurant Exemple POS1 │                                    │
└─────────────────────────┴────────────────────────────────────┘
```

1. Toucher sa tuile.
2. Si l'employé a un PIN, un pavé "Code PIN" s'affiche à droite : taper
   les 4 chiffres. La connexion part d'elle-même au 4e chiffre
   ("connexion au 4ᵉ chiffre").
3. Un PIN erroné affiche "PIN incorrect" et vide le pavé ; "Retour"
   revient aux tuiles.

"Autre personne (INSZ)" permet de se connecter en tapant son numéro
INSZ (11 chiffres) puis "Connexion" ("Retour" revient aux tuiles). Si ce
compte a un PIN, le pavé du PIN s'affiche ensuite, comme depuis la
tuile.

L'identité système `00000000029` (rapport Z automatique) n'a pas de
tuile et ne peut pas se connecter.

**Première connexion d'un Gérant.** Un Gérant doit avoir un PIN. Ce
choix à l'écran de connexion n'existe qu'à la toute première mise en
service, tant qu'aucun Gérant n'a de PIN : toucher sa tuile ouvre
"Choisissez votre PIN" ; taper un PIN de 4 chiffres, puis une seconde
fois ("Confirmer le PIN"). La session s'ouvre et "PIN enregistré"
s'affiche. Si les deux saisies diffèrent, "Les PIN ne correspondent pas"
s'affiche et personne n'est connecté. Le PIN est ensuite demandé à chaque
connexion. Dès qu'un Gérant a un PIN, un autre Gérant sans PIN ne peut
plus le choisir lui-même : la caisse répond "Un gérant doit choisir le
PIN de ce compte dans Paramètres." et un Gérant déjà connecté le fixe
(§2.2, §12.3).

### 2.2 Le PIN

Un Gérant a toujours un PIN. Un Serveur ou un Apprenti peut se connecter
sans PIN, mais alors n'importe qui peut ouvrir sa session en touchant
sa tuile. Tant qu'il n'a pas de PIN, un avertissement s'affiche à sa
connexion : "PIN non configuré : touchez votre nom en haut de l'écran".

- **Son propre PIN** : sur l'écran de commande, toucher son nom dans
  l'en-tête (il porte la mention "PIN"). Taper le nouveau PIN ("Saisir
  le PIN"), puis une seconde fois ("Confirmer le PIN"). S'ils diffèrent,
  "Les PIN ne correspondent pas" s'affiche et rien n'est enregistré.
  Sinon, "PIN modifié".
- **Le PIN d'un autre employé** (Gérant) : bouton "Config", puis le bouton
  "Code PIN" sur la ligne de l'employé (§12.3). Le Gérant tape d'abord
  le nouveau PIN de l'employé (deux fois), puis la caisse demande
  "Confirmez avec votre PIN de gérant" : le Gérant tape son propre PIN.
  Sans ce PIN, rien n'est enregistré.

Le PIN est enregistré sous forme de hachage (bcrypt), jamais en clair.

**Verrou après des PIN erronés.** Après 5 PIN incorrects de suite pour un
même employé (à la connexion ou lors d'une confirmation par PIN), la
caisse refuse tout essai, même avec le bon PIN, pendant 30 secondes. Elle
affiche "Trop de PIN incorrects : réessayez dans {seconds} s.", avec le
temps restant. Chaque nouvel échec double l'attente, jusqu'à 15 minutes.
Un PIN correct remet le compteur à zéro. Un redémarrage de la caisse le
remet aussi à zéro.

### 2.3 Verrouiller, fin de service et verrouillage automatique

- "Verrouiller" (en haut à droite) ferme la session et revient à l'écran
  de connexion. L'employé **reste en service** : sa tuile affiche "en
  service". Quand il se reconnecte, aucune nouvelle arrivée n'est
  pointée.
- Après **5 minutes sans toucher l'écran**, la caisse se verrouille
  d'elle-même, de la même façon. L'employé reste en service.
- Le départ de l'employé se signe uniquement depuis l'écran de connexion
  ("Fin de service", §3).

Dans les deux cas de verrouillage, la commande en cours est d'abord
enregistrée auprès du FDM (§6.1).

## 3. Pointage d'arrivée et de départ (événement S)

- **À la connexion**, la caisse enregistre l'arrivée de l'employé
  (événement S, `signWorkIn`), sauf s'il est déjà "en service" (écran
  seulement verrouillé). Il n'y a pas de bouton.
- **À la fin de service**, le départ (événement S, `signWorkOut`) se
  signe sur l'écran de connexion : toucher sa tuile (elle indique "en
  service"), toucher "Fin de service", puis taper son PIN ("le code PIN
  termine le service"). La caisse signe le départ, ferme la session et
  affiche "Service terminé · <nom>". La tuile repasse à "hors service".
  Un employé sans PIN touche sa tuile puis "Fin de service" sans PIN.
- "Verrouiller" et le verrouillage automatique ne pointent **pas** le
  départ (§2.3).

Si le FDM ne répond pas, la connexion ou la fin de service se fait quand
même, mais un message rouge l'indique : "Pointage non enregistré", suivi
de la raison. Prévenir le Gérant pour qu'il régularise.

## 4. L'écran de commande

```
┌───────────────────────────────────────────────────────────────────┐
│ FDM ● Ticket ● Cuisine ●  Marie · PIN   Table 4   14:05           │
│     [Tables] [À emporter] [Rapports] [Config] [Verrouiller]       │
├───────────────────────────────────────────┬───────────────────────┤
│ [Sushi] [Plats chauds] [Boissons] ...     │ Commande · Table 4    │
│                                           │ 14:05 · 12 min        │
│ ┌────────┐ ┌────────┐ ┌────────┐ ┌──────┐ │ Saumon roll x8  8,50  │
│ │Saumon  │ │Califor.│ │Sashimi │ │Coca  │ │   [-] 1 [+]      [x]  │
│ │roll x8 │ │roll x8 │ │mix     │ │33cl  │ │ Pas encore en cuisine │
│ │ 8,50 € │ │ 9,00 € │ │14,00 € │ │3,50 €│ │ Coca 33cl       3,50  │
│ └────────┘ └────────┘ └────────┘ └──────┘ │   [-] 1 [+]      [x]  │
│                                           │ 12,00 €   TVA ...     │
│                                           │ [Envoyer en cuisine 1]│
│                                           │ [Addition]            │
│                                           │ [PAYER        12,00 €]│
└───────────────────────────────────────────┴───────────────────────┘
```

L'en-tête montre en permanence :

- les voyants des appareils : **FDM** (la "boîte noire"), **Ticket**
  (imprimante de tickets), **Cuisine** (imprimante du bar ou de la
  cuisine), **Terminal** (le terminal de paiement relié, seulement s'il
  y en a un) et **HubRise** (seulement s'il est activé). Vert : tout va
  bien. Orange ou rouge : un problème. Gris : appareil non configuré. Le
  FDM est interrogé toutes les 30 secondes (moins souvent tant qu'il ne
  répond pas) ; son message en clair s'affiche dans un bandeau sous
  l'en-tête (ex. "Connexion perdue avec la boîte noire. Vérifiez le
  câble réseau."). Les imprimantes et le terminal de paiement sont
  vérifiés toutes les 30 secondes (le voyant "Terminal" est orange
  pendant qu'un paiement est en cours sur le terminal, et rouge quand le
  terminal ne répond pas ; le toucher ou le survoler en donne la raison
  quand la caisse la connaît). Cette vérification ne débite jamais rien
  et ne dérange pas un paiement en cours (capture `03a` : voyant "Terminal" vert) ;
- le nom de l'employé connecté, son profil et "PIN" (le toucher pour
  changer son PIN, §2.2), puis la table ("Table 4") ou "À emporter", et
  l'heure ;
- les boutons "Tables", "À emporter", "En ligne" (seulement si HubRise
  est activé), "Rapports" et "Config" (ces deux derniers pour le Gérant
  seulement) et "Verrouiller".

**Rappel du rapport Z** : un bandeau "Rapport Z non effectué
aujourd'hui" s'affiche sur l'écran de commande quand une journée est
ouverte et qu'il est plus de 22 h, ou qu'une journée précédente est
restée ouverte (§10.2). L'onglet "Journée" des rapports affiche aussi
"Rapport Z pas encore fait aujourd'hui".

## 5. Tables, à emporter et prise de commande

### 5.1 Choisir une table

"Tables" affiche le plan des tables, tel que fixé à l'installation et
modifiable par le Gérant (§12.9). Une table hors plan qui a encore une
commande ouverte reste affichée, pour qu'aucune commande non réglée ne
soit masquée. Une table **libre** porte la
mention "Libre". Une table **occupée** affiche son total, son nombre
d'articles et depuis combien de minutes elle est ouverte ("3 art. · 12
min"), et la mention "Addition" quand l'addition provisoire a été
imprimée.

- Toucher une table libre ouvre une nouvelle commande pour cette table.
- Toucher une table occupée reprend sa commande en cours, avec tous ses
  articles dans le panier. Une table n'a jamais qu'une seule commande
  ouverte.

### 5.1 bis Changer de table

Une commande ouverte, déjà servie en salle, peut passer à une **autre
table libre** (client qui change de place, tables regroupées).

- Dans la commande : touchez « Changer de table » (en haut du panier),
  choisissez la table libre, puis « Valider ».
- Sur le plan des tables : touchez « Déplacer une table », puis la table
  occupée, puis la table libre, et confirmez par « Valider » (« Retour »
  annule).

Ce que fait la caisse : les articles pas encore envoyés en cuisine sont
d'abord enregistrés sur l'ancienne table, puis le déplacement est signé
(événement P, « changement de centre de coût »). La commande garde son
contenu et sa référence ; aucun argent ne bouge. Un bon « TRANSFERT DE
TABLE » (« Table 4 -> Table 9 », sans prix) sort à l'imprimante du bar.
« Commande déplacée vers la table N » confirme.

Refus : une table qui a déjà une commande en cours n'est pas
fusionnée (« Cette table a déjà une commande en cours : choisissez une
table libre ») ; seule une commande en salle, avec au moins un article,
peut changer de table. Si le FDM ne répond pas, le déplacement reste
en attente et est repris comme une vente (§7.4) ; la table ne change
qu'à la signature.

### 5.2 Vente à emporter

"À emporter" (en-tête) ouvre une commande sans table. L'addition (§6.2)
n'existe pas pour une commande à emporter : on encaisse directement.

Une vente à emporter est une vente directe : quitter la commande avant
de l'encaisser n'enregistre rien auprès du FDM (pas d'événement P). La
commande reste en cours, listée sur l'écran "Tables" sous "À emporter en
cours" ("Commencée à 14:05", nombre d'articles, total) ; la toucher la
reprend. "Nouvelle vente à emporter" en ouvre une autre. Elle se termine
par l'encaissement (§7) ou, si le client est parti sans payer, par une
vente sans paiement (§7.5).

### 5.3 Ajouter, modifier, retirer

- Choisir une catégorie dans la barre du haut, puis toucher un article :
  il s'ajoute au panier (à droite).
- Dans le panier, "+" ajoute une unité et "−" en retire une, sans
  descendre sous 1. Pour retirer une ligne, toucher la croix à sa
  droite.
- **Remise (Gérant, code PIN)** : le panier propose « Remise » (sur
  toute la commande) et, sur une ligne, « Offrir ». « Remise » ouvre la
  fenêtre « Remise » : choisir « % » (raccourcis 5, 10, 15, 20
  et 50 %), « Montant » (à taper comme sur un terminal de paiement) ou
  « Offert », puis un motif (« Geste commercial », « Repas du personnel »,
  « Problème de service », « Autre ») et, si besoin, une précision libre de 120
  caractères au plus. L'aperçu montre le total après remise ; « Appliquer la remise »
  demande le code PIN d'un Gérant. Le panier, l'écran de paiement et le
  ticket affichent alors le prix net, avec une ligne « Remise » sous la
  ligne concernée. « Retirer la remise » (même code PIN) annule la
  remise. Une seule remise s'applique à la fois (retirer l'actuelle avant d'en
  poser une autre) ; elle ne peut pas dépasser le total, et un article
  offert garde sa ligne à 0,00 sur le ticket. Une ligne remisée n'est
  plus fusionnée avec une nouvelle unité du même article. Sur une
  commande déjà enregistrée en salle (§6), la remise est signée
  tout de suite (événement P) ; si le FDM ne répond pas, un avertissement
  s'affiche et l'envoi est refait à la commande suivante. Le motif reste dans
  le journal local de la caisse et n'est jamais envoyé au FDM. Les
  remises d'une commande en ligne viennent de la plateforme (§12.8).
- **Prix libre** : certains articles n'ont pas de prix fixe (ex. "Plat
  du jour"). En les touchant, la fenêtre "Prix libre" demande le
  prix : taper les chiffres comme sur un terminal de paiement
  (2 0 0 0 = 20,00 €), puis "Valider" ("Retour" annule).
- **Options** : quand HubRise est activé (§12.7), un article qui a des
  groupes d'options ouvre la fenêtre "Options". Chaque groupe indique sa
  règle : "Choisir {min}", "{min} à {max}", "Jusqu'à {max}" ou
  "Facultatif". Un groupe à un seul choix remplace la sélection
  précédente ; les autres se cochent. "Ajouter" (avec le prix) reste
  grisé tant qu'un minimum n'est pas atteint ; un maximum ne peut pas
  être dépassé. Le prix des options s'ajoute à celui de l'article et
  figure dans le nom de la ligne. Sans option à choisir, l'article
  s'ajoute directement.

**Annulation.** Avant l'encaissement, retirer la ligne suffit. Une fois
la vente signée, elle ne se modifie plus : l'annuler, c'est la
rembourser (§9).

**Factures B2B.** La facturation entre entreprises n'est pas proposée
dans cette version : le POS n'émet que des tickets de caisse TVA.

Le panier affiche en permanence le total et le détail de la TVA par
taux.

## 6. Commande enregistrée et addition (événements P)

### 6.1 Envoyer en cuisine et enregistrement de la commande

Dans une commande de table, les articles pas encore transmis sont listés
sous la mention **"Pas encore en cuisine"**. "Envoyer en cuisine"
(avec le nombre d'articles concernés) les enregistre auprès du FDM
(événement P, `signOrder`) et imprime un **bon de commande** pour le bar
ou la cuisine, sans quitter la table. "Envoyé en cuisine · n articles"
confirme.

La même chose se produit **dès que l'on quitte la commande d'une table**
(jamais pour une vente à emporter, §5.2) : "Tables", "À emporter",
"Rapports", "Config", "Verrouiller" ou verrouillage automatique. Les
articles retirés entre-temps sont enregistrés aussi, comme corrections.

Le bon de commande ("BON DE COMMANDE") ne liste que les lignes qui
viennent d'être enregistrées, pour les articles marqués "cuisine" au
catalogue ; il ne porte pas de prix et la mention "CECI N'EST PAS UN
TICKET DE CAISSE T.V.A. VALABLE". Il sort sur
l'imprimante de la cuisine si elle est configurée, sinon sur celle des
tickets. Pour une vente à emporter, le bon sort à l'encaissement.

Si l'impression du bon échoue, la commande est quand même enregistrée et
le message "Bon de commande non imprimé : prévenez le bar ou la cuisine."
s'affiche : prévenir le bar ou la cuisine de vive voix.

Si le FDM ne répond pas, le message "Commande non enregistrée par le
FDM" s'affiche et l'on reste sur la commande ; réessayer une fois le
FDM revenu. Rien n'est perdu : ce qui n'a pas pu être envoyé part au
prochain enregistrement, à l'addition ou à l'encaissement.

### 6.2 Addition

"Addition" (Serveur et Gérant, commandes en salle avec au moins un
article) imprime l'addition provisoire à présenter au client
(événement P, `signPreBill`) et enregistre d'abord ce qui n'était pas
encore transmis. "Addition provisoire imprimée" confirme ; en cas
d'échec, "Addition non enregistrée" suivi de la raison.

L'addition est marquée "**PRO FORMA**" et "**CECI N'EST PAS UN TICKET
DE CAISSE T.V.A. VALABLE**". Elle porte les données de contrôle du FDM
(identifiant, date, compteurs), mais ni QR code ni signature courte :
le FDM ne les donne qu'aux ventes (N). Elle ne clôture pas la table.

## 7. Encaissement (vente, événement N)

"PAYER" (Serveur et Gérant) ouvre l'écran de paiement, qui affiche le
total à payer. Les trois moyens de paiement ("Espèces", "Carte",
"Chèques-repas") sont côte à côte, l'espèces étant choisie d'office.

### 7.1 Choix du support du ticket (si activé sur ce terminal)

Quand le ticket numérique est configuré (`MANUEL_INSTALLATION_FR.md`
§4.2), trois boutons apparaissent sur la ligne "Ticket" :

- **Papier** (choix par défaut).
- **E-mail** : le ticket part par e-mail seulement. Ce bouton n'apparaît
  que si l'installateur l'a autorisé et que le service d'envoi répond.
- **Les deux** : le papier sort, et l'e-mail part en plus.

"E-mail" ou "Les deux" ouvre un clavier ("Adresse e-mail du client")
avec des raccourcis (`.be`, `.com`, `@gmail.com`). Une adresse mal
formée est signalée ("Adresse invalide"). Un écran "Confirmer
l'adresse" l'affiche ensuite en grand : la faire relire au client si
possible, puis "Confirmer l'adresse" ("Modifier" pour corriger,
"Annuler" pour revenir au papier).

Si le service d'envoi ne répond pas, la caisse le dit ("Service e-mail
indisponible : le ticket papier sera imprimé, l'e-mail sera envoyé dès
que possible").

Sans ticket numérique configuré, ces boutons n'apparaissent pas : le
ticket est imprimé.

### 7.2 Paiement

Une vente se paie en un ou plusieurs moyens de paiement, **sur un seul
ticket** (un seul événement N, même payée en plusieurs parts).

- **Espèces** : taper le montant reçu sur le pavé ("Montant reçu"), ou
  toucher "Exact" ou un raccourci de billet. La caisse affiche le montant
  à rendre ("Rendu"). Le montant en espèces est **arrondi aux 5 cents**
  (règle belge) ; le montant exact avant arrondi reste affiché
  ("exact 3,34 · arrondi 5 cts"). "Valider".
- **Carte** : le pavé propose le montant à payer ; le laisser ou taper
  un montant plus petit (jamais plus que le reste : "Plus que le reste à
  payer"). Sans terminal de paiement relié à la caisse : encaisser ce
  montant sur le terminal de paiement ("Encaisser sur le terminal"), puis
  "Valider" une fois le paiement accepté. Avec un terminal relié, voir
  "Carte sur le terminal relié" ci-dessous. Pas d'arrondi pour une carte.
- **Chèques-repas** (chèques-repas électroniques, payés sur le terminal
  de paiement) : comme la carte, pour le montant payé en chèques-repas.
  Pas d'arrondi, pas de rendu sur des chèques-repas.

**Carte sur le terminal relié (si l'installation en a un).** Quand le
terminal de paiement est relié à la caisse (réglage de l'installation,
`MANUEL_INSTALLATION_FR.md` §4.2), l'opérateur ne tape plus la carte :
la caisse envoie le montant au terminal et reçoit sa réponse.

La caisse montre aussi si le terminal relié est joignable : le voyant
"Terminal" de l'en-tête (§4) et la ligne "Terminal de paiement" du
panneau "Démarrage" (§2.1). Cette vérification se contente de demander
au terminal s'il répond : elle ne débite jamais rien, elle n'est jamais
faite pendant un paiement (le voyant est alors orange) et elle
n'empêche pas de se connecter ni de vendre.

Captures : `09h` (choix "Terminal" ou "Saisie manuelle"), `09i` (fenêtre
"Paiement sur le terminal"), `09j` (paiement refusé) et `09k` (réponse
inconnue et bouton "Vérifier").

1. Choisir "Carte" (le choix "Terminal" est actif, l'écran indique "Le
   montant est envoyé au terminal à la validation"), laisser ou taper le
   montant (comme ci-dessus), puis toucher "Valider". Une fenêtre
   "Paiement sur le terminal" s'ouvre avec le montant et le message "Le
   client présente sa carte sur le terminal". La caisse n'envoie le montant que si le
   FDM est opérationnel à cet instant : sinon le message "FDM not
   connected" s'affiche et rien n'est débité.
2. Le terminal répond. Paiement accepté : la fenêtre se ferme et la part
   est prise, avec le type de carte et la marque que le terminal
   rend (la marque s'imprime sur le ticket, par exemple "Carte
   Bancontact"). Paiement refusé ou abandonné par le client : la fenêtre
   le dit, rien n'est débité et l'opérateur peut recommencer ou prendre
   un autre moyen de paiement.
3. "Annuler" dans la fenêtre demande au terminal d'abandonner le
   paiement en cours. Si le terminal l'avait déjà accepté, c'est lui qui
   l'emporte : le paiement accepté reste, il faut l'utiliser.
4. Si la réponse du terminal est inconnue (câble ou réseau coupé,
   terminal éteint, panne de courant de la caisse), la fenêtre affiche
   "Réponse du terminal inconnue" : un débit est peut-être passé. **Tant que la
   réponse n'est pas connue, la vente de cette commande est bloquée.**
   Toucher "Vérifier" redemande le résultat au terminal (tout
   utilisateur peut le faire) ; la fenêtre affiche alors accepté, refusé
   ou toujours inconnu. Après un redémarrage de la caisse, un paiement
   envoyé et sans réponse devient lui aussi "inconnu" et se vérifie de
   la même façon.
5. Un paiement accepté **doit être utilisé par la vente** de sa commande,
   pour son montant exact : la caisse refuse de signer la vente autrement
   et le reste à payer se prend par un autre moyen (espèces,
   chèques-repas, ou une autre carte).
6. Si le terminal est injoignable et que rien n'a été débité, l'opérateur
   peut aussi saisir la carte à la main, comme sans terminal relié
   (choix "Saisie manuelle" pour la part carte, paiement tapé sur le
   terminal, puis "Valider") :
   la ligne est alors `MANUAL`. Ce repli est refusé tant qu'un paiement de
   cette commande attend sa réponse ou qu'un paiement accepté n'est pas
   encore utilisé par la vente.

Tant qu'un paiement accepté n'est pas enregistré dans la vente de sa
commande, la table ou la vente à emporter porte la pastille "Carte
encaissée", et la clôture Z (§10.2) le signale ; la commande ne peut
alors pas descendre sous le montant déjà débité.

Une carte payée sur le terminal relié n'est jamais arrondie. En mode
formation (§13), aucune vraie carte n'est jamais débitée : la caisse
refuse d'envoyer le montant au terminal, la carte se saisit à la main.
Le remboursement reste rendu en espèces (§9) : l'annulation d'un
paiement carte déjà accepté n'est pas prise en charge par la caisse.

**Partager la note en parts égales.** Sous les moyens de paiement, la
ligne "Partager en" propose 1 à 5 "parts égales". Les centimes de reste
vont aux premières parts. Le choix se fait avant le premier paiement. Une
pastille par part ("Part 1", "Part 2"...) indique le moyen et le montant,
avec une coche une fois la part prise. Le bouton devient "Valider part
n" ; chaque part se paie par le moyen de son choix (par exemple une part
en espèces et l'autre par carte). La vente n'est signée qu'une fois la
dernière part validée, sur **un seul ticket**.

**Une seule part en espèces** au plus : une fois qu'une part est payée en
espèces, le bouton "Espèces" est grisé et la caisse rappelle "Une part
est déjà payée en espèces : les autres se paient par carte ou
chèques-repas." Les espèces prennent ce qui reste après les parts payées
par carte ou chèques-repas.

Tant qu'il reste un montant, chaque paiement pris s'affiche avec son
montant et la caisse affiche le "Reste à payer". La vente est signée
dès que le reste est payé. Exemple : 14,86 € payés 11,52 € en
chèques-repas, puis 3,34 € en espèces, encaissés 3,35 € (arrondi
+0,01 €).

"Annuler les paiements" (qui remplace "Retour" dès qu'un paiement est
pris) efface les paiements pris tant que la vente n'est pas signée (rien
n'est encore enregistré ; rembourser au client ce qui a déjà été passé
sur le terminal).

Le ticket liste chaque moyen de paiement et son montant, puis
l'arrondi des espèces.

"Communication FDM..." s'affiche pendant la signature. La vente n'est
enregistrée qu'une fois signée par le FDM.

### 7.3 Fin de la vente

L'écran "OK" rappelle le type de ticket, le total, le rendu éventuel et
l'identifiant du FDM, puis ce qui est arrivé au ticket :

- "Ticket imprimé" ;
- "Ticket envoyé à j\*\*\*@e\*\*\*.be" (adresse masquée) ;
- "E-mail en attente, nouvel essai automatique" : l'envoi n'a pas abouti
  tout de suite, la caisse réessaie seule. Si le client avait choisi
  "E-mail" seul et que l'envoi échoue, la caisse imprime le ticket
  papier à la place : le client repart toujours avec son ticket.
- **"Imprimer le ticket"** : ce bouton apparaît sur cet écran de fin
  (de vente, de remboursement ou de copie) quand un ticket papier était
  prévu mais n'a pas pu sortir (ex. papier épuisé). Il est réservé au
  Serveur et au Gérant. La caisse reconstitue le ticket original à partir
  de l'archive du FDM et l'imprime, **une seule fois**. Elle refuse si
  l'original est déjà imprimé ("Le ticket original est déjà imprimé :
  imprimez une copie.") ou si le ticket a été envoyé par e-mail ("Le
  ticket a été envoyé par e-mail : imprimez une copie."). Le bouton
  n'existe que sur cet écran : après "Terminer", il disparaît. Pour toute
  autre reproduction, faire une copie (§8).

Le tiroir-caisse s'ouvre quand une partie est payée en espèces (jamais
pour une carte ou des chèques-repas seuls). La table redevient libre.
"Terminer" revient à la prise de commande.

### 7.4 Si le FDM ne répond pas

Sans FDM, **aucune vente n'est possible**. Un voile rouge couvre les
écrans de vente (tables, commande, paiement) avec le message du FDM et le
bouton "Réessayer maintenant" ; la caisse réessaie aussi seule ("nouvel
essai automatique dans {s} s", de moins en moins souvent, jusqu'à 5
minutes). "Rapports" et "Config" restent accessibles au Gérant. Rien
n'est enregistré, la commande reste ouverte, le tiroir reste fermé.

Vérifier le câble réseau et l'alimentation du FDM, puis réessayer.
Appeler le distributeur si le problème persiste. Ne jamais encaisser en
dehors de la caisse. Les messages du FDM et de la caisse sont réunis
dans la table du §14.1.

Si le message est "La caisse attend la réponse de la boîte noire pour
l'opération précédente. Vérifiez la connexion et réessayez.", la
dernière opération a été envoyée au FDM mais sa réponse n'est pas
revenue. Rien n'est perdu : la caisse la renvoie seule toutes les
minutes, et toute nouvelle opération la renvoie d'abord. Vérifier le
câble et l'alimentation du FDM, attendre une minute, puis réessayer. Ne
pas refaire la vente à la main. Tant que cette réponse est attendue, la
commande concernée ne peut pas être modifiée ("Cette commande attend la
réponse de la boîte noire : elle ne peut pas changer pour l'instant.").
Si le message persiste, appeler le distributeur.

Si le FDM répond autre chose que ce qui lui a été demandé, la caisse
refuse l'opération : "Réponse de la boîte noire incohérente. Vente non
enregistrée. Contactez votre technicien." Rien n'est enregistré.

Le message "La caisse enregistre une autre opération : réessayez dans un
instant." apparaît si la caisse a attendu environ 30 secondes derrière
une autre opération (par exemple la clôture automatique de la
journée) : réessayer.

### 7.5 Client parti sans payer

Réservé au Gérant. Quand un client part sans payer, ce qui a été servi
est tout de même vendu : la commande ne doit pas rester ouverte.

1. Ouvrir la commande (la table, ou l'emporter en cours).
2. Dans le panier, toucher "**Client parti sans payer**".
3. Confirmer avec son PIN ("La vente sera enregistrée sans paiement et
   son ticket imprimé. Confirmez avec votre PIN.").

Le FDM signe la vente (événement N) sans aucun paiement. Le ticket de
caisse TVA est imprimé, avec "**Non paye**" à la place des paiements.
Le tiroir ne s'ouvre pas et aucun paiement n'est compté dans les
rapports ; le chiffre d'affaires, lui, compte la vente. La table
redevient libre ("Vente clôturée sans paiement").

Une vente sans paiement ne se rembourse pas (rien n'a été encaissé : "Cette
vente n'a pas été payée : rien à rembourser.") et n'apparaît pas dans la
liste des remboursements. Sa copie (§8) est possible. Un PIN erroné est
refusé ("PIN incorrect") et rien n'est enregistré ; une commande à
0,00 € s'encaisse normalement ("Rien à payer : encaissez la vente
normalement."). En mode formation, la vente est une vente de formation
(T).

## 8. Copie de ticket (événement C)

Réservé au Gérant. Pour un client qui a perdu son ticket ou son e-mail,
ou qui demande un double :

1. "Rapports", onglet "Tickets". La liste "Tickets récents" montre les
   derniers tickets remis à un client : ventes, remboursements,
   additions et tickets de formation, avec leur numéro, leur type ("Vente",
   "Remboursement", "Addition", "Formation"), la table, l'heure et le
   montant ("Choisissez un ticket pour en faire une copie ou le
   rembourser.").
2. Toucher "Copie" sur la ligne du ticket, choisir le support (Papier /
   E-mail / Les deux, §7.1) puis "Valider". "Retour" annule.
3. "Communication FDM..." s'affiche, puis l'écran de résultat ; "Terminer"
   ferme la fenêtre.

La copie est signée par le FDM (événement C) et marquée "**COPIE**" et
"**CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE**". Un e-mail de copie
ne contient ni lien ni QR code : le FDM n'en donne pas pour une copie.
C'est la **seule** façon de reproduire un ticket déjà remis : la caisse
n'imprime jamais un original deux fois.

Un rapport Z se recopie depuis "Réimprimer un Z" (§10.3).

## 9. Remboursement

Réservé au Gérant. Un remboursement annule tout ou partie d'une vente
déjà signée ; c'est la seule façon d'annuler une vente après coup.

1. "Rapports", onglet "Tickets". Toucher "Rembourser" sur la ligne de la
   vente à rembourser (seules les ventes ont ce bouton).
2. "Articles à rembourser" : la fenêtre liste chaque **ligne du ticket
   d'origine**. Toucher chaque ligne concernée, et ajuster la quantité
   avec "-" / "+" (jamais plus que la quantité vendue). Tout cocher
   rembourse le ticket entier ; une partie seulement fait un
   remboursement partiel. Le bouton "Rembourser" affiche le montant à
   rendre en espèces, arrondi aux 5 centimes ("Rembourser 3,05" pour un
   Coca Cola de 3,04).
3. Étape du support du ticket (§7.1) : choisir Papier / E-mail / Les
   deux, puis "Rembourser" avec le montant de nouveau. "Retour" revient
   à la sélection des articles.
4. "Communication FDM...", puis le résultat ; "Terminer" ferme la
   fenêtre.

Le FDM signe un nouveau ticket (événement N, quantités négatives,
référence au ticket d'origine) : un ticket de caisse TVA marqué
"REMBOURSEMENT", qui imprime les données de contrôle du ticket
d'origine sous « Informations FDM d'origine ». Le ticket d'origine
n'est jamais modifié. Le tiroir s'ouvre pour rendre les espèces.

Le remboursement est toujours rendu en espèces, quel que soit le moyen
de paiement de la vente (carte, chèques-repas, plusieurs moyens). Comme
pour une vente en espèces, le montant rendu est arrondi aux 5 centimes :
le ticket imprime le montant exact puis une ligne "Arrondi" (3,04
remboursés : "Espèces -3,04", "Arrondi -0,01", 3,05 rendus).

La caisse refuse :

- de rembourser un remboursement ("Un remboursement ne se rembourse
  pas.") ;
- de rembourser plus que ce qui a été vendu ("Produit {produit} :
  {demandé} demandé(s), {vendu} vendu(s), {remboursé} déjà
  remboursé(s).") ;
- de rembourser une vente sans paiement (§7.5) ;
- de rembourser une vente réelle en mode formation ("Une vente réelle
  ne se rembourse pas en mode formation.") ;
- un produit vendu à plusieurs prix sur le même ticket sans que la
  ligne soit choisie : "Le produit {produit} a été vendu à plusieurs
  prix : choisissez la ligne à rembourser." (choisir la ligne exacte
  dans la liste) ;
- une ligne vendue avec une remise (commande en ligne) : "Ligne vendue
  avec une remise : remboursez la commande entière." ;
- de valider sans article choisi ("Choisissez au moins un article à
  rembourser.").

Si le FDM refuse, rien n'est enregistré et la vente reste remboursable.

## 9 bis. Corriger un moyen de paiement (événement F)

Quand une vente a été encaissée avec le mauvais moyen (par exemple en
espèces alors que le client a payé par carte), le Gérant corrige la
ligne de paiement sans rembourser.

1. Rapports > « Tickets ».
2. Sur la vente, touchez « Corriger le paiement ».
3. Choisissez la ligne de paiement à corriger (une carte prise sur le
   terminal de paiement lié porte la mention « Terminal » et ne se
   choisit pas), puis le moyen réel : « Espèces », « Carte » ou
   « Titres-repas ».
4. Le résumé s'affiche (par exemple « Espèces 52,00 € → Carte
   52,00 € ») ; touchez « Corriger » et saisissez votre PIN.

La caisse signe un événement F qui référence la vente. Aucun ticket ne
s'imprime et le tiroir ne s'ouvre pas ; « Paiement corrigé » confirme et
la vente porte désormais la mention « Paiement corrigé : Espèces →
Carte ». Les rapports X et Z comptent la correction dans les moyens de
paiement (corrections négatives et positives) et le Z imprimé ajoute une
ligne « Corrections de paiement ».

Refus (le texte s'affiche dans la fenêtre) :

- seule une vente peut être corrigée (ni remboursement, ni vente non
  payée, ni commande en ligne) ;
- la vente doit appartenir à la journée en cours (pas encore clôturée
  par un Z) ;
- une vente déjà remboursée, en tout ou en partie, ne se corrige plus ;
- **une seule correction par vente** ;
- une carte prise sur le terminal de paiement lié ne se corrige pas ;
- le nouveau moyen doit différer de celui de la ligne ;
- une correction qui demanderait un arrondi en espèces est refusée.

Si le FDM ne répond pas, rien n'est enregistré et la vente reste
corrigeable. Le PIN est celui du Gérant connecté.

## 10. Rapports X et Z (événements R)

Réservé au Gérant : "Rapports". L'écran a trois onglets : "Journée",
"Caisse" (§11) et "Tickets" (§8, §9).

### 10.1 Rapports X

Onglet "Journée" :

- "Rapport X" ("Résumé en cours de journée") : chiffre d'affaires, TVA
  et paiements depuis le dernier Z.
- "Rapport utilisateurs X" ("Ventes et pointages par employé") : les
  mêmes chiffres par employé, avec les pointages.

Un rapport X ne clôture rien : il peut être tiré à tout moment, autant
de fois que nécessaire. Il est signé par le FDM et imprimé ; "Rapport
généré (X)" confirme.

### 10.2 Rapport Z (clôture de la journée)

Onglet "Journée", en bas de l'écran ("La clôture remet les compteurs à
zéro. Le premier appui arme le bouton, le second dans les 5 secondes
confirme.") :

1. "Clôturer la journée (Z)". Le bouton passe à l'orange : "Confirmer la
   clôture ?".
2. Toucher à nouveau dans les 5 secondes pour clôturer. Sans second
   appui, ou en changeant d'onglet, le bouton revient à son état normal
   et rien ne se passe.

Si un paiement par carte a été encaissé sur le terminal relié mais pas
encore enregistré dans une vente, la caisse l'indique au-dessus du bouton
("Paiements par carte encaissés mais pas encore enregistrés dans une
vente : ...") : le Z reste possible, ce paiement sera enregistré avec la
vente de sa commande (capture `15a`). Sur l'écran des tables et la liste
des ventes à emporter, la commande concernée porte la pastille "Carte
encaissée" (capture `03a`).

Le Z est signé par le FDM, imprimé, et "Rapport généré (Z #n)"
confirme. La journée est close ; la suivante commence au prochain
enregistrement. Terminer **chaque** journée par un Z.

Une journée restée ouverte au-delà de l'heure de fin de journée du
terminal (5 h par défaut) est clôturée par un Z automatique. Tant que
ce Z automatique n'est pas passé, la caisse répond "La journée
précédente n'est pas clôturée : son rapport Z automatique est en cours.
Réessayez dans une minute." Un enregistrement dont la journée est déjà
clôturée est refusé ("La journée de cet enregistrement est déjà
clôturée : rien n'a été enregistré.").

### 10.3 Réimprimer un Z

La carte "Réimprimer un Z" indique le dernier Z ("Dernier : Z #n ·
date"). La toucher ouvre "Historique Z" (numéro et date de chaque Z
passé). "Réimprimer" en produit une copie (événement C, §8) : choisir le
support, "Valider".

## 11. Mouvements de caisse, tiroir et comptage (événements F)

Réservé au Gérant : "Rapports", onglet "Caisse".

### 11.1 Entrée ou sortie d'espèces

L'écran suit trois étapes, puis un "Récapitulatif" :

1. "1 · Sens" : "Entrée" ou "Sortie".
2. "2 · Motif" : les motifs dépendent du sens. En "Entrée" : "Fond de
   caisse" ou "Pourboire". En "Sortie" : "Retrait" ou "Pourboire".
3. "3 · Montant" : "10 €", "20 €", "50 €", "100 €", ou "Autre" pour taper
   un autre montant ("Autre montant").

Le bouton "Enregistrer l'entrée" (ou "Enregistrer la sortie") reste
grisé tant qu'aucun montant n'est choisi. Le mouvement
est signé par le FDM (événement F) ; "Mouvement enregistré" confirme.

### 11.2 Ouvrir le tiroir hors vente

"Ouvrir le tiroir" ouvre le tiroir en dehors d'une vente. Chaque
ouverture est signée par le FDM (événement F) et figure dans les
rapports ("Tiroir ouvert"). En dehors de ce bouton, le tiroir ne s'ouvre
que pour un encaissement ou un remboursement en espèces.

### 11.3 Déclarer le contenu du tiroir (comptage)

"Déclarer le tiroir" enregistre le montant d'espèces compté dans le
tiroir, par exemple en fin de service. La fenêtre "Déclarer le contenu
du tiroir" ("Montant compté dans le tiroir") demande le montant ; taper
0 pour un tiroir vide, puis "Valider". La caisse fait signer la
déclaration par le FDM (événement F, cas d'usage M133) : "Contenu du
tiroir déclaré" confirme. La déclaration ne modifie aucun solde : elle
consigne ce que le Gérant a compté.

## 12. Configuration (Gérant)

Le bouton "Config" de l'en-tête ouvre la page Paramètres.

### 12.1 Mode formation

Voir §13.

### 12.2 Langue

Français, English ou 中文 : la langue de l'interface et des noms de
produits à l'écran ("Les noms des produits restent bilingues."). Les
mentions imprimées sur les tickets suivent la langue de l'établissement
fixée à l'installation, pas ce choix.

### 12.3 Employés

La liste montre chaque employé (nom, profil, "PIN défini" ou "sans
PIN") avec deux boutons :

- **"Code PIN"** : fixer ou changer son PIN (§2.2).
- **"Désactiver"** : pour un employé qui quitte l'établissement. Toucher
  deux fois ("Confirmer ?"). Son historique reste dans l'archive ; il
  ne peut plus se connecter. On ne peut pas se désactiver soi-même.

"Ajouter" : taper le nom et l'INSZ (11 chiffres), choisir le profil
(Gérant, Serveur ou Apprenti), puis "Ajouter l’employé". Pour un Serveur
ou un Apprenti, demander ensuite à l'employé de fixer son PIN (§2.2).
Pour un **nouveau Gérant**, la caisse demande d'abord le PIN du nouveau
Gérant (deux fois), puis "Confirmez avec votre PIN de gérant" : le
Gérant qui le crée tape son propre PIN. Le nouveau Gérant a donc son PIN
dès sa création.

### 12.4 Informations

Identifiant du POS ("POS ID"), "Terminal", "Restaurant", "FDM" (adresse)
et "Imprimante", tels que configurés à l'installation (lecture seule).

Si cette configuration contient une erreur que le FDM refuserait (ex.
un numéro de TVA mal formé), un **bandeau rouge** s'affiche en bas de
l'écran : "Configuration invalide : le FDM refusera les tickets.
Appelez l'installateur." suivi de la liste des champs en cause. Chaque ligne dit en français (ou dans la langue de l'écran) quel réglage est en cause, par exemple « Numéro de TVA du restaurant invalide : BE suivi de 10 chiffres (par exemple BE0499999960). », puis, après « Détail : », le texte technique anglais pour l'installateur. Il reste
affiché tant que l'erreur n'est pas corrigée.

### 12.5 Tickets par email

Panneau "Tickets par email". Visible dans tous les cas : il indique si
le service d'envoi est "Configuré" ou "Non configuré", puis "Joignable"
ou "Injoignable", et liste les derniers envois : adresse **masquée**
(ex. `j***@e***.be`), statut ("En attente", "Envoi…", "Envoyé", "Échec",
"Imprimé (secours)", "Purgé"), nombre de tentatives ("Tentatives") et
dernière erreur. "Actualiser" recharge la liste.

L'adresse complète n'est jamais affichée ; elle est effacée dès l'envoi
réussi, et au plus tard après 72 h (`TICKET_NUMERIQUE_FR.md`).

### 12.6 Assistance à distance

Quand le technicien doit voir la caisse à distance :

1. "Config", panneau "Assistance à distance", bouton "Démarrer".
2. Taper son PIN ("Saisir le PIN"). Un PIN erroné affiche "PIN
   incorrect" et rien ne démarre.
3. RustDesk s'ouvre et affiche un identifiant et un mot de passe : les
   lire au technicien par téléphone.

Tant que la session dure, un bandeau "Assistance à distance en cours
(démarrée par <nom>)" s'affiche en bas de chaque écran, pour tout le
personnel. Le panneau indique depuis quand elle dure. Quand le
technicien a terminé, toucher "Arrêter la session". La session
continue après un verrouillage : l'écran de connexion garde le bandeau,
avec un bouton "Arrêter la session" que tout le monde peut toucher.
Elle s'arrête d'elle-même 2 heures après le démarrage. Une seule
session à la fois ; si RustDesk n'est pas installé, le panneau
l'indique ("Non installée sur cette caisse"). Cette session sert à
voir l'écran et à guider le Gérant : elle ne sert jamais à installer
un programme sur la caisse.

### 12.7 Intégrations et HubRise

Panneau "Intégrations". Les commandes des plateformes de livraison et
du webshop ne sont reçues que si HubRise est activé sur cette caisse
(désactivé à l'installation, sauf demande du client). HubRise est
inclus dans le programme certifié : seul l'interrupteur décide s'il
fonctionne.

- **"Activer" / "Désactiver"** : réservé au Gérant. Chaque changement
  est inscrit dans "Historique" (date, gérant ou fichier de
  configuration) et ne peut pas être effacé.
- Tant qu'une commande en ligne n'est pas terminée, "Désactiver" est
  refusé et le panneau indique "<n> commande(s) en ligne à terminer".

Une fois HubRise activé, le panneau "HubRise" apparaît dans "Config" :

- **Connexion** : "Se connecter à HubRise". La caisse n'a pas de
  navigateur : elle affiche un QR code. Le scanner avec un téléphone, se
  connecter à HubRise, choisir l'établissement, puis taper le code
  affiché ("Code d'autorisation") et toucher "Valider la connexion". Le
  panneau indique ensuite "Connecté", le compte, l'établissement et le
  catalogue ; "Déconnecter" met fin à la connexion.
- **Catalogue** : la date de la dernière synchronisation ("Synchronisé
  …") et "Resynchroniser". Si HubRise refuse un catalogue, "Catalogue
  refusé : le menu précédent reste en place." et la liste des anomalies
  s'affichent. À la première connexion, "Reprise du menu" permet au Gérant
  de cocher les produits locaux qui gardent leur historique, puis
  "Reprendre le menu" ; les autres sont désactivés.
- **Menu géré par HubRise** : une fois le catalogue repris, le bandeau
  "Menu géré par HubRise : les produits se modifient dans HubRise."
  s'affiche. Les produits se modifient dans HubRise, pas à la caisse.
- **Canaux** : un canal par plateforme ou par webshop (Plateforme ou
  Webshop, "Frais inclus" ou "Frais exclus"), avec "Ajouter un canal".
  Une commande d'un canal inconnu reste bloquée jusqu'à ce que le canal
  soit enregistré.
- Le panneau signale aussi une connexion refusée ("HubRise refuse la
  connexion : reconnectez-vous."), des envois en attente et des messages
  non traités.

### 12.8 Commandes en ligne

Le bouton "En ligne" de l'en-tête (seulement si HubRise est activé)
ouvre le tableau des commandes reçues, en trois colonnes : "Nouvelles",
"En cuisine" et "Prêtes". Un filtre "Terminées" montre les commandes
closes. Une alarme s'affiche quand des commandes attendent depuis plus
de 5 minutes. Chaque carte porte l'heure, le canal, le type de service
("Livraison", "À emporter", "Sur place"), les articles et le total, et
l'étiquette "Payée" ou "À encaisser".

L'avancement d'une commande suit ces boutons :

- **"Accepter"** (Serveur ou Gérant) : une commande déjà payée sur la
  plateforme ou le webshop est vendue (événement N) et son ticket de
  caisse TVA imprimé ; une commande "À encaisser" est enregistrée
  (événement P) et le bon part en cuisine. **"Refuser"** : la commande
  est refusée, rien n'est signé.
- **"Préparer"** (la commande passe en "En cuisine"), puis **"Prête"**
  (colonne "Prêtes"), pour tous les profils.
- **"Retarder"** (commande acceptée ou en préparation) : les boutons
  +15, +30 et +45 repoussent l'heure promise de ce nombre de minutes
  (message "Délai appliqué").
- Pour une livraison du webshop, **"En livraison"** précède
  **"Terminer"** (commande déjà payée). Pour une commande "À encaisser",
  la remise se fait par **"Encaisser espèces"** ou **"Encaisser carte"** :
  vente (événement N) et ticket, comme au §7. Avec un terminal de
  paiement relié, "Encaisser carte" envoie le montant au terminal et suit
  les mêmes étapes qu'au §7.2 ("Carte sur le terminal relié").
- **"Réimprimer"** ouvre "Ticket cuisine" et "Ticket client" pour
  réimprimer les bons ("Ticket envoyé à l'imprimante").
- **"À vérifier"** : la commande ne correspond pas au menu (produit
  absent, TVA différente, total incohérent...) ; elle ne peut pas être
  acceptée. "Resynchroniser" relit le menu, sinon la refuser.
- **"Rembourser"** (Gérant) : une commande annulée par la plateforme
  après la vente ; le remboursement cite la vente d'origine, le tiroir
  reste fermé (la plateforme rembourse le client).
- **"Clôturer"** : une commande à encaisser annulée après son
  enregistrement ; ticket à 0,00 EUR.

Si le FDM ne répond pas, rien n'est signé et la commande reste à
accepter (§7.4). Une commande en ligne ne se prend pas en mode formation
("Pas de commande en ligne en mode formation."). Le détail fiscal est
dans `COMMANDES_EN_LIGNE_FR.md`.

### 12.9 Plan de salle

Panneau "Plan de salle", réservé au Gérant. Il indique le plan en
vigueur ("20 tables : 1-20"). Pour le changer, taper les numéros de
table séparés par des virgules, avec des plages si l'on veut (ex.
"1-12, 20-25"), de 1 à 999, 200 tables au plus, puis **"Enregistrer"**
("Plan de salle enregistré"). Un plan mal écrit est refusé avec le
message qui indique le format.

- Le plan de départ vient de la configuration de l'installation (clés
  `tables` ou `table_count`). Le dernier changement l'emporte ; chaque
  changement est inscrit dans un historique qui ne peut pas être effacé.
- Une table retirée du plan qui a encore une commande ouverte reste
  affichée sur l'écran "Tables" (§5.1) jusqu'à son règlement.
- Le plan ne change ni les commandes déjà prises, ni les tickets, ni ce
  qui est signé par le FDM.

## 13. Mode formation (événements T)

Pour former un employé sans toucher aux ventes réelles. Seul un Gérant
l'active. Il le fait pour toute la caisse : tant qu'il est actif, un
Apprenti prend des commandes comme d'habitude mais ne peut pas
encaisser (§1), et tout ce qu'un Serveur ou un Gérant encaisse est un
événement de formation.

1. Encaisser ou vider les commandes ouvertes : sinon la caisse refuse
   ("Encaissez ou videz les commandes ouvertes avant de changer de
   mode"), pour qu'une vraie table ne soit jamais encaissée en
   formation, ni l'inverse.
2. "Config", panneau "Mode Formation" ("Fond violet, bandeau et filigrane
   sur tous les écrans. Les tickets sont marqués TRAINING.") : le
   bouton passe de "Inactif" à "Actif".
3. Un bandeau violet "MODE FORMATION · Les transactions ne sont PAS
   enregistrées" et un filigrane "TRAINING" restent affichés.

En formation, tout ce qui est signé est un événement **T** : compteur à
part, tickets marqués "**TRAINING**" et "**CECI N'EST PAS UN TICKET DE
CAISSE T.V.A. VALABLE**", sans effet sur le chiffre d'affaires des
rapports. Un remboursement de vente réelle est refusé en formation.

Pour revenir au mode normal : même bouton, qui repasse à "Inactif"
(mêmes conditions).

Le réglage du bouton vaut jusqu'au redémarrage de la caisse : au
démarrage, le mode est celui du fichier de configuration (`false` en
production, `MANUEL_INSTALLATION_FR.md` §4.2).

## 14. Que faire en cas de problème

### 14.1 Messages de la boîte noire (FDM) et de la caisse

Les messages ci-dessous s'affichent dans le bandeau sous l'en-tête ou, pour
les erreurs bloquantes, dans le voile rouge qui couvre les écrans de
vente. Un avertissement du FDM s'affiche dans un bandeau orange, qui se
referme seul au bout de 10 secondes ("se ferme dans {s} s"), sauf s'il
est obligatoire : il reste alors tant que le FDM le signale. Pour une
erreur bloquante, "Réessayer maintenant" relance une vérification
immédiate ; la caisse réessaie aussi seule. Les écrans correspondants
sont les captures 20 à 42 de `CAPTURES_ECRAN_FR.pdf`.

| Code | Message affiché | Effet | Que faire |
|---|---|---|---|
| `BUFFER_FULL` | La boîte noire est pleine. Contactez votre technicien. | Bloquant : plus de vente | Appeler le distributeur |
| `FDM_LOCKED` | La boîte noire est verrouillée. Contactez votre technicien. | Bloquant | Appeler le distributeur |
| `FDM_NOT_OPERATIONAL` | La boîte noire ne fonctionne pas. Impossible d'enregistrer les ventes. | Bloquant | Vérifier câble et alimentation, puis appeler le distributeur |
| `UNKNOWN_POS` | Cette caisse n'est pas reconnue par la boîte noire. | Bloquant | Appeler le distributeur |
| `BUFFER_NEAR_FULL` | La boîte noire est presque pleine. Programmez une intervention. | Avertissement obligatoire | Prévenir le distributeur |
| `CLIENT_CERT_EXPIRED` | Le certificat de la boîte noire a expiré. Contactez votre technicien. | Avertissement obligatoire | Appeler le distributeur |
| `UPDATE_POS_VATNO` | Le numéro de TVA de la caisse ne correspond pas à celui de la boîte noire. Contactez votre technicien. | Avertissement obligatoire | Appeler l'installateur |
| `UPDATE_POS_ESTNO` | Le numéro d'unité d'établissement de la caisse ne correspond pas à celui de la boîte noire. Contactez votre technicien. | Avertissement obligatoire | Appeler l'installateur |
| `CLIENT_CERT_NEAR_EXPIRATION` | Le certificat de la boîte noire expire bientôt. | Avertissement | Prévenir le distributeur |
| `UNAUTHORIZED` | La caisse n'est pas autorisée à parler à la boîte noire. | Avertissement | Appeler le distributeur |
| `TRANSACTION_UPLOAD_FAILED` | La boîte noire échoue à transmettre les transactions au SPF Finances. | Avertissement | Vérifier la connexion Internet du FDM |
| `FDM_ANSWER_PENDING` | La caisse attend la réponse de la boîte noire pour l'opération précédente. Vérifiez la connexion et réessayez. | Opération refusée, renvoi automatique | Attendre une minute, réessayer (§7.4) |
| `FDM_RESPONSE_MISMATCH` | Réponse de la boîte noire incohérente. Vente non enregistrée. Contactez votre technicien. La vente n'est pas encore enregistrée : le prochain essai envoie à nouveau la même demande à la boîte noire. | Opération en attente, renvoi à l'identique | Appeler le distributeur |
| `INTERNAL_ERROR`, `UNDEFINED_ERROR`, `UNDEFINED_OTHER` (sur une vente) | Le message de la boîte noire (« Erreur technique de la boîte noire. Contactez votre technicien. », « Erreur indéfinie de la boîte noire. Contactez votre technicien. », « Erreur de la boîte noire. Contactez votre technicien. »), suivi de : La vente n'est pas encore enregistrée : le prochain essai envoie à nouveau la même demande à la boîte noire. | Opération en attente, renvoi à l'identique | Réessayer ; si cela persiste, appeler le distributeur |
| `FDM_NOT_INITIALIZED` | La boîte noire n'est pas encore initialisée. | Voyant orange | Appeler le distributeur |
| `FDM_CERTIFICATE_REFUSED` | Connexion sécurisée à la boîte noire refusée (certificat). Contactez votre technicien. | Bloquant | Appeler l'installateur |
| (réseau coupé) | Connexion perdue avec la boîte noire. Vérifiez le câble réseau. | Bloquant | Vérifier le câble réseau et l'alimentation du FDM |

Les autres codes du FDM (erreurs de communication, d'horloge, de mise à
jour des certificats ou des tâches...) s'affichent avec un message
analogue ; leur liste complète est dans le manuel de certification, §V.

Messages de la caisse elle-même :

| Message | Cause | Que faire |
|---|---|---|
| La caisse enregistre une autre opération : réessayez dans un instant. | Une autre opération signe (ex. Z automatique) | Réessayer |
| Cette commande attend la réponse de la boîte noire : elle ne peut pas changer pour l'instant. | Réponse du FDM attendue pour cette commande | Attendre, réessayer (§7.4) |
| La journée précédente n'est pas clôturée : son rapport Z automatique est en cours. Réessayez dans une minute. | Une journée restée ouverte est en cours de clôture | Attendre une minute |
| La journée de cet enregistrement est déjà clôturée : rien n'a été enregistré. | Un enregistrement visait une journée déjà close | Refaire l'opération |
| Une autre opération était en cours : rien n'a été enregistré, réessayez. | Deux opérations simultanées | Réessayer |
| Trop de PIN incorrects : réessayez dans {seconds} s. | Verrou après 5 PIN erronés (§2.2) | Attendre la fin du décompte |

### 14.2 Autres problèmes

| Problème | Que faire |
|---|---|
| Voyant "FDM" orange ou rouge, vente refusée | Lire le message affiché (§14.1). Vérifier le câble réseau et l'alimentation du FDM, puis réessayer. Si le problème persiste, appeler le distributeur. **Ne jamais vendre en dehors de la caisse.** |
| Voyant "Ticket" rouge, ou "Papier épuisé", "Capot ouvert", "Vérifiez l'imprimante" | Vérifier l'imprimante. Plus de papier : charger un rouleau thermique de 80 mm, puis toucher "Imprimer le ticket" sur l'écran de fin de vente (§7.3). Rien n'est réimprimé automatiquement. |
| Voyant "Terminal" rouge | La caisse ne joint pas le terminal de paiement. Vérifier son alimentation et son câble ou son réseau. Tant qu'il ne répond pas, les cartes se prennent à la main (saisie manuelle, §7.2) ; rien n'est débité par cette vérification. |
| Imprimante hors ligne | La vente est quand même enregistrée. Toucher "Imprimer le ticket" une fois l'imprimante revenue (§7.3), ou faire plus tard une copie (§8). |
| "Bon de commande non imprimé : prévenez le bar ou la cuisine." | Vérifier l'imprimante de la cuisine (voyant "Cuisine") ; la commande est enregistrée, prévenir le bar ou la cuisine (§6.1). |
| "Pointage non enregistré" | Le FDM n'a pas répondu à la connexion ou à la fin de service. Prévenir le Gérant. |
| "Commande non enregistrée par le FDM" | Rester sur la commande et réessayer quand le FDM répond (§6.1). |
| "La caisse attend la réponse de la boîte noire pour l'opération précédente" | Vérifier câble et alimentation du FDM, attendre une minute (la caisse renvoie seule), puis réessayer. Ne pas refaire l'opération. Appeler le distributeur si le message persiste (§7.4). |
| Le tiroir ne s'ouvre pas | Vérifier le câble RJ11 entre l'imprimante et le tiroir, et la serrure du tiroir. |
| L'écran ne réagit plus | Maintenir le bouton marche 5 secondes pour redémarrer. pili-pos redémarre seul ; les commandes non encaissées restent ouvertes. |
| "PIN incorrect" répété, ou "Trop de PIN incorrects" | Attendre la fin du décompte (§2.2). Le Gérant peut fixer un nouveau PIN (§12.3). Redémarrer la caisse remet le compteur à zéro. |
| INSZ refusé | Vérifier la saisie. L'employé doit exister et être actif (§12.3). |
| Bandeau rouge "Configuration invalide" | Appeler l'installateur (§12.4). |

En cas de doute : **ne jamais vendre sans le FDM**, et **terminer chaque
journée par un rapport Z**.
