# Démo Pili POS Android (émulateur D3 Pro GMS)

Démo sur l'émulateur `sunmi_d3_pro_gms_api33` du Mac, avec la carte de Tokyo Sushi Bar.
Ce n'est pas l'environnement de certification : FDM, imprimantes, HubRise et réception de
l'export, terminal de paiement par carte sont des simulateurs locaux (aucun compte HubRise réel).

## Lancer, arrêter

Depuis `moyun-pos-wt/int` :

```sh
scripts/android/demo/demo.sh start --no-build   # émulateur (fenêtre), simulateurs, appli ; garde les données
scripts/android/demo/demo.sh status
scripts/android/demo/demo.sh stop               # arrête les simulateurs, l'émulateur reste ouvert
scripts/android/demo/demo.sh reset              # efface l'appli : la caisse repart de zéro au prochain start
scripts/android/demo/demo.sh order deliveroo    # nouvelle commande en ligne : deliveroo, ubereats, web ou cash
scripts/android/demo/demo.sh card decline       # règle la fin du PROCHAIN paiement par carte : approve, decline ou unknown
```

Sans `--no-build`, l'APK est reconstruit (quelques minutes).
Une caisse neuve démarre avec HubRise activé (voyant HUBRISE vert) et quatre commandes en ligne
en attente : Deliveroo et Uber Eats (payées sur la plateforme), webshop payé en ligne, webshop
à encaisser au retrait.
Si l'émulateur de test `sunmi_d3_pro_api33` tourne, il faut d'abord le fermer, car un seul émulateur à la fois est possible.

## Payer par carte sur le terminal

Une caisse neuve est liée à un terminal Worldline simulé (`pos-eft-mock`, port 19400). Aucun vrai
terminal, aucun vrai paiement. Une caisse déjà installée garde son ancien réglage : `reset` puis `start`.
À l'étape PAYER, choisir Carte : « Terminal » (par défaut) ou « Saisie manuelle » (le terminal est en panne).
Valider envoie le montant au terminal ; un écran bloquant « Paiement sur le terminal » montre le montant et
« Le client présente sa carte sur le terminal », avec Annuler. Le simulateur répond après 3 secondes.

| Commande avant de valider | Ce qui se passe |
|---|---|
| `demo.sh card approve` (ou rien) | Accepté : la part est verrouillée avec la marque de la carte (Bancontact par défaut, ou `card approve Visa`). |
| `demo.sh card decline` | Refusé : la raison s'affiche, la part reste modifiable, on peut réessayer ou passer en saisie manuelle. |
| `demo.sh card unknown` | « Réponse du terminal inconnue » : la vente est bloquée ; « Vérifier » interroge le terminal et trouve le paiement accepté. |

Chaque commande `card` ne vaut que pour un paiement. Quitter l'application pendant un paiement ne l'annule pas :
à la réouverture du paiement, la part acceptée revient verrouillée, ou « Vérifier » est proposé.
Commandes en ligne payables par carte (webshop « À encaisser ») : « Encaisser carte » passe aussi par le terminal.
En mode formation, la saisie manuelle reste la seule voie.

## Codes PIN

| Personne | Rôle | PIN |
|---|---|---|
| Mengmeng Z. | gérante (rapports, Z, réglages) | 1111 |
| Sa X. | serveuse | 2222 |

## Voir ce qui est imprimé

- Tickets de caisse et additions : http://127.0.0.1:19391/
- Bons de cuisine : http://127.0.0.1:19393/
- Terminal de paiement (simulateur) : http://127.0.0.1:19400/ (page des paiements), échanges sur
  http://127.0.0.1:19400/mock/requests
- Échanges avec HubRise (simulateur) : http://127.0.0.1:3001/mock/requests
- Export nocturne reçu : `~/.cache/pili-demo/exports/` (sauvegarde chiffrée `.db.age`,
  fichier de statistiques `.sqlite`, manifeste). Déchiffrer :
  `age -d -i ~/.cache/pili-demo/age-key.txt <fichier>.db.age > pos.db`

## Déroulé conseillé (12 minutes)

1. **Connexion** : toucher « Mengmeng Z. », PIN 1111. Montrer les voyants FDM, ticket et cuisine en haut.
2. **Salle** : toucher une table, ajouter un plateau, des sushis et deux bières (onglet Boisson).
   Les noms suivent la langue de l'interface. Config, « Affichage des produits » : Code affiche
   les codes du restaurant (onglet Sushi : Saumon = A1, Thon = A3) avec le nom en dessous ;
   les plateaux, bentos, boissons et desserts n'ont pas de code et gardent leur nom. Nom pour revenir.
3. **Envoyer en cuisine** : le bon de cuisine ne contient que les plats, pas les boissons
   (voir http://127.0.0.1:19393/).
4. **Addition** : addition provisoire signée par le FDM (événement P).
5. **PAYER** : espèces, « Exact », Valider. Le ticket T.V.A. est imprimé, avec la TVA par taux, les données
   FDM et le QR code. Le tiroir-caisse s'ouvre.
6. **À emporter** : « Nouvelle vente à emporter », un poke bowl, un Coca et une bière, paiement par carte.
   La TVA passe à 6 % sur la nourriture et le Coca, et reste à 21 % sur la bière.
   La caisse est liée à un terminal de paiement simulé (Config, À propos : « Terminal de paiement :
   Worldline ») : la carte part au terminal à la validation (voir « Payer par carte sur le terminal »).
7. **Commandes en ligne** : bouton « En ligne ». Accepter la commande Deliveroo : elle est déjà
   payée, le ticket T.V.A. (événement N) est signé à l'acceptation et le bon part en cuisine.
   Accepter la commande webshop « À encaisser » : addition provisoire (P), puis Préparer, Prête,
   « Encaisser espèces » au retrait (N). Pendant la démo, `demo.sh order ubereats` fait arriver
   une nouvelle commande. La carte est gérée par HubRise : la modification locale du menu est verrouillée.
8. **Langue** : Config, Langue 中文 (interface pour le personnel), puis retour en Français.
9. **Rapports** : Rapport X (chiffre du jour, TVA, départements).
10. **Clôture Z** : toucher deux fois « Clôturer la journée (Z) ». Ensuite, Config puis Export nocturne :
   l'export du jour apparaît comme envoyé.
11. **À propos** (Config) : version `pili-pos-android/1.0.0`, empreintes SHA de l'application.

Après la démo, `demo.sh reset` puis `start --no-build` donnent une caisse neuve.
