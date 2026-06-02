# Manuel utilisateur - moyun-pos v1.0.0

Ce manuel s'adresse aux opérateurs de caisse (Gérant, Serveur,
Apprenti) du système `moyun-pos`. Il couvre l'utilisation quotidienne :
connexion, prise de commande, encaissement, rapports, mode formation.

## Sommaire

1. Démarrage et écran de connexion
2. Pointage entrée / sortie
3. Plan de table et prise de commande
4. Modificateurs (sauces, cuisson...)
5. Envoi en cuisine et pré-ticket
6. Encaissement (vente normale)
7. Copie de ticket
8. Remboursement et correction
9. Rapports X et Z
10. Mode formation
11. Mouvements de caisse
12. Que faire en cas de problème

---

## 1. Démarrage et écran de connexion

Le terminal de caisse démarre automatiquement sur moyun-pos en plein
écran. **Aucune intervention n'est requise au démarrage.**

L'écran de connexion affiche un pavé numérique. Pour se connecter :

1. Composer le **numéro INSZ à 11 chiffres** de l'opérateur sur le
   pavé numérique.
2. Si l'utilisateur a un PIN, le pavé bascule sur la saisie du PIN.
   Composer le PIN.
3. Valider avec le bouton vert.

```
┌─────────────────────────────────────┐
│  Tokyo Sushi Bar - Connexion        │
│                                     │
│  INSZ : _ _ _ _ _ _ _ _ _ _ _       │
│                                     │
│   ┌───┐ ┌───┐ ┌───┐                 │
│   │ 1 │ │ 2 │ │ 3 │                 │
│   ├───┤ ├───┤ ├───┤                 │
│   │ 4 │ │ 5 │ │ 6 │                 │
│   ├───┤ ├───┤ ├───┤                 │
│   │ 7 │ │ 8 │ │ 9 │                 │
│   ├───┤ ├───┤ ├───┤                 │
│   │ Effacer │ │ 0 │ │ ✓ │                 │
│   └───┘ └───┘ └───┘                 │
└─────────────────────────────────────┘
```

En cas d'INSZ invalide (mauvais format ou inconnu), un message rouge
s'affiche : "Numéro INSZ invalide ou utilisateur inconnu". Vérifier
la saisie et réessayer.

## 2. Pointage entrée / sortie (Événement S)

### Pointage à l'arrivée (work-in)

La première connexion de la journée déclenche automatiquement un
**Événement S - signWorkIn** envoyé au FDM. Cette opération est
**obligatoire** : c'est ce qui consigne légalement la présence de
l'employé.

Confirmation : une icône verte "durée Présent" s'affiche dans l'en-tête.

### Pointage au départ (work-out)

Pour terminer la session, taper sur l'icône utilisateur en haut à
droite, puis "Pointer sortie". Un **Événement S - signWorkOut**
est envoyé.

L'opération est elle aussi **obligatoire**. Ne pas se contenter de
fermer l'écran sans pointer.

## 3. Plan de table et prise de commande

Après la connexion, l'écran principal affiche le **plan de table**
(`TablesView.vue`).

```
┌─────────────────────────────────────────┐
│  Tokyo Sushi Bar  durée Présent  utilisateur Marie   │
├─────────────────────────────────────────┤
│   ┌────┐  ┌────┐  ┌────┐  ┌────┐        │
│   │ T1 │  │ T2 │  │ T3 │  │ T4 │        │
│   │ *  │  │    │  │ *  │  │    │        │
│   └────┘  └────┘  └────┘  └────┘        │
│   occupée libre   occupée libre         │
└─────────────────────────────────────────┘
```

- **Cercle plein rouge** = table occupée (commande en cours).
- **Vide** = table libre.

### Ouvrir une commande

Taper sur la case d'une table libre. L'écran bascule sur la grille
de menu (`OrderView.vue`).

### Reprendre une commande existante

Taper sur la case d'une table occupée. La grille de menu s'ouvre
sur la commande en cours, avec les lignes déjà ajoutées dans le
panier (à droite).

### Grille de menu

```
┌──────────────┬──────────────────────┐
│ Catégories   │   Items              │
│              │                      │
│ Sushi 寿司   │ ┌──────┐ ┌──────┐    │
│ Plats chauds │ │Saumon│ │Calif.│    │
│ Boissons     │ │ x8   │ │ x8   │    │
│ Alcools      │ │8,50€ │ │9,00€ │    │
│              │ └──────┘ └──────┘    │
│              │ ┌──────┐ ┌──────┐    │
│              │ │Tuna  │ │Sashi.│    │
│              │ │ x6   │ │ mix  │    │
│              │ └──────┘ └──────┘    │
├──────────────┴──────────────────────┤
│ Panier - Table 1                    │
│ 1× Saumon x8     8,50 €             │
│ 1× Coca 33cl     3,50 €             │
│                                     │
│ Total : 12,00 €                     │
│ [Envoyer cuisine] [Encaisser]       │
└─────────────────────────────────────┘
```

Taper sur un item l'ajoute au panier. Taper plusieurs fois augmente
la quantité.

### Retirer ou modifier une ligne

Taper sur une ligne du panier pour ouvrir le menu d'action :
- "-" : diminuer la quantité (jusqu'à 0 = suppression).
- "+" : augmenter la quantité.
- "supprimer" : supprimer la ligne.

## 4. Modificateurs (sauces, cuisson...)

Quand un article a des modificateurs (ex. choix de sauce pour les
sushi), une modale s'ouvre dès l'ajout :

```
┌─────────────────────────────────┐
│  Saumon roll x8 - Sauces        │
│                                 │
│  [✓] Soja                       │
│  [ ] Wasabi                     │
│  [ ] Gingembre                  │
│                                 │
│  Maximum : 2 sauces             │
│  [Annuler]      [Valider]       │
└─────────────────────────────────┘
```

Cocher les options puis valider. Les modificateurs s'inscrivent
dans le panier sous la ligne principale.

## 5. Envoi en cuisine et pré-ticket

### Envoi en cuisine (Événement P - signOrder)

Bouton "**Envoyer cuisine**" dans le panier. Effet :

1. Un **Événement P** est signé par le FDM.
2. Le ticket cuisine s'imprime sur l'imprimante de cuisine (si
   configurée).
3. La commande reste **ouverte** (les clients n'ont pas encore payé).

### Pré-ticket (Événement P - signPreBill)

Pour produire un pré-ticket à présenter au client avant
l'encaissement ("l'addition"), taper "**Pré-ticket**" dans le
menu d'action de la commande.

Le pré-ticket s'imprime, marqué "**PRO FORMA - pas un ticket TVA
valable**". Il porte le QR code et la signature du FDM, mais ne
clôture pas la commande.

## 6. Encaissement (vente normale, Événement N - signSale)

Bouton "**Encaisser**" dans le panier. Trois étapes :

1. **Choix du mode de paiement** :
   - Espèces
   - Carte (saisie manuelle du montant)
   - (Mixte espèces + carte : non disponible en v1.0.0)
2. **Saisie du montant reçu** (pour les espèces).
   - Le système calcule la monnaie à rendre.
   - **Arrondi 5 cents** appliqué automatiquement (loi belge).
3. **Validation** → événement N envoyé au FDM.

Dès acceptation par le FDM :
- Le ticket fiscal s'imprime (avec QR code et signature courte).
- **Le tiroir-caisse s'ouvre** pour les paiements espèces.
- La commande passe à `closed` et la table redevient libre.

### Si le FDM est injoignable

Un message rouge bloquant s'affiche :

```
┌─────────────────────────────────┐
│  Attention : FDM non joignable            │
│                                 │
│  Impossible d'effectuer la      │
│  vente. Vérifier la connexion   │
│  Ethernet entre la caisse et    │
│  le boîtier FDM, puis réessayer.│
│                                 │
│  [Réessayer]    [Annuler]       │
└─────────────────────────────────┘
```

**Action obligatoire** : ne pas continuer la vente. Vérifier le
câble Ethernet, la prise courant du FDM, et appeler le distributeur
si le problème persiste.

## 7. Copie de ticket (Événement C - signCopy)

Pour réimprimer un ticket déjà encaissé (client a perdu le sien,
litige...) :

1. Aller dans **Menu → Historique tickets**.
2. Sélectionner le ticket à copier.
3. Taper "Copier".

La copie s'imprime, marquée "**COPIE - pas un original**". Un
nouvel événement C est signé par le FDM et lié au ticket original.

## 8. Remboursement et correction

### Remboursement complet (M111)

1. **Menu → Remboursements**.
2. Sélectionner le ticket de vente à rembourser.
3. Confirmer "Rembourser intégralement".
4. Le FDM signe un nouvel événement N avec quantités négatives et
   référence au ticket original.
5. Un ticket de remboursement s'imprime ; le tiroir s'ouvre pour
   rendre les espèces au client.

### Remboursement partiel (M112B)

Pour rembourser **certains items** d'un ticket :

1. **Menu → Remboursements**.
2. Sélectionner le ticket.
3. Cocher les lignes à rembourser.
4. Valider.

Le remboursement est un **événement fiscal séparé** ; le ticket
original reste intact.

### Correction de paiement (M132)

Pour corriger une erreur de mode de paiement (le client a payé carte
mais on a tapé espèces) :

1. **Menu → Historique tickets**.
2. Sélectionner le ticket.
3. "Corriger mode de paiement".

À utiliser **avant** la clôture Z de la journée. Au-delà, un
remboursement + nouvelle vente est nécessaire.

## 9. Rapports X et Z

### Rapport X (consultation, ne ferme pas la journée)

1. **Menu → Rapports → Rapport X**.
2. Le système agrège les ventes / TVA / paiements de la journée.
3. Un événement R est signé par le FDM.
4. Le ticket de rapport s'imprime.

Aucun compteur n'est remis à zéro. Le rapport X peut être généré
plusieurs fois par jour, à toute heure.

### Rapport Z (clôture journalière)

1. **Menu → Rapports → Rapport Z**.
2. Confirmer la clôture.
3. Le système agrège la journée, signe un événement R, imprime le
   ticket Z.
4. Une **nouvelle période de réservation** démarre.

**Bannière d'alerte** : à partir de **22 h** sans Z du jour, une
bannière rouge en haut de l'écran rappelle de générer le Z avant
de quitter le service.

## 10. Mode formation

Pour former un nouvel employé sans contaminer les ventes réelles :

1. **Menu → Paramètres** (réservé Gérant).
2. Activer "Mode formation".
3. Un filigrane rouge plein écran s'affiche.
4. Tous les événements générés en mode formation sont des **T** :
   - Comptés sur un compteur séparé.
   - Tickets marqués "**TRAINING - NE TIENT PAS LIEU DE TICKET TVA
     VALABLE**".
   - Sans impact sur le rapport Z.

Pour quitter : "Désactiver mode formation" dans le même menu.

## 11. Mouvements de caisse (Événement F - signMoneyInOut)

Pour enregistrer un mouvement d'espèces hors-vente :

1. **Menu → Caisse → Mouvement**.
2. Choisir "Entrée" (fonds de caisse, retour client externe...) ou
   "Sortie" (retrait, dépense).
3. Saisir le montant et un libellé court.
4. Valider - événement F signé par le FDM.

### Ouverture tiroir hors-vente (F-WAR - signDrawerOpen)

Pour ouvrir le tiroir-caisse en dehors d'une vente :

1. **Menu → Caisse → Ouvrir tiroir**.
2. Un événement F-WAR est signé par le FDM.
3. Le tiroir s'ouvre.

Attention : Réservé aux Gérants - chaque ouverture est tracée et apparaît dans
le rapport Z.

## 12. Que faire en cas de problème

| Problème | Action |
|---|---|
| FDM bloqué | Vérifier câble Ethernet, prise FDM. Appeler le distributeur. **Ne pas continuer à vendre.** |
| Imprimante manque de papier | Charger un rouleau de papier 80 mm thermique. La caisse réimprimera automatiquement le dernier ticket si interrompu. |
| Imprimante hors-ligne | Le ticket est quand même archivé fiscalement. Le client peut demander une "Copie" plus tard. |
| Tiroir-caisse ne s'ouvre pas | Vérifier le câble RJ11 entre imprimante et tiroir. Vérifier serrure manuelle (parfois en position verrouillée). |
| Écran ne réagit pas | Redémarrer le terminal (bouton power 5 sec). Au redémarrage, moyun-pos relance automatiquement. Toute commande non encaissée reste ouverte. |
| Erreur "Compteur saturé" | Compteur arrivé à 999 999 999 - impossible dans la vie d'un restaurant. Appeler le distributeur. |
| Numéro INSZ refusé | Vérifier la saisie. Si correct : appeler le Gérant, l'utilisateur doit être créé dans Paramètres. |

En cas de doute, **ne jamais effectuer de vente hors-FDM** et
toujours **terminer la journée par un rapport Z**.
