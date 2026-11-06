# UX Design Guide - Restaurant POS Touch Interface

> Guide de design pour l'interface POS tactile pili-pos.
> Synthèse des meilleures pratiques issues de l'industrie POS restaurant.
> À lire avant d'implémenter les composants Vue dans `ui/src/`.

---

## 1. L'utilisateur : qui est-il ?

Le staff de Tokyo Sushi Bar n'est pas un utilisateur tech. C'est un serveur ou un caissier qui travaille debout, sous pression, pendant le rush du midi et du soir. Il a les mains parfois mouillées ou grasses. Il gère simultanément des clients, des commandes cuisine, du bruit. Il veut que la caisse fonctionne sans y penser.

**Profil type :**
- Utilise le POS 4-8h par jour, debout
- Rush hours : 12h-14h et 18h-21h (2-3x plus de transactions/heure qu'en temps normal)
- Turnover élevé dans l'horeca : un nouvel employé doit être opérationnel en 10 minutes
- Probablement sinophone (ZH natif), avec un français limité. Interface en chinois simplifié comme langue principale de travail, français pour les clients et la conformité fiscale.
- Pas le temps de lire des instructions ou des popups

**Conséquence pour le design :** l'interface doit être utilisable par quelqu'un qui n'a jamais vu le système. Si un nouveau serveur a besoin de plus de 10 minutes de formation, le design a échoué.

---

## 2. Principes fondamentaux

### 2.1 - Réduire la charge cognitive au maximum

Le staff est déjà surchargé sensoriellement (bruit, clients, cuisine). Chaque élément visuel superflu, chaque popup inattendu, chaque étape inutile consomme de l'attention qu'il n'a pas.

**Règles :**
- Maximum 6-8 actions visibles par écran. Pas plus.
- Pas de popups décoratives, pas d'animations cosmétiques. Seules les animations informatives sont acceptées (loading spinner pendant l'appel FDM, flash vert de confirmation).
- Pas de texte long. Labels courts (1-3 mots max sur les boutons).
- Si un utilisateur doit réfléchir à quoi faire → le design a échoué.

### 2.2 - Chaque micro-friction se multiplie par milliers

Un bouton qui demande 0.5 seconde de trop pour être trouvé, c'est 500 secondes perdues sur 1000 transactions. Penser chaque interaction comme si elle allait être répétée 10 000 fois.

**Règles :**
- Les actions les plus fréquentes (ajouter un item, valider, payer) doivent être accessibles en 1 tap.
- Les actions rares (remboursement, rapport Z, settings) peuvent être à 2-3 taps de profondeur.
- Zéro navigation inutile entre les écrans pour le flow principal : menu → cart → paiement → ticket.

### 2.3 - La consistance crée l'automatisme

Après 2-3 semaines d'utilisation intensive, le staff développe des automatismes musculaires (muscle memory). Les boutons doivent toujours être au même endroit. Changer le layout casse ces automatismes et génère des erreurs.

**Règles :**
- Le bouton principal d'action est TOUJOURS en bas à droite du cart. Sa place ne bouge pas ; ce qu'il fait suit l'état de la commande (« Envoyer en cuisine » tant que des lignes attendent, puis PAYER : §3, audit C4).
- Le cart/sidebar est TOUJOURS à droite de l'écran.
- Les catégories sont TOUJOURS en haut de la grille menu.
- La barre de statut (FDM, user, heure) est TOUJOURS en haut de l'écran.
- Ne jamais déplacer un élément entre les mises à jour.

---

### 2.4 - Commandes en ligne : refuser avec un motif, droits par rôle, encaisser, retard (audit O1 à O3, L6)

« Refuser » une commande en ligne est définitif (la plateforme et le client sont prévenus), donc il ouvre une confirmation avec les motifs en grosses touches (« Rupture de stock », « Trop de commandes », « Restaurant fermé », « Autre ») et un bouton rouge « Refuser la commande » grisé tant qu'aucun motif n'est choisi : trois touches, jamais par accident. Sur la carte, « Refuser » est à l'autre bout de la rangée d'actions que « Accepter », contour rouge sans remplissage. HubRise n'a pas de champ de motif de refus : le motif reste sur la caisse, il n'est ni envoyé ni signé.

Accepter, refuser, remettre et encaisser demandent les droits d'encaissement. Pour un Apprenti ces boutons sont grisés avec la ligne « Réservé au serveur ou au gérant » au lieu d'échouer avec « Action réservée à un autre profil ». À l'écran, « Serveur » est le rôle interne `cashier` et « Apprenti » le rôle `waiter` (les noms internes ne changent pas). Préparer, « Prête » et « En livraison » restent ouverts à tous.

« Encaisser espèces » et « Encaisser carte » d'une commande en ligne non payée ouvrent le même écran de paiement que la salle (montant exact déjà saisi, rendu, partage, « Carte » déjà choisie pour le bouton carte) ; la vente est la remise de la commande, la puce de l'en-tête dit « En ligne » et la fin de la vente revient à la liste des commandes en ligne (audit O2). Une commande en ligne n'est jamais la commande active du moteur : un verrouillage ou une sortie d'écran n'enregistre rien dessus.

Dès 5 minutes d'attente, un bandeau ambre sous l'en-tête de tous les écrans du personnel (sauf la liste elle-même, qui garde son alarme) dit « N commandes en ligne attendent depuis plus de 5 min » avec un bouton « Voir » ; la sonnerie se répète toutes les 2 minutes tant qu'une commande est en retard (audit O3). Ambre = en attente, donc le bouton « Voir » est en encre sur l'ambre.

### 2.5 - Copie et remboursement : pas d'étape vide (audit R1)

Sans tickets par e-mail, l'étape « Papier / E-mail » ne propose rien et ne ferait que répéter le bouton de confirmation : « Copie » (et la réimpression d'un Z) signe donc la copie en papier au premier toucher, sans étape intermédiaire. Avec les tickets par e-mail configurés, l'étape du moyen reste avant la signature. Le remboursement a une seule étape de confirmation, montrée une fois avant de signer : les lignes cochées avec leur quantité, la TVA incluse, le total à rembourser en espèces et le bouton « Confirmer le remboursement » (le bouton de la ligne du ticket reste « Rembourser » ; le choix du moyen s'y ajoute seulement si l'e-mail est configuré).

### 2.6 - Tiroir avec un mouvement, retrouver un ticket (audit R3, R2)

**Tiroir (R3).** « Enregistrer la sortie » et « Enregistrer l'entrée » (onglet Caisse des Rapports) signent le mouvement F et ouvrent le tiroir dans le même geste : un tap, un seul coup de tiroir. Aucun événement « ouverture du tiroir » n'est signé en plus, car le mouvement est lui-même la trace de l'ouverture (description POS v2.2 : DRAWER_OPEN est l'ouverture du tiroir « en dehors des ventes » ; FAQ Q405 : signDrawerOpen n'est pas nécessaire avant un signMoneyInOut). Le bouton « Ouvrir le tiroir » reste, et la déclaration du tiroir (M133) ne l'ouvre jamais.

**Ticket par numéro (R2).** Onglet Tickets des Rapports : le champ « N° de ticket » (pavé numérique) retrouve dans l'archive la vente que le client montre, pour la copie et le remboursement, avec les règles d'aujourd'hui (période comptable, déjà remboursée, bon cadeau utilisé) : une vente qui ne se rembourse pas affiche son motif et reste copiable. La liste garde les dernières ventes ; « Plus ancien » en charge 20 de plus. Au plus 10 résultats, parce que N, P et T comptent chacun leurs numéros.

### 2.7 - Une seule commande, un seul code du gérant (audit P8)

Quand un gérant a confirmé une remise, un article offert ou une ligne offerte avec son code, les remises, offres et retraits suivants sur LA MÊME commande par LE MÊME gérant connecté ne redemandent pas le code pendant 2 minutes (comptées depuis la saisie du code, jamais prolongées par un réemploi). Le moteur tient cette autorisation en mémoire ; elle tombe au verrouillage, à la déconnexion, en fin de service, au changement de commande et à la vente. Chaque usage est écrit dans la piste d'audit des remises avec l'indication « code réemployé ». « Client parti sans payer », la correction de paiement, le remboursement et l'assistance à distance redemandent toujours le code. Le premier toast de remise dit que le code est gardé deux minutes. Rien de signé ne change.

## 3. Layout principal - OrderView (écran le plus utilisé)

C'est l'écran où le staff passe 80% de son temps. Il doit être parfait.

```
┌──────────────────────────────────────────────────────────────────────┐
│  pili │ ● FDM ● Ticket ● Cuisine │ Ahmed │ Table 12 │ 14:32 │ [Verrouiller] │  ← Header (fixe, 64px)
├──────────────────────────────────────────┬───────────────────────────┤
│                                          │                           │
│  [Sushi] [Maki] [Sashimi] [Boissons]    │   COMMANDE #127           │
│  ─────────────────────────────────────   │   Table 12                │
│                                          │                           │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐      │   2x Salmon Roll    12.00 │
│  │Salm.│ │Tuna │ │Calif│ │Drag.│      │   1x Edamame         5.50 │
│  │Roll │ │Roll │ │Roll │ │Roll │      │   1x Asahi           4.50 │
│  │12.00│ │14.00│ │10.00│ │11.00│      │   ────────────────────── │
│  └─────┘ └─────┘ └─────┘ └─────┘      │                           │
│                                          │   Sous-total:      22.00 │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐      │   TVA 12%:          2.36 │
│  │Edam.│ │Gyoza│ │Miso │ │Sake │      │   TVA 21%:          0.78 │
│  │ 5.50│ │ 7.00│ │ 4.00│ │ 6.50│      │   ══════════════════════ │
│  └─────┘ └─────┘ └─────┘ └─────┘      │   TOTAL:           22.00 │
│                                          │                           │
│                                          │   [  PAYER  ▶  ]          │  ← Bouton principal, grand, couleur accent
│  Menu grid (scrollable)                  │   Cart sidebar (fixe)     │
└──────────────────────────────────────────┴───────────────────────────┘
         ~65% de la largeur                      ~35% de la largeur
```

**Pied du panier (audit C4, C3) :** tant que la commande en salle a des lignes pas encore envoyées en cuisine (hors lignes retenues par un service), « Envoyer en cuisine » est le bouton principal Volt avec une pastille « 2 lignes », et PAYER devient un bouton secondaire à contour qui affiche toujours le total. Une fois tout envoyé, PAYER redevient le principal. Pour un Apprenti (sans droit d'encaisser), « Envoyer en cuisine » est toujours le principal. « Changer de table », « Remise » et « Client parti sans payer » ne sont plus dans le pied : ils sont dans le menu « Plus » de l'en-tête du panier.

**Barre d'outils et pavé de code (audit C1, C6).** Les modes (Épuisé, Produits épuisés, Saisie code, Allergènes, Bon cadeau) tiennent sur une seule rangée de 56 px après les catégories : boutons de 48 px avec une icône Material et un libellé court, l'aide du mode actif sur la même rangée, la légende des allergènes seulement quand le mode est actif. Le pavé de code est un bandeau de 300 px ancré sous la grille (champ et lettres à gauche, 7 8 9 au milieu, un OK haut à droite) ; la grille garde ses 5 colonnes et défile au-dessus.

**Onglet Favoris (audit C12).** Premier onglet, une étoile, avant les catégories ; caché tant que le moteur n'a aucun produit à y mettre, ouvert par défaut quand il en a, et retour à la première catégorie s'il se vide. Config, panneau « Favoris » (gérant) : « Plus vendus sur 30 jours » (par défaut : les 18 meilleures ventes signées des 30 derniers jours, recalculées au plus une fois par heure ou après un Z) ou « Ma sélection » (l'étoile de chaque produit dans l'éditeur de menu, dans l'ordre d'ajout, 30 produits au plus). Un produit s'y comporte comme dans sa catégorie : code, marque « Épuisé », mode allergènes, pastilles, et il garde la couleur de sa catégorie. Donnée locale : rien n'entre dans une requête signée.

**Panier dense (audit C2, C11).** Une ligne de 56 px par article (marque cuisine, quantité, code, nom, pastilles, prix) : une douzaine de lignes tiennent dans le panier à 1080 p. Les actions d'une ligne (−, +, Note, Offrir, service, Supprimer) ne s'ouvrent que sous la ligne choisie : la plus récente l'est par défaut, un tap choisit ou replie, un ajout choisit sa ligne. La fenêtre de note propose des pastilles de notes fréquentes (Config, « Notes rapides », 24 au plus) ; un tap ajoute la pastille à la note, séparée par une virgule, et la limite de 60 caractères reste la même.

**Quantité et retour à l'ajout (audit C7, C9).** Juste après l'ajout d'un produit simple (grille ou pavé de code), pendant 3 s, un chiffre tapé sur le pavé ou le clavier fixe la quantité de cet ajout (un second chiffre prolonge, jusqu'à 99) : « plat puis 3 » donne trois portions. Les produits pesés, à options ou à prix libre n'ouvrent pas cette fenêtre. Chaque ajout fait un bip court et fait clignoter sa ligne 200 ms ; le bip se coupe par terminal dans Config (« Bip à chaque ajout »).

**Proportions :** menu grid = ~65% de l'écran, cart sidebar = ~35%. Sur 1920px, ça donne ~1250px pour le menu et ~670px pour le cart.

**Grille menu :** 5 colonnes de boutons items (environ 230 × 120 px, au-dessus du minimum de 140 × 100 px). Scrollable verticalement avec inertie tactile.

**Écarts assumés (charte Pili) :** en-tête de 64 px au lieu de 48, pour que ses boutons fassent 48 px (§4.1, §13) ; 5 colonnes au lieu de 4 ; catégories sur deux rangées fixes de 9 au lieu d'onglets à faire défiler, pour que les 17 catégories (et l'onglet Favoris) soient à un seul appui (§2.1). L'en-tête est le même sur tous les écrans (`AppHeader.vue`).

### 3.1 - Écran Tables (audit T1, T2, T3, T5, T7, C8)

- **Zones et lettres (T1).** Le plan de salle (Config, « Plan de salle ») accepte des zones : une lettre (ou deux) devant une plage nomme la zone, « A 1-20, B 21-40 ». Les numéros restent uniques sur tout le plan, comme A-19 et B-30 chez Xudox. Des noms de zones facultatifs (« A · Salle, B · Terrasse ») donnent les en-têtes sous lesquels les tables sont groupées. Partout où une table est nommée à l'écran (carte, puce de l'en-tête, sélecteur de table, messages, rapports) et sur les bons cuisine, bar et le bon de transfert, elle s'écrit « A-19 ». Un plan sans lettres reste « 19 ». Le numéro de table de toute requête signée et du ticket fiscal est toujours le numéro seul (19).
- **Serveur et couverts (T5).** Une carte occupée montre l'initiale de la personne qui a ouvert la commande dans une pastille ronde. Les couverts sont facultatifs : le bouton « Couverts » de l'en-tête du panier ouvre un pavé de 1 à 30 (0 les efface), la carte affiche « 4 couv. ». Donnée locale, jamais signée. Au paiement, le bouton de partage du même nombre est marqué (§7.4).
- **Emporter numéroté (T2).** Chaque vente à emporter reçoit un numéro du jour, « E-61 », qui repart chaque jour au premier numéro du réglage (61 par défaut, Config, « Plan de salle »). L'en-tête dit « À emporter E-61 · Dupont » ; « Ajouter un nom » demande le nom (clavier à l'écran) puis le téléphone (pavé), tous deux facultatifs. Le numéro et le nom sont en gros sur les bons cuisine et bar ; le téléphone ne s'imprime jamais, nom et téléphone sont effacés 72 h après la vente, le numéro reste. Les commandes en ligne gardent la référence de leur plateforme.
- **Déplacer (T3, C8).** La puce de table de l'en-tête (commande et paiement) est un bouton de 48 px qui ouvre le sélecteur : une table libre déplace la commande aussitôt, sans fenêtre de validation, et un message « Table 4 déplacée en 9 » reste 5 s avec « Annuler » (un second transfert, signé comme tout autre). « Déplacer » du menu Plus et du plan de salle suit le même geste.
- **Appui long (T7).** Un appui de 500 ms sur une carte occupée ouvre une feuille « Ouvrir · Addition · Déplacer · Payer » ; un appui court ouvre la table comme avant. « Addition » et « Payer » demandent les droits d'encaissement (un Apprenti ne voit que « Ouvrir » et « Déplacer »). « Addition » imprime l'addition de cette commande sans l'ouvrir. L'appui long est un raccourci : chaque action existe aussi sans lui (§11).

### 3.2 - En-tête, session et langue (audit L1, R5, T6)

Le bouton de l'utilisateur ouvre un menu : sa langue (français, chinois, anglais, gardée par utilisateur sur le terminal et appliquée à sa connexion), « Changer mon PIN », « Fin de service » (armée au premier tap, 5 s, signe la sortie de l'opérateur puis verrouille) et « Verrouiller ». La tuile de connexion d'un opérateur en service propose « Se connecter » et « Terminer le service ».

---

## 4. Composants tactiles - Spécifications

### 4.1 - Taille des boutons

| Élément | Taille minimum | Recommandé | Espacement |
|---------|---------------|------------|------------|
| Bouton menu item | 120×90px | 140×100px | 8px gap |
| Bouton catégorie (tab) | 100×48px | 120×52px | 4px gap |
| Bouton action principal (Payer) | pleine largeur cart × 64px | pleine largeur × 72px | 16px margin |
| Bouton secondaire (Annuler) | 100×48px | 120×52px | 8px gap |
| Touche NumPad | 80×64px | 96×72px | 4px gap |
| Ligne order dans cart | pleine largeur × 56px | pleine largeur × 64px | 0 (séparateur 1px) |

**Disposition du pavé numérique :** tous les pavés numériques (PIN, INSZ, montants, prix, quantités, bons cadeaux, dates des rapports, code produit) suivent la disposition d'une calculatrice, 7 8 9 en haut, puis 4 5 6, 1 2 3, et la ligne du bas propre à chaque pavé (C, 0, effacer ; virgule sur la caisse Linux). Les mêmes touches dans le même ordre sur Linux et Android (`ui/e2e/pad-layout.spec.ts`, `PadLayoutAssert.kt`).

**Aucun champ de texte natif (audit C13, R6).** Le terminal de salle n'a pas de clavier système : chaque champ s'ouvre sur un clavier ou un pavé à l'écran (clavier AZERTY à majuscules et accents pour le texte, pavé numérique pour les nombres et les numéros à longueur exacte). Une exception déclarée : le nom chinois d'un produit dans l'éditeur de menu, qu'aucun clavier à l'écran ne sait écrire (`docs/android/DECISIONS.md`, UX-C13-DEV).

**Pourquoi ces tailles :** le standard mobile est 48×48px minimum (Apple HIG / Material Design), mais en POS restaurant avec les mains mouillées/grasses et l'écran à 30-50cm de distance, il faut aller plus grand. Les tests utilisateurs montrent que 60-70px est le sweet spot pour éviter les misclicks sous stress.

### 4.2 - Feedback tactile visuel

Chaque tap doit donner un feedback immédiat (<100ms). Pas d'animation longue, juste un changement d'état visuel :

- **Bouton menu item tapé :** léger scale down (transform: scale(0.95)) pendant 100ms + changement de background-color vers la couleur accent pendant 200ms. L'item apparaît dans le cart instantanément.
- **Item ajouté au cart :** brève surbrillance verte (200ms) sur la ligne du cart.
- **Bouton Payer :** couleur plus foncée au toucher (active state).
- **Erreur :** shake horizontal (3 oscillations, 300ms) + border rouge.
- **PAS de :** bounce, slide-in lent, fade-in/out > 200ms, confettis, transitions de page lentes.

### 4.3 - Zones de tap et espacement

Pas de hover states (pas de souris). Pas de tooltips. Pas de right-click.

L'espacement entre les boutons doit être suffisant pour qu'un doigt ne tape pas accidentellement sur le voisin. Minimum 8px de gap entre les boutons interactifs. Pour les boutons destructifs (supprimer ligne, annuler commande), mettre plus d'espace ou demander confirmation.

---

## 5. Couleurs et contraste

### 5.1 - Mode sombre par défaut

L'environnement restaurant est souvent tamisé le soir. Un écran blanc éclatant fatigue les yeux et est visible par les clients (distraction). Mode sombre comme base.

### 5.2 - Palette de tokens CSS

La palette suit la charte Pili (`docs/BRAND.md`). Source de vérité : `ui/src/styles/tokens.css`.

| Token | Valeur | Rôle |
|---|---|---|
| `--bg-primary` | `#0B0D0E` (Encre) | fond de l'écran |
| `--bg-secondary` | `#15181A` (Graphite) | en-tête, panier, cartes |
| `--bg-surface` | `#1F2327` (Ardoise) | boutons secondaires |
| `--border` | `#2B3035` | filets, bordures |
| `--text-primary` | `#F2F4F2` (Brume) | texte |
| `--text-secondary` | `#9AA3A8` | texte secondaire |
| `--selected-bg` / `--selected-fg` | Brume / Encre | sélection : onglet actif, table, méthode choisie |
| `--accent-primary` | `#D7FF3A` (Volt) | **l'action principale, une seule par écran** (PAYER, VALIDER, Enregistrer) |
| `--accent-warning` | `#FFB020` | en attente : pas encore en cuisine, nouvelle commande |
| `--accent-danger` | `#FF4D3D` | supprimer, erreur, FDM hors ligne |
| `--accent-success` | `#22C55E` | connecté, payé, rendu monnaie |
| `--accent-info` | `#00D4FF` | information, commandes en ligne |
| `--accent-training` | `#9B59B6` | mode formation, et rien d'autre |

Les prix et totaux sont en couleur neutre : la couleur est réservée aux actions et aux états. Tous les chiffres (prix, heures, quantités) sont en JetBrains Mono (`--font-mono`), alignés.

### 5.3 - Contraste

Ratio minimum WCAG AA : 4.5:1 pour le texte, 3:1 pour les éléments graphiques. Sur fond Encre (#0B0D0E), le texte Brume (#F2F4F2) donne un ratio de ~18:1. Les prix et totaux doivent être en bold pour être lisibles à 50cm.

---

## 6. Design des boutons menu items

Chaque bouton d'item dans la grille doit montrer exactement 3 informations, dans cet ordre de priorité visuelle :

1. **Nom du produit** - le plus grand, centré, 16-18px bold. En ZH (chinois simplifié) par défaut pour le staff, avec le nom FR en dessous en plus petit (pour la conformité fiscale et les clients).
2. **Prix** - en bas du bouton, 14-16px, couleur accent ou texte secondaire.
3. **Couleur de catégorie** - bordure gauche ou top de 4px dans la couleur de la catégorie.

**Pas d'images de produits.** Dans un restaurant asiatique avec 80-120 items, les images :
- prennent trop de place sur l'écran
- sont difficiles à maintenir à jour
- des produits similaires (maki saumon vs maki thon) sont visuellement identiques en photo
- ralentissent le rendu

Les noms textuels avec code couleur de catégorie sont plus rapides à scanner visuellement.

---

## 7. Paiement - CheckoutView

### 7.1 - Flow en minimum d'étapes

```
[PAYER] → [Encaisser 47,30 € en espèces] → Appel FDM → Ticket imprimé → retour aux Tables (2 s)
```

Le montant exact est déjà saisi à l'arrivée (espèces présélectionnées, arrondi de 5 centimes compris) et le bouton principal dit ce qu'il encaisse et comment (« Encaisser 47,30 € en espèces », « … par carte », « … en chèques-repas », « Encaisser la part 2 · 15,77 € »). Un paiement simple (cash, montant exact) fait donc 2 taps : Payer → Encaisser. Une touche du pavé ou un billet rapide remplace le montant. Carte sur terminal : Payer → Carte → Encaisser, puis l'approbation du terminal.

L'écran de fin affiche « OK » et le rendu monnaie en très grand, puis retourne seul aux Tables après 2 s (un tap, ou « Terminer », y retourne tout de suite). Il reste affiché seulement s'il y a quelque chose à lire ou à décider : ticket non imprimé (« Imprimer le ticket »), ticket numérique en attente, FDM non connecté, note d'un bon cadeau.

**Nombre de taps des gestes courants** (depuis l'écran de commande, en comptant la confirmation finale ; un nombre saisi sur un pavé compte pour un seul geste). Ces nombres sont des plafonds : une régression qui en ajoute un est un défaut.

| Geste | Taps | Enchaînement |
|---|---|---|
| Ajouter un plat | 1 | la grille (au pavé de code : le code puis OK, 2) |
| Ajouter 3 portions | 2 | le plat, puis le chiffre 3 (fenêtre de 3 s) |
| Envoyer en cuisine | 1 | le bouton principal Volt tant que des lignes attendent |
| Espèces, montant exact | 2 | Payer, Encaisser |
| Espèces avec un billet rapide | 3 | Payer, le billet, Encaisser |
| Carte (terminal ou saisie) | 3 | Payer, Carte, Encaisser (puis l'approbation du terminal) |
| Chèques-repas | 3 | Payer, Chèques-repas, Encaisser |
| Payer depuis l'écran Tables | 3 | appui long sur la table, Payer, Encaisser |
| Partage en 2, 3 ou 4 parts égales, espèces | 2 + N | Payer, le nombre de parts, puis Encaisser par part (la méthode reste choisie) |
| Partage en plus de 4 parts | 4 + N | Payer, Autre, le nombre, OK, puis Encaisser par part |
| Partage par plats, N parts | 2 + U + N | Payer, Par plats, une case par unité mise dans une part (U au total), Encaisser par part |
| Déplacer la commande | 2 | la puce de table, une table libre (puis 1 pour annuler) |
| Seconde remise du même gérant, même commande | 0 de plus | plus de pavé PIN pendant 2 minutes (§2.7) |
| Addition (pré-ticket) | 1 | le bouton du panier ; depuis Tables : appui long, Addition |
| Fin de service | 3 | le bouton de l'utilisateur, Fin de service, Fin de service (armé 5 s) |

### 7.2 - Boutons de paiement

Présenter les méthodes de paiement comme des gros boutons côte-à-côte, pas une liste déroulante :

```
┌──────────────┐  ┌──────────────┐
│   💶 CASH    │  │   💳 CARTE   │
│              │  │              │
└──────────────┘  └──────────────┘
```

### 7.3 - Paiement cash : montants rapides

Ne pas forcer la saisie manuelle du montant reçu. Proposer des boutons prédéfinis :

```
Total: €22.00

[€22.00 Exact]  [€25.00]  [€30.00]  [€50.00]  [Autre montant...]

Rendu monnaie: €3.00
```

### 7.4 - Partager l'addition (audit P3, P4, P6)

Le système fait les calculs, le staff choisit seulement comment partager. Le partage reste UNE vente : un seul ticket N, plusieurs lignes de paiement, aucun événement de scission de table. Trois façons :

- **Parts égales** : les boutons 2, 3, 4, ou « Autre » qui ouvre un pavé (2 à 20 parts). Les centimes qui restent vont aux premières parts. Quand la table a des couverts (§3.1), le bouton du même nombre est marqué, « N couverts » est rappelé et le pavé d'« Autre » commence à N ; la caisse ne partage jamais d'elle-même, une table payée d'un coup reste un seul geste.
- **« Par plats »** : les lignes de la commande sont listées, chaque unité se met dans la part (une ligne de 3 peut donner 1 unité à une part). La part est la somme des unités choisies, les remises et les lignes offertes réparties au prorata, les centimes de reste sur les premières unités ; le bouton dit « Encaisser la part 1 · 18,40 € ». Les lignes déjà payées sont grisées. Les unités ne servent qu'à l'écran : rien n'en entre dans la requête signée.
- Le nombre de parts est figé dès qu'une part est prise ; « Annuler les parts » rend la main, et « Retour à la commande » garde les parts déjà saisies (le panier affiche « Parts déjà saisies : N »).

Après chaque part, une seule ligne « Reste 12,30 € : » suit, avec les méthodes encore permises (Espèces, Carte…). **La méthode de la part précédente reste choisie** pour la suivante, avec le reste déjà saisi : chaque part suivante est donc le seul bouton principal. Toucher une autre méthode de la ligne « Reste » la choisit.

**Plusieurs parts en espèces.** La requête signée ne porte qu'UNE ligne de paiement en espèces, arrondie (description POS v2.2, §7.1 : les paiements en espèces « apparaissent comme une seule ligne de paiement sur le ticket, ARRONDIE », et le total des arrondis ne dépasse pas 2 centimes). L'écran laisse donc chaque ami payer en espèces avec son propre montant donné et son propre rendu, puis fusionne les contributions en cette ligne unique à la signature ; l'arrondi de 5 centimes est réparti pour que les montants dus s'additionnent exactement à la ligne arrondie. L'écran de fin montre le rendu de la dernière part en espèces.

### 7.5 - Arrondi cash

L'arrondi belge (5 centimes) doit être automatique et visible. Afficher clairement le montant arrondi à côté du montant exact.

### 7.6 - Bon cadeau : lecteur de code-barres (audit P7)

Le bon cadeau imprimé (et son DUPLICATA) porte un code-barres CODE128 des 12 chiffres du code, sous le code en grand. Dans la fenêtre « Bon cadeau » du paiement, un lecteur USB en mode clavier (les chiffres puis Entrée) remplit le code et lance la recherche tout de suite, sans toucher « Chercher » ; taper sur le pavé marche toujours. Le code reste masqué à l'écran (`**** **** 9095`), un code lu qui n'a pas 12 chiffres est refusé avec un message, et les essais restent limités par le moteur (5 échecs par minute). Pendant que la fenêtre est ouverte, elle prend les lectures pour elle : un code-barres de bon n'est jamais pris pour un badge du personnel, et le lecteur marche même si le réglage des badges est coupé.

---

## 8. Gestion des erreurs et statut FDM

### 8.1 - Indicateur FDM permanent

Le statut de connexion FDM doit être visible en permanence dans le header, pas dans un menu caché :

- 🟢 Vert = connecté et opérationnel
- 🟡 Jaune = warning (buffer > 70%, certificat expire bientôt)
- 🔴 Rouge = erreur (déconnecté, buffer full, locked) → **bloque les transactions**

Chaque voyant est un vrai bouton (zone de tap de 48 px, le point reste petit) : un tap ouvre une fiche avec le nom de
l'appareil, son état en toutes lettres, la raison (jamais seulement une infobulle), l'heure de la dernière vérification
et « Réessayer maintenant », qui relance la vérification de cet appareil seulement. Les lignes de démarrage de l'écran
de connexion ouvrent la même fiche.

### 8.2 - Erreurs non-bloquantes vs bloquantes

- **Non-bloquantes** (warnings) : bandeau jaune en haut de l'écran, auto-dismiss après 10 secondes, ne bloque pas le flow.
- **Bloquantes** (FDM déconnecté, buffer full) : voile semi-transparent sur les écrans de vente, **sous le header de 64 px** (Tables, Rapports, Config et Verrouiller restent accessibles), message clair, pas de moyen de contourner. Les ventes sont impossibles tant que le problème n'est pas résolu.
- **Toasts** : en haut au centre, juste sous le header, 560 px au plus, empilés vers le bas, trois au maximum (les plus récents). Le conteneur ne prend jamais un tap : un tap traverse jusqu'à l'écran dessous. Info, succès et avertissement disparaissent seuls ; une **erreur reste jusqu'à ce qu'on la touche** (« Touchez pour fermer »), et c'est le seul élément qui prend le tap. Un toast de succès peut porter un seul bouton (« Annuler » 5 s après un déplacement de table) ; il prend alors le tap sur ce bouton seulement.
- **Verrouillage pour inactivité** : le verrouillage automatique (5 minutes) appartient au moteur. 30 secondes avant, un bandeau ambre sous le header (et sous le bandeau de formation) annonce « Verrouillage dans 30 s · touchez l'écran pour rester connecté » avec un compte à rebours ; le moteur envoie l'événement `idle-lock-warning`, les deux interfaces ne font que compter ce qu'il annonce. Tout toucher compte comme activité et fait disparaître le bandeau. Après le verrouillage, l'écran de connexion garde la ligne « Caisse verrouillée après 5 min d'inactivité » au-dessus des tuiles jusqu'à la prochaine connexion. Une connexion mène directement à Tables, sauf après un verrouillage pour inactivité : le même opérateur retrouve sa commande ouverte (jamais une commande en ligne à son paiement), un autre opérateur, un verrouillage manuel ou une commande fermée mènent à Tables (audit T6).

### 8.3 - Messages d'erreur

Pas de jargon technique. Le staff ne sait pas ce qu'est un "buffer" ou un "certificat mTLS".

| Code technique | Message affiché |
|---------------|----------------|
| BUFFER_FULL | "La boîte noire est pleine. Contactez votre technicien." |
| FDM_NOT_OPERATIONAL | "La boîte noire ne fonctionne pas. Impossible d'enregistrer les ventes." |
| UNKNOWN_POS | "Cette caisse n'est pas reconnue par la boîte noire. Contactez votre technicien." |
| CLIENT_CERT_EXPIRED | "Le certificat de la boîte noire a expiré. Contactez votre technicien." |
| Connection timeout | "Connexion perdue avec la boîte noire. Vérifiez le câble réseau." |

---

## 9. Mode Training

Le mode training doit être **visuellement impossible à confondre** avec le mode normal. Exigence légale SCE 2.0.

- Background color change : `--bg-primary` passe à `#2D1B4E` (violet foncé) et les surfaces suivent (`:root.training` dans `tokens.css`)
- Bannière permanente, juste sous l'en-tête : **"MODE FORMATION · Les transactions ne sont PAS enregistrées"**
- Watermark diagonal semi-transparent "TRAINING" sur tout l'écran
- Tous les tickets imprimés portent "TRAINING" en haut et "THIS IS NOT A VALID VAT RECEIPT" en bas
- **Le commutateur est gardé** (audit R4). Dans Settings, un premier toucher arme seulement l'interrupteur (il devient rouge et pose la question « Activer le mode formation ? », ou « Désactiver le mode formation ? »), il se désarme seul après 5 s. Le second toucher dans les 5 s désactive le mode, ou ouvre le PIN du gérant pour l'activer (aide : « Les ventes suivantes ne seront pas comptées. »). Le moteur vérifie ce PIN (`set_training_mode` accepte `operatorPin`, obligatoire pour activer) : une session de gérant laissée ouverte ne suffit pas à lancer des ventes non comptées.

---

## 10. Typographie et CJK

Tokyo Sushi Bar a besoin d'afficher des noms en chinois simplifié (三文鱼卷, 拉面, 毛豆). Le staff est sinophone et le menu utilise des termes japonais transcrits en chinois. La police doit supporter le français ET le chinois.

**Police :** Noto Sans + Noto Sans CJK SC. Noto Sans et JetBrains Mono (chiffres) sont embarqués dans l'application (`@fontsource`, la caisse est hors ligne) ; Noto Sans CJK vient de l'image (`fonts-noto-cjk`).

**Fallback stack :** `'Noto Sans', 'Noto Sans CJK SC', 'PingFang SC', 'Microsoft YaHei', sans-serif`

L'interface est en français par défaut, et passe en chinois (ou en anglais) dans Config. Les noms des produits et des catégories restent bilingues : la langue de l'interface en principal, l'autre langue (chinois ou français) en secondaire, pour que le staff francophone et sinophone lise le même bouton :
```
┌───────────────┐
│ Saumon avocat │  ← nom principal (langue de l'interface), 17px bold
│ 三文鱼牛油果卷  │  ← nom secondaire (l'autre langue), 13px, text-secondary
│     5,60      │  ← prix (mono, neutre)
└───────────────┘
```

---

## 11. Accessibilité tactile

- **Pas de double-tap requis** pour aucune action courante
- **Pas de swipe obligatoire** - toujours fournir un bouton alternatif pour les actions swipe
- **Pas de long-press** comme seul moyen d'accéder à une fonction. Le seul appui long de l'application est celui de la carte de table (500 ms, audit T7) : un raccourci vers « Ouvrir · Addition · Déplacer · Payer », qui existent tous sans lui (toucher la table, bouton Addition du panier, puce de table, PAYER). Un appui court fait l'action de toujours, il n'y a pas de menu contextuel, et un doigt qui glisse est un défilement, pas un appui long.
- **Scroll** : inertie naturelle, scrollbar visible fine (4px), zone de scroll clairement délimitée
- **Zone morte** de 16px en bordure d'écran (les touch panels sont moins précis sur les bords)

---

## 12. Responsive - un seul form factor

Ce POS est conçu pour un seul écran : **15.6" 1920×1080 touch**. Pas de responsive multi-device. Pas de version mobile. Pas de version tablette.

Cela simplifie énormément le CSS : pas de media queries, pas de breakpoints, layout fixe optimisé pour cette résolution exacte. Le Tauri app tourne en fullscreen kiosk mode.

---

## 13. Anti-patterns - ce qu'il ne faut PAS faire

- ❌ Hamburger menu / menu caché
- ❌ Carousel / slider pour les catégories (utiliser des tabs scrollables)
- ❌ Modal de confirmation pour chaque ajout d'item au cart
- ❌ Images de produits dans la grille menu
- ❌ Animations longues (> 200ms)
- ❌ Tooltips
- ❌ Hover effects (pas de souris)
- ❌ Scroll horizontal sur le cart
- ❌ Texte plus petit que 12px
- ❌ Boutons plus petits que 48×48px
- ❌ Popups qui bloquent le flow pour des informations non-critiques
- ❌ Double-tap ou long-press comme seul moyen d'accéder à une action
- ❌ Fond blanc brillant (fatigue visuelle en environnement sombre)

---

## Sources

Ce guide est une synthèse des analyses UX suivantes :

- **Creative Navy** - "The Design Principles Of The POS System" (uxjournal/Medium). Agence UX spécialisée produits professionnels. Points clés : design pour des heures d'utilisation, muscle memory des caissiers, chaque micro-friction ×1000, les design patterns web ne s'appliquent pas au POS, les images de produits sont un faux ami.

- **Bright Inventions** - "Payment in POS Design: UI/UX Best Practices". Points clés : animations uniquement informatives, icônes avec labels, hiérarchie de boutons (principal plus grand), formulaires empilés verticalement, "Don't Make Me Think" appliqué aux pourboires et split payments.

- **DevPro** - "Designing a POS System: 10 UX Tactics". Points clés : cas Subway (POS tellement intuitif que les formations ont été annulées), designer pour un nouveau serveur pas pour un power user, rush hours = 2-3x plus de transactions, les 10% de gauchers, surcharge sensorielle en environnement restaurant.

- **Agente Studio** - "POS System Design: Principles, Examples". Points clés : KISS principle, réduire la charge cognitive, findability = clé de l'efficacité.

- **UXmatters** - "Designing Intuitive POS Touch Interfaces". Points clés : learnability et discoverability en quelques secondes, indiquer visuellement les éléments interactifs, tester sur des utilisateurs naïfs.

- **Figma Community** - Restaurant POS UI kits (pour inspiration visuelle).
