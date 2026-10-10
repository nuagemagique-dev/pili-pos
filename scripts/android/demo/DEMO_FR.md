# Démo Pili POS Android (émulateur D3 Pro GMS)

Démo sur l'émulateur `sunmi_d3_pro_gms_api33` du Mac, avec la carte de Tokyo Sushi Bar.
Ce n'est pas l'environnement de certification : FDM, imprimantes et réception de l'export
sont des simulateurs locaux.

## Lancer, arrêter

Depuis `moyun-pos-wt/int` :

```sh
scripts/android/demo/demo.sh start --no-build   # émulateur (fenêtre), simulateurs, appli ; garde les données
scripts/android/demo/demo.sh status
scripts/android/demo/demo.sh stop               # arrête les simulateurs, l'émulateur reste ouvert
scripts/android/demo/demo.sh reset              # efface l'appli : la caisse repart de zéro au prochain start
```

Sans `--no-build`, l'APK est reconstruit (quelques minutes).
Si l'émulateur de test `sunmi_d3_pro_api33` tourne, il faut d'abord le fermer, car un seul émulateur à la fois est possible.

## Codes PIN

| Personne | Rôle | PIN |
|---|---|---|
| Mengmeng Z. | gérante (rapports, Z, réglages) | 1111 |
| Sa X. | serveuse | 2222 |

## Voir ce qui est imprimé

- Tickets de caisse et additions : http://127.0.0.1:19391/
- Bons de cuisine : http://127.0.0.1:19393/
- Export nocturne reçu : `~/.cache/pili-demo/exports/` (sauvegarde chiffrée `.db.age`,
  fichier de statistiques `.sqlite`, manifeste). Déchiffrer :
  `age -d -i ~/.cache/pili-demo/age-key.txt <fichier>.db.age > pos.db`

## Déroulé conseillé (10 minutes)

1. **Connexion** : toucher « Mengmeng Z. », PIN 1111. Montrer les voyants FDM, ticket et cuisine en haut.
2. **Salle** : toucher une table, ajouter un plateau, des sushis et deux bières (onglet Boisson).
   Les noms suivent la langue de l'interface. Config, « Affichage des produits » : Code affiche
   les codes (Sushi saumon = A1) avec le nom en dessous ; Nom pour revenir.
3. **Envoyer en cuisine** : le bon de cuisine ne contient que les plats, pas les boissons
   (voir http://127.0.0.1:19393/).
4. **Addition** : addition provisoire signée par le FDM (événement P).
5. **PAYER** : espèces, « Exact », Valider. Le ticket T.V.A. est imprimé, avec la TVA par taux, les données
   FDM et le QR code. Le tiroir-caisse s'ouvre.
6. **À emporter** : « Nouvelle vente à emporter », un poke bowl, un Coca et une bière, paiement par carte.
   La TVA passe à 6 % sur la nourriture et le Coca, et reste à 21 % sur la bière.
7. **Langue** : Config, Langue 中文 (interface pour le personnel), puis retour en Français.
8. **Rapports** : Rapport X (chiffre du jour, TVA, départements).
9. **Clôture Z** : toucher deux fois « Clôturer la journée (Z) ». Ensuite, Config puis Export nocturne :
   l'export du jour apparaît comme envoyé.
10. **À propos** (Config) : version `pili-pos-android/1.0.0`, empreintes SHA de l'application.

Après la démo, `demo.sh reset` puis `start --no-build` donnent une caisse neuve.
