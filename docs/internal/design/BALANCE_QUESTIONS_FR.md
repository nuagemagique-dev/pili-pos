# Balance : questions prêtes à envoyer

Brouillons pour le propriétaire, tirés de la section 11 de `BALANCE_DIBAL_FR.md`. **Rien n'a été envoyé.** Le propriétaire
relit, signe et envoie lui-même. Les réponses vont dans EV-21 (`docs/android/DOSSIER_EVIDENCE.md`).

Ordre conseillé : SPF Économie et Dibal ou son revendeur en même temps, car ce sont les seuls vrais délais ; SPF Finances
ensuite, avec la réponse de métrologie si elle est arrivée. Avant l'envoi, trois choix du propriétaire restent à faire
(P1 à P4 de la note) : la date limite, le premier restaurant, et la tare fixe ou posée.

## 1. SPF Économie, service Métrologie

**À :** metrology.regulation@economie.fgov.be
**Objet :** Caisse raccordée à une balance pour la vente directe au public : questions de métrologie

Madame, Monsieur,

Je développe un système de caisse enregistreuse pour la restauration en Belgique, en cours de certification SCE 2.0 auprès
du SPF Finances. Un restaurant doit pouvoir y vendre au poids, directement au client : un bol composé par le client est
posé sur une balance de caisse, et le prix au kilo vient de la caisse. La balance envisagée est une Dibal DPOS-400, reliée
à la caisse par le protocole Bizerba « Checkout-Dialog 06 ». Dans ce protocole, la balance demande à la caisse une somme de
contrôle de son logiciel et refuse les prix tant que cette somme n'est pas acceptée.

Je souhaite connaître les règles qui s'appliquent avant toute mise en service, et je vous remercie de bien vouloir répondre
par écrit aux questions suivantes.

1. Dans une vente directe au public avec un instrument de pesage à fonctionnement non automatique et une caisse raccordée,
   la caisse est-elle légalement pertinente ? Comment sa somme de contrôle et sa valeur de correction sont-elles attribuées
   et enregistrées en Belgique, et par qui (fabricant de la balance, éditeur, organisme notifié, SPF) ? Que déclenche une
   mise à jour de la caisse ?
2. Un protocole sans somme de contrôle est-il acceptable lorsque la balance affiche elle-même le poids, le prix unitaire et
   le prix sur son écran client ?
3. Une balance autonome vérifiée, dont l'opérateur saisit le poids dans la caisse, est-elle admise pour la vente directe au
   public ?
4. Que doit imprimer le ticket de caisse pour une ligne pesée : poids, unité, prix unitaire, prix, tare, mention, référence
   de la balance ? Quelles sont les règles d'arrondi ?
5. Une tare envoyée par la caisse à la balance est-elle admise, et doit-elle figurer sur le ticket ? Une remise accordée
   pour le contenant pose-t-elle un problème métrologique ?
6. Quels guides suivez-vous (WELMEC 2.2, 2.5, et pour le logiciel 2.3, 7.2 ou 7.5) ? Quelle est la vérification périodique
   de la balance, et à qui incombe-t-elle ?
7. La correction ou le remboursement d'une pesée a-t-il une exigence propre ?

Je peux vous transmettre la description technique du raccordement et rencontrer vos services si cela est utile.

Veuillez agréer, Madame, Monsieur, mes salutations distinguées.

## 2. Dibal ou son revendeur belge

**À :** adresse à trouver par le propriétaire (Dibal, ou son revendeur en Belgique)
**Objet :** DPOS-400 et logiciel de caisse tiers : protocole 24 (Checkout-Dialog 06)

Madame, Monsieur,

Je développe un système de caisse pour la restauration en Belgique, sur terminal Android, et je souhaite le raccorder à une
DPOS-400 pour la vente au poids directement au client. Avant de commander une balance, j'aurais besoin des informations
suivantes, ainsi que du manuel technique complet de la DPOS-400.

1. Quel est l'algorithme exact de l'enregistrement 10 ? Que signifie `Z`, combien de sommes sont attendues, et quel est le
   format de la valeur de correction ?
2. Comment un éditeur tiers reçoit-il sa valeur : quelle procédure, quels documents, quel coût ? Une somme portant sur une
   bibliothèque séparée est-elle acceptée ? Que se passe-t-il à une mise à jour du logiciel, et le réglage de la balance
   change-t-il ?
3. La DPOS-400 proposée en Belgique est-elle livrée vérifiée avec le protocole 24, et qui le règle (menu de test, scellé) ?
4. Existe-t-il, parmi les 40 protocoles de la balance, un protocole sans contrôle autorisé en Belgique, et comment renvoie-t-il
   le poids et le prix ?
5. Quelle est la règle d'arrondi du prix de vente ? Quels sont la plage, la précision et le cadrage du poids et du prix,
   5 ou 6 chiffres, et le poids minimal ?
6. Le port USB transporte-t-il le protocole 24 ? Quelle classe USB et quelle puce ? Quel courant demande le câble unique ?
7. Y a-t-il des cadences, délais et accusés de réception imposés ? Existe-t-il un simulateur officiel ou un outil de validation
   pour les éditeurs ?
8. La balance a-t-elle des exigences sur le terminal (certificat de compatibilité, liste d'éditeurs) ?
9. Peut-on obtenir une balance de prêt pour un essai sur banc ?

Veuillez agréer, Madame, Monsieur, mes salutations distinguées.

## 3. SPF Finances (boîte des producteurs)

**À :** secr.gksce@minfin.fed.be (adresse de la procédure de certification du 2026-09-03, à confirmer)
**Objet :** Demande de certification pili-pos-android : balance et prix au poids

Madame, Monsieur,

Dans le cadre de ma future demande de certification de pili-pos-android 1.0.0, je prévois de déclarer le prix au poids
(UC4112) et une balance de caisse en option. Je vous remercie de bien vouloir préciser les points suivants.

1. Déclarer la balance change-t-il autre chose que les lignes « Balance » et « Prix au poids » du manuel, et un rejeu de
   UC4112 suffit-il ?
2. Le FDM accepte-t-il un `vat.price` différent d'un centime de `quantity x unitPrice`, lorsque l'instrument légal impose son
   prix ?
3. Une ligne pesée peut-elle se rembourser seulement entière ?
4. Un bol vendu à prix fixe sans pesée (commande en ligne) peut-il être signé en `PIECE`, alors que la FAQ Q201 e) réserve ce
   type aux articles pré-emballés à prix fixe ?
5. La machine de test peut-elle présenter une balance simulée, comme le FDM et l'imprimante, ou faut-il une vraie balance à
   l'admission ?
6. La certification peut-elle être demandée avec la balance déclarée alors que la réponse du service Métrologie du SPF
   Économie n'est pas encore là, la liaison restant éteinte chez les clients ? Une preuve métrologique est-elle attendue dans
   le dossier ?

Veuillez agréer, Madame, Monsieur, mes salutations distinguées.
