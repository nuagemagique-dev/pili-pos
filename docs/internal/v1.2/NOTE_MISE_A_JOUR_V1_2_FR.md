# Note de mise à jour v1.2

Brouillon pour la déclaration de mise à jour après certification (préavis d'un mois, au
plus douze mises à jour par an). Une section par fonction. Le certificat ne couvre que les
fonctions documentées : chaque section dit si la fonction touche le binaire certifié.

| Fonction | Dans le binaire certifié | Données fiscales | État par défaut |
| --- | --- | --- | --- |
| A. Export comptable | Non : outil séparé | lit les données signées, n'écrit rien | sans objet |

## A. Export comptable (hors du binaire certifié)

**Ce qui change.** Un outil en ligne de commande, `pili-accounting`, produit pour le
comptable un dossier de fichiers CSV et une écriture de journal équilibrée par jour, à
partir d'une copie de la base `pos.db` (dossier d'export de la caisse ou sauvegarde
déchiffrée).

**Est-ce dans le binaire certifié ?** Non. Le code vit dans une crate à part
(`crates/pos-accounting`) dont aucune crate livrée ne dépend et qui ne dépend d'aucune
crate de la caisse à l'exécution ; un test automatique le vérifie. Le binaire Linux et
l'application Android ne changent pas à cause de cette fonction : pas d'écran, pas de
commande moteur, pas de migration de base. L'outil s'exécute hors de la caisse.

**Impact fiscal.** Aucun : l'outil lit uniquement l'archive signée (événements N, I, F, R
avec réponse du FDM, rapports Z), en lecture seule et immuable, et n'écrit jamais dans la
base. Il n'envoie rien au FDM ni au SPF.

**Garde-fous.** Chaque période comptable est comparée à son rapport Z signé (nombre de
tickets, chiffre d'affaires, TVA par code, paiements par type) ; au moindre écart, aucun
fichier n'est écrit. Seules les périodes clôturées par un Z sont exportées. Aucun numéro
national ni nom d'employé dans les fichiers.

**Tests.** Rapprochement d'un jour réel construit par le moteur (scénario Reports 1.3),
fichiers de référence octet pour octet, équilibre du journal, détection d'altération,
preuve de lecture seule, test d'isolation. Détail dans `EXPORT_COMPTABLE_FR.md`.

**Cas d'usage officiels.** Aucun cas d'usage SPF n'est rejoué : la fonction ne produit
pas d'événement.

**Écrans.** Aucun écran de la caisse. Sortie texte de l'outil et fichier `LISEZMOI.txt`
à joindre au dossier comme exemple.
