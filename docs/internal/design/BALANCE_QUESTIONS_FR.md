# Balance : questions prêtes à envoyer

Brouillons pour le propriétaire, tirés de la section 11 de `BALANCE_DIBAL_FR.md`. **Rien n'a été envoyé.** Le propriétaire
relit, signe et envoie lui-même. Les réponses vont dans EV-21 (`docs/android/DOSSIER_EVIDENCE.md`).

Ordre conseillé : SPF Économie et Dibal ou son revendeur en même temps, car ce sont les seuls vrais délais ; SPF Finances
ensuite, avec la réponse de métrologie si elle est arrivée. Avant l'envoi, trois choix du propriétaire restent à faire
(P1 à P4 de la note) : la date limite, le premier restaurant, et la tare fixe ou posée.

## 1. SPF Économie, service Métrologie

**À :** metrology.regulation@economie.fgov.be, adresse non confirmée le 2026-10-30. En cas d'échec : info.eco@economie.fgov.be, en demandant de transmettre au service Métrologie.
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

## 2. Dibal

**À :** dibal@dibal.com (siège, page de contact de dibal.com, lue le 2026-10-30)
**Objet :** DPOS-400 integration with third-party POS software: protocol and software check

Dear Sir or Madam,

I develop a cash register system for restaurants in Belgium (pili-pos, Android, in certification with the Belgian tax
authority, SPF Finances). I would like to connect it to a DPOS-400 to sell by weight directly to the customer, with the price
per kilo sent by the POS. Before ordering a scale, could you please send me the DPOS-400 technical manual and answer the
following questions?

1. Who is your dealer or technical contact for Belgium?
2. Which protocols of the DPOS-400 may be used in Belgium for direct sale to the public? I have seen protocol 24 (Bizerba
   Checkout-Dialog 06, with a software check) and, in a POS vendor's setup guide, protocol 16. What does protocol 16 do, does it
   return weight, unit price and selling price, and does it require a software check?
3. For protocol 24: what is the exact algorithm of record 10, the meaning of `Z`, the number of checksums and the format of the
   correction value?
4. How does a third-party POS publisher obtain its value: which procedure, documents and cost? Is a checksum over a separate
   library accepted? What happens when the POS software is updated, and does the scale's setting change?
5. Is the DPOS-400 sold in Belgium delivered verified with that protocol, and who sets it (test menu, seal)?
6. What is the rounding rule of the selling price? What are the range, precision and scaling of weight and price, 5 or 6
   digits, and the minimum weight?
7. Does the USB port carry these protocols? Which USB class and chip? How much current does the single cable need?
8. Are timings, delays or acknowledgements imposed? Is there an official simulator or a validation tool for publishers?
9. Does the scale have requirements on the terminal (compatibility certificate, list of publishers)?
10. Could I borrow a scale for a bench test?

Kind regards,

## 2 bis. Revendeur belge (facultatif)

Onlinefact (info@onlinefact.be, +32 78 70 02 02, Heusden-Zolder) vend la D-POS 400 « + calibration » à 499 € TTC d'après sa
boutique. Rien n'indique que ce soit un revendeur officiel, et sa page parle de licence : il semble éditer sa propre caisse. Ne
lui poser que les questions d'achat : la balance est-elle livrée vérifiée, avec quel protocole, et à quel délai ?

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
