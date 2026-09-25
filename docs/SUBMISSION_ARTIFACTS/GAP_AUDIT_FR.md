# Audit de conformité - écarts entre exigences cert et dossier

**Date :** 2026-05-14
**Auditeur :** rescan exhaustif après remarque utilisateur sur les screenshots manqués.

Honnêteté complète : le dossier actuel (`SUBMISSION_ARTIFACTS/*_FR.md`) est constitué d'**annexes** propres et bien documentées. Mais le SPF Finances n'attend pas seulement des annexes : il attend que le **manuel de certification de 110 pages** soit rempli case par case (chaque ☐ coché, chaque champ libre rempli), avec les annexes comme support.

Je n'avais pas planifié cette partie.

Le présent document liste section par section :
- Ce que demande le manuel
- Ce qu'on a (ou pas) côté dossier ET côté code
- Action requise pour atteindre 100 %

Source : `docs/20250307_POS_CERTIFICATION_MANUEAL_FRV1.1/20250307_FR - Manuel de certification_POS_V11.md`.

---

## §I - DOCUMENTATION REQUISE

### §I.A Formulaire de demande
- ☐ Formulaire complété - À FAIRE (étape opérateur, formulaire PDF source)
- ☐ Formulaire signé - À FAIRE (signature électronique qualifiée)

### §I.B Résultats de test
- ☐ Document PDF complété - **GAP MAJEUR**. Le manuel attend un PDF de tests pré-remplis pour chaque UC (M110, M111, M121, M123, etc.). On a `JOURNAL_TESTS_PRE_SOUMISSION_FR.md` mais c'est un placeholder pour la répétition Checkbox, pas le PDF de résultats UC par UC avec JSON et tickets joints en annexe.

### §I.C Manuels du POS
- ☐ Manuel d'utilisateur - **OK** : `MANUEL_UTILISATEUR_FR.md`
- ☐ Manuel de configuration/installation - **OK** : `MANUEL_INSTALLATION_FR.md`
- ☐ Manuel de programmation - **OK** : `MANUEL_PROGRAMMATION_FR.md`
- ☐ Manuel de service (si existant) - N/A pour MVP single-developer
- ☐ Schéma d'assemblage - **GAP** : non produit. Le manuel exige un schéma de montage physique POS + FDM + imprimante + tiroir + réseau.

### §I.D Hash SHA-1 du logiciel
- ☐ Le SHA-1 est envoyé en pièce jointe - **OK** (script `scripts/build_and_hash.sh` produit l'empreinte). À exécuter au moment du build cert.

### §I.E Documents d'identification
**Producteur** :

- Nom de l'entité juridique - **OK** dans `LISTE_DISTRIBUTEURS_FR.md`
- Nom commercial - **OK**
- Structure juridique - **PARTIEL** (à confirmer "indépendant" vs SRL)
- Adresse - **PARTIEL** (à compléter au formulaire final)
- Numéro TVA - **À COMPLÉTER**
- Site web - **OK** (nuagemagique.dev)
- Représentants autorisés - **OK** (Bruno Moyse)
- Contact technique - **OK** (Bruno Moyse, +tél, cloud@nuagemagique.dev)

**Importateurs/Distributeurs** : producteur = distributeur, à indiquer.

### §I.F Profil de l'entreprise
**OK** dans `PROFIL_ENTREPRISE_FR.md`. Couvre les sous-sections :
- Contrôle interne (enregistrement, plugins matériels)
- Organisation interne (nombre de devs, internalisation)
- Distributeurs (dispositions contractuelles)
- Support technique
- Analyse de risques (incl. RGPD)
- Contrôle externe (autres certificats, clients, assurance)

---

## §II - DESCRIPTION DU POS

### §II.A Identification du logiciel
- Nom du POS, Modèle, N° version, Langage(s) - **OK** dans `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md` et `LISTE_FONCTIONNALITES_FR.md`

### §II.B Déclaration d'accès à la base de données
- ☐ Les utilisateurs n'ont accès qu'en lecture seule - **OK** documenté dans `SECURITE_BD_FR.md`

### §II.C Caractéristiques de la base de données
- DB Centrale / Locale / Les deux - **OK** (Locale, SQLite) dans `SCHEMA_BD_FR.md`
- DB + POS cloud-based - **OK** (Non, embarqué)
- Identification DBMS, n° version - **OK** (SQLite 3.x)
- Chemin d'accès - **OK** (`/var/lib/moyun-pos/pos.db`)
- Aperçu des tables, colonnes, types, descriptions - **OK** dans `SCHEMA_BD_FR.md`
- ☐ La liste contient toutes les colonnes - **OK**
- ☐ Description brève et claire - **OK**

**Informations supplémentaires SPF Finances :**

- ☐ Description format DB - **OK** (`SCHEMA_BD_FR.md`)
- ☐ Sécurité de la base de données - **OK** (`SECURITE_BD_FR.md`)
- ☐ Données utilisées/créées - **OK** (`FEUILLE_ACQUISITION_DONNEES_FR.md`)
- ☐ Aperçu tables events - **OK** (table `fiscal_events` documentée)
- ☐ Disposition des tables - **OK**
- ☐ Relations entre tables - **GAP** : Diagramme ER attendu, on a la liste textuelle mais pas le visuel.

**Captation de la base de données :**

- ☐ Agents SPF peuvent accéder/copier - **OK** (procédure dans `FEUILLE_ACQUISITION_DONNEES_FR.md`)
- ☐ URL, nom d'utilisateur, mot de passe - **GAP** : besoin de définir l'accès pour le rendez-vous d'admission (compte SPF sur la machine de test)
- ☐ Données exportées doivent être converties ? - À déclarer Non (SQLite directement lisible)
- ☐ Scripts/programmes de conversion - N/A

**Sécurité DB :**

- ☐ Audit trail (timestamps, users, PK) - **OK** documenté
- Mesures contre manipulation frauduleuse - **OK** (`DECLARATION_FONCTIONS_INTERDITES_FR.md` + `SECURITE_BD_FR.md`)

### §II.D Garantie du produit
- Garanties machine test / SCE installé identiques - **GAP** : à décrire (notre garantie = build reproductible + SHA-1 vérifiable + tag git immutable). Pas dans le dossier.

### §II.E Mises à jour
- Procédure de mise à jour - **PARTIEL** : `MANUEL_INSTALLATION_FR.md` mentionne update procedure, mais le manuel cert demande aussi :
  - Comment les MAJ sont déployées sur la machine de test
  - Solution d'accès à distance (TeamViewer, AnyDesk, etc.) - **GAP** : aucune solution déclarée. À choisir + déclarer.

### §II.F Matériel (tableau ☐ "Délivré" / "Production")
**GAP partiel** : on a la liste matériel dans `LISTE_FONCTIONNALITES_FR.md` mais pas formatée selon le tableau spécifique du manuel avec colonnes "Délivré machine-test à l'Intake" vs "Production". À reformater.

Items à cocher dans le tableau :
- POS (Ordinateur fixe / portable / tablette / kiosk) - **kiosk**
- FDM - oui
- Imprimante thermique - oui
- Router/Hub, Câbles, Wifi - à préciser (le matériel réseau n'a pas été cadré dans le dossier)
- Périphériques entrée : souris, clavier, scanner, lecteur carte, clés Dallas - **non** pour TSB
- Périphériques sortie : écran utilisateur, écran cuisine, balance - **non** pour TSB
- Livraison, robot, commande en ligne - **non**
- Tiroir-caisse connecté - **oui**

### §II.G Schéma d'assemblage
- ☐ Schéma d'assemblage fourni en annexe - **GAP MAJEUR** : aucun schéma produit. Le manuel veut un diagramme physique avec ports utilisés. À dessiner (Draw.io, Excalidraw, ou main).

---

## §III - FONCTIONNALITÉS DU POS

### §III.A Paramètres d'installation
- ☐ Liste claire et complète - **OK** dans `MANUEL_INSTALLATION_FR.md`
- ☐ Paramètres définis indiqués - **OK**
- ☐ Paramètres définis peuvent être fixes - À déclarer (Oui)
- ☐ Logs stockés Centralement / Localement - À déclarer (Localement)
- ☐ Logs ne peuvent pas être supprimés - **OK** documenté

### §III.B Configuration Internet
- ☐ Le POS nécessite Internet - À déclarer (Non requis - le FDM oui mais pas le POS)
- ☐ Internet par Ethernet / Wifi / Les deux - À déclarer (Ethernet pour la prod, les deux supportés)
- Besoins spécifiques mise en place réseau - À décrire (IP statique pour FDM)

### §III.C Utilisateurs & Mots de Passe
**Tableau à remplir** :

- OS login - À déclarer (kiosk Linux, mot de passe simple pour test)
- POS login - **OK** : INSZ + PIN, doc dans `PROFILS_UTILISATEURS_FR.md`
- Administrateur, Serveur, Technicien (NISS 00000000097), Kiosk (NISS 00000000029), Autre - **À FORMATER** dans le tableau exact

**Gestion utilisateurs/rôles :**

- ☐ Description de la gestion + rôles - **OK** (`PROFILS_UTILISATEURS_FR.md`)
- ☐ Description des droits d'accès - **OK**

### §III.D Connexion du POS au FDM
- Mode : Autonome / LAN / Cloud / Configurable - À déclarer (**LAN**)
- Load-balancing - À déclarer (Non)
- POS compatible avec d'autres marques FDM - À déclarer (Oui par conception - protocole standard ; référence Checkbox CBX01)
- Terminaux identifiés par posId - À déclarer (Oui)

### §III.E Connexion au FDM
- ☐ La connexion au POS est expliquée - **OK** dans `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md`
- Logiciel intermédiaire externe : Aucun / Fabricant FDM / Autre - À déclarer (Aucun)

### §III.F FDM Non-Opérationnel
- ☐ La déconnexion du FDM est expliquée - **OK** dans `STRATEGIE_COMPTEURS_FR.md` §4 et `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md` §5

### §III.A.1 Events possibles (tableau)
- N - Sales : Obligatoire (Disponible / Imprimable papier / digital / Copie ?)
  - **OK** : disponible, imprimable papier, digital optionnel, copie via M160
- R - Report : Obligatoire (idem)
  - **OK** : disponible, imprimable papier
- S - Social : Obligatoire (N/A pour imprimable)
  - **OK** : disponible
- P - Pro Forma : Optionnel - À cocher si supporté
  - **OK** : Disponible, imprimable
- T - Training : Optionnel - À cocher
  - **OK** : disponible, imprimable
- I - Invoice : Optionnel - À cocher
  - **PARTIEL** : signInvoice implémenté côté pos-fiscal, pas d'UI flow
- C - Copy : Optionnel - À cocher
  - **OK** : disponible, copies imprimables
- F - Financial : Optionnel - À cocher
  - **OK** : money_in_out + drawer_open

### §III.B Numérotation d'events
- PosFiscalTicketNo : Globale / Par type d'event / Par Terminal
  - **OK** : **Par type d'event** (documenté dans `STRATEGIE_COMPTEURS_FR.md`)
- Numérotation des tickets : idem
  - **OK** : par type d'event

### §III.C Fonctionnalités interdites
- ☐ Le producteur déclare que de telles fonctionnalités ne sont pas disponibles - **OK** (`DECLARATION_FONCTIONS_INTERDITES_FR.md` signée par le producteur)

### §III.D Fonctions obligatoires
- Enregistrement zéro - À décrire (Oui possible via signSale avec transactionTotal=0 ?)
  - **GAP** : à clarifier dans la doc. Le POS le permet-il ? Sinon le déclarer.

### §III.E Enregistrement de la présence
- ☐ Description utilisation - **OK** dans `MANUEL_UTILISATEUR_FR.md` (work in/out)
- ☐ Lié aux boutons Work In/Out - À déclarer (Oui)

### §III.F Fonctionnalités avancées (gros tableau de 40+ lignes)
À cocher pour chaque ligne si supportée. La plupart concerne Advanced Set. Notre dossier indique :
- Normal (N) : Obligatoire - **OK**
- Remboursement total : **OK** (M111)
- Remboursement partiel : **OK** (M112B déclaré)
- Report (R), Social (S) : Obligatoire - **OK**
- Pro Forma, Commande en attente, Prebill - **OK** (M121, M123)
- Gestion cost centers : Attribution / Division / Fusion - **NON** déclaré (Advanced UC pas en MVP)
- Gestion d'addition (Division table / addition / parts égales) - **NON**
- Invoice (I) / Note de crédit - **PARTIEL** (Peppol planifié)
- Copy (C) : **OK**
- Training (T) : **OK**
- Gestion cash flows / tiroir-caisse : **OK** (M130, M131)
- Moyens de paiement : Cash + Carte débit + Carte crédit - **OK**
- Crédit client / Facturation hôtel / Chèque repas - **NON**
- Changement de paiement (M132) - **NON** (déclaré reporté)
- Vouchers multi-/uni-usage - **NON**
- Devise étrangère - **NON** (déclaré : EUR uniquement)
- Arrondissement cash - **OK** (arrondi belge implémenté)
- Arrondissement carte EFT - **NON** (déclaré : par carte = pas d'arrondi)
- Réductions / Produits composés / Modif prix ligne / event / multiple - **NON** pour MVP
- Corrections (UC230) - **NON**
- Article offert / Happy Hour - **NON**
- Gestion clients (réduction membre, mensualités) - **NON**
- Prix libre, prix au poids - **NON**
- Ticket de service (bar/cuisine) - **OK** (kitchen printer)

**Conformité Peppol B2B** (encadré obligatoire) : **OK** dans `DECISION_PEPPOL_AP_FR.md` mais à recopier dans le formulaire avec l'option choisie (Storecove).

### §III.G CostCenterChange (Gestion de Table)
- Permet costCenterChange vers ROOM - À déclarer (Non pour MVP)
- Recalcul priceChanges - À déclarer (Non)
- Liens DB décrits - N/A si non supporté

### §III.H Emission tickets numériques
- **GAP** : la spec dit que l'émission numérique est obligatoire. On a uniquement papier dans le MVP. À déclarer / planifier.

### §III.I Terminaux et appareils
- Plusieurs périphériques sur un terminal (roaming) - À déclarer (Non - single device)

### §III.J Périodes
- bookingDate + bookingPeriod - À déclarer + décrire la config

### §III.K Tiroirs-Caisse
- Disponible ? Oui
- Physique ? Oui
- Plusieurs tiroirs ? Non
- Virtuel ? Non
- Hardcodé pour mutations ? Non

### §III.L Rapports
- Z : un seul par bookingDate - **OK**
- Financier Z et Utilisateurs Z liés/ensemble - **OK** (atomic M184)
- Émis automatiquement - À déclarer (Non, déclenché par l'opérateur)
- Autres rapports que X/Z - **OK** (Non)

### §III.M Intégration tiers
- Deliveroo / TakeAway / Uber Eats / Autres - ☐ Non (aucune intégration tierce active dans le build cert)

### §III.N Lignes de produit prédéfinies
**GAP initial (résolu)** : le manuel attend que le POS soit pré-chargé avec une liste exacte de produits de test (Dry Martini, Burger of the Chef, Carpaccio Beef, Catch of the day, Dame Blanche, etc. avec productId 10006, 12007, etc.). Le seed `pos.db` de développement contenait au départ le menu d'un client pilote, différent de cette liste.

**Résolu** : un seed dédié "machine de test SPF" (`src-tauri/seed_spf_test.sql`) charge exactement la liste produits du manuel + leurs IDs/TVA sur la machine d'admission - voir `REPONSES_MANUEL_CERT_FR.md` §III "Lignes de produits prédéfinies" et `LIGNES_PRODUITS_FR.md`.

---

## §IV - PROCÉDURE DE COMMUNICATION GRAPHQL (le gros bloc)

Le manuel a une page par UC avec :
- Description du scénario à jouer
- Tableau "Champs à vérifier" avec 10-20 checkboxes par UC
- Tableau de calcul TVA attendu
- Résultat = capture du JSON request + response + ticket imprimé

UCs documentés dans le manuel :
- M110 signSale (vente normale)
- M111 signSale Refund (remboursement total)
- M112 signSale RefundPartial (remboursement partiel)
- M121 signOrder (Pro Forma)
- M122 signCostCenterChange
- M123 signPrebill
- M130 signMoneyInOut
- M131 signDrawerOpen
- M132 signPaymentCorrection
- M140/141 signWorkIn/Out
- M150 signInvoice
- M160 signCopy
- M171 signSale Training
- M180-M184 Rapports
- + Advanced UC (UC200, UC220-224, UC230, UC240-243, UC250-252, UC260/261, UC270/271, UC296)

**Statut actuel :**

- Les **fixtures sont chargées et parsées** par nos tests (`uc_fixtures.rs`)
- L'**implémentation est e2e-tested** pour 12 UC Basic Set + 5 Reports
- **MAIS** : on n'a pas le **PDF de résultats** rempli UC par UC avec captures du JSON et tickets correspondants en annexe.

**GAP MAJEUR** : produire un document `RESULTATS_TESTS_UC_FR.pdf` qui, pour chaque UC obligatoire (au minimum les 22 Basic Set) :
1. Coche les "Champs à vérifier"
2. Joint le JSON request capturé
3. Joint le JSON response du FDM
4. Joint le ticket imprimé (papier ou image)

Cela nécessite :
- L'app fonctionne contre un FDM (mock ou virtuel)
- On capture chaque exécution
- On produit le document

---

## §V - COMPORTEMENT AUX RÉPONSES DU FDM

**Tableau de 30 messages FDM** (avertissements + erreurs) avec colonne "Sur le POS" ☐ OUI ☐ NON.

Cases à cocher par message (M=Mandatoire, O=Optionnel, N=non-affichable) :

| Code | Nature | Affichage | Statut |
|---|---|---|---|
| CLIENT_CERT_NEAR_EXPIRATION | Warning | O | UI mappé dans `FdmStatus.vue` mais non testé |
| CLIENT_CERT_EXPIRED | Warning | M | **GAP** : à tester + capturer |
| RTC_SYNC_FAILED | Warning | O | **GAP** |
| UPDATE_URLS_FAILED | Warning | O | **GAP** |
| UPDATE_TRUST_CERT_FAILED | Warning | O | **GAP** |
| UPDATE_TASK_LIST_FAILED | Warning | O | **GAP** |
| TASK_FEEDBACK_FAILED | Warning | O | **GAP** |
| NOP_FAILED | Warning | O | **GAP** |
| BUFFER_NEAR_FULL | Warning | M | **GAP** : à tester + capturer |
| SERVER_CERT_RESOLVE_FAILED | Warning | O | **GAP** |
| TRANSACTION_UPLOAD_FAILED | Warning | O | **GAP** |
| INITIALIZATION_FAILED | Warning | O | **GAP** |
| RTC_NOT_INITIALIZED | Warning | N | non-affichable, OK |
| CORRUPT_RECORD_ENCOUNTERED | Warning | N | non-affichable, OK |
| UPDATE_PARAMS_FAILED | Warning | N | non-affichable, OK |
| UPDATE_CLIENT_CERT_FAILED | Warning | N | non-affichable, OK |
| UPDATE_VAT_RATES_FAILED | Warning | N | non-affichable, OK |
| UPDATE_POS_ALLOWLIST_FAILED | Warning | N | non-affichable, OK |
| DUPLICATE_REQUEST | Warning | N | non-affichable, OK |
| BUFFER_FULL | Erreur | M | **GAP** : à tester + capturer |
| FDM_LOCKED | Erreur | M | **GAP** : à tester + capturer |
| TOO_MANY_MEMORY_ERRORS | Erreur | N | non-affichable |
| UNAUTHORIZED | Erreur | O | **GAP** |
| INVALID_REQUEST | Erreur | O | **GAP** |
| INTERNAL_ERROR | Erreur | O | **GAP** |
| UNDEFINED_ERROR | Erreur | O | **GAP** |
| UNDEFINED_OTHER | Erreur | O | **GAP** |
| FDM_NOT_OPERATIONAL | Erreur | M | **GAP** : à tester + capturer |
| UNKNOWN_POS | Erreur | M | **GAP** : à tester + capturer |

Case finale : ☐ Le test a été effectué et le JSON ainsi que les captures d'écran ont été ajoutés.

**5 messages mandatoires + au moins 1 optionnel à capturer.** Mock FDM actuel n'accepte pas l'injection d'erreurs runtime - travail de code requis (CLI flag ou env var sur `pos-fdm-mock`).

---

## §VI - TICKETS ET FACTURES

**Tableau du contenu obligatoire de ticket** avec ☐ par type de ticket (N, N Refund, P, C, T, Autres) et 18 lignes d'éléments :

| Élément | N | N Refund | P | C | T | Autres |
|---|---|---|---|---|---|---|
| I. Dénomination complète | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| II. Identification (nom, adresse, TVA) | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| III. Dénomination commerciale | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| IV. N° unité établissement (EST) | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| V. N° fabrication POS + version | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| VI. Identification terminal | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| VII. Identification dispositif (MAC, SN) | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| VIII. Identification utilisateur (INSZ) | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| IX. Date et heure | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| X. N° ticket continu | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| XI. Opérations produits (qté, desc, prix, TVA) | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| XII. Total à payer TVA comprise | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| XIII. Base imposable par taux | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| XIV. Montant TVA par taux | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| XV. Modifications prix non-lignes | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| XVI. Données FDM (footer, fdmId, dateTime, label, counters, shortSignature) | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| XVII. Lignes paiement (arrondi, tips, remboursements) | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |
| XVIII. QR Code URL FDM | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ |

**Statut** : on a les **transcriptions ASCII** dans `TICKETS_EXEMPLES_FR.md` qui couvrent les éléments visuellement, mais **pas le tableau coché**. Le SPF reviewer doit pouvoir vérifier chaque ligne x chaque type. À reformater en tableau de coches.

**Contenu de la facture imprimée** : 11 éléments (a-k) supplémentaires obligatoires. **PARTIEL** : nos exemples de factures B2B n'existent pas encore (Peppol I event implémenté côté code mais pas de ticket exemple I).

**Règles de langue** : ticket en FR, NL, DE obligatoires (clientèle locale). **GAP** : on a FR uniquement (UI multilingue FR/ZH/EN, mais pas NL/DE pour les tickets - le ticket fiscal officiel doit être disponible dans la langue de l'opérateur ou du client, ZH n'est pas une langue officielle belge).

---

## §VII - ACQUISITION DES DONNÉES

### §VII.A Données locales
- Localisation - **OK**
- Accessibilité SPF - **OK**
- Procédure copie - **OK**
- Procédure extraits - **OK**
Tout dans `FEUILLE_ACQUISITION_DONNEES_FR.md`.

### §VII.B Données cloud
- ☐ J'ai pris ceci en considération - **À cocher** (Non applicable, SCE 100 % local)

### Feuille d'acquisition des données (DAS)
**OK** dans `FEUILLE_ACQUISITION_DONNEES_FR.md`. Vérifier que tous les champs des §A-F sont remplis.

### §VII.D Rapports X/Z
- Comment visibles/imprimables ? - **PARTIEL** : décrit dans `MANUEL_UTILISATEUR_FR.md` mais **demande explicite "Veuillez joindre des captures d'écran si nécessaire"** - **GAP** : captures à produire.

---

## §VIII - SIGNATURE

- ☐ Demande complète et signée
- ☐ Résultats des tests présents

Action opérateur : signature électronique qualifiée au moment de la soumission.

---

# RÉCAPITULATIF DES GAPS À COMBLER

## Gaps "papier" (à rédiger ou reformater)

1. **Document de réponses du manuel** : un seul fichier qui parcourt §I à §VIII et coche/répond chaque case dans l'ordre du manuel. Notre dossier a les réponses dispersées dans 27 annexes, le SPF veut un parcours linéaire.
2. **Schéma d'assemblage** (§I.C + §II.G) : diagramme physique POS + FDM + imprimante + tiroir + réseau.
3. **Diagramme ER de la base** (§II.C "Relations entre tables") : visuel des relations entre `orders`, `order_lines`, `fiscal_events`, `fiscal_counters`, `payments`, `users`, etc.
4. **Liste produits SPF prédéfinie** (§III.N) : recréer un seed alternatif avec les produits exacts du manuel (Dry Martini, Burger of the Chef, etc.).
5. **Tableau coché ticket x type** (§VI) : reformater `TICKETS_EXEMPLES_FR.md` avec le tableau 18 lignes x 6 colonnes.
6. **Documents de résultats UC** (§IV) : un PDF par UC qui coche les "champs à vérifier" + joint JSON request/response + ticket. 22 UC Basic Set minimum.
7. **Garantie identité machine-test / production** (§II.D) : note sur la reproductibilité du build (SHA-1, Cargo.lock, rust-toolchain.toml épinglés).
8. **Procédure mise à jour + accès distant** (§II.E) : choisir une solution (TeamViewer, AnyDesk, etc.) et déclarer.
9. **Mention NL/DE sur les tickets** (§VI règles de langue) : décision politique - obligatoire en Belgique. Soit on rend l'UI bilingue ticket, soit on déclare avec justification "client TSB francophone uniquement".

## Gaps "code" (à implémenter ou ajuster)

10. **Mock FDM injection d'erreurs runtime** : permettre via env var ou CLI flag de retourner BUFFER_FULL, FDM_LOCKED, etc. Sinon impossible de capturer les écrans §V.
11. **Captures d'écran de l'UI** :
    - Login (LoginView)
    - Tables/floor plan
    - Order/cart
    - Checkout (cash + card paths)
    - Reports (X + Z + cash in/out + refund flow)
    - FDM error messages (au moins les 5 mandatoires)
    - Pro forma display
    - Copy ticket display
    - Training mode banner
12. **Émission ticket numérique** (§III.H) : la spec dit obligatoire. Aujourd'hui on imprime papier uniquement. À implémenter (mail / QR scan / autre) OU à déclarer.
13. **Enregistrement zéro** (§III.D) : à vérifier dans le code, sinon à déclarer comme non-supporté.

## Gaps "process" (étapes opérateur)

14. **Adresse complète + n° TVA du producteur** : à compléter
15. **Signature électronique qualifiée** : Bruno doit obtenir un certificat eID ou Itsme pour signer le PDF final
16. **Kit de développement Checkbox** : FDM physique + émulateur à commander (€380 HTVA, devis TAIGER/Eutronix) pour la répétition
17. **Image kiosk** : script packer/debootstrap pour produire l'ISO bootable
18. **Mot de passe SPF** sur la machine de test : compte utilisateur SPF avec credentials à communiquer au rendez-vous d'admission

---

# PROPOSITION D'ORDRE D'EXÉCUTION

## Bloc A - Gaps papier rapides (1-2 jours)
- A1. Document de réponses du manuel (`REPONSES_MANUEL_CERT_FR.md`) parcourant §I à §VIII
- A2. Schéma d'assemblage (Excalidraw ou ASCII enrichi)
- A3. Diagramme ER de la base
- A4. Tableau coché ticket x type (§VI)
- A5. Compléter §II.D garantie, §II.E mises à jour, §VI règles de langue

## Bloc B - Travail de code (2-3 jours)
- B1. Mock FDM injection d'erreurs (env var `MOCK_FDM_ERROR=BUFFER_FULL`)
- B2. Captures d'écran : démarrer `cargo tauri dev`, traverser chaque flux mandaté, capturer chaque écran (15-20 captures)
- B3. Décider de l'émission numérique (implémenter ou déclarer)

## Bloc C - Tests UC documentés (2-3 jours)
- C1. Pour chaque UC Basic Set (22) : exécuter le test, capturer JSON request/response, capturer ticket, produire 1 page du PDF de résultats
- C2. Idem pour les 5 UC Reports

## Bloc D - Liste produits SPF (1 jour)
- D1. Créer `crates/pos-db/src/queries/spf_test_seed.rs` avec la liste exacte du manuel
- D2. Variable d'env / feature `spf-test-seed` qui charge cette base au démarrage

## Bloc E - Process opérateur (variable)
- E1. eID / Itsme pour signature qualifiée
- E2. Kit de développement Checkbox
- E3. Image kiosk

---

**Verdict honnête :** on est à ~60 % du dossier complet (en termes de "matching 100% les exigences"). Les annexes FR sont propres mais le formulaire central et plusieurs gros bloc-de-preuve manquent. Compter **5-8 jours de travail** pour atteindre 100 %.

Le bloc le plus critique est **Bloc A (réponses du manuel)** + **Bloc C (résultats UC)** parce que c'est ce que le reviewer SPF lit en premier. Les screenshots (B2) sont indispensables pour §V. La liste produits SPF (D) est indispensable pour exécuter les UC correctement au rendez-vous d'admission.

---

# MISE À JOUR DU 2026-07-02

Écarts comblés depuis le scan initial :

- **Gap 9 (langues NL/DE des tickets) — RÉSOLU côté code.** Les mentions
  fiscales imprimées existent désormais en FR/NL/DE
  (`ReceiptLanguage` dans `crates/pos-fiscal/src/ticket.rs`, libellés
  fixes dans `crates/pos-print/src/receipt.rs`), pilotées par la clé
  `language` de `/etc/moyun-pos/pos.json`. Au passage, correction d'un
  écart de conformité : les mentions étaient en anglais ("VAT RECEIPT",
  "THIS IS NOT A VALID VAT RECEIPT") ; elles utilisent maintenant les
  libellés exacts du manuel §VI ("TICKET DE CAISSE TVA",
  "REMBOURSEMENT DU TICKET DE CAISSE TVA", "CECI N'EST PAS UN TICKET
  DE CAISSE TVA VALABLE", "COPIE", …). Le libellé DE est une traduction
  de style officiel à confirmer sur la version DE de la spec.
- **Gap 8 (accès distant) — DÉCIDÉ.** RustDesk portable (open source,
  binaire autonome, démarré ponctuellement à la demande), déclaré dans
  `REPONSES_MANUEL_CERT_FR.md` §II.E et `GARANTIE_MAJ_LANGUES_FR.md`.
- **Nouveau gap découvert et RÉSOLU : chargement du fichier de
  configuration.** `MANUEL_INSTALLATION_FR.md` documentait
  `/etc/moyun-pos/pos.json` mais le binaire ne lisait aucun fichier
  (identité codée en dur). `PosConfig::load_from_file` +
  `AppState::load_config` implémentés : env `MOYUN_POS_CONFIG`
  prioritaire, puis `/etc/moyun-pos/pos.json` (erreur de parse =
  fatal), sinon identité de développement avec avertissement.
- **Étape 1 du runbook (`ASSEMBLAGE_FINAL_FR.md`) revalidée** :
  `cargo test --workspace` (194 tests verts), `cargo clippy -D
  warnings` propre, `cargo fmt --check` propre, build cert
  (`--no-default-features --features cert`) OK, zéro symbole hubrise
  dans le binaire. Chemins de build corrigés (`target/release/`, pas
  `src-tauri/target/release/`).

Écarts restants inchangés : répétition Checkbox (kit de
développement en cours de commande), captures d'écran UI (bloc B2), image
kiosk (bloc E3), signature qualifiée (E1).

---

# MISE À JOUR DU 2026-09-24

Branche `feat/cert-dossier-kiosk-image`.

## Écarts comblés

- **Identité producteur (gap 14)** — déjà renseignée (Bruno Moyse,
  indépendant, BCE 0783.544.917, régime de franchise TVA, Saint-Gilles).
- **Dossier neutre vis-à-vis des clients.** Le formulaire ne demande
  qu'« un aperçu des distributeurs potentiels » ; aucun client n'est
  déclaré. `LISTE_DISTRIBUTEURS_FR.md` réécrit, exemples de tickets
  régénérés avec « Restaurant Exemple ».
- **Règle de langue (§VI p. 99)** — la langue du ticket suit la région
  linguistique de l'établissement ; paramètre d'installation
  `pos.json::language` (section « Langue du ticket » du manuel
  d'installation). L'ancienne justification « langue du client /
  Bruxelles bilingue » est supprimée.
- **Lignes de produits (§III.N, gap 4)** — `LIGNES_PRODUITS_FR.md`
  décrit les 25 produits du manuel chargés depuis `seed_spf_test.sql`.
- **Accès distant (gap 8)** — un seul modèle partout : pas de SSH,
  RustDesk portable (AppImage 1.4.9, SHA-256 épinglé) présent mais
  jamais lancé au boot, session ouverte à la demande.
- **Défauts de code/déploiement découverts et corrigés :**
  - la build cert ne contenait pas l'UI (mode dev Tauri → écran vide) ;
  - la base était écrite sous `/var/lib/moyun-pos/.local/share/…` et
    non au chemin déclaré `/var/lib/moyun-pos/pos.db` ;
  - le seed de `install.sh` échouait silencieusement (pas de schéma) ;
  - unité systemd sans session logind pour Cage, `cage.ini` sans
    effet, règle udev HID inopérante, chrony absent ;
  - `build-release.sh` ne produisait pas la build cert.

## Nouveaux écarts identifiés

- **Preuves UC (§IV)** — `evidence/uc/*.json` rejoue un article
  synthétique (« Test item », 10 €) au lieu des scénarios officiels
  (ex. M110 = 2× Dry Martini + 1× Burger of the Chef, 52 €). À
  recapturer contre le FDM Checkbox en rejouant les fixtures.
- **Toolchain** — une build hors conteneur (cargo Homebrew) ignore
  `rust-toolchain.toml` (1.98.1 au lieu de 1.96.0). Seule la build
  `deploy/build-release.sh --docker` fait foi ; vérifier par deux
  builds successives du tag cert que le SHA-1 est identique.
- **Adresses IP / port FDM** — plan d'adressage aligné (POS .10, FDM
  .20, imprimante .30) ; port GraphQL du Checkbox CBX01 à confirmer à
  réception du kit.

## Plan restant (sans kit Checkbox)

1. Image kiosque avec mkosi (Debian 12, variantes admission / production,
   même binaire), boot testé sous QEMU (UEFI, 1920×1080, tactile virtio).
2. Spike RustDesk sous Cage dans l'image : capture via
   `xdg-desktop-portal-wlr` + PipeWire, saisie clavier, déclenchement
   de session depuis l'interface (rôle administrateur).
3. Captures d'écran (gap 11, bloc B2) depuis l'image sous QEMU, avec
   le seed SPF et `MOCK_FDM_ERROR` pour les messages §V.
4. Racine en lecture seule + LUKS/TPM2 dans l'image.

## Plan restant (avec kit Checkbox)

Répétition (`JOURNAL_TESTS_PRE_SOUMISSION_FR.md`), recapture des
preuves UC, port FDM, puis soumission (`ASSEMBLAGE_FINAL_FR.md`).
