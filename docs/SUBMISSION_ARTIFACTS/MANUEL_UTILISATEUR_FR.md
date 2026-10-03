# Manuel utilisateur - pili-pos v1.0.0

Ce manuel s'adresse au personnel de salle et de caisse qui utilise
`pili-pos` au quotidien : connexion, prise de commande, addition,
encaissement, copies, remboursements, rapports, mouvements de caisse et
mode formation. Les libellés cités entre guillemets sont ceux de
l'écran, en français (l'interface existe aussi en anglais et en
chinois, §12.2).

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
11. Mouvements de caisse et tiroir (événements F)
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
| Se connecter, pointer, prendre des commandes | ☑ | ☑ | ☑ |
| Changer son propre PIN | ☑ | ☑ | ☑ |
| Imprimer l'addition, encaisser | - | ☑ | ☑ |
| Rapports X / Z, copies, remboursements, mouvements de caisse, tiroir | - | - | ☑ |
| Clôturer une vente sans paiement (client parti sans payer, §7.5) | - | - | ☑ |
| Mode formation, employés, PIN des autres, configuration | - | - | ☑ |
| Démarrer l'assistance à distance (§12.6) | - | - | ☑ |

## 2. Démarrage, connexion et PIN

Le terminal démarre seul sur pili-pos, en plein écran. Aucune
intervention n'est nécessaire.

### 2.1 Se connecter

L'écran de connexion affiche le nom de l'établissement et une tuile par
employé (initiale, nom, profil).

```
┌──────────────────────────────────────────┐
│            Restaurant Exemple            │
│                                          │
│   ┌────────┐  ┌────────┐  ┌────────┐     │
│   │   G    │  │   S    │  │   A    │     │
│   │ Gérant │  │Serveur │  │Apprenti│     │
│   │ Gérant │  │Serveur │  │Apprenti│     │
│   └────────┘  └────────┘  └────────┘     │
│                                          │
│              Autre (INSZ)                │
└──────────────────────────────────────────┘
```

1. Toucher sa tuile.
2. Si l'employé a un PIN, un pavé s'affiche ("Saisir le PIN") : taper
   les 4 chiffres. La connexion part d'elle-même au 4e chiffre.
3. Un PIN erroné affiche "PIN incorrect" et vide le pavé ; "Retour"
   revient aux tuiles.

"Autre (INSZ)" permet de se connecter en tapant son numéro INSZ
(11 chiffres) puis "Connexion". Si ce compte a un PIN, le pavé du PIN
s'affiche ensuite, comme depuis la tuile.

L'identité système `00000000029` (rapport Z automatique) n'a pas de
tuile et ne peut pas se connecter.

**Première connexion d'un Gérant.** Un Gérant doit avoir un PIN. S'il
n'en a pas encore (première connexion après l'installation), toucher
sa tuile ouvre "Choisissez votre PIN" : taper un PIN de 4 chiffres, puis
une seconde fois ("Confirmer le PIN"). La session s'ouvre et "PIN
enregistré" s'affiche. Si les deux saisies diffèrent, "Les PIN ne
correspondent pas" s'affiche et personne n'est connecté. Le PIN est
ensuite demandé à chaque connexion.

### 2.2 Le PIN

Un Gérant a toujours un PIN (voir §2.1, première connexion). Un Serveur
ou un Apprenti peut se connecter sans PIN, mais alors n'importe qui peut
ouvrir sa session en touchant sa tuile. Tant qu'il n'a pas de PIN, un
avertissement s'affiche à sa connexion : "PIN non configuré : touchez
votre nom en haut de l'écran".

- **Son propre PIN** : sur l'écran de commande, toucher son nom dans
  l'en-tête. Taper le nouveau PIN ("Saisir le PIN"), puis une seconde
  fois ("Confirmer le PIN"). S'ils diffèrent, "Les PIN ne correspondent
  pas" s'affiche et rien n'est enregistré. Sinon, "PIN modifié".
- **Le PIN d'un autre employé** (Gérant) : "Config", puis le bouton
  "PIN" sur la ligne de l'employé (§12.3), même saisie en deux fois.

Le PIN est enregistré sous forme de hachage (bcrypt), jamais en clair.

### 2.3 Quitter et verrouillage automatique

- "Sortir" (en haut à droite) ferme la session.
- Après **5 minutes sans toucher l'écran**, la caisse revient d'elle-même
  à l'écran de connexion.

Dans les deux cas, la commande en cours est d'abord enregistrée auprès
du FDM (§6.1) et le départ de l'employé est pointé (§3).

## 3. Pointage d'arrivée et de départ (événement S)

Le pointage est automatique, il n'y a pas de bouton :

- **À la connexion**, la caisse enregistre l'arrivée de l'employé
  (événement S, `signWorkIn`).
- **À la sortie** ("Sortir" ou verrouillage automatique), elle
  enregistre son départ (événement S, `signWorkOut`).

Si le FDM ne répond pas, la connexion ou la sortie se fait quand même,
mais un message rouge l'indique : "Pointage non enregistré", suivi de
la raison. Prévenir le Gérant pour qu'il régularise.

## 4. L'écran de commande

```
┌───────────────────────────────────────────────────────────────────┐
│ ● Caisse  ● Ticket   Marie   Table 4   14:05                      │
│             [Tables] [À emporter] [Rapports] [Config] [Sortir]    │
├───────────────────────────────────────────┬───────────────────────┤
│ [Sushi] [Plats chauds] [Boissons] ...     │ Commande          T4  │
│                                           │ Saumon roll x8  8,50  │
│ ┌────────┐ ┌────────┐ ┌────────┐ ┌──────┐ │   [-] 1 [+]      [x]  │
│ │Saumon  │ │Califor.│ │Sashimi │ │Coca  │ │ Coca 33cl       3,50  │
│ │roll x8 │ │roll x8 │ │mix     │ │33cl  │ │   [-] 1 [+]      [x]  │
│ │ 8,50 € │ │ 9,00 € │ │14,00 € │ │3,50 €│ │                       │
│ └────────┘ └────────┘ └────────┘ └──────┘ │ TVA A 21 %   ...      │
│                                           │ TOTAL        12,00 €  │
│                                           │ [Addition]  [PAYER]   │
└───────────────────────────────────────────┴───────────────────────┘
```

L'en-tête montre en permanence :

- **Caisse** : l'état du FDM (la "boîte noire"). Pastille verte : tout
  va bien. Orange ou rouge : un message explique le problème en clair
  (ex. "Connexion perdue avec la boîte noire. Vérifiez le câble
  réseau."). L'état est vérifié toutes les 30 secondes.
- **Ticket** : l'état de l'imprimante ("Papier épuisé", "Capot
  ouvert", "Vérifiez l'imprimante").
- le nom de l'employé connecté (le toucher pour changer son PIN, §2.2),
  la table ou "À emporter", l'heure.
- les boutons "Tables", "À emporter", "Rapports" et "Config" (ces deux
  derniers pour le Gérant seulement) et "Sortir".

**Rappel du rapport Z** : un bandeau "Rapport Z non effectué
aujourd'hui" s'affiche quand la journée n'est pas clôturée après 22 h,
ou quand une journée précédente est restée ouverte (§10.2).

## 5. Tables, à emporter et prise de commande

### 5.1 Choisir une table

"Tables" affiche le plan des tables. Une table **libre** porte la
mention "Libre" ; une table **occupée** affiche son nombre d'articles et
son total.

- Toucher une table libre ouvre une nouvelle commande pour cette table.
- Toucher une table occupée reprend sa commande en cours, avec tous ses
  articles dans le panier. Une table n'a jamais qu'une seule commande
  ouverte.

### 5.2 Vente à emporter

"À emporter" ouvre une commande sans table. L'addition (§6.2) n'existe
pas pour une commande à emporter : on encaisse directement.

Une vente à emporter est une vente directe : quitter la commande avant
de l'encaisser n'enregistre rien auprès du FDM (pas d'événement P). La
commande reste en cours, listée sur l'écran "Tables" sous "À emporter en
cours" (heure, nombre d'articles, total) ; la toucher la reprend. Elle
se termine par l'encaissement (§7) ou, si le client est parti sans
payer, par une vente sans paiement (§7.5).

### 5.3 Ajouter, modifier, retirer

- Choisir une catégorie dans la barre du haut, puis toucher un article :
  il s'ajoute au panier (à droite).
- Dans le panier, "+" et "-" changent la quantité ; "-" sur une
  quantité de 1, ou la croix, retire la ligne.
- **Prix libre** : certains articles n'ont pas de prix fixe (ex. "Plat
  du jour"). En les touchant, la fenêtre "Prix libre" demande le
  prix : taper les chiffres comme sur un terminal de paiement
  (2 0 0 0 = 20,00 €), puis "Valider".

Le panier affiche en permanence le détail de la TVA par taux et le
total.

## 6. Commande enregistrée et addition (événements P)

### 6.1 Enregistrement automatique de la commande

Tant que la table n'est pas payée, ce qui a été commandé est enregistré
auprès du FDM (événement P, `signOrder`) **dès que l'on quitte la
commande d'une table** (jamais pour une vente à emporter, §5.2) : "Tables", "À emporter", "Rapports", "Config", "Sortir" ou
verrouillage automatique. Rien n'est imprimé. Les articles retirés
entre-temps sont enregistrés aussi, comme corrections.

Si le FDM ne répond pas, le message "Commande non enregistrée par le
FDM" s'affiche et l'on reste sur la commande ; réessayer une fois le
FDM revenu. Rien n'est perdu : ce qui n'a pas pu être envoyé part au
prochain enregistrement, à l'addition ou à l'encaissement.

v1.0.0 n'imprime pas de ticket cuisine.

### 6.2 Addition

"Addition" (Serveur et Gérant, commandes en salle avec au moins un
article) imprime l'addition provisoire à présenter au client
(événement P, `signPreBill`). "Addition provisoire imprimée" confirme.

L'addition est marquée "**PRO FORMA**" et "**CECI N'EST PAS UN TICKET
DE CAISSE T.V.A. VALABLE**". Elle porte les données de contrôle du FDM
(identifiant, date, compteurs), mais ni QR code ni signature courte :
le FDM ne les donne qu'aux ventes (N). Elle ne clôture pas la table.

## 7. Encaissement (vente, événement N)

"PAYER" (Serveur et Gérant) ouvre l'écran de paiement, qui affiche le
total à payer.

### 7.1 Choix du support du ticket (si activé sur ce terminal)

Quand le ticket numérique est configuré (`MANUEL_INSTALLATION_FR.md`
§4.2), trois boutons apparaissent au-dessus des modes de paiement :

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

Une vente se paie en un ou plusieurs moyens de paiement, sur un seul
ticket.

- **CASH** : paie toujours tout ce qui reste. Taper le montant reçu sur
  le pavé, ou toucher "Exact" ou un raccourci de billet. La caisse
  affiche le montant à rendre ("Rendu"). Le reste en espèces est
  **arrondi aux 5 cents** (règle belge) ; le montant exact avant arrondi
  reste affiché. "Valider".
- **CARTE** : le pavé propose le reste à payer ; le laisser ou taper un
  montant plus petit (jamais plus que le reste : "Plus que le reste à
  payer"). Encaisser ce montant sur le terminal de paiement (qui n'est
  pas relié à la caisse), puis "Valider" une fois le paiement accepté.
  Pas d'arrondi pour une carte.
- **CHÈQUES-REPAS** (chèques-repas électroniques, payés sur le terminal
  de paiement) : comme la carte, pour le montant payé en chèques-repas.
  Pas d'arrondi, pas de rendu sur des chèques-repas.

Tant qu'il reste un montant, chaque paiement pris s'affiche avec son
montant et la caisse affiche le "Reste à payer". La vente est signée
dès que le reste est payé : par une carte ou des chèques-repas pour le
montant exact du reste, ou par le paiement en espèces. Exemple : 14,86 €
payés 11,52 € en chèques-repas, puis 3,34 € en espèces, encaissés 3,35 €
(arrondi +0,01 €).

"Annuler les paiements" efface les paiements pris tant que la vente
n'est pas signée (rien n'est encore enregistré ; rembourser au client ce
qui a déjà été passé sur le terminal).

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
- **"Imprimer le ticket"** : ce bouton apparaît quand un ticket papier
  était prévu mais n'a pas pu sortir (ex. papier épuisé). Il imprime
  l'original, **une seule fois**. Pour toute autre reproduction, faire
  une copie (§8).

Le tiroir-caisse s'ouvre quand une partie est payée en espèces (jamais
pour une carte ou des chèques-repas seuls). La table redevient libre.
"Terminer" revient à la prise de commande.

### 7.4 Si le FDM ne répond pas

Sans FDM, **aucune vente n'est possible** : la caisse refuse
l'encaissement, affiche la raison au-dessus des modes de paiement (ex.
"Connexion perdue avec la boîte noire. Vérifiez le câble réseau.") et
reste sur l'écran de paiement. Rien n'est enregistré, la commande reste
ouverte, le tiroir reste fermé.

Vérifier le câble réseau et l'alimentation du FDM, puis réessayer.
Appeler le distributeur si le problème persiste. Ne jamais encaisser en
dehors de la caisse.

Si le message est "La caisse attend la réponse de la boîte noire pour
l'opération précédente. Vérifiez la connexion et réessayez.", la
dernière opération a été envoyée au FDM mais sa réponse n'est pas
revenue. Rien n'est perdu : la caisse la renvoie seule toutes les
minutes, et toute nouvelle opération la renvoie d'abord. Vérifier le
câble et l'alimentation du FDM, attendre une minute, puis réessayer. Ne
pas refaire la vente à la main. Tant que cette réponse est attendue, la
commande concernée ne peut pas être modifiée. Si le message persiste,
appeler le distributeur.

Le message "La caisse enregistre une autre opération : réessayez dans un
instant." apparaît si la caisse a attendu environ 30 secondes derrière
une autre opération (par exemple la clôture automatique de la
journée) : réessayer.

### 7.5 Client parti sans payer

Réservé au Gérant. Quand un client part sans payer, ce qui a été servi
est tout de même vendu : la commande ne doit pas rester ouverte.

1. Ouvrir la commande (la table, ou l'emporter en cours).
2. Dans le panier, toucher "**Client parti sans payer**".
3. Confirmer avec son PIN.

Le FDM signe la vente (événement N) sans aucun paiement. Le ticket de
caisse TVA est imprimé, avec "**Non paye**" à la place des paiements.
Le tiroir ne s'ouvre pas et aucun paiement n'est compté dans les
rapports ; le chiffre d'affaires, lui, compte la vente. La table
redevient libre ("Vente clôturée sans paiement").

Une vente sans paiement ne se rembourse pas (rien n'a été encaissé) et
n'apparaît pas dans la liste des remboursements. Sa copie (§8) est
possible. Un PIN erroné est refusé ("PIN incorrect") et rien n'est
enregistré ; une commande à 0,00 € s'encaisse normalement. En mode
formation, la vente est une vente de formation (T).

## 8. Copie de ticket (événement C)

Réservé au Gérant. Pour un client qui a perdu son ticket ou son e-mail,
ou qui demande un double :

1. "Rapports", puis "Copie de ticket".
2. La liste "Ticket à copier" montre les derniers tickets remis à un
   client : ventes, remboursements, additions et tickets de formation,
   avec leur numéro, leur type, la table, le montant et l'heure.
3. Toucher le ticket, choisir le support (Papier / E-mail / Les deux,
   §7.1) puis "Valider".

La copie est signée par le FDM (événement C) et marquée "**COPIE**" et
"**CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE**". Un e-mail de copie
ne contient ni lien ni QR code : le FDM n'en donne pas pour une copie.
C'est la **seule** façon de reproduire un ticket déjà remis : la caisse
n'imprime jamais un original deux fois.

Un rapport Z se recopie depuis l'historique des Z (§10.3).

## 9. Remboursement

Réservé au Gérant.

1. "Rapports", puis "Remboursement".
2. "Sélectionner le ticket" : toucher la vente à rembourser (numéro,
   table, montant, heure).
3. "Articles à rembourser" : toucher chaque article concerné, et ajuster
   la quantité avec "-" / "+" (jamais plus que la quantité vendue).
   Tout cocher rembourse le ticket entier ; une partie seulement fait un
   remboursement partiel.
4. "Rembourser" (avec le montant), choisir le support du ticket (§7.1),
   puis "Rembourser" à nouveau.

Le FDM signe un nouveau ticket (événement N, quantités négatives,
référence au ticket d'origine) : un ticket de caisse TVA marqué
"REMBOURSEMENT", qui imprime les données de contrôle du ticket
d'origine sous « Informations FDM d'origine ». Le ticket d'origine
n'est jamais modifié. Le tiroir
s'ouvre pour rendre les espèces.

Le remboursement est toujours rendu en espèces, quel que soit le moyen
de paiement de la vente (carte, chèques-repas, plusieurs moyens).

La caisse refuse de rembourser un remboursement, de rembourser plus que
ce qui a été vendu, de rembourser une vente sans paiement (§7.5) et de
rembourser une vente réelle en mode formation. Si le FDM refuse, rien n'est enregistré et la vente reste
remboursable.

## 10. Rapports X et Z (événements R)

Réservé au Gérant : "Rapports".

### 10.1 Rapports X

- "Générer X" : chiffre d'affaires, TVA et paiements depuis le dernier Z.
- "Générer X util." : les mêmes chiffres par employé, avec les
  pointages.

Un rapport X ne clôture rien : il peut être tiré à tout moment, autant
de fois que nécessaire. Il est signé par le FDM et imprimé.

### 10.2 Rapport Z (clôture de la journée)

1. "Générer Z". Le bouton passe à l'orange : "Confirmer la clôture ?".
2. Toucher à nouveau dans les 5 secondes pour clôturer. Sans second
   appui, le bouton revient à son état normal et rien ne se passe.

Le Z est signé par le FDM, imprimé, et "Rapport généré (Z #n)"
confirme. La journée est close ; la suivante commence au prochain
enregistrement. Terminer **chaque** journée par un Z.

### 10.3 Historique des Z

"Historique Z" liste les Z passés (numéro, date). "Réimprimer" en
produit une copie (événement C, §8).

## 11. Mouvements de caisse et tiroir (événements F)

Réservé au Gérant : "Rapports", section "Gestion caisse".

### 11.1 Entrée ou sortie d'espèces

1. Choisir le montant : 10, 20, 50, 100 ou 200 €.
2. Toucher le motif, dans la rangée "+" (entrée) ou "-" (sortie) :
   "Fond de caisse", "Retrait" ou "Pourboire".

Le mouvement est signé par le FDM (événement F) ; "Mouvement
enregistré" confirme.

### 11.2 Ouvrir le tiroir hors vente

"Ouvrir tiroir" ouvre le tiroir en dehors d'une vente. Chaque ouverture
est signée par le FDM (événement F) et figure dans les rapports. En
dehors de ce bouton, le tiroir ne s'ouvre que pour un encaissement ou
un remboursement en espèces.

## 12. Configuration (Gérant)

"Config" ouvre les paramètres.

### 12.1 Mode formation

Voir §13.

### 12.2 Langue

Français, English ou 中文 : la langue de l'interface et des noms de
produits à l'écran. Les mentions imprimées sur les tickets suivent la
langue de l'établissement fixée à l'installation, pas ce choix.

### 12.3 Employés

La liste montre chaque employé (nom, profil, INSZ) avec deux boutons :

- **"PIN"** : fixer ou changer son PIN (§2.2).
- **"Désactiver"** : pour un employé qui quitte l'établissement. Toucher
  deux fois ("Confirmer ?"). Son historique reste dans l'archive ; il
  ne peut plus se connecter. On ne peut pas se désactiver soi-même.

"Ajouter" : taper le nom et l'INSZ (11 chiffres), choisir le profil
(Gérant, Serveur ou Apprenti), puis "Ajouter". Demander ensuite à
l'employé de fixer son PIN. Un nouveau Gérant le choisit lui-même à
sa première connexion (§2.1).

### 12.4 Informations

Identifiant du POS, terminal, établissement, adresse du FDM et de
l'imprimante, tels que configurés à l'installation (lecture seule).

Si cette configuration contient une erreur que le FDM refuserait (ex.
un numéro de TVA mal formé), un **bandeau rouge** s'affiche en bas de
l'écran : "Configuration invalide : le FDM refusera les tickets.
Appelez l'installateur." Il reste affiché tant que l'erreur n'est pas
corrigée.

### 12.5 Tickets numériques

Visible quand le ticket numérique est configuré. Indique si le service
d'envoi est "Configuré" et "Joignable", et liste les derniers envois :
support, adresse **masquée** (ex. `j***@e***.be`), statut ("En
attente", "Envoyé", "Échec", "Imprimé (secours)", "Purgé"), nombre de
tentatives et dernière erreur. "Actualiser" recharge la liste.

L'adresse complète n'est jamais affichée ; elle est effacée dès l'envoi
réussi, et au plus tard après 72 h (`TICKET_NUMERIQUE_FR.md`).

### 12.6 Assistance à distance

Quand le technicien doit voir la caisse à distance :

1. "Config", section "Assistance à distance", bouton "Démarrer".
2. Taper son PIN ("Saisir le PIN"). Un PIN erroné affiche "PIN
   incorrect" et rien ne démarre.
3. RustDesk s'ouvre et affiche un identifiant et un mot de passe : les
   lire au technicien par téléphone.

Tant que la session dure, un bandeau "Assistance à distance en cours
(démarrée par <nom>)" s'affiche en bas de chaque écran, pour tout le
personnel. La section indique depuis quand elle dure. Quand le
technicien a terminé, toucher "Arrêter la session". La session
continue après une déconnexion : l'écran de connexion garde le bandeau,
avec un bouton "Arrêter la session" que tout le monde peut toucher.
Elle s'arrête d'elle-même 2 heures après le démarrage. Une seule
session à la fois ; si RustDesk n'est pas installé, la section
l'indique ("Non installée sur cette caisse").

### 12.7 Commandes en ligne

Section "Intégrations". Les commandes des plateformes de livraison et
du webshop ne sont reçues que si elles sont activées sur cette caisse
(désactivées à l'installation, sauf demande du client).

- **"Activer" / "Désactiver"** : réservé au Gérant. Chaque changement
  est inscrit dans "Historique" (date, gérant ou fichier de
  configuration) et ne peut pas être effacé.
- Tant qu'une commande en ligne n'est pas terminée, "Désactiver" est
  refusé et la section indique "<n> commande(s) en ligne à terminer".

Une fois activées, le bouton "En ligne" de l'en-tête ouvre les
commandes reçues :

- **"Accepter"** (Serveur ou Gérant) : une commande déjà payée sur la
  plateforme ou le webshop est vendue (événement N) et son ticket de
  caisse TVA imprimé ; une commande "À encaisser" est enregistrée
  (événement P) et le bon part en cuisine.
- **"Encaisser espèces" / "Encaisser carte"** : à la remise d'une
  commande "À encaisser" ; vente (événement N) et ticket, comme au §7.
- **"Refuser"** : la commande est refusée, rien n'est signé.
- **"À vérifier"** : la commande ne correspond pas au menu ; elle ne
  peut pas être acceptée. "Resynchroniser" relit le menu, sinon la
  refuser.
- **"Rembourser"** (Gérant) : une commande annulée par la plateforme
  après la vente ; le remboursement cite la vente d'origine, le tiroir
  reste fermé (la plateforme rembourse le client).
- **"Clôturer"** : une commande à encaisser annulée après son
  enregistrement ; ticket à 0,00 EUR.

Si le FDM ne répond pas, rien n'est signé et la commande reste à
accepter (§7.4). Le détail fiscal est dans `COMMANDES_EN_LIGNE_FR.md`.

## 13. Mode formation (événements T)

Pour former un employé sans toucher aux ventes réelles.

1. Encaisser ou vider les commandes ouvertes : sinon la caisse refuse
   ("Encaissez ou videz les commandes ouvertes avant de changer de
   mode"), pour qu'une vraie table ne soit jamais encaissée en
   formation, ni l'inverse.
2. "Config", puis le bouton "Mode Formation" : il passe de "INACTIF" à
   "ACTIF".
3. Un bandeau violet "MODE FORMATION : les transactions ne sont PAS
   enregistrées" et un filigrane "TRAINING" restent affichés.

En formation, tout ce qui est signé est un événement **T** : compteur à
part, tickets marqués "**TRAINING**" et "**CECI N'EST PAS UN TICKET DE
CAISSE T.V.A. VALABLE**", sans effet sur le chiffre d'affaires des
rapports. Un remboursement de vente réelle est refusé en formation.

Pour revenir au mode normal : même bouton, qui repasse à "INACTIF"
(mêmes conditions).

## 14. Que faire en cas de problème

| Problème | Que faire |
|---|---|
| Pastille "Caisse" orange ou rouge, vente refusée | Lire le message affiché. Vérifier le câble réseau et l'alimentation du FDM, puis réessayer. Si le problème persiste, appeler le distributeur. **Ne jamais vendre en dehors de la caisse.** |
| Plus de papier | Charger un rouleau thermique de 80 mm, puis toucher "Imprimer le ticket" sur l'écran de fin de vente (§7.3). Rien n'est réimprimé automatiquement. |
| Imprimante hors ligne | La vente est quand même enregistrée. Toucher "Imprimer le ticket" une fois l'imprimante revenue (§7.3), ou faire plus tard une copie (§8). |
| "Pointage non enregistré" | Le FDM n'a pas répondu à la connexion ou à la sortie. Prévenir le Gérant. |
| "Commande non enregistrée par le FDM" | Rester sur la commande et réessayer quand le FDM répond (§6.1). |
| "La caisse attend la réponse de la boîte noire pour l'opération précédente" | Vérifier câble et alimentation du FDM, attendre une minute (la caisse renvoie seule), puis réessayer. Ne pas refaire l'opération. Appeler le distributeur si le message persiste (§7.4). |
| Le tiroir ne s'ouvre pas | Vérifier le câble RJ11 entre l'imprimante et le tiroir, et la serrure du tiroir. |
| L'écran ne réagit plus | Maintenir le bouton marche 5 secondes pour redémarrer. pili-pos redémarre seul ; les commandes non encaissées restent ouvertes. |
| "PIN incorrect" répété | Le Gérant peut fixer un nouveau PIN (§12.3). |
| INSZ refusé | Vérifier la saisie. L'employé doit exister et être actif (§12.3). |
| Bandeau rouge "Configuration invalide" | Appeler l'installateur (§12.4). |

En cas de doute : **ne jamais vendre sans le FDM**, et **terminer chaque
journée par un rapport Z**.
