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
- Le bouton principal d'action est TOUJOURS en bas à droite du cart.
- Le cart/sidebar est TOUJOURS à droite de l'écran.
- Les catégories sont TOUJOURS en haut de la grille menu.
- La barre de statut (FDM, user, heure) est TOUJOURS en haut de l'écran.
- Ne jamais déplacer un élément entre les mises à jour.

---

## 3. Layout principal - OrderView (écran le plus utilisé)

C'est l'écran où le staff passe 80% de son temps. Il doit être parfait.

```
┌──────────────────────────────────────────────────────────────────────┐
│  FdmStatus    │  User: Ahmed  │  Table: 12  │  14:32  │  [🔒 Lock] │  ← Header bar (fixe, 48px)
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

**Proportions :** menu grid = ~65% de l'écran, cart sidebar = ~35%. Sur 1920px, ça donne ~1250px pour le menu et ~670px pour le cart.

**Grille menu :** 4 colonnes de boutons items. Chaque bouton fait minimum 140x100px. Scrollable verticalement avec inertie tactile.

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

```css
:root {
  /* Backgrounds */
  --bg-primary: #1a1a2e;        /* fond principal */
  --bg-secondary: #16213e;      /* fond cart sidebar */
  --bg-surface: #0f3460;        /* fond boutons menu */
  --bg-hover: #533483;          /* état actif/pressed */

  /* Text */
  --text-primary: #e8e8e8;      /* texte principal */
  --text-secondary: #a0a0b0;    /* texte secondaire (labels, hints) */
  --text-on-accent: #ffffff;    /* texte sur boutons accent */

  /* Accent */
  --accent-primary: #e94560;    /* bouton Payer, actions principales */
  --accent-success: #2ecc71;    /* confirmation, paiement réussi */
  --accent-warning: #f39c12;    /* avertissements FDM */
  --accent-danger: #e74c3c;     /* erreurs, suppressions */
  --accent-training: #9b59b6;   /* mode training (violet distinctif) */

  /* Categories (couleurs pour les onglets catégories) */
  --cat-sushi: #e94560;
  --cat-maki: #f39c12;
  --cat-sashimi: #3498db;
  --cat-ramen: #2ecc71;
  --cat-boissons: #9b59b6;
  --cat-desserts: #e67e22;

  /* Spacing */
  --space-xs: 4px;
  --space-sm: 8px;
  --space-md: 16px;
  --space-lg: 24px;
  --space-xl: 32px;

  /* Border radius */
  --radius-sm: 6px;
  --radius-md: 10px;
  --radius-lg: 16px;

  /* Typography */
  --font-family: 'Noto Sans', 'Noto Sans CJK SC', sans-serif;
  --font-size-xs: 12px;
  --font-size-sm: 14px;
  --font-size-md: 16px;
  --font-size-lg: 20px;
  --font-size-xl: 28px;
  --font-size-price: 18px;      /* prix sur les boutons items */
  --font-weight-normal: 400;
  --font-weight-bold: 700;
}
```

### 5.3 - Contraste

Ratio minimum WCAG AA : 4.5:1 pour le texte, 3:1 pour les éléments graphiques. Sur fond sombre (#1a1a2e), le texte blanc (#e8e8e8) donne un ratio de ~14:1. Les prix et totaux doivent être en bold pour être lisibles à 50cm.

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
[PAYER] → Choix méthode (Cash/Carte) → Montant reçu (si cash) → Appel FDM → Ticket imprimé
```

Maximum 3 taps pour un paiement simple (cash, montant exact) : Payer → Cash → Exact.

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

### 7.4 - Split payment

Le système fait les calculs, le staff saisit seulement le nombre de parts. Afficher le solde restant après chaque paiement partiel.

### 7.5 - Arrondi cash

L'arrondi belge (5 centimes) doit être automatique et visible. Afficher clairement le montant arrondi à côté du montant exact.

---

## 8. Gestion des erreurs et statut FDM

### 8.1 - Indicateur FDM permanent

Le statut de connexion FDM doit être visible en permanence dans le header, pas dans un menu caché :

- 🟢 Vert = connecté et opérationnel
- 🟡 Jaune = warning (buffer > 70%, certificat expire bientôt)
- 🔴 Rouge = erreur (déconnecté, buffer full, locked) → **bloque les transactions**

### 8.2 - Erreurs non-bloquantes vs bloquantes

- **Non-bloquantes** (warnings) : bandeau jaune en haut de l'écran, auto-dismiss après 10 secondes, ne bloque pas le flow.
- **Bloquantes** (FDM déconnecté, buffer full) : overlay fullscreen, fond semi-transparent, message clair, pas de moyen de contourner. Le POS est inutilisable tant que le problème n'est pas résolu.

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

- Background color change : remplacer `--bg-primary` par `--accent-training` (violet)
- Bannière permanente en haut : **"⚠️ MODE FORMATION - Les transactions ne sont PAS enregistrées"**
- Watermark diagonal semi-transparent "TRAINING" sur tout l'écran
- Tous les tickets imprimés portent "TRAINING" en haut et "THIS IS NOT A VALID VAT RECEIPT" en bas

---

## 10. Typographie et CJK

Tokyo Sushi Bar a besoin d'afficher des noms en chinois simplifié (三文鱼卷, 拉面, 毛豆). Le staff est sinophone et le menu utilise des termes japonais transcrits en chinois. La police doit supporter le français ET le chinois.

**Police :** Noto Sans + Noto Sans CJK SC (Google Fonts, gratuit, couverture complète chinois simplifié).

**Fallback stack :** `'Noto Sans', 'Noto Sans CJK SC', 'PingFang SC', 'Microsoft YaHei', sans-serif`

Les noms d'items affichent le chinois en principal (pour le staff) et le français en secondaire (pour la conformité et les clients) :
```
┌─────────────┐
│ 三文鱼卷 ×8  │  ← nom principal (ZH), 16px bold - le staff lit ça
│ Salmon Roll  │  ← nom secondaire (FR), 12px, text-secondary
│        12.00 │  ← prix
└─────────────┘
```

---

## 11. Accessibilité tactile

- **Pas de double-tap requis** pour aucune action courante
- **Pas de swipe obligatoire** - toujours fournir un bouton alternatif pour les actions swipe
- **Pas de long-press** comme seul moyen d'accéder à une fonction
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
