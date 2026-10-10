<!-- Variante Android, sans équivalent Linux -->
# Checklist de déploiement - pili-pos-android

**Distributeur :** Bruno Moyse / nuagemagique.dev (Bruxelles, BE)
**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
**Date :** À effectuer (EV-18 : date de dépôt)

Ce que le relecteur d'admission doit s'attendre à trouver sur la machine
de test, du terminal sorti de sa boîte au premier ticket, et ce que le
producteur vérifie avant de la livrer. Pour la variante Android, ce
document remplace la checklist du kiosque Linux
(`docs/SUBMISSION_ARTIFACTS/CHECKLIST_DEPLOIEMENT_KIOSK_FR.md`) : ni image
système, ni disque chiffré, ni console de maintenance
(`PLATEFORME_ANDROID_FR.md`). Les cases à cocher sont un **modèle**, remplies
à l'installation de chaque terminal et à la visite d'admission ; les
procédures sont dans `MANUEL_INSTALLATION_FR.md`.

## Couche matériel

- [ ] Terminal Sunmi D3 Pro avec services Google, Android 13, écran 1920
      x 1080 en paysage, ou, si aucun vrai terminal n'est disponible au
      dépôt, le profil émulé `sunmi_d3_pro_gms_api33` déclaré comme tel
      (À effectuer, EV-15 : fiche d'un vrai D3 Pro, ou déclaration du
      profil émulé).
- [ ] Numéro de série du terminal relevé et consigné : c'est son
      deviceId, imprimé sur chaque ticket
      (`PLATEFORME_ANDROID_FR.md` §4). Valeur de la machine de test :
      À effectuer (EV-16 : deviceId de la machine de test).
- [ ] Imprimante thermique Epson TM-m30III raccordée au LAN (TCP 9100,
      ni USB ni Bluetooth), tiroir-caisse RJ11 raccordé à l'imprimante.
- [ ] FDM module2 sur le LAN, ou, pour la machine de test, simulateur en
      ligne `https://fdm.module2.be/graphql/` : le terminal a alors un
      accès Internet sortant (`SCHEMA_ASSEMBLAGE_FR.md` §7).
- [ ] Réseau : réservations DHCP du terminal, du FDM et de l'imprimante
      (`SCHEMA_ASSEMBLAGE_FR.md` §4) ; aucun port ouvert vers le terminal.
- [ ] Clé USB du lot de mise en service ; carte microSD exFAT de 64 Go
      (128 Go à fort volume) pour exports et mises à jour, la base restant
      sur le stockage interne (`MANUEL_INSTALLATION_FR.md` §8) ; onduleur
      recommandé.

## Couche Android et propriétaire de l'appareil

- [ ] **Terminal provisionné avant tout compte Google** : aucun compte
      Google n'a jamais été ajouté (Réglages, Comptes : vide), ni au banc
      ni chez le client.
- [ ] L'application `dev.nuagemagique.pilipos` est le propriétaire de
      l'appareil (`adb shell dpm list-owners` au banc ; sur une caisse de
      production adb est coupé, la preuve est l'écran « À propos » et le
      fonctionnement du kiosque). Provisionnement par `dpm
      set-device-owner` au banc, par code QR après réinitialisation
      d'un terminal hors service (`MANUEL_INSTALLATION_FR.md` §3.2 et
      §3.3).
- [ ] Mode kiosque (Lock Task) : l'application est le seul écran
      d'accueil, sans barre système, menu d'alimentation ni applications
      récentes, et revient au premier plan, verrouillée, après un
      redémarrage (`PLATEFORME_ANDROID_FR.md` §3.2). Essai sur le produit :
      À effectuer (EV-10 : manuel d'exécution du D3 Pro sur les deux
      profils).
- [ ] Les 13 restrictions de l'ensemble constant sont présentes
      (`PLATEFORME_ANDROID_FR.md` §3.3) ; débogage USB coupé sur la version
      de production (§3.5) ; ni le montage des supports amovibles ni le
      réglage du Wi-Fi ne sont restreints.
- [ ] Fuseau Europe/Brussels fixé, heure automatique active, horloge
      exacte à la seconde (`PLATEFORME_ANDROID_FR.md` §3.4) ; mise à jour
      du système en report (comme construit).
- [ ] **Boutique Play et applications Google destinées à l'utilisateur
      masquées**, installations bloquées sauf par l'outil de mise à jour
      déclarée ; tout message de Play Protect noté. Comme construit :
      aucune application masquée ; À effectuer (EV-13 : paquets masqués,
      restriction d'installation, fenêtre de mise à jour système, Play
      Protect, sur le profil avec services Google).
- [ ] Aucun accès à distance (`PLATEFORME_ANDROID_FR.md` §7).

## Couche application

- [ ] APK `pili-pos-android` 1.0.0 (versionCode 10000) signé, installé
      tel que déclaré, issu du tag `android-v1.0.0`. Jamais une version
      `-dev`. Empreintes : À effectuer (EV-01 : SHA-1 et SHA-256 de l'APK
      signé ; EV-02 : SHA-1 de `libpos_ffi.so` ; EV-03 : SHA-256 du
      certificat de signature).
- [ ] Moteur fiscal de l'étiquette de gel `linux-v1.0.0-rc1` (puis
      l'étiquette finale), embarqué sous forme de `libpos_ffi.so` : À
      effectuer (EV-04 : commit de l'étiquette et SHA-1 du binaire Linux
      du même moteur).
- [ ] Lot de mise en service signé importé (écran « Mise en service de
      la caisse ») ; sans lot valide le moteur reste fermé. Pour la
      machine de test : lot à l'identité du jeu de test (`pos_id`
      `CPOS0031234567`, terminal `POS-2-ANDROID`, 25 tables) avec la
      graine `src-tauri/seed_spf_test.sql` (lignes de produits du manuel
      §III.N et personnel du jeu de test : `MANUEL_INSTALLATION_FR.md`
      §7.1 et `LIGNES_PRODUITS_FR.md`).
- [ ] `pos.json` : `fdm_url` en `https://`, jeton et certificats module2
      dans le lot, `printer_address` (et `kitchen_printer_address` si le
      bar ou la cuisine a sa propre imprimante), `training_mode` à
      `false` (hors machine de formation), `backup_dir` absent,
      `language` selon la région linguistique de l'établissement.
- [ ] Contrôle du FDM « connecté » à l'écran de connexion ; première
      connexion d'un Gérant : événement S signé.
- [ ] Sauvegardes : copie de `pos.db` après chaque rapport Z, les 7 plus
      récentes conservées (`PLATEFORME_ANDROID_FR.md` §5.2 ; commit du
      moteur : À effectuer, EV-19).
- [ ] Export nocturne activé ou non sur la machine de test, détenteurs
      des clés age, opérateur du point de réception : À effectuer (EV-20 :
      état de l'export nocturne).
- [ ] Mise à jour par « Config », « Mise à jour » seulement, en ligne ou
      depuis une clé USB (`GARANTIE_MAJ_LANGUES_FR.md` §2.1).

## Documentation dans la clé USB du dossier

Les documents sont ceux du dossier de soumission (`MANIFEST.txt`,
`INDEX_DOSSIER_FR.md`) ; chacun est fourni en PDF.

- [ ] Tous les documents listés par `MANIFEST.txt` et `INDEX_DOSSIER_FR.md`
      (annexes 0 à 36, dont `CADRE_DEMANDE_FR`, `PLATEFORME_ANDROID_FR`,
      `MANUEL_INSTALLATION_FR` et le présent `CHECKLIST_DEPLOIEMENT_FR`),
      les documents communs aux deux demandes étant ceux du dossier Linux
      au même état du moteur.
- [ ] Preuves : dossier `Report_Testing` (requêtes et réponses JSON,
      tickets imprimés, rapports de construction) et captures d'écran
      (EV-06, EV-08, EV-09, EV-05).
- [ ] Formulaire de demande de certification signé (PDF + copie scannée
      signée).

## Vérifications du producteur avant livraison

À exécuter sur le poste du producteur (dépôt source présent), jamais sur
le terminal : l'APK déclaré ne contient ni source ni outil de compilation.

```bash
# 1) Reproductibilité : deux builds du même commit tagué, deux conteneurs
#    sans réseau, APK non signés comparés octet par octet (REPRO.md).
scripts/android/check_repro.sh            # attendu : REPRO | total | PASS
# 2) Contrôle de l'APK non signé (alignement, une architecture, bibliothèques).
scripts/android/check_apk.sh <APK non signé>
# 3) Répétition de la déclaration, SANS rien publier : clés jetables hors
#    dépôt, --dry-run obligatoire.
scripts/release/declare_android.sh --apk <APK non signé> --version 1.0.0 \
  --sha1 <SHA-1 du APK signé> --keystore <clé jetable> --alias <alias> --dry-run
                                          # attendu : DECLARE | total | PASS
```

- [ ] `check_repro.sh` : APK, bibliothèques natives et inventaire des
      entrées identiques, rapport joint au dossier (EV-05) ;
      `declare_android.sh --dry-run` : toutes les étapes `PASS`.
- [ ] **Jamais de déclaration réelle** (sans `--dry-run`) par la chaîne de
      développement : elle ne se fait qu'avec la clé de signature et la
      clé de déclaration du producteur, hors ligne, **après acceptation du
      SHA-1 par le SPF Finances** et sur décision du producteur
      (`GARANTIE_MAJ_LANGUES_FR.md` §2.1).
- [ ] Tests automatisés verts au commit de release : À effectuer (EV-17 :
      comptes de tests au commit de release).
- [ ] Après la déclaration, l'artefact public `declared-android` a le
      SHA-1 de EV-01.

## Vérifications sur la machine de test

Sans adb : tout se lit à l'écran de la caisse.

1. **Empreintes.** « À propos », sections « Version » et « Intégrité » :
   SHA-1 et SHA-256 de `base.apk` égaux à EV-01, SHA-1 de `libpos_ffi.so`
   égal à EV-02, SHA-256 du certificat égal à EV-03, version et code de
   version 1.0.0 / 10000, « Version logicielle du moteur »
   pili-pos-android/1.0.0, deviceId égal à EV-16.
2. **Politiques.** Restrictions et kiosque de la couche Android : essai de
   sortie du kiosque sans résultat (retour, accueil, applications
   récentes), ajout de compte refusé.
3. **Export SPF.** Gérant connecté, « Config », « Export pour le SPF » :
   dossier `PILI-EXPORT` sur la clé ou la carte, « Exporter », écran
   « Export terminé ». Sur un ordinateur : `shasum -a 256 -c
   MANIFEST.sha256` tout `OK`, `sqlite3 pos.db "PRAGMA integrity_check;"`
   donne `ok`, `identity.json` reprend les empreintes de l'étape 1
   (`SECURITE_BD_FR.md` §8.3). Déroulé complet : À effectuer (EV-10).
4. **Voie de mise à jour.** « Config », « Mise à jour » : le refus est
   motivé tant qu'une commande est ouverte ou qu'un rapport Z est dû ;
   avec les trois fichiers de la déclaration sur clé USB, l'outil affiche
   « Mise à jour disponible et vérifiée » (version, SHA-1, signataire)
   sans installer sur la machine de test pendant l'admission.
5. **Sauvegardes.** Après un rapport Z de l'essai, une copie de `pos.db`
   existe (taille et nombre dans « À propos » ; au plus 7).

## Script de démonstration pour le rendez-vous d'admission

0. **Avant le rendez-vous** : terminal provisionné et lot importé, premier
   démarrage fait, deviceId et empreintes relevés, réseau vérifié. Le
   rendez-vous n'est pas le moment de découvrir l'écran de mise en service.
1. Mise sous tension ou redémarrage : l'application, seul écran d'accueil
   du terminal, se lance verrouillée en kiosque. Montrer « À propos »
   (empreintes, deviceId, versions).
2. Connexion (tuile + PIN) ; montrer l'événement fiscal **S** de
   pointage archivé.
3. Ouvrir la table 4 ; ajouter trois articles ; « Envoyer en cuisine » :
   événement **P** et bon de commande imprimé (sans prix).
4. Payer la table en deux parts égales (« Partager en » 2) : une part en
   espèces, l'autre par carte, **un seul** événement **N** (un ticket,
   deux paiements).
5. « Rapports », onglet « Tickets » : « Copie » du ticket de la vente,
   événement **C** avec `fdmRef`.
6. Onglet « Journée » : « Rapport X », événement **R**.
7. « Clôturer la journée (Z) », puis « Confirmer la clôture » :
   événements **R** et **R**.
8. Écran de connexion : tuile, « Fin de service », PIN : pointage sortie,
   événement **S**.
9. « Config », « Export pour le SPF » : export sur la clé ou la carte du
   relecteur ; il vérifie `MANIFEST.sha256`, ouvre `pos.db` en lecture
   (`sqlite3 -readonly`), compare `identity.json` aux valeurs du
   formulaire et vérifie que les compteurs sont strictement monotones par
   label et que chaque ligne a un `fdm_response_json` non nul
   (`FEUILLE_ACQUISITION_DONNEES_FR.md`).
10. Démontrer le blocage en cas de FDM déconnecté en coupant le
    simulateur ou le réseau du FDM en plein milieu d'un flux.
11. Montrer l'écran « Mise à jour » (refus motivé, contrôles de la
    déclaration), sans installer, et rappeler qu'aucun accès à distance
    n'existe sur le terminal.
