<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/TICKETS_EXEMPLES_FR.md @ 86088e4 -->
# Tickets de test - exemples imprimés - pili-pos-android

**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
**Date :** À effectuer (EV-18 : date de dépôt)

Conformément au §IV §1209-1210 du Manuel de certification SCE 2.0, le dossier doit contenir un exemple de chaque type de ticket imprimé par le système.

Ces rendus seront produits par le pipeline d'impression réel du moteur embarqué (mêmes fonctions `format_receipt` et `format_report` que la caisse en service), imprimés sur l'imprimante simulée `pos-printer-mock` puis transcrits en texte, avec l'identifiant du terminal de test sur chaque ticket. Largeur de 48 caractères, papier 80 mm, Epson TM-m30III, code page CP858. Aucun ticket du dossier Linux n'est repris ici : `À effectuer (EV-08 : tickets imprimés sur l'imprimante simulée, et sur l'Epson réelle si disponible)`. Identifiant d'appareil imprimé : `À effectuer (EV-16 : deviceId du terminal de test tel qu'imprimé)`.

## Matrice de conformité du contenu (§VI du manuel)

Réponse au tableau « Contenu du ticket » du manuel §VI. Une ligne par élément obligatoire de la spec POS v2.2 §III.3, une colonne par type de ticket. L'élément VII (identification du dispositif d'entrée, `deviceId`) est sur Android le numéro de série matériel de la caisse, non une adresse MAC (PLATEFORME_ANDROID_FR.md §4).

| Élément | N | N Refund | P (Pro Forma) | C (Copy) | T (Training) | Autres (cuisine) |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| À effectuer (EV-08 : matrice vérifiée sur les tickets imprimés) | À effectuer | À effectuer | À effectuer | À effectuer | À effectuer | À effectuer |

**Mentions spécifiques cochées :**

| Mention | N | N Refund | P | C | T | Autres |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| À effectuer (EV-08 : mentions relevées sur les tickets imprimés) | À effectuer | À effectuer | À effectuer | À effectuer | À effectuer | À effectuer |

**Règles de langue :** les mentions fiscales sont imprimées dans la langue officielle de la région linguistique de l'établissement (FR/NL/DE), fixée par installation (GARANTIE_MAJ_LANGUES_FR.md §3.2). Les exemples seront présentés en français ; le rapport Z l'est aussi en néerlandais et en allemand.

---

## 1. Vente normale (M110 - Événement N)
Scénario du manuel : 2 Dry Martini (12,00 €, TVA A) et 1 Burger du chef (28,00 €, TVA B), payé en espèces. `À effectuer (EV-08 : ticket imprimé et transcrit)`.
## 2. Addition, pré-ticket (M123 - Événement P)
Même commande qu'au M110, juste avant l'encaissement. `À effectuer (EV-08 : ticket imprimé et transcrit)`.
## 3. Remboursement complet (M111 - Événement N négatif)
Remboursement intégral du M110. `À effectuer (EV-08 : ticket imprimé et transcrit)`.
## 4. Copie de ticket (M160 - Événement C)
Copie du ticket M110. `À effectuer (EV-08 : ticket imprimé et transcrit)`.
## 5. Vente en formation (M171 - Événement T)
Même commande que M110 en mode formation. `À effectuer (EV-08 : ticket imprimé et transcrit)`.
## 6. Rapport X (M180 - Événement R, intermédiaire)
Rapport X en cours de service. `À effectuer (EV-08 : ticket imprimé et transcrit)`.
## 7. Rapport Z (M181 - Événement R, clôture)
Clôture journalière (Z n°1). `À effectuer (EV-08 : ticket imprimé et transcrit)`.
### Rapport Z en néerlandais (NL)
Même Z, ticket en néerlandais. `À effectuer (EV-08 : ticket imprimé et transcrit)`.
### Rapport Z en allemand (DE)
Même Z, ticket en allemand. `À effectuer (EV-08 : ticket imprimé et transcrit)`.
## 8. Bon de commande bar / cuisine (sans prix)
Lignes envoyées d'une table ou d'une vente directe, jamais de prix. `À effectuer (EV-08 : ticket imprimé et transcrit)`.
## 9. Ticket numérique par e-mail (exemple)
Même vente M110 envoyée par e-mail (TICKET_NUMERIQUE_FR.md §2-3 et §5). `À effectuer (EV-08 : ticket imprimé et transcrit)`.
## 10. Paiement en plusieurs moyens (ROUND5 - Événement N)
Chèques-repas, carte et reste en espèces arrondi. `À effectuer (EV-08 : ticket imprimé et transcrit)`.
## 11. Vente sans paiement (UC4109 - Événement N)
Client parti sans payer, clôturé par le gérant avec son PIN. `À effectuer (EV-08 : ticket imprimé et transcrit)`.
## 11 bis. Autres documents imprimés (sans transcription ici)
Ticket cuisine et ticket client d'une commande en ligne, rapports X et Z par utilisateur (M182, M183, M184) : non produits par la suite de transcription ; `À effectuer (EV-08 : exemplaires imprimés si le dossier les joint)`.
## 12. Note sur la reproduction
Les rendus sont générés par le pipeline d'impression réel et ne peuvent pas diverger de la sortie papier. Au rendez-vous d'admission, le distributeur apporte la machine test connectée à l'imprimante réelle (`À effectuer (EV-15 : imprimante réelle, ou mention du profil émulé)`) ; les exemplaires physiques sont archivés avec le dossier final.
