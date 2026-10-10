<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/MANUEL_UTILISATEUR_FR.md @ 86088e4 -->
# Manuel utilisateur - pili-pos-android v1.0.0

**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)

Ce manuel s'adresse au personnel de salle et de caisse qui utilise
`pili-pos-android` au quotidien : connexion, prise de commande,
addition, encaissement, copies, remboursements, rapports, mouvements de
caisse et mode formation, ainsi que les écrans propres au terminal
Android (à propos, export pour le SPF, mise à jour, export nocturne).
Les libellés cités entre guillemets sont ceux de l'écran, en français
(l'interface existe aussi en anglais et en chinois, §12.2). Les écrans
correspondants figurent dans `CAPTURES_ECRAN_FR.pdf` : chaque renvoi
"À effectuer (EV-09 : capture ...)" du manuel désigne une page de ce
fichier, à réaliser sur le terminal D3 Pro émulé.

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
| Imprimer l'addition, encaisser, "Imprimer le ticket" (§7.3) | - | ☑ | ☑ |
| Commandes en ligne : "Accepter", "Refuser", "Terminer", "Encaisser espèces", "Encaisser carte", "Clôturer" | - | ☑ | ☑ |
| Rapports X / Z, copies, remboursements, mouvements de caisse, tiroir, comptage du tiroir | - | - | ☑ |
| Clôturer une vente sans paiement (client parti sans payer, §7.5, avec son PIN) | - | - | ☑ |
| Commandes en ligne : "Rembourser" | - | - | ☑ |
| Mode formation, employés (ajouter, "Désactiver", PIN des autres), HubRise (activer, catalogue, canaux), configuration | - | - | ☑ |
| "Export nocturne", "À propos", "Export pour le SPF", "Mise à jour" (§12.9 à §12.12) | - | - | ☑ |

## 2. Démarrage, connexion et PIN

Le terminal est une caisse dédiée : à l'allumage, il démarre seul sur
pili-pos-android, en plein écran, et ne laisse lancer aucune autre
application. Aucune intervention n'est nécessaire. Pendant le
démarrage, l'écran affiche "Démarrage de la caisse…".

### 2.1 Se connecter

L'écran de connexion est coupé en deux.

- À gauche, le panneau "Démarrage" donne l'état des appareils : "FDM"
  ("connecté", "avertissement" ou "hors ligne"), "Imprimante ticket"
  et "Imprimante cuisine" ("prête", "injoignable" ou "non configuré"),
  "HubRise" (seulement s'il est activé : "connecté" ou "non connecté")
  et "Email tickets" ("joignable", "injoignable" ou "non configuré"). Le
  nom de l'établissement et l'identifiant du terminal sont en bas, avec
  la version du logiciel ("Version pili-pos-android/<numéro>").
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

À effectuer (EV-09 : capture A01 écran de connexion).

En bas à droite des tuiles, trois boutons "Français", "English" et
"中文" changent la langue de l'interface avant la connexion. Le choix
est conservé sur la caisse ; il se règle aussi dans la Configuration
(§12.2).

1. Toucher sa tuile.
2. Si l'employé a un PIN, un pavé "Code PIN" s'affiche à droite : taper
   les 4 chiffres. La connexion part d'elle-même au 4e chiffre
   ("connexion au 4ᵉ chiffre"). Le pavé a une touche pour effacer le
   dernier chiffre et une touche "C" pour tout effacer.
   À effectuer (EV-09 : capture A02 pavé du code PIN).
3. Un PIN erroné affiche "PIN incorrect" et vide le pavé ; "Retour"
   revient aux tuiles.

"Autre personne (INSZ)" permet de se connecter en tapant son numéro
INSZ (11 chiffres) puis "Connexion" ("Retour" revient aux tuiles). Si ce
compte a un PIN, le pavé du PIN s'affiche ensuite, comme depuis la
tuile. À effectuer (EV-09 : capture A03 saisie de l'INSZ).

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
(§2.2, §12.3). À effectuer (EV-09 : capture A04 choix du PIN à la première connexion).

### 2.2 Le PIN

Un Gérant a toujours un PIN. Un Serveur ou un Apprenti peut se connecter
sans PIN, mais alors n'importe qui peut ouvrir sa session en touchant
sa tuile. Tant qu'il n'a pas de PIN, un avertissement s'affiche à sa
connexion : "PIN non configuré : touchez votre nom en haut de l'écran".

- **Son propre PIN** : sur l'écran de commande, toucher son nom dans
  l'en-tête (il porte la mention "PIN"). Taper le nouveau PIN ("Saisir
  le PIN"), puis une seconde fois ("Confirmer le PIN"). S'ils diffèrent,
  "Les PIN ne correspondent pas" s'affiche et rien n'est enregistré.
  Sinon, "PIN modifié". À effectuer (EV-09 : capture A05 changement de son propre PIN).
- **Le PIN d'un autre employé** (Gérant) : bouton "Config", puis le bouton
  "Code PIN" sur la ligne de l'employé (§12.3). Le Gérant tape d'abord
  le nouveau PIN de l'employé (deux fois), puis la caisse demande
  "Confirmez avec votre PIN de gérant" : le Gérant tape son propre PIN.
  Sans ce PIN, rien n'est enregistré.
  À effectuer (EV-09 : capture A06 confirmation par le PIN du Gérant).

Le PIN est enregistré sous forme de hachage (bcrypt), jamais en clair.

**Verrou après des PIN erronés.** Après 5 PIN incorrects de suite pour un
même employé (à la connexion ou lors d'une confirmation par PIN), la
caisse refuse tout essai, même avec le bon PIN, pendant 30 secondes. Elle
affiche "Trop de PIN incorrects : réessayez dans {seconds} s.", avec le
temps restant. Chaque nouvel échec double l'attente, jusqu'à 15 minutes.
Un PIN correct remet le compteur à zéro. Un redémarrage de la caisse le
remet aussi à zéro. À effectuer (EV-09 : capture A07 verrou après des PIN erronés).

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
enregistrée auprès du FDM (§6.1). Le terminal reste en mode caisse
dédiée : verrouiller la session ne donne accès à aucune autre
application ni aux réglages de l'appareil.

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
  À effectuer (EV-09 : capture A08 fin de service).
- "Verrouiller" et le verrouillage automatique ne pointent **pas** le
  départ (§2.3).

Si le FDM ne répond pas, la connexion ou la fin de service se fait quand
même, mais un message rouge l'indique : "Pointage non enregistré", suivi
de la raison. Prévenir le Gérant pour qu'il régularise.
À effectuer (EV-09 : capture A09 message Pointage non enregistré).

## 4. L'écran de commande

```
┌───────────────────────────────────────────────────────────────────┐
│ FDM ● Ticket ● Cuisine ●  Marie · PIN   Table 4   14:05           │
│     [Tables] [À emporter] [Rapports] [Config] [Verrouiller]       │
├───────────────────────────────────────────┬───────────────────────┤
│ [Makis] [Plats chauds] [Boissons] ...     │ Commande · Table 4    │
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

À effectuer (EV-09 : capture A10 écran de commande).

L'en-tête montre en permanence :

- les voyants des appareils : **FDM** (la "boîte noire"), **Ticket**
  (imprimante de tickets), **Cuisine** (imprimante du bar ou de la
  cuisine) et **HubRise** (seulement s'il est activé). Vert : tout va
  bien. Orange ou rouge : un problème. Gris : appareil non configuré. Le
  FDM est interrogé toutes les 30 secondes (moins souvent tant qu'il ne
  répond pas) ; son message en clair s'affiche dans un bandeau sous
  l'en-tête (ex. "Connexion perdue avec la boîte noire. Vérifiez le
  câble réseau."). Les imprimantes sont vérifiées toutes les 30
  secondes ;
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
À effectuer (EV-09 : capture A11 bandeau Rapport Z non effectué aujourd'hui).

## 5. Tables, à emporter et prise de commande

### 5.1 Choisir une table

"Tables" affiche le plan des tables (c'est le premier écran après la
connexion). Une table **libre** porte la
mention "Libre". Une table **occupée** affiche son total, son nombre
d'articles et depuis combien de minutes elle est ouverte ("3 art. · 12
min"), et la mention "Addition" quand l'addition provisoire a été
imprimée. À effectuer (EV-09 : capture A12 plan des tables).

- Toucher une table libre ouvre une nouvelle commande pour cette table.
- Toucher une table occupée reprend sa commande en cours, avec tous ses
  articles dans le panier. Une table n'a jamais qu'une seule commande
  ouverte.

### 5.2 Vente à emporter

"À emporter" (en-tête) ouvre une commande sans table. L'addition (§6.2)
n'existe pas pour une commande à emporter : on encaisse directement.

Une vente à emporter est une vente directe : quitter la commande avant
de l'encaisser n'enregistre rien auprès du FDM (pas d'événement P). La
commande reste en cours, listée sur l'écran "Tables" sous "À emporter en
cours" ("Commencée à 14:05", nombre d'articles, total) ; la toucher la
reprend. "Nouvelle vente à emporter" en ouvre une autre. Elle se termine
par l'encaissement (§7) ou, si le client est parti sans payer, par une
vente sans paiement (§7.5). À effectuer (EV-09 : capture A13 ventes à emporter en cours).

### 5.3 Ajouter, modifier, retirer

- Choisir une catégorie dans la barre du haut, puis toucher un article :
  il s'ajoute au panier (à droite).
- Dans le panier, "+" ajoute une unité et "−" en retire une, sans
  descendre sous 1. Pour retirer une ligne, toucher la croix à sa
  droite. Il n'y a pas de bouton de remise : le POS ne propose pas de
  remise à la saisie (les remises d'une commande en ligne viennent de la
  plateforme, §12.8).
- **Prix libre** : certains articles n'ont pas de prix fixe (ex. "Plat
  du jour"). En les touchant, la fenêtre "Prix libre" demande le
  prix : taper les chiffres comme sur un terminal de paiement
  (2 0 0 0 = 20,00 €), puis "Valider" ("Retour" annule).
  À effectuer (EV-09 : capture A14 fenêtre Prix libre).
- **Options** : quand HubRise est activé (§12.7), un article qui a des
  groupes d'options ouvre la fenêtre "Options". Chaque groupe indique sa
  règle : "Choisir {min}", "{min} à {max}", "Jusqu'à {max}" ou
  "Facultatif". Un groupe à un seul choix remplace la sélection
  précédente ; les autres se cochent. "Ajouter" (avec le prix) reste
  grisé tant qu'un minimum n'est pas atteint ; un maximum ne peut pas
  être dépassé. Le prix des options s'ajoute à celui de l'article et
  figure dans le nom de la ligne. Sans option à choisir, l'article
  s'ajoute directement. À effectuer (EV-09 : capture A15 fenêtre Options).

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
confirme. À effectuer (EV-09 : capture A16 panier avec Pas encore en cuisine et Envoyer en cuisine).

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
À effectuer (EV-09 : capture A17 addition provisoire imprimée).

## 7. Encaissement (vente, événement N)

"PAYER" (Serveur et Gérant) ouvre l'écran de paiement, qui affiche le
total à payer. Les trois moyens de paiement ("Espèces", "Carte",
"Chèques-repas") sont côte à côte, l'espèces étant choisie d'office.
À effectuer (EV-09 : capture A18 écran de paiement).

### 7.1 Choix du support du ticket (si activé sur ce terminal)

Quand le ticket numérique est configuré (`MANUEL_INSTALLATION_FR.md`
§4.2), trois boutons apparaissent sur la ligne "Ticket" :

- **Papier** (choix par défaut).
- **E-mail** : le ticket part par e-mail seulement. Ce bouton n'apparaît
  que si l'installateur l'a autorisé et que le service d'envoi répond.
- **Les deux** : le papier sort, et l'e-mail part en plus.

À effectuer (EV-09 : capture A19 choix du support du ticket).

"E-mail" ou "Les deux" ouvre un clavier ("Adresse e-mail du client")
avec des raccourcis (`.be`, `.com`, `@gmail.com`). Une adresse mal
formée est signalée ("Adresse invalide"). À effectuer (EV-09 : capture A20 clavier Adresse e-mail du client).
Un écran "Confirmer
l'adresse" l'affiche ensuite en grand : la faire relire au client si
possible, puis "Confirmer l'adresse" ("Modifier" pour corriger,
"Annuler" pour revenir au papier). À effectuer (EV-09 : capture A21 écran Confirmer l'adresse).

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
  payer"). Encaisser ce montant sur le terminal de paiement ("Encaisser
  sur le terminal"), qui n'est pas relié à la caisse, puis "Valider" une
  fois le paiement accepté. Pas d'arrondi pour une carte.
- **Chèques-repas** (chèques-repas électroniques, payés sur le terminal
  de paiement) : comme la carte, pour le montant payé en chèques-repas.
  Pas d'arrondi, pas de rendu sur des chèques-repas.

**Partager la note en parts égales.** Sous les moyens de paiement, la
ligne "Partager en" propose 1 à 5 "parts égales". Les centimes de reste
vont aux premières parts. Le choix se fait avant le premier paiement. Une
pastille par part ("Part 1", "Part 2"...) indique le moyen et le montant,
avec une coche une fois la part prise. Le bouton devient "Valider part
n" ; chaque part se paie par le moyen de son choix (par exemple une part
en espèces et l'autre par carte). La vente n'est signée qu'une fois la
dernière part validée, sur **un seul ticket**.
À effectuer (EV-09 : capture A22 paiement partagé en parts).

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

- "Ticket imprimé" ; écran : À effectuer (EV-09 : capture A23 fin de vente, ticket imprimé) ;
- "Ticket envoyé à j\*\*\*@e\*\*\*.be" (adresse masquée) ; écran :
  À effectuer (EV-09 : capture A24 fin de vente, ticket envoyé par e-mail) ;
- "E-mail en attente, nouvel essai automatique" : l'envoi n'a pas abouti
  tout de suite, la caisse réessaie seule. Si le client avait choisi
  "E-mail" seul et que l'envoi échoue, la caisse imprime le ticket
  papier à la place : le client repart toujours avec son ticket. Écran
  du repli : À effectuer (EV-09 : capture A25 fin de vente, repli sur le ticket papier).
- **"Imprimer le ticket"** : ce bouton apparaît sur cet écran de fin
  (de vente, de remboursement ou de copie) quand un ticket papier était
  prévu mais n'a pas pu sortir (ex. papier épuisé). Il est réservé au
  Serveur et au Gérant. La caisse reconstitue le ticket original à partir
  de l'archive du FDM et l'imprime, **une seule fois**. Elle refuse si
  l'original est déjà imprimé ("Le ticket original est déjà imprimé :
  imprimez une copie.") ou si le ticket a été envoyé par e-mail ("Le
  ticket a été envoyé par e-mail : imprimez une copie."). Le bouton
  n'existe que sur cet écran : après "Terminer", il disparaît. Pour toute
  autre reproduction, faire une copie (§8). Écran :
  À effectuer (EV-09 : capture A26 fin de vente avec le bouton Imprimer le ticket).

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
À effectuer (EV-09 : capture A27 voile rouge, FDM indisponible).

Vérifier le câble réseau et l'alimentation du FDM, que la caisse reste
connectée au réseau de l'établissement, puis réessayer.
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
   À effectuer (EV-09 : capture A28 confirmation par PIN de la vente sans paiement).

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
   rembourser."). À effectuer (EV-09 : capture A29 onglet Tickets).
2. Toucher "Copie" sur la ligne du ticket, choisir le support (Papier /
   E-mail / Les deux, §7.1) puis "Valider". "Retour" annule.
   À effectuer (EV-09 : capture A30 copie, choix du support).
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
   rendre en espèces, arrondi aux 5 centimes ("Rembourser 3,05 €" pour
   un Coca Cola de 3,04) ; le total est repris sous "À rembourser en
   espèces". Si le montant d'une ligne ne peut pas être calculé à
   l'écran, la fenêtre indique "Montant calculé par le moteur à la
   signature." et le montant exact figure sur le ticket.
   À effectuer (EV-09 : capture A31 sélection des articles à rembourser).
3. Étape du support du ticket (§7.1) : choisir Papier / E-mail / Les
   deux, puis "Rembourser" avec le montant de nouveau. "Retour" revient
   à la sélection des articles. À effectuer (EV-09 : capture A32 support du ticket de remboursement).
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
  dans la liste) ; écran :
  À effectuer (EV-09 : capture A33 remboursement d'un produit vendu à plusieurs prix) ;
- une ligne vendue avec une remise (commande en ligne) : "Ligne vendue
  avec une remise : remboursez la commande entière." ;
- de valider sans article choisi ("Choisissez au moins un article à
  rembourser.").

Si le FDM refuse, rien n'est enregistré et la vente reste remboursable.

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
généré (X)" confirme. Le résultat du dernier rapport reste affiché dans
un cadre sous les boutons (le texte de l'erreur s'y affiche en cas de
refus). À effectuer (EV-09 : capture A34 onglet Journée, rapports X).

### 10.2 Rapport Z (clôture de la journée)

Onglet "Journée", en bas de l'écran ("La clôture remet les compteurs à
zéro. Le premier appui arme le bouton, le second dans les 5 secondes
confirme.") :

1. "Clôturer la journée (Z)". Le bouton affiche alors "Confirmer la
   clôture ?". À effectuer (EV-09 : capture A35 bouton Z armé).
2. Toucher à nouveau dans les 5 secondes pour clôturer. Sans second
   appui, ou en changeant d'onglet, le bouton revient à son état normal
   et rien ne se passe.

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
support, "Valider". À effectuer (EV-09 : capture A36 Historique Z).

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
À effectuer (EV-09 : capture A37 onglet Caisse, mouvement d'espèces).

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
consigne ce que le Gérant a compté. À effectuer (EV-09 : capture A38 déclaration du contenu du tiroir).

## 12. Configuration (Gérant)

Le bouton "Config" de l'en-tête ouvre la page Paramètres. Elle regroupe
le mode formation, la langue, les employés, les informations, les
intégrations et les envois par e-mail ; sous "Autres réglages", elle
liste les écrans propres au terminal : "Export nocturne", "À propos",
"Export pour le SPF" et "Mise à jour" (§12.9 à §12.12).
À effectuer (EV-09 : capture A39 page Paramètres).

### 12.1 Mode formation

Voir §13.

### 12.2 Langue

Français, English ou 中文 : la langue de l'interface et des noms de
produits à l'écran ("Les noms des produits restent bilingues."). Les
mentions imprimées sur les tickets suivent la langue de l'établissement
fixée à l'installation, pas ce choix.

### 12.3 Employés

La liste montre chaque employé (nom, profil, "PIN défini" ou "sans
PIN") avec deux boutons : À effectuer (EV-09 : capture A40 liste des employés).

- **"Code PIN"** : fixer ou changer son PIN (§2.2).
- **"Désactiver"** : pour un employé qui quitte l'établissement. Une
  fenêtre demande confirmation ("Désactiver {nom} ? Cet employé ne
  pourra plus se connecter.") : "Désactiver" confirme, "Retour" annule.
  Son historique reste dans l'archive ; il ne peut plus se connecter. On
  ne peut pas se désactiver soi-même. À effectuer (EV-09 : capture A41 confirmation de désactivation).

"Ajouter" : taper le nom et l'INSZ (11 chiffres), choisir le profil
(Gérant, Serveur ou Apprenti), puis "Ajouter l’employé". Pour un Serveur
ou un Apprenti, demander ensuite à l'employé de fixer son PIN (§2.2).
Pour un **nouveau Gérant**, la caisse demande d'abord le PIN du nouveau
Gérant (deux fois), puis "Confirmez avec votre PIN de gérant" : le
Gérant qui le crée tape son propre PIN. Le nouveau Gérant a donc son PIN
dès sa création. À effectuer (EV-09 : capture A42 ajout d'un employé).

### 12.4 Informations

Identifiant du POS ("POS ID"), "Terminal", "Restaurant", "FDM" (adresse)
et "Imprimante", tels que configurés à l'installation (lecture seule).
À effectuer (EV-09 : capture A43 panneau Informations).

Si cette configuration contient une erreur que le FDM refuserait (ex.
un numéro de TVA mal formé), un **bandeau rouge** s'affiche en bas de
l'écran : "Configuration invalide : le FDM refusera les tickets.
Appelez l'installateur." suivi de la liste des champs en cause. Il reste
affiché tant que l'erreur n'est pas corrigée.

### 12.5 Tickets par email

Panneau "Tickets par email". Visible dans tous les cas : il indique si
le service d'envoi est "Configuré" ou "Non configuré", puis "Joignable"
ou "Injoignable", et liste les derniers envois : adresse **masquée**
(ex. `j***@e***.be`), statut ("En attente", "Envoi…", "Envoyé", "Échec",
"Imprimé (secours)", "Purgé"), nombre de tentatives ("Tentatives") et
dernière erreur. "Actualiser" recharge la liste. À effectuer (EV-09 : capture A44 panneau Tickets par email).

L'adresse complète n'est jamais affichée ; elle est effacée dès l'envoi
réussi, et au plus tard après 72 h (`TICKET_NUMERIQUE_FR.md`).

### 12.6 Assistance à distance

Sans objet sur Android : `pili-pos-android` ne comporte aucun programme
d'assistance à distance, ni bouton, ni bandeau, ni mot de passe à lire
au téléphone. Le technicien intervient sur place
(`PLATEFORME_ANDROID_FR.md` §7).

### 12.7 Intégrations et HubRise

Panneau "Intégrations". Les commandes des plateformes de livraison et
du webshop ne sont reçues que si HubRise est activé sur cette caisse
(désactivé à l'installation, sauf demande du client). HubRise est
inclus dans le programme certifié : seul l'interrupteur décide s'il
fonctionne. À effectuer (EV-09 : capture A45 panneau Intégrations).

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
  catalogue ; "Déconnecter" demande d'abord confirmation ("Les
  commandes en ligne ne seront plus reçues tant que HubRise n'est pas
  reconnecté.") puis met fin à la connexion.
  À effectuer (EV-09 : capture A46 connexion à HubRise par QR code).
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
l'étiquette "Payée" ou "À encaisser". À effectuer (EV-09 : capture A47 tableau des commandes en ligne).

L'avancement d'une commande suit ces boutons :

- **"Accepter"** (Serveur ou Gérant) : une commande déjà payée sur la
  plateforme ou le webshop est vendue (événement N) et son ticket de
  caisse TVA imprimé ; une commande "À encaisser" est enregistrée
  (événement P) et le bon part en cuisine. **"Refuser"** : la caisse
  demande confirmation ("La plateforme et le client seront prévenus du
  refus. Cette action est définitive."), puis la commande est refusée ;
  rien n'est signé.
- **"Préparer"** (la commande passe en "En cuisine"), puis **"Prête"**
  (colonne "Prêtes"), pour tous les profils.
- **"Retarder"** (commande acceptée ou en préparation) : une fenêtre
  propose "+15 min", "+30 min" et "+45 min" ; un appui repousse l'heure
  promise de ce nombre de minutes (message "Délai appliqué") et ferme la
  fenêtre. À effectuer (EV-09 : capture A48 fenêtre Retarder).
- Pour une livraison du webshop, **"En livraison"** précède
  **"Terminer"** (commande déjà payée). Pour une commande "À encaisser",
  la remise se fait par **"Encaisser espèces"** ou **"Encaisser carte"** :
  vente (événement N) et ticket, comme au §7.
- **"Réimprimer"** ouvre "Ticket cuisine" et "Ticket client" pour
  réimprimer les bons ("Ticket envoyé à l'imprimante").
- **"À vérifier"** : la commande ne correspond pas au menu (produit
  absent, TVA différente, total incohérent...) ; elle ne peut pas être
  acceptée. "Resynchroniser" relit le menu, sinon la refuser.
- **"Rembourser"** (Gérant) : une commande annulée par la plateforme
  après la vente ; la caisse demande de confirmer le montant
  ("Rembourser {montant} ? Le remboursement est enregistré au nom de
  {nom}."). Le remboursement cite la vente d'origine, le tiroir reste
  fermé (la plateforme rembourse le client).
- **"Clôturer"** : une commande à encaisser annulée après son
  enregistrement ; ticket à 0,00 EUR.

Si le FDM ne répond pas, rien n'est signé et la commande reste à
accepter (§7.4). Une commande en ligne ne se prend pas en mode formation
("Pas de commande en ligne en mode formation."). Le détail fiscal est
dans `COMMANDES_EN_LIGNE_FR.md`.

### 12.9 Export nocturne

Écran « Export nocturne » : « Config », section « Autres réglages »,
ligne « Export nocturne » (« Retour » revient aux réglages). Réservé au
Gérant. Il montre où en est l'envoi automatique, après chaque rapport Z réel, de la
sauvegarde chiffrée et de la copie statistique de l'établissement
(`EXPORT_NOCTURNE_FR.md`). Cet écran n'est qu'un témoin : une vente, un
Z ou un rapport n'attendent jamais l'export, et rien n'est à faire tant
que l'état est « À jour ». L'écran se met à jour tout seul toutes les
30 secondes tant qu'il est affiché ("Chargement…" à l'ouverture).

L'état, en haut de l'écran :

| État | Signification | Que faire |
|---|---|---|
| « Non configuré » | L'établissement n'envoie rien : la fonction n'a pas été installée. | Rien. |
| « Configuration invalide » | La configuration de l'export est refusée (la cause s'affiche dessous). L'export est arrêté, la caisse vend normalement. | Appeler l'installateur (§14). |
| « À jour » | Tous les exports préparés sont arrivés au point de réception. | Rien. |
| « n exports en attente » (« 1 export en attente » au singulier) | Des exports ne sont pas encore livrés (réseau coupé, point de réception en panne, jeton refusé). Ils sont réessayés automatiquement, de plus en plus espacés (de 1 à 60 minutes), tant qu'ils sont conservés (`keep_days` jours). | Si l'état dure, lire « Dernière erreur » et appeler l'installateur. |

Les lignes de l'écran, affichées dans les états « À jour » et « n
exports en attente » : « Destination » (serveur de réception, sans
chemin), « Dernier export » (identifiant et heure de préparation ;
« Jamais » s'il n'y en a pas),
« Dernier envoi » (heure du dernier export arrivé complet), « Prochain
essai » (quand des exports attendent), « Dernière erreur » (avec son
heure ; disparaît au premier succès), « Conservés : n sur 14 jours »
(les copies gardées sur la caisse ; le nombre de jours est celui de
l'installation) et « Abandonnés » (ligne affichée seulement s'il y en a : exports jamais
livrés qui ont été supprimés de la caisse après la durée de
conservation). Le bouton « Réessayer maintenant » (en haut à droite ;
« Nouvel essai… » pendant l'essai) rend tout de suite possible un nouvel essai des
exports en attente ; il n'est affiché que si l'export est actif. Si
« Abandonnés » augmente, prévenir l'installateur : la caisse n'a pas pu
joindre le point de réception pendant toute la durée de conservation.

Écrans : À effectuer (EV-09 : capture A49 export nocturne, état À jour) ; À effectuer (EV-09 : capture A50 export nocturne, exports en attente).

Un Z de formation (§13) ne produit aucun export.

### 12.10 À propos (identité de la caisse)

« Config », section « Autres réglages », ligne « À propos ». Écran en
lecture seule qui donne l'identité exacte de la caisse, utile pour un
contrôle ou un appel au distributeur ; « Actualiser » relit les valeurs
et « Indisponible » remplace une valeur illisible. Quatre blocs :

- « Version » : « Version de l'application », « Code de version »,
  « Version logicielle du moteur » (la version affichée à la
  connexion, §2.1) et « Référence du moteur » (version du moteur commun et
  étiquette de gel : À effectuer (EV-04 : étiquette et commit du gel du
  moteur)).
- « Intégrité » : « SHA-1 de base.apk », « SHA-256 de base.apk »,
  « SHA-1 de libpos_ffi.so » et « SHA-256 du certificat de signature ».
  Un contrôleur les compare à celles de la version déposée : À effectuer
  (EV-01 : empreintes de l'APK signé), À effectuer (EV-02 : SHA-1 de
  libpos_ffi.so) et À effectuer (EV-03 : empreinte du certificat de
  signature).
- « Appareil » : « Identifiant de l'appareil (deviceId) » (le numéro de
  série du terminal, imprimé sur les tickets), « Identifiant de la
  caisse (posId) », « Identifiant du terminal (terminalId) », « Android »
  (version et niveau d'API) et « Modèle ».
- « Stockage » : « Espace libre » et « Sauvegardes de la base » (nombre
  de copies et taille). Deux avertissements peuvent s'afficher :
  « Moins de 2 Go libres sur la caisse. Prévenez le support. » et « Les
  sauvegardes occupent plus de la moitié de l'espace libre. Prévenez le
  support. » ; dans les deux cas, prévenir le distributeur.

Écran : À effectuer (EV-09 : capture A51 À propos).

### 12.11 Export pour le SPF (clé USB ou carte SD)

Réservé au Gérant : « Config », « Autres réglages », « Export pour le
SPF ». Cet écran remet aux contrôleurs du SPF Finances les données de la
caisse : il copie la base de données, les sauvegardes, les journaux et
l'identité de la caisse, puis relit et vérifie chaque fichier (voir
`FEUILLE_ACQUISITION_DONNEES_FR.md`).

1. Insérer la clé USB ou la carte SD. Tant que rien n'est détecté,
   l'écran affiche « Insérez la clé USB ou la carte SD. » (ports et
   lecteur du matériel réel : À effectuer (EV-15 : ports USB et lecteur
   de carte du D3 Pro réel)). Écran :
   À effectuer (EV-09 : capture A52 export pour le SPF, aucun support).
2. À la première utilisation d'un support, « Choisir le dossier » : créer
   sur la clé ou la carte un dossier nommé PILI-EXPORT, puis le choisir.
   La racine du support et un dossier d'un autre support sont refusés.
   L'écran rappelle ensuite « Support : » et « Dossier : » ; « Changer de
   dossier » en choisit un autre.
3. « Exporter » (écran : À effectuer (EV-09 : capture A53 export pour le SPF, prêt)).
   Pendant l'export (« Export en cours. Ne retirez pas la clé ou la
   carte. »), l'écran compte les fichiers et la taille copiés.
4. « Export terminé » : le dossier créé, le nombre de fichiers, la taille
   et l'empreinte SHA-256 de MANIFEST.sha256. On peut alors retirer le
   support ; le contrôleur vérifie l'export sur un ordinateur avec
   « shasum -a 256 -c MANIFEST.sha256 ». Écran :
   À effectuer (EV-09 : capture A54 export pour le SPF, terminé).

L'export ne démarre pas tant qu'une commande ou une vente à emporter est
ouverte (« Une commande ou une vente à emporter est ouverte. Terminez-la
avant d'exporter. ») ou que le moteur n'est pas prêt. Il ne supprime
rien, ni sur le support ni sur la caisse. S'il s'arrête (« Export
interrompu » avec l'étape, le fichier et le détail), les fichiers déjà
écrits sont conservés : refaire l'export. Si le message évoque un
fichier de 4 Go ou plus, le support est probablement en FAT32 : utiliser
un support formaté en exFAT. Si un fichier relu ne correspond pas à ce
qui a été écrit, l'export est invalide : le recommencer avec un autre
support. Écran : À effectuer (EV-09 : capture A55 export pour le SPF, interrompu).

### 12.12 Mise à jour de l'application

Réservé au Gérant : « Config », « Autres réglages », « Mise à jour ».
Aucune mise à jour ne se fait seule : seul le Gérant la lance, et la
caisse n'installe qu'une version déclarée et signée par le producteur
(`GARANTIE_MAJ_LANGUES_FR.md`).

1. L'écran rappelle « Version installée : … (code …) » et « SHA-1 de
   l'APK installé : … ». Écran : À effectuer (EV-09 : capture A56 mise à jour, écran initial).
2. « Vérifier en ligne » cherche la version déclarée sur Internet ;
   « Depuis une clé USB » la lit sur la clé ou la carte (« Aucune clé USB
   ni carte SD détectée. Insérez celle qui contient la mise à jour, puis
   recommencez. » sinon). Pendant la vérification (« Vérification de la
   mise à jour en cours. Ne quittez pas cet écran. »), la caisse contrôle
   la signature, le paquet, la version et les empreintes.
3. Si tout est correct, « Mise à jour disponible et vérifiée » affiche la
   « Version déclarée », le « SHA-1 de l'APK » et le « Signataire
   (SHA-256) ». Sinon « Mise à jour refusée » donne la cause (« Détail : … »)
   et rien n'est installé : par exemple version pas plus récente que celle
   installée, ou APK signé par une autre clé. Écran :
   À effectuer (EV-09 : capture A57 mise à jour vérifiée).
4. « Installer la mise à jour » n'est possible que si la caisse est prête.
   Sinon « La mise à jour ne peut pas démarrer maintenant : » liste ce qu'il
   faut faire : une commande ou une vente à emporter est encore ouverte
   (l'encaisser ou l'annuler) ; un rapport Z est dû ou n'a pas été fait
   depuis les dernières opérations fiscales (le faire, §10.2) ; le moteur
   n'est pas prêt (attendre quelques secondes) ; ou la personne connectée
   n'est pas Gérant. Écran : À effectuer (EV-09 : capture A58 mise à jour bloquée).
5. L'installation sauvegarde d'abord la base, ferme la caisse, installe la
   nouvelle version puis la redémarre seule, sur l'écran de connexion
   (« Installation en cours, la caisse redémarre. » : ne toucher à rien
   et ne pas éteindre la caisse). Si elle ne peut pas commencer (sauvegarde
   échouée, moteur impossible à arrêter, installation refusée par le
   système), « La mise à jour n'a pas été installée » le dit et la caisse
   continue de fonctionner avec la version installée.

Il n'existe pas de retour en arrière : une version défectueuse se corrige
par une version plus récente, déclarée et vérifiée de la même façon.

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
   À effectuer (EV-09 : capture A59 mode formation actif).

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
sont indiqués dans la colonne "Capture" du premier tableau
(`CAPTURES_ECRAN_FR.pdf`).

| Code | Message affiché | Effet | Que faire | Capture |
|---|---|---|---|---|
| `BUFFER_FULL` | La boîte noire est pleine. Contactez votre technicien. | Bloquant : plus de vente | Appeler le distributeur | À effectuer (EV-09 : capture A60 BUFFER_FULL) |
| `FDM_LOCKED` | La boîte noire est verrouillée. Contactez votre technicien. | Bloquant | Appeler le distributeur | À effectuer (EV-09 : capture A61 FDM_LOCKED) |
| `FDM_NOT_OPERATIONAL` | La boîte noire ne fonctionne pas. Impossible d'enregistrer les ventes. | Bloquant | Vérifier câble et alimentation, puis appeler le distributeur | À effectuer (EV-09 : capture A62 FDM_NOT_OPERATIONAL) |
| `UNKNOWN_POS` | Cette caisse n'est pas reconnue par la boîte noire. | Bloquant | Appeler le distributeur | À effectuer (EV-09 : capture A63 UNKNOWN_POS) |
| `BUFFER_NEAR_FULL` | La boîte noire est presque pleine. Programmez une intervention. | Avertissement obligatoire | Prévenir le distributeur | À effectuer (EV-09 : capture A64 BUFFER_NEAR_FULL) |
| `CLIENT_CERT_EXPIRED` | Le certificat de la boîte noire a expiré. Contactez votre technicien. | Avertissement obligatoire | Appeler le distributeur | À effectuer (EV-09 : capture A65 CLIENT_CERT_EXPIRED) |
| `UPDATE_POS_VATNO` | Le numéro de TVA de la caisse ne correspond pas à celui de la boîte noire. Contactez votre technicien. | Avertissement obligatoire | Appeler l'installateur | À effectuer (EV-09 : capture A66 UPDATE_POS_VATNO) |
| `UPDATE_POS_ESTNO` | Le numéro d'unité d'établissement de la caisse ne correspond pas à celui de la boîte noire. Contactez votre technicien. | Avertissement obligatoire | Appeler l'installateur | À effectuer (EV-09 : capture A67 UPDATE_POS_ESTNO) |
| `CLIENT_CERT_NEAR_EXPIRATION` | Le certificat de la boîte noire expire bientôt. | Avertissement | Prévenir le distributeur | À effectuer (EV-09 : capture A68 CLIENT_CERT_NEAR_EXPIRATION) |
| `UNAUTHORIZED` | La caisse n'est pas autorisée à parler à la boîte noire. | Avertissement | Appeler le distributeur | À effectuer (EV-09 : capture A69 UNAUTHORIZED) |
| `TRANSACTION_UPLOAD_FAILED` | La boîte noire échoue à transmettre les transactions au SPF Finances. | Avertissement | Vérifier la connexion Internet du FDM | À effectuer (EV-09 : capture A70 TRANSACTION_UPLOAD_FAILED) |
| `FDM_ANSWER_PENDING` | La caisse attend la réponse de la boîte noire pour l'opération précédente. Vérifiez la connexion et réessayez. | Opération refusée, renvoi automatique | Attendre une minute, réessayer (§7.4) | À effectuer (EV-09 : capture A71 FDM_ANSWER_PENDING) |
| `FDM_RESPONSE_MISMATCH` | Réponse de la boîte noire incohérente. Vente non enregistrée. Contactez votre technicien. | Opération refusée | Appeler le distributeur | À effectuer (EV-09 : capture A72 FDM_RESPONSE_MISMATCH) |
| `FDM_NOT_INITIALIZED` | La boîte noire n'est pas encore initialisée. | Voyant orange | Appeler le distributeur | À effectuer (EV-09 : capture A73 FDM_NOT_INITIALIZED) |
| `FDM_CERTIFICATE_REFUSED` | Connexion sécurisée à la boîte noire refusée (certificat). Contactez votre technicien. | Bloquant | Appeler l'installateur | À effectuer (EV-09 : capture A74 FDM_CERTIFICATE_REFUSED) |
| (réseau coupé) | Connexion perdue avec la boîte noire. Vérifiez le câble réseau. | Bloquant | Vérifier le câble réseau et l'alimentation du FDM | À effectuer (EV-09 : capture A75 réseau coupé) |

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
| Imprimante hors ligne | La vente est quand même enregistrée. Toucher "Imprimer le ticket" une fois l'imprimante revenue (§7.3), ou faire plus tard une copie (§8). |
| "Bon de commande non imprimé : prévenez le bar ou la cuisine." | Vérifier l'imprimante de la cuisine (voyant "Cuisine") ; la commande est enregistrée, prévenir le bar ou la cuisine (§6.1). |
| "Pointage non enregistré" | Le FDM n'a pas répondu à la connexion ou à la fin de service. Prévenir le Gérant. |
| "Commande non enregistrée par le FDM" | Rester sur la commande et réessayer quand le FDM répond (§6.1). |
| "La caisse attend la réponse de la boîte noire pour l'opération précédente" | Vérifier câble et alimentation du FDM, attendre une minute (la caisse renvoie seule), puis réessayer. Ne pas refaire l'opération. Appeler le distributeur si le message persiste (§7.4). |
| Le tiroir ne s'ouvre pas | Vérifier le câble RJ11 entre l'imprimante et le tiroir, et la serrure du tiroir. |
| L'écran ne réagit plus | Maintenir le bouton marche jusqu'à l'extinction de l'appareil (le menu d'alimentation n'est pas proposé en mode caisse), puis le rallumer ; durée de l'appui sur le matériel réel : À effectuer (EV-15 : appui long sur le D3 Pro réel). pili-pos-android redémarre seul ; les commandes non encaissées restent ouvertes (reprise après coupure : À effectuer (EV-12 : essai de coupure d'alimentation)). |
| "PIN incorrect" répété, ou "Trop de PIN incorrects" | Attendre la fin du décompte (§2.2). Le Gérant peut fixer un nouveau PIN (§12.3). Redémarrer la caisse remet le compteur à zéro. |
| INSZ refusé | Vérifier la saisie. L'employé doit exister et être actif (§12.3). |
| Bandeau rouge "Configuration invalide" | Appeler l'installateur (§12.4). |
| "La caisse démarre, veuillez patienter." | Le moteur de la caisse n'est pas encore prêt : attendre quelques secondes, puis réessayer. Si le message reste, redémarrer la caisse (ligne "L'écran ne réagit plus"), puis appeler le distributeur. |
| Écran "La caisse doit redémarrer" | Une erreur interne a arrêté le moteur. La caisse redémarre seule en quelques secondes ; après trois redémarrages en dix minutes, elle reste sur cet écran : l'éteindre et la rallumer. Si le problème se répète, appeler le technicien en lui lisant la ligne de détail affichée sous le message. |
| Écran "Mise en service de la caisse", "Appareil non préparé", "Configuration d'un autre appareil", "Numéro de série illisible" ou "Caisse non verrouillée" | Ces écrans concernent la préparation de l'appareil par l'installateur. Ne pas insister : appeler le distributeur. |

En cas de doute : **ne jamais vendre sans le FDM**, et **terminer chaque
journée par un rapport Z**.
