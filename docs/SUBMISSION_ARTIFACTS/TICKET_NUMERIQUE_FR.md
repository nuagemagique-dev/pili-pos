# Émission de tickets numériques

**Référence :** §III.H "Emission des tickets numériques" du manuel de
certification.

> "Bien que la possibilité d'émettre des tickets sous forme numérique
> soit obligatoire, il n'existe pas de spécifications obligatoires
> concernant la mise en œuvre de cette mesure. Veuillez décrire
> comment cette mesure est mise en œuvre dans votre POS et comment la
> remise réelle du ticket numérique peut être garantie."

---

## 1. Position adoptée par moyun-pos v1.0.0

`moyun-pos v1.0.0` implémente l'émission de tickets numériques **via le
QR Code imprimé sur le ticket papier**, qui contient l'URL de
vérification publique du FDM SPF Finances. Le client peut, à tout
moment après la transaction, scanner ce QR Code avec son téléphone
pour obtenir :

- Une page web hébergée par SPF Finances avec le détail complet du
  ticket (montant, TVA, fdmId, eventCounter, signature).
- Le téléchargement d'un PDF officiel du ticket (selon la disponibilité
  du service SPF Finances).
- Une preuve de transaction archivable dans l'historique des
  paiements du téléphone (capture d'écran, partage par email, etc.).

Le QR Code est un QR Code Model 2 (« type 2 »), correction d'erreur M,
contenant l'URL de 38 caractères fournie par le FDM dans la réponse de
signature (champ `verificationUrl` du `SignResult`).

Taille : la description POS v2.2 §6.5 cite 25×25 modules (version 2).
Cette version ne peut pas contenir l'URL : son préfixe est en minuscules
et la valeur encodée est en base 62 (minuscules, majuscules, chiffres),
ce qui impose le mode octet (ISO/IEC 18004), limité à 34 octets en
version 2 même avec la correction d'erreur la plus faible. moyun-pos
utilise donc la plus petite version qui contient les 38 caractères avec
la correction M, soit la version 3 (29×29 modules). Ce point est
signalé au SPF pour confirmation.

## 2. Pourquoi cette approche

**Conforme à la lettre de la spec :** le manuel §III.H demande que
l'émission numérique soit possible et que la remise effective soit
garantie. Il n'impose pas de canal spécifique (email, app dédiée,
etc.). Le QR Code sur le ticket papier remplit les deux conditions :

- **Émission numérique :** l'information du ticket est entièrement
  encodée dans l'URL du QR Code. Le client n'a besoin que d'un
  scanner de QR Code (présent par défaut sur tous les smartphones
  modernes) pour accéder au contenu numérique.
- **Garantie de remise :** le QR Code est imprimé visiblement sur le
  ticket papier remis au client. La remise est physique, donc
  garantie. Le client choisit ensuite de scanner ou de jeter le
  papier.

**Simple et fiable :** pas de saisie d'email côté caisse (source
d'erreur opérateur fréquente), pas de connexion serveur tiers
supplémentaire à maintenir, pas de risque de bug de
livraison numérique. Le QR Code est imprimé en même temps que le
reste du ticket, donc même cycle de vie.

**Reposant sur le service officiel SPF Finances :** l'URL pointe vers
`gs2.be` (ou équivalent SPF), c'est-à-dire l'autorité officielle. Pas
de dépendance à un service tiers du producteur.

## 3. Implémentation technique

**Côté code :**

- `crates/pos-fiscal/src/qr.rs` génère le QR Code à partir de
  l'URL fournie par le FDM dans la réponse signSale (et tous les
  autres sign*). des tests unitaires couvrent : l'exemple d'URL de la
  spécification FDM (38 caractères), un symbole carré valide, la
  correction M et la version 3 (29×29 modules).
- `crates/pos-print/src/receipt.rs` rend le QR Code dans le ticket
  ESC/POS en mode trame raster (commande `GS ( k`).
- Le QR est positionné en bas du ticket, juste sous le bloc des
  données de contrôle FDM (cf. *TICKETS_EXEMPLES_FR.md* §1).

**Côté ticket :**

Tout ticket de type N, P, C, T, I porte un QR Code (cf. *TICKETS_EXEMPLES_FR.md*
matrice §VI). Le QR est imprimé physiquement et donc le client a un
accès immédiat à la version numérique du ticket dès qu'il quitte la
caisse.

## 4. Garantie de remise effective

**Question du manuel :** "comment la remise réelle du ticket numérique
peut être garantie ?"

**Réponse :**

1. **Remise physique du ticket papier obligatoire.** L'opérateur
   remet le ticket papier au client en fin de transaction. C'est une
   obligation du restaurant, vérifiable par observation directe.
2. **QR Code imprimé toujours.** Le code de l'application (crate
   `pos-print`) impose le rendu du QR Code dans tous les tickets
   fiscaux (N, P, C, T, I). Un test unitaire dans `pos-print`
   vérifie sa présence dans chaque type. Pas de chemin de code qui
   imprime un ticket sans QR Code.
3. **URL de vérification valide à long terme.** L'URL de
   vérification du FDM SPF Finances reste valide pendant toute la
   durée de conservation légale de la transaction (10 ans selon
   Article 60 §4 du Code TVA). Le client peut scanner le QR Code
   à tout moment pendant cette période.
4. **Archive locale en SQLite.** En complément du QR Code, le ticket
   complet (request + response FDM, données de signature) est
   archivé localement dans `fiscal_events` pendant 10 ans. En cas
   de demande SPF Finances de récupération d'un ticket, le
   distributeur peut produire le ticket complet à partir de
   `fdm_response_json` et `event_data_json`. *Voir :
   FEUILLE_ACQUISITION_DONNEES_FR.md*.

## 5. Évolutions prévues post-cert

Le QR Code suffit pour la première certification. Des évolutions
sont prévues en v0.2.0 et au-delà, pilotées par la demande client :

### v0.2.0 - Aperçu QR à l'écran

À la fin du checkout, afficher le QR Code à l'écran POS pendant
20 secondes pour permettre au client de scanner directement sans
attendre que le papier soit remis. Le ticket papier est toujours
imprimé en parallèle (comportement actuel inchangé).

**Effort :** 1-2 jours (intégration d'une lib QR JS dans la UI Vue,
composant `<QrPreview>` dans `CheckoutView.vue`). Pas de changement
fiscal, donc pas de nouveau cycle de certification (configuration
visuelle uniquement).

### v0.3.0 - Email opt-in

Permettre au client de saisir une adresse email facultative à
l'écran de checkout (clavier virtuel). Le POS envoie le ticket en
PDF + lien de vérification FDM par email via un service tiers
(Mailgun, SendGrid, ou SMTP du restaurant).

**Pré-requis :** validation RGPD spécifique (collecte d'adresse
email pour finalité limitée à l'envoi du ticket, à oublier
immédiatement après envoi). Charte RGPD à ajouter aux mentions
clients. *Voir : PROFIL_ENTREPRISE_FR.md §RGPD*.

**Effort :** 3-5 jours. Configuration SMTP, template HTML du ticket,
gestion d'erreur (retry, queue locale), RGPD. Pas de changement
fiscal donc pas de re-certification.

### v0.4.0 - Wallet integration

Apple Wallet et Google Wallet permettent d'ajouter des pass-types
de type "receipt". Le QR Code actuel peut être encapsulé dans un
pass-type qui s'ouvre automatiquement quand le client scanne.

**Effort :** 2-3 jours par plateforme + frais d'enrôlement
(Apple Developer Program 99$/an, Google gratuit).

## 6. Déclaration officielle pour la cert

Le ticket numérique est émis sous forme de QR Code imprimé sur le
ticket papier. La remise est garantie par la procédure opérateur
"remettre le ticket au client" et par l'invariant côté code que
tout ticket fiscal porte un QR Code.

Cette implémentation satisfait les deux obligations du §III.H :
- Émission numérique possible : OUI (QR Code Type 2, URL FDM officielle).
- Remise effective garantie : OUI (impression physique systématique
  + archive locale 10 ans + service de vérification SPF Finances
  long terme).
