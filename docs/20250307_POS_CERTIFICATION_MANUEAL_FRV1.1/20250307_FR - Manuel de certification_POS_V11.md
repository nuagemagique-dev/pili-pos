# 20250307_FR - Manuel de certification_POS_V11

> 1:1 text extraction of [`20250307_FR - Manuel de certification_POS_V11.pdf`](<../original_pdfs/20250307_POS_CERTIFICATION_MANUEAL_FRV1.1/20250307_FR - Manuel de certification_POS_V11.pdf>) (via `pdftotext -layout`). Diagrams, images, and exact PDF visual layout are not preserved — refer to the source PDF for those. Text content is verbatim.

```text
                          GKS     SCE




              POS 2.0
        Manuel de Certification
                  V 1.1




                  2025




1/110
                                                                                Table des matières
  I.         Documentation requise ....................................................................................................................................................... 8
                  A.         Formulaire de demande .................................................................................................................................... 8
                  B.         Résultats de test ................................................................................................................................................. 8
                  C.         Manuels du POS .................................................................................................................................................. 8
                  D.         HASH SHA-1 du logiciel .................................................................................................................................... 8
                  E.         Documents d’identification .............................................................................................................................. 9
                  F.         Profil de l’entreprise ......................................................................................................................................... 10
  II.             Description du POS ....................................................................................................................................................... 13
                  A.         Identification du logiciel .................................................................................................................................. 13
                  B.         Déclaration d’accès à la base de données ................................................................................................. 13
                  C.         Caractéristiques de la base de données .................................................................................................... 13
                  D.         Garantie du produit .......................................................................................................................................... 15
                  E.         Mises à Jour ........................................................................................................................................................ 15
                  F.         Matériel ................................................................................................................................................................ 16
                  G.         Schéma d’assemblage..................................................................................................................................... 17
  III.            Fonctionnalités du POS ............................................................................................................................................... 18
         Configuration du POS.............................................................................................................................................................. 18
                  A.         Paramètres d’installation ............................................................................................................................... 18
                  B.         Configuration Internet ..................................................................................................................................... 18
                  C.         Utilisateurs & Mots de Passe ........................................................................................................................ 19
         Connexion avec le FDM .......................................................................................................................................................... 20
                  A.         Connexion du POS au FDM ............................................................................................................................ 20
                  B.         Connexion au FDM............................................................................................................................................ 20
                  C.         FDM Non-Opérationnel .................................................................................................................................. 21
         Fonctionnalités du POS .......................................................................................................................................................... 22
                  A.         Events possibles ............................................................................................................................................... 22
                  B.         Numérotation d’Events ................................................................................................................................... 22
                  C.         Fonctionnalités interdites .............................................................................................................................. 22
                  D.         Fonctions obligatoires .................................................................................................................................... 22
                  E.         Enregistrement de la présence..................................................................................................................... 23
                  F.         Fonctionnalités Avancées .............................................................................................................................. 24
                  G.         CostCenterChange (Gestion de Table) ........................................................................................................ 26
                  H.         Emission des tickets numériques ................................................................................................................ 27
                  I.         Terminaux et Appareils ................................................................................................................................... 27

2/110
             J.         Périodes ............................................................................................................................................................... 28
             K.         Tiroirs-Caisse ..................................................................................................................................................... 28
             L.         Rapports .............................................................................................................................................................. 29
             M.         Intégration des Tiers-parties (API, ETC) ..................................................................................................... 29
        Lignes de produit (Product Lines) ................................................................................................................................... 30
  IV.        Procédure de communication GraphQL.................................................................................................................. 31
    Procédure de test..................................................................................................................................................................... 31
    Vente (N) - signSale mutation - M110 ............................................................................................................................... 32
    Remboursement intégral (N) – signSale mutation – M111 ........................................................................................ 33
    Remboursement partiel (N) - signSale mutation – M112............................................................................................ 34
    Pro Forma (P) - signOrder mutation – M121 ................................................................................................................... 35
    Changement de cost center (P) – signCostCenterChange – M122 ............................................................................. 36
    Addition Provisoire (P) - signPreBill - M123 ..................................................................................................................... 37
    Cash-flow (F) – signMoneyInOut - M130........................................................................................................................... 38
    Tiroir-caisse - signDrawerOpen - M131............................................................................................................................. 39
    Changement de moyen de paiement – signPaymentcorrection – M132 ................................................................. 40
    Social (S) – signWorkIn /signWorkOut - UC140/141 ....................................................................................................... 41
        TEST 1 : TRAVAIL In (Work In) .......................................................................................................................................... 41
        TEST 2 : TRAVAIL Out (Work Out) ................................................................................................................................... 41
    Facture (I) - SignInvoice – UC150 ......................................................................................................................................... 42
    Copie (C) - SignCopy - M160 .................................................................................................................................................. 44
    Training (T) – isTraining Boolean - M171 .......................................................................................................................... 45
    Produit composé – signSale – UC200 ................................................................................................................................ 46
    Modification de prix par ligne - signSale – UC220 .......................................................................................................... 47
    Modification de prix par event – signSale – UC223 ....................................................................................................... 48
    Modification de Prix Multiple – SIGNSALE – UC224 ..................................................................................................... 50
    Correction – signsale (N) – UC230 ...................................................................................................................................... 52
    Division de table – signCostCenterChange – UC240 ....................................................................................................... 53
    Division de table – signCostCenterChange – UC241 ....................................................................................................... 56
    Division de table – signCostCenterChange – UC243 ....................................................................................................... 59
    Fusion de tables – singcostcenterchange – UC250 ......................................................................................................... 62
    Fusion de tables – signcostcenterchange – UC251 ......................................................................................................... 64
    Fusion de tables – SIGNsale (N) – UC252 ......................................................................................................................... 67
    Vente d’un Bon à Usage multiple – signSale (N) – UC260 ........................................................................................... 70
    Utilisation d’un Bon à usage multiple – SIGNSALE (N) – UC261 ............................................................................... 71


3/110
       Vente D’un bon à usage unique – SIGNSALE (n) – UC270 .......................................................................................... 73
       Utilisation d’un bon à usage unique – SIGNSALE (N) – UC271 ................................................................................... 75
       Devises étrangères – signSale – UC296 ........................................................................................................................... 77
       Rapport financier X (R) – signReportTurnoverX – M180 ................................................................................................ 78
       Rapport financier Z (R) – signReportTurnoverZ................................................................................................................. 82
       Rapport utilisateur X (R) – signReportUserX - M182 ...................................................................................................... 83
       Rapport utilisateur Z (R) – signReportUserZ ................................................................................................................... 86
       Testing des tickets des Rapports : EXEMPLE .................................................................................................................. 87
       Arrondissement des méthodes de paiement .................................................................................................................. 91
           Test de base 1 : Arrondi du cash ..................................................................................................................................... 91
           Test de base 2 : Chèque-repas ........................................................................................................................................ 92
           Test de base 3 : Multi-Paiements 1............................................................................................................................... 93
           Test de base 4 : Multi-Paiements 2............................................................................................................................... 93
  V.            Comportement du POS aux Réponses du FDM ................................................................................................... 95
  VI.           Testing des ticketS : Tickets et Factures ................................................................................................................ 97
       Contenu des tickets ................................................................................................................................................................. 97
       Contenu de la facture imprimée ....................................................................................................................................... 100
  VII.          Acquisition des Données .......................................................................................................................................... 102
       Obligation de conservation (art. 60 Code TVA) ............................................................................................................ 102
           La loi ..................................................................................................................................................................................... 102
           Concrètement pour les SCE ........................................................................................................................................... 102
       Obligation de soumission (articles 61 et 62 Code TVA)............................................................................................. 103
           La loi ..................................................................................................................................................................................... 103
           Concrètement pour les SCE ........................................................................................................................................... 103
                A.          Les données sont conservées localement ............................................................................................. 103
                B.          Les données sont conservées dans ‘le cloud’ ....................................................................................... 104
       Feuille d’Acquisition des Données ................................................................................................................................... 105
                A.          Informations Générales ............................................................................................................................... 105
                B.          Réseaux & Appareils ..................................................................................................................................... 105
                C.          Logs .................................................................................................................................................................... 106
                D.          Rapports X/Z ................................................................................................................................................... 108
                E.          Base de Données ........................................................................................................................................... 109
                F.          Version du logiciel .......................................................................................................................................... 109
  VIII.         Signature ....................................................................................................................................................................... 110




4/110
5/110
                                      ABRÉVIATIONS


  DAS    Data Acquisition Sheet

  DB     Database

  DBMS   Database Management System

  JSON   JavaScript Object Notation

  LAN    Local Area Network

  NISS   Numéro d'Identification à la Sécurité sociale

  PoE    Power over Ethernet

  POS    Point of Sale

  SCE    Système de Caisse Enregistreuse

  SHA1   Secure Hash Algorithm 1

  UC     Use Cases

  USB    Universal Serial Bus

  WiFi   Wireless Fidelity




6/110
  Changelog

        Changelog
                           Ajout de la partie « profil de l'entreprise ».
                           Ajout de quelques descriptions nécessaires
                           Corrections d'erreurs dans les rapports :
                            reportBookingDate, type de paiement CARD_DEBIT,
                            negQuantityChanges, user Reports
                           Ajout de mentions légales sur la détention et le stockage
                            d’informations personnelles
        V1.1               Ajout d’une colonne “Taux TVA/à emporter” dans la
                            section “product lines”
                           Ajouts pour clarifier le contenu et corrections pour clarifier
                            les termes
                           Ajout
                           Modifications esthétiques.
                           Changement dans le contenu des reçus

        V1.0        Version de base




7/110
               I. DOCUMENTATION REQUISE

        A. FORMULAIRE DE DEMANDE

              Formulaire complété                    ☐
              Formulaire signé                       ☐

        B. RÉSULTATS DE TEST

              Document PDF complété                  ☐

        C. MANUELS DU POS

              Manuel d’utilisateur                   ☐
              Manuel de configuration/installation   ☐
              Manuel de programmation                ☐
              Manuel de service (si existant)        ☐
              Schéma d’assemblage                    ☐

        D. HASH SHA-1 DU LOGICIEL

              Le SHA-1 est envoyé en pièce jointe    ☐




8/110
               E. DOCUMENTS D’IDENTIFICATION

                    Producteur
      Identification du Producteur

      Nom de l’entité juridique
      Nom commercial
      Structure juridique

      Adresse
      Numéro d’identification à
      la TVA
      Site web
      Représentants autorisés

      Représentant
      Numéro de téléphone
      Adresse électronique
      Contact technique
      Numéro de téléphone
      Adresse électronique

                    Importateurs & Distributeurs
                                        Identification du/des   Identification du/des
                                        importateur(s)*         distributeur(s)**

      Nom de l’entité juridique
      Nom commercial

      Structure juridique
      Adresse
      Numéro d’identification à
      la TVA
      Site web
      Représentants autorisés
      Représentant
      Numéro de téléphone
      Adresse électronique
      Contact technique

      Numéro de téléphone
      Adresse électronique
(*) Un producteur peut également être son propre importateur




9/110
(**) Un producteur peut également être son propre distributeur (si vous avez plusieurs distributeurs, veuillez s’il-vous-plait en fournir la
liste en appendice)


                F. PROFIL DE L’ENTREPRISE

                     Contrôle interne
 Enregistrement

 L'utilisation du logiciel et
 des données est-elle
 enregistrée ?
 Si oui, quelles actions, de
 quelle manière et pour
 combien de temps ?

 Les plugins matériels (clés
 USB) sont-ils enregistrés ?


 Organisation interne

 Combien de développeurs
 travaillent sur le produit ?

 Comment s'organise le
 développement dans votre
 entreprise ?
 (Interne/externe)

 Les missions de
 développement sont-elles
 externalisées ?


 Distributeurs

 Avez-vous imposé des
 dispositions contractuelles
 à vos
 distributeurs/installateurs
 externes ?

 Quel type d'accès les
 distributeurs externes ont-
 ils au logiciel ?




10/110
 Support technique



 Comment s’organise le
 support technique des
 utilisateurs ?


 Analyse de risques


 Avez-vous réalisé une
 analyse de risque pour le
 logiciel et/ou les (bases de)
 données ?


 Les données des clients
 sont-elles collectées,
 stockées ou partagées avec
 des tiers, y compris des
 partenaires commerciaux
 ou des prestataires
 externes ?

 Si des données sont
 partagées ou revendues,
 les clients en sont-ils
 informés de manière
 transparente et explicite ?
 Leur consentement est-il
 recueilli avant toute
 transmission à des tiers ?

 Avertissement – Conformité RGPD
 Les producteurs de système de caisse doivent impérativement respecter le Règlement Général sur la Protection
 des Données (RGPD). Toute collecte, stockage ou transmission de données personnelles doit être conforme aux
 exigences légales en matière de transparence, de sécurité et de consentement des utilisateurs.




11/110
                 Contrôle externe
 Autres certificats

 Une certification par un
 autre organisme de
 certification a-t-elle été
 accordée ?
      A) Pour ce type de
           caisse
           enregistreuse ? Si
           oui, lequel ?

     B) Pour votre
        entreprise ? Si oui,
        lequel ? (Ex :
        IS09001)
Dans l’affirmative, veuillez joindre la copie du certificat en annexe à cette demande.


 Clients



 Est-ce que des clients qui
 ont fixé des exigences de
 sécurité ? Si oui, lesquelles
 ?




Dans l’affirmative, veuillez fournir les dispositions légales (ou un modèle de contrat) des exigences, ainsi
qu'une description technique.


 Compagnies d’assurance

 Avez-vous souscrit une
 assurance pour ce produit ?
 Si oui, pour quels risques
 êtes-vous assuré ? Quelles
 exigences votre compagnie
 d’assurance fixe-t-elle ?

Dans l’affirmative, veuillez fournir une copie du contrat, accompagnée d'une description technique si elle doit
accompagner le contrat d'assurance.




12/110
                             II. DESCRIPTION DU POS
             A. IDENTIFICATION DU LOGICIEL

     Identification du logiciel

     Nom du POS
     Modèle du POS
     N° de version du logiciel
     Langage(s) de
     programmation


             B. DÉCLARATION D’ACCÈS À LA BASE DE DONNÉES

         Déclaration selon laquelle les utilisateurs n’ont accès qu’en lecture seule            ☐

             C. CARACTÉRISTIQUES DE LA BASE DE DONNÉES

      Caractéristiques de la base de données
      La base de données est…                          ☐ Centrale     ☐ Locale     ☐ Les deux
      La base de données et le POS sont tous les       ☐ OUI          ☐ NON
      deux basés sur le cloud
      Identification du DBMS

      N° de version de la base de
      données
      Nom du logiciel et version



      Chemin d’accès




Un aperçu doit être fourni, en annexe à cette demande, sous la forme d’une liste des champs suivants :

            Nom de la table, nom du champ, type de champ, longueur du champ
            Brève description si le nom du champ n’est pas explicite (en anglais, néerlandais ou français)

     Base de données

     La liste contient toutes les colonnes demandées                                  ☐
     La description est brève et claire                                               ☐




13/110
      Informations supplémentaires à fournir au SPF Finances
      Description du format de la base de données                                          ☐
      Sécurité de la base de données*                                                      ☐
      Données utilisées et crées par le POS                                                ☐
      Aperçu des tables contenant les données des events                                   ☐
      Disposition des tables de données                                                    ☐
      Relations entre les tables                                                           ☐
*Description des mécanismes de sécurité pour l’accès à la base de données et sa conservation (droits
d’accès, profils, etc.).

      Captation de la base de données

      Les agents du SPF peuvent accéder/copier les données pertinentes.                    ☐
      L’URL, le nom d’utilisateur et le mot de passe                                       ☐
      Les données exportées doivent être converties ?                                      ☐
      Les scripts/programmes de conversion sont fournis                                    ☐


      Sécurité de la base de données
                                                                                           ☐ OUI
      Audit trail possible (enregistrement des timestamps, utilisateurs, Primary Key...)
                                                                                           ☐ NON
      Quelles sont les mesures de sécurité mises en place contre la manipulation frauduleuse
      (suppression, modification etc.) des données ? Veuillez décrire svp.




14/110
             D. GARANTIE DU PRODUIT

Quelles sont les garanties que vous avez mis en place afin de s’assurer que le logiciel de la machine-test et
celui du SCE installé chez le client sont identiques ?




             E. MISES À JOUR

Pour chaque modification sur le POS, une demande de certification de mise à jour est nécessaire. Ces mises à
jour sont testées sur la machine de test. Comment implémentez-vous les mises à jour sur la machine de test ?
(Mise à jour à distance, mise à jour via “store”, etc.)




Dans le cadre de la certification, il est recommandé d’intégrer une solution permettant un accès à distance
sécurisé à la caisse enregistreuse afin de faciliter les interventions techniques, les mises à jour logicielles et
l’installation de correctifs.

L’utilisation d’un logiciel d’accès à distance tel que TeamViewer, AnyDesk, RustDesk, VNC, Zoho Assist ou
Splashtop est encouragée afin de :

    1.   Permettre l’installation de mises à jour et de correctifs à distance,
    2.   Faciliter l’assistance technique en cas de problème,
    3.   Réduire les temps d’intervention,

Si une telle solution est mise en place, vous devrez :

     Installer le logiciel sur une machine test afin de valider son bon fonctionnement,
     Fournir, si nécessaire, les identifiants et mots de passe pour accéder à la session de test,
     Privilégier si possible un accès complet sans limitation de temps afin d’éviter toute déconnexion
      intempestive (non obligatoire, mais recommandé).

Le choix d’une solution d’accès à distance reste à la discrétion du producteur, mais son intégration permettrait
de garantir une meilleure gestion des mises à jour et des interventions techniques, tout en assurant un gain
de temps significatif pour l’exploitant.



15/110
                     F. MATÉRIEL

Ce tableau couvre à la fois le matériel délivré à vos clients et le matériel fourni au service SCE en tant que
machine-test.

Il n'est pas prévu de recevoir un système de caisse enregistreuse équipé de toutes les options possibles, mais
les options obligatoires nécessaires pour permettre le test complet de toutes ses fonctionnalités sont
néanmoins requises.

La colonne « délivré » servira de base pour la feuille d’Intake, listant le matériel que vous nous prêterez avec
leurs numéros de série respectifs.

 Type                                          Délivré en tant que   Machine de            Description et informations
                                               machine-test à        production pour vos   supplémentaires
                                               l’Intake              clients
 Ordinateur(s)
 Station d’accueil + backend éventuel.

                                         POS   ☐                     ☐
                      Ordinateur fixe          ☐                     ☐
              Ordinateur portable              ☐                     ☐
                                 Tablette      ☐                     ☐
                                     Kiosk     ☐                     ☐                     ☐ Le POS est seulement un kiosk
 FDM                                           ☐                     ☐
 Multiprise                                    ☐                     ☐
 Imprimante thermique                          ☐                     ☐
 Connectivité                                  ☐                     ☐
                           Router/Hub          ☐                     ☐
                                   Câbles      ☐                     ☐
                                  Clé Wifi     ☐                     ☐
                                     Moxa      ☐                     ☐
                         Raspberry Pi          ☐                     ☐
 Périphériques d’entrée                        ☐                     ☐
                                    Souris     ☐                     ☐
                                   Clavier     ☐                     ☐
  Scanner de produits (client)                 ☐                     ☐
                    Lecteur de carte           ☐                     ☐
                             Clés Dallas       ☐                     ☐
                       Périphérique            ☐                     ☐                     Précisez, svp.
                     supplémentaire
 Périphériques de sortie                       ☐                     ☐
                 Ecran d’utilisateur           ☐                     ☐
  Ecran pour la cuisine/le bar                 ☐                     ☐
                                 Balance       ☐                     ☐                     Pour la fonction "prix au kg"
 Livraison                                     ☐                     ☐
     Distributeur de nourriture                ☐                     ☐


16/110
                         Robot        ☐               ☐
 Commande à distance                  ☐               ☐
            Boutique en ligne         ☐               ☐
          Commande en ligne           ☐               ☐
          Scanner de code QR          ☐               ☐
 Facturation/paiement                 ☐               ☐
     App de commande avec             ☐               ☐
                  facturation
   Processus via site internet        ☐               ☐
       Tiroir-caisse connecté         ☐               ☐
 Autres              Précisez, svp.   ☐               ☐

              G. SCHÉMA D’ASSEMBLAGE

Un schéma de montage permettra l'installation physique du POS au cas où les agents RCRS ne le
connaîtraient pas. Il précisera quel port doit être utilisé pour quel élément matériel. Si le montage implique
une configuration supplémentaire (adressage IP/Mac/Port/réseau), veuillez fournir les données pertinentes
à ce sujet.

Merci de fournir le schéma d’assemblage en annexe.



                   Schéma d’assemblage fourni en annexe                                 ☐

      Remarques point I.




17/110
                      III. FONCTIONNALITÉS DU POS

                                   CONFIGURATION DU POS
             A. PARAMÈTRES D’INSTALLATION

Décrivez les paramètres qui doivent/peuvent être configurés lors de l'installation. Veuillez indiquer quels
paramètres sont fixes et ne peuvent plus être modifiés. Décrivez qui (quel rôle) peut installer et modifier les
paramètres par la suite. Pour cela, une référence à un manuel d'installation est suffisante, à condition qu'il
contienne toutes les informations demandées.




      Configuration

      La liste est claire et complète                                          ☐
      Les paramètres définis sont indiqués                                     ☐
      Les paramètres définis peuvent être fixes                                ☐ Oui      ☐ Non
      Les logs sont stockés                                                    ☐ Centralement
                                                                               ☐ Localement
      Les logs ne peuvent pas être supprimés                                   ☐

             B. CONFIGURATION INTERNET

     Connectivité

     Le POS nécessite une connexion internet       ☐ Oui            ☐ Non
     La connexion internet peut se faire par...    ☐ Ethernet       ☐ Wifi       ☐ Les deux

     Besoins spécifiques pour la mise en place
     du réseau




18/110
             C. UTILISATEURS & MOTS DE PASSE

                                Nom d’utilisateur         Mot de passe              Commentaires

     OS login                                                                 Idéalement, aucun.

     POS login                                                                Variations de 1234 ...

         Administrateur
                 Serveur

              Technicien                                                      NISS = 00000000097

             Kiosk / App                                                      NISS = 00000000029

                   Autre



                 Gestion des utilisateurs
Veuillez fournir la description de la gestion des utilisateurs et des rôles ainsi que l'endroit sur la caisse (nom
de la table, nom du champ...) où sont stockées ces données utilisateurs (identité, enregistrements de temps,
identifiants). Indiquez les droits d'accès (lecture, ajout, modification, suppression) aux différentes données :
base de données et logs.




     Gestion des utilisateurs

     Description de la gestion des utilisateurs et des rôles                   ☐
     Description des droits d’accès                                            ☐


      Remarques point II.




19/110
                                  CONNEXION AVEC LE FDM
             A. CONNEXION DU POS AU FDM

                            Mode
                            Autonome                                        ☐
                            LAN                                             ☐
                            Basée sur le cloud                              ☐
                            Configurable                                    ☐


                    Options
                    Load-balancing                                        ☐ Oui ☐ Non
                          Si OUI, utilisation d’un middleware ?          ☐ Oui ☐ Non
                    POS compatible avec d’autres marques FDM              ☐ Oui ☐ Non
                    Les terminaux ne peuvent pas communiquer          ☐
                    avec le FDM à moins d’être identifiés avec
                    posId.
                    Si un terminal peut communiquer directement
                    avec le FDM, il doit être considéré comme un
                    POS avec son propre posId.



             B. CONNEXION AU FDM

Veuillez décrire comment le lien FDM peut être créé et configuré dans la caisse enregistreuse. Si vous utilisez
un logiciel intermédiaire externe (éventuellement de fabricants FDM), veuillez clairement l’indiquer.




                            La connexion au POS est expliquée               ☐


                 Logiciel intermédiaire externe

     Aucun                                 ☐
     Logiciel du fabricant du FDM          ☐                                          Veuillez préciser

     Autre application                     ☐                                          Veuillez préciser



20/110
             C. FDM NON-OPÉRATIONNEL

Si le FDM est déconnecté, l'utilisateur ne peut plus être en mesure de saisir des transactions.

    a)   Veuillez préciser si la caisse enregistreuse génère immédiatement une erreur ou seulement lorsqu’il
         n’y a pas de réponse du FDM quand des données de transaction lui sont envoyées.
    b)   Veuillez décrire le comportement de la caisse enregistreuse lorsqu'une transaction a été saisie et
         envoyée au FDM, mais qu'elle ne peut pas être enregistrée dans le FDM.




                            La déconnection du FDM est expliquée            ☐




21/110
                                       FONCTIONNALITÉS DU POS
                   A. EVENTS POSSIBLES

                                   Disponible ?        Imprimable     sur   Imprimable          Copie possible ?
                                                       papier ?             digitalement    ?   (Event C)
                                                                            (Comment ?)
           N – Sales                 Obligatoire       Obligatoire          Obligatoire         ☐
           R – Report                Obligatoire       ☐                    ☐                   ☐
           S – Social                Obligatoire              N/A                   N/A
           P – Pro Forma           ☐                   ☐                    ☐                   ☐
           T – Mode Training       ☐                   ☐                    ☐                   ☐
           I – Invoice             ☐                   ☐                    ☐                   ☐
           C – Copy                ☐                   ☐                    ☐                        Interdit
           F - Financial           ☐                   ☐                    ☐                   ☐

                   B. NUMÉROTATION D’EVENTS

                           Numérotation           Numérotation

                           PosFiscalTicketNo      ☐ Globale ☐ Par type d’event ☐ Par Terminal
                           Numérotation des       ☐ Globale ☐ Par type d’event ☐ Par Terminal
                           tickets
                                                  ☐ Autre, veuillez préciser.




                   C. FONCTIONNALITÉS INTERDITES

Un système de point de vente (POS) enregistré ne doit pas comporter de fonctionnalités permettant de supprimer,
modifier ou ajouter des enregistrements déjà enregistrés.


           Le producteur déclare que de telles fonctionnalités ne sont pas disponibles.         ☐

            D. FONCTIONS OBLIGATOIRES

Enregistrement zéro
Veuillez décrire comment un enregistrement sans ligne de produit peut être généré, si cela est possible. Si un tel
enregistrement zéro n'est jamais possible, veuillez l’indiquer formellement dans le rapport.



      22/110
             E. ENREGISTREMENT DE LA PRÉSENCE

La connexion d'un utilisateur et l'enregistrement de son temps de présence sont deux concepts distincts.

Aucune opération ne peut être possible sur le système POS sans qu'un utilisateur ne soit connecté. Ainsi, chaque
opération est attribuée à un utilisateur spécifique.

L'enregistrement de temps de présence est utilisé pour les questions de sécurité sociale, liées à d'éventuelles
réductions sur les cotisations de sécurité sociale. Bien que son utilisation ne soit pas obligatoire, le POS doit offrir la
possibilité d'enregistrer le temps de présence, en utilisant l’event S (Work In/Out). Cela signifie également que
l'enregistrement du temps des utilisateurs qui ne travaillent généralement pas avec le POS doit également être
possible.

Veuillez décrire comment l'enregistrement est implémenté dans le système POS (et si cela est lié à l’event S)




Veuillez décrire comment l’event S est implémenté sur le POS.




Enregistrement de la présence
Description de l’utilisation                                                              ☐
L’enregistrement du temps de présence est lié aux boutons “Work In/Work ☐ OUI ☐ NON
Out”.




23/110
                F. FONCTIONNALITÉS AVANCÉES

                                                                                                              UseCase
           Fonctions                                  Description                               Utilisée
                                                                                                           correspondant
  Normal (N)                                                                                Obligatoire    M110

     Remboursement total                                                                    ☐              M111

     Remboursement partiel                                                                  ☐              M112

  Report (R)                                                                                Obligatoire    Reports

  Social (S)                                                                                Obligatoire    M140/141

                                 Ticket (non-TVA) pour enregistrer les opérations
  Pro Forma                                                                                 ☐              M121

                                 La commande est enregistrée et peut être modifiée
    Commande en attente
                                 avant le paiement
                                                                                            ☐
                                 Résumé de l’état de la commande avant paiement
    Prebill Pro Forma                                                                       ☐              M123

                                 Optionnel. Veuillez préciser si le format est digital ou
  Tous les events P sont-ils
  imprimables ?                  papier.
                                                                                            ☐
                                 Possibilité d’attribuer et déplacer des transactions et
                                 des produits vers des cost centers spécifiques (tels
                                                                                            ☐              M122
  Gestion des cost centers
                                 que des tables, chaises, clients…). Implique des tickets
                                 Pro Forma.
                                 Permet d’affecter des produits à un cost center ou les
    Attribution
                                 déplacer vers un nouveau cost center.
                                                                                            ☐
                                 Permet de déplacer des produits d’un cost center vers
    Division
                                 différents cost centers (one-to-many)
                                                                                            ☐              UC240/241/243

                                 Permet de déplacer des produits de différents cost
    Fusion
                                 centers vers un seul cost center (many-to-one)
                                                                                            ☐              UC250/251/252

                                 Permet plusieurs paiements par addition ou plusieurs
  Gestion d’addition
                                 tickets TVA par addition.
                                                                                            ☐
                                 Une addition est payée avec plusieurs paiements ou
    Division de table
                                 méthodes de paiement. 1 ticket de TVA
                                                                                            ☐
    Division de l’addition (en   Une addition est divisée en plusieurs additions (par
    parts égales)                exemple 1 par client), plusieurs tickets de TVA
                                                                                            ☐
                                 Une addition est divisée en plusieurs additions de
    Division de l’addition
                                 montants égaux, plusieurs tickets de TVA
                                                                                            ☐
                                                                                                           UC150
  Invoice (I)                                                                               ☐
                                 = facture négative
    Note de crédit                                                                          ☐
                                 Copy d’un event précédent.                                                M160
  Copy (C)                                                                                  ☐
                                 Mode formation
  Training (T)                                                                              ☐              M171

                                                                                                           M130
  Gestion des cash flows                                                                    ☐
   Gestion du tiroir-caisse                                                                 ☐              M131

  Moyens de paiement                                                                        ☐
                                 Implique d’avoir un tiroir-caisse (connecté ou non)
    Cash                                                                                    ☐
    Carte de débit                                                                          ☐
    Carte de crédit                                                                         ☐
                                 Implique d’avoir un système de gestion des clients
    Crédit client                                                                           ☐
                                 Implique d’avoir un système de facturation externe
    Facturation à l’hôtel                                                                   ☐


24/110
                                Tickets restaurant, cartes restaurant.
    Chèque repas                                                                           ☐
                                La possibilité de changer le moyen de paiement après
  Changement de paiement
                                le paiement initial.
                                                                                           ☐   M132

                                Bon d’achat à but multiple. Considéré comme un
  Vouchers à but multiple
                                moyen de paiement.
                                                                                           ☐   UC260/261

                                Peut être acheté et taxé comme un repas normal.
  Vouchers à but unique                                                                    ☐   UC270/271

                                Paiements dans des monnaies autres que l’euro.
  Devise étrangère                                                                         ☐   UC296

                                Obligatoire pour le cash, interdit pour les bons
  Arrondissement des
                                                                                           ☐   Roundings
  paiements
                                Optionnel. Doit être décidé une fois pour toute à
    Arrondissement des
    paiements par carte EFT     l’installation.
                                                                                           ☐   Roundings

                                Le ticket TVA est-il imprimé avant ou après la livraison
  Gestion des livraisons
                                ?
                                                                                           ☐
                                Ne peuvent pas excéder 100%
  Réductions                                                                               ☐
                                Produits composés de 2 ou plus d’autres produits
   Produits composés
                                existants (ex : menus)                                     ☐
                                Applique un changement de prix à une ligne de                  UC220
   Modification de prix ligne
                                produit.
                                                                                           ☐
                                Applique un changement de de prix à la totalité de
   Modification de prix event
                                l’event.
                                                                                           ☐   UC223

                                Les lignes peuvent avoir plusieurs modifications de
   Modification de prix
                                prix (provenant de différents line price change et/ou
                                                                                           ☐   UC224
  multiple                      event price change)
                                Modification quantité
   Corrections                                                                             ☐   UC230

                                100% réduction
   Article offert/gratuit                                                                  ☐
                                Réductions automatiques sur certaines plages
   Happy Hour programmé
                                horaires
                                                                                           ☐
  Gestion des clients                                                                      ☐
    Réduction membre            Réduction membre                                           ☐
    Mensualités                 Paiement mensuel                                           ☐
  Prix libre                    Prix libre / « prix du jour »                              ☐
  Prix au poids                 Prix au poids                                              ☐
                                Bar et cuisine. Pas de prix, pas de donnée de contrôle
  Ticket de service                                                                        ☐
                                Veuillez préciser, s’il-vous-plait
  Autres                                                                                   ☐
           Remarques




25/110
                  Remarque importante concernant la facturation
À partir du 01.01.2026, toutes les factures émises dans un contexte B2B (et B2G) doivent légalement être émises
de manière électronique structurée et envoyées au destinataire final via un point d'accès du réseau Peppol. Le POS
peut en plus, toujours créer des impressions papier et autres impressions numériques des factures.

Les émissions de factures dans le contexte B2C ne sont pas concernées par ce nouveau règlement.

Veuillez décrire dans l'encadré ci-dessous comment votre POS se conforme à cette nouvelle réglementation
(connexion POS directe au réseau Peppol, API du POS au point d'accès du réseau Peppol, API vers un logiciel de
facturation conforme…).

Veuillez noter qu'en cas de non-conformité, l'émission de factures sur le POS doit être désactivée.




              G. COSTCENTERCHANGE (GESTION DE TABLE)

     Changement de cost center
     Le POS permet le changement de cost center (costCenterChange) vers                    ☐ Oui ☐ Non
     ‘ROOM’ (chambre)
     Le POS permet de recalculer le changement de prix (priceChanges) en cas de            ☐ Oui ☐ Non
     changement de cost center*

     *Si oui, la meilleure pratique consiste à utiliser 2 étapes :
                                 1. Effectuer le costCenterChange
                                 2. Un autre signOrder avec le recalcul



Veuillez décrire où se trouve le lien dans la base de données entre :
                  a. le ticket d'origine et les tickets divisés/fusionnés
                  b. le cost center d’origine et le cost center divisé/fusionné
                  c. le cost center d'origine et le cost center vers lequel il a été modifié




26/110
     Changement de cost center
     Les liens dans la base de données sont décrits                                   ☐

             H. EMISSION DES TICKETS NUMÉRIQUES

Bien que la possibilité d'émettre des tickets sous forme numérique soit obligatoire, il n'existe pas de spécifications
obligatoires concernant la mise en œuvre de cette mesure. Veuillez décrire comment cette mesure est mise en
œuvre dans votre POS et comment la remise réelle du ticket numérique peut être garantie.




             I.   TERMINAUX ET APPAREILS

         Terminaux et Appareils
         Le POS permet-il de travailler avec plusieurs périphériques sur un      ☐ OUI       ☐ NON
         terminal (roaming terminals) ?




27/110
             J.    PÉRIODES

         Périodes
         Le POS permet-il de travailler à la fois avec des périodes               ☐ OUI ☐ NON
         d'ouverture (bookingDate) et des créneaux horaires
         (bookingPeriod) ?
         Si oui, comment cela est-ce configuré ?




             K. TIROIRS-CAISSE

         Drawers

         Est-ce que cette fonctionnalité est disponible sur votre POS ?
                                                                                  ☐ OUI ☐ NON

                        SI OUI UNIQUEMENT

         S'agit-il d'un tiroir-caisse/portefeuille physique ?                           ☐ OUI ☐ NON
         Est-il possible d'avoir plusieurs tiroirs par POS ou par utilisateur ?         ☐ OUI ☐ NON
         S'agit-il d'un tiroir-caisse virtuel ?                                         ☐ OUI ☐ NON
         Aucun (tiroir-caisse par défaut “hardcodé” pour les mutations graphQL)         ☐ OUI ☐ NON
         Veuillez préciser comment cela est implémenté
         ! Si vous utilisez un tiroir-caisse, les rapports utilisateurs doivent être ventilés par moyen de
         paiement




28/110
           L. RAPPORTS



    Rapports

    Rapports Z : Un seul rapport par BookingDate                                   ☐ OUI ☐ NON
    Rapports Financier Z et Utilisateurs Z : Sont-ils liés et émis ensemble ?      ☐ OUI ☐ NON
    Rapports Financier Z et Utilisateurs Z : Sont-ils émis automatiquement ?       ☐ OUI ☐ NON
    (Préférable)
    Le POS dispose-t-il d'autres rapports que les rapports obligatoires X et Z ?   ☐ OUI ☐ NON

                  Si oui uniquement : veuillez les décrire et montrer comment ils correspondent
                   aux montants des rapports fiscaux. Veuillez également fournir les requêtes
                   utilisées.




           M. INTÉGRATION DES TIERS-PARTIES (API, ETC)

    Nom
    Deliveroo                       ☐
    TakeAway.com                    ☐
    Uber Eats                       ☐
    Autres partenaires              Veuillez préciser.




    Remarques sur les fonctionnalités du POS




29/110
                                 LIGNES DE PRODUIT (PRODUCT LINES)
                                                                                                          Label TVA
                                Nom du produit                         Nom du
Disponible    productId                               departmentId                 Prix        Unité
                                (productName)                        département                        Sur           À
                                                                                                       place       emporter

    ☐         10006          Dry Martini                       10 Aperitifs           12       PIECE           A         A

    ☐         12007          Perrier                           12 Soft Drinks         2,5      PIECE           A         C

    ☐         12009          Coca Cola                         12 Soft Drinks       3,04       PIECE           A         C

    ☐         15001          Bourgogne Red                     15 Wines               35       PIECE           A         A

    ☐         15002          Chardonnay White                  15 Wines               40       PIECE           A         A

    ☐         15003          Wine (glass)                      15 Wines                    8   PIECE           A         A

    ☐         15023          Matching Wines                    15 Wines               24       PIECE           A         A

    ☐         20001          Carpaccio Beef                    20 Starters            20       PIECE           B         C

    ☐         22001          Burger of the Chef                22 Main Dishes         28       PIECE           B         C

    ☐         22005          Catch of the day                  22 Main Dishes         45          KG           B         C
                                                                                   Prix
    ☐         22010          Dish of the day                   22 Main Dishes
                                                                                   libre
                                                                                               PIECE           B         C

    ☐         25003          Dame Blanche                      25 Desserts         11,52       PIECE           B         C

    ☐         25008          Crème brûlée                      25 Desserts            10       PIECE           B         C

    ☐         28007          Tapas variation                   28 Tapas             3,34       PIECE           B         C

    ☐         30001          Business menu (*)                 30 Menus               38       PIECE           B         C
                             Business menu all-in
    ☐         30010
                             (**)
                                                               30 Menus               58       PIECE    A&B           A&C

    ☐         30020          Menu drink (***)                  30 Menus               12       PIECE           A      A&C

    ☐         40001          Gift Voucher General             400 Vouchers            50       PIECE           X         X

    ☐         40002          Gift Voucher General             400 Vouchers           100       PIECE           X         X

    ☐         40003          Gift Voucher General             400 Vouchers           150       PIECE           X         X

    ☐         40004          Gift Voucher General             400 Vouchers           250       PIECE           X         X
                             Voucher Special
    ☐         40020
                             Diner Offer
                                                              400 Vouchers            75       PIECE           B         C

                                                                     Bottle
    ☐         90001          Bottle S                          90
                                                                     Deposits
                                                                                    0,01       PIECE           X         X

                                                                     Bottle
    ☐         90100          Beer bin/ keg beer                90
                                                                     Deposits
                                                                                    4,49       PIECE           X         X

    ☐         500001         Cigarettes (20 pcs)              500 Cigarettes          8,4      PIECE           D         D
     (*) = 1 Carpaccio (EUR 15) + 1 Burger (EUR 23)
     (**) = 1 Carpaccio (EUR 16) + 1 Burger (EUR 24) + 1 Matching Wines (EUR 18)
     (***) = 1 Aperitif + 1 Soft Drink



30/110
     IV. PROCÉDURE DE COMMUNICATION GRAPHQL
                                     PROCÉDURE DE TEST
  Les étapes suivantes doivent être suivies :

         1. A l’aide d’un simulateur FDM, appliquez le scénario décrit à votre POS (et fournissez un
            simulateur avec la machine test).
         2. Vérifiez les montants TVA à l’aide des grilles TVA, si nécessaire.
         3. Vérifiez votre communication JSON à l’aide des Uses Cases du SPF Finances, fournis ci-
            dessous.
         4. Ajoutez les résultats suivants :
                a. La communication JSON entre le POS et le FDM (request et response)
                b. Le ticket (si délivré)
          En annexe à votre demande ou dans un document.

  Si votre POS n’utilise pas une des fonctionnalités, le test ne doit pas être rempli.

  Les Uses Cases peuvent être téléchargés ici :
  https://www.systemedecaisseenregistreuse.be/fr/sce-2-0




31/110
                        VENTE (N) - SIGNSALE MUTATION - M110
                                  L’utilisation de cette mutation est obligatoire.

Description : Enregistrement d'une vente.

 Testing du JSON
 Conditions : Vente directe (in the house)
 Scénario :
     1. Enregistrez 2x Dry Martini et 1x Burger of the Chef
     2. Payez en Cash
     3. Imprimez le ticket de caisse TVA sur papier


 Champs                               À vérifier                                                          Résultats
 Mutation                             signSale                                                                ☐
 language                             FR = Français dans la communication POS-FDM                             ☐
 ticketMedium                         PAPER                                                                   ☐
 transactionLines                                                                                             ☐
           lineType                   SINGLE_PRODUCT                                                          ☐
           quantityType               PIECE                                                                   ☐
           unitPrice                  Dry Martini: 12 | Burger of the Chef: 28                                ☐
           vats                       Dry Martini: label : A | price : 24                                     ☐
                                      Burger of the Chef: label : B | price : 28
 transactionTotal                     52.00                                                                   ☐
 financials                           Type : CASH | amount : 52 | amountType : PAYMENT                        ☐


   Produits            Quantité            Prix TVA         TVA (%)        TVA (euros)      TVA excl.    TVA incl.
                                         incl. / unité
 Dry Martini                      2            € 12,00        21%                  € 4,17      € 19,83      € 24,00
 Burger    of                     1
 the Chef                                      € 28,00        12%                  € 3,00      € 25,00      € 28,00
 Total                                                                             € 7,17      € 44,83      € 52,00


                  Résultats du POS
    1.    Requête JSON
    2.    TICKET

         Remarques




32/110
 REMBOURSEMENT INTÉGRAL (N) – SIGNSALE MUTATION – M111
La fonctionnalité de remboursement intégral est disponible sur le POS : ☐ OUI                          ☐ NON
Description : Permet de rembourser entièrement une vente.

 Testing du JSON
 Scénario :
     1. Remboursez la commande précédente (N SignSale : 2x Dry Martini ; 1x Burger of the Chef)
     2. Faites le remboursement en cash
     3. Imprimez le ticket TVA en version papier


 Champs                          À vérifier                                                              Résultats
 Mutation                        signSale                                                                      ☐
 posFiscalTicketNo               Incrémental                                                                   ☐
 fdmRef                          Données du ticket original (fdmId, fdmDateTime, eventLabel,                   ☐
                                 eventCounter, totalCounter)
 quantity                        Dry Martini : -2 | Burger of the Chef : -1                                    ☐
 negQuantityReason               REFUND                                                                        ☐
 unitPrice                       Dry Martini : 12.00 | Burger of the Chef : 28.00 (lignes positives)           ☐
 lineTotal | price (vats)        Dry Martini : -24.00 | Burger of the Chef : -28.00 (lignes                    ☐
                                 négatives)
 transactionTotal, amount        -52.00 (ligne négative)                                                       ☐


   Produits          Quantité         Prix TVA        TVA (%)       TVA (euros)       TVA excl.         TVA incl.
                                    incl. / unité
 Dry Martini                -2           € 12,00        21%               -€ 4,17        -€ 19,83         -€ 24,00
 Burger    of               -1           € 28,00        12%               -€ 3,00        -€ 25,00         -€ 28,00
 the Chef
 Total                                                                    -€ 7,17        -€ 44,83         -€ 52,00


                   Résultats du POS
    1.    Requête JSON
    2.    TICKET

         Remarques




33/110
  REMBOURSEMENT PARTIEL (N) - SIGNSALE MUTATION – M112
La fonctionnalité de remboursement partiel est disponible sur le POS :                  ☐ OUI       ☐ NON

Description : Permet de rembourser une ou plusieurs lignes d’une vente.

 Testing du JSON
 Scénario :
     1. Ticket original : Vente directe de 2x Dry Martini, 1x Burger of the Chef. (M110)
     2. Remboursez 1x Dry Martini. Enregistrez dans le même enregistrement 1x Coca Cola and 1x
          Crème brûlée
     3. Payez cash 1,04€
     4.     Si votre POS gère la fonction de pourboire, donnez un pourboire de 0,01€. Sinon, arrondissez
            votre paiement à 1,05€ pour vous conformer aux règles de l’arrondi.


 Champs                           À vérifier                                                        Résultats
 mutation                         signSale                                                                 ☐
 quantity                         Dry Martini: -1 | Coca Cola : 1 | Crème brûlée : 1                       ☐
 unitPrice                        Dry Martini: 12.00 (+) | Coca Cola : 3.04 | Crème brûlée : 10            ☐
 lineTotal                        Dry Martini: -12.00 (-) | Coca Cola : 3.04 | Crème brûlée : 10           ☐
 vats | price                     Dry Martini: -12.00 (-) | Coca Cola : 3.04 | Crème brûlée : 10           ☐
 transactionTotal                 1.04                                                                     ☐
 financials                                                                                                ☐
            Pour le cash          Type : CASH | amount : 1.04 | amountType : PAYMENT                       ☐
            Pour le               Type : CASH| amount : 0,01| amountType : TIP/ROUNDING                    ☐
            pourboire/l’arrondi


    Produits         Quantité        Prix TVA         TVA (%)       TVA (euros)        TVA excl.     TVA incl.
                                   incl. / unité
 Dry Martini                 -1          € 12,00        21%              -€ 2,08          -€ 9,92      -€ 12,00
 Coca Cola                    1           € 3,04        21%               € 0,53           € 2,51        € 3,04
 Crème brûlée                 1          € 10,00        12%               € 1,07           € 8,93       € 10,00
 Total                                                                    € 2,52          € 26,52       € 53,04


                   Résultats du POS
          1.    REQUÊTE JSON
          2.    TICKET

     Remarques




34/110
               PRO FORMA (P) - SIGNORDER MUTATION – M121
La fonctionnalité est disponible sur le POS : ☐ OUI             ☐ NON

Description : Permet d’enregistrer et de modifier une commande en mode “gestion de table”.

 Testing du JSON
 Conditions : Commande sur la Table1
 Scenario :
     1. Commandez 2x Dry Martini et 4x Tapas variation
     2. Imprimez le ticket sous format papier




 Champs                  À vérifier                                                          Résultats
 mutation                signOrder                                                              ☐
 ticketMedium            PAPER                                                                  ☐
 costCenter              Id : T1 | type : TABLE                                                 ☐
 transactionTotal        37.36 mais pas de paiement                                             ☐
 financials              Pas de ligne                                                           ☐

                   Résultats du POS
    1.    REQUÊTE JSON
    2.    TICKET

         Remarques




35/110
 CHANGEMENT DE COST CENTER (P) – SIGNCOSTCENTERCHANGE –
                        M122
La fonctionnalité est disponible sur le POS : ☐ OUI                  ☐ NON

Description : En mode gestion de table, cette fonctionnalité est utilisée pour déplacer une commande d’un
cost center à un autre. Elle peut concerner les tables, chaises, clients ou chambres.

 Testing du JSON
 Scénario :
         1. La commande de la T1 (Bar) :
            1x Burger of the Chef (prix par unité 28 EUR, TVA code B)
            1x Dry Martini (prix par unité 12 EUR, TVA code A)
         est déplacée vers la T5 (Salle à manger).
         2. Il n’y a pas de paiement ici.
         3. Cet event n’a pas besoin d’être imprimé




 Champs                  À vérifier                                                       Résultats
 mutation                signCostCenterChange                                                ☐
 ticketMedium            NONE                                                                ☐
 transfer                from: costCenter (id: T1 | type: TABLE | reference: xxxx)           ☐
                         to: costCenter (id: T5 | type: TABLE | reference: same)             ☐
 quantity                T1: -1 | T5: 1                                                      ☐
 unitPrice               Est toujours une ligne positive                                     ☐
 price | linetotal       T1 : (-) | T5: (+)                                                  ☐
 negQuantityReason       COST_CENTER_CHANGE                                                  ☐
 financials              Pas de financials/lignes de paiement                                ☐


                Résultats du POS
1.   REQUÊTE JSON
2.   TICKET

      Remarques




36/110
               ADDITION PROVISOIRE (P) - SIGNPREBILL - M123
La fonctionnalité d’addition provisoire est disponible sur le POS :      ☐ OUI    ☐ NON

Description : L’addition provisoire présentée avant le paiement.

 TESTING du JSON
 Conditions : Table 1
    1. Commandez 2x Dry Martini et 4x Tapas variation sur la Table1 (ref: O158)
    2. Faites une ‘PreBill’ (addition provisoire)
    3. Imprimez la Prebill sous format papier


 Champs                  À vérifier                                               Résultats
 mutation                signPreBill                                                 ☐
 ticketMedium            PAPER                                                       ☐
 CostCenter              Id: T1 | type: TABLE | reference: 0158                      ☐
 transaction             Transactionlines + transactiontotal (37.36)                 ☐
 financials              Pas de financials ou de lignes de paiement                  ☐


                   Résultats du POS
    1.    REQUÊTE JSON
    2.    TICKET

         Remarques




37/110
                   CASH-FLOW (F) – SIGNMONEYINOUT - M130
La fonctionnalité Cash-Flow est disponible sur le POS :          ☐ OUI         ☐ NON

Description : L'enregistrement des opérations financières qui ne sont pas considérées comme un chiffre
d'affaires pour la TVA et qui se rapportent à l'une des situations suivantes :
- l'apport en espèces ;
- la sortie en espèces ;
- l’échange de Vouchers à but multiple contre un paiement en espèces ;
- le paiement des tickets de casse TVA précédemment émis ;

 TESTING du JSON
 Scénario :
 1. Déposez 100€ CASH dans votre tiroir-caisse sans transaction de vente.



 Champs                   À vérifier                                                    Résultats
 mutation                 signMoneyInOut                                                   ☐
 ticketMedium             NONE                                                             ☐
 financials                                                                                ☐
           id             i.e: 1                                                           ☐
           name | type    CASH                                                             ☐
           amount         100                                                              ☐
           amountType     MONEY_IN_OUT                                                     ☐
           drawer         i.e id:1 | name: Tiroir1                                         ☐


                    Résultats du POS
    1.    REQUÊTE JSON
    2.    TICKET

         Remarques




38/110
                     TIROIR-CAISSE - SIGNDRAWEROPEN - M131
La fonctionnalité de tiroir-caisse est disponible sur le POS : ☐ OUI     ☐ NON
Description : Utilisation d’un tiroir-caisse.

 TESTING du JSON
 Scénario :
 1. Le tiroir-caisse « Tiroir1 » est ouvert sans transaction de vente.



 Champs                    À vérifier                                            Résultats
 Mutation                  signDrawerOpen                                           ☐
 Drawer                    i.e id: 1 | Tiroir1                                      ☐


                   Résultats du POS
    1.    REQUÊTE JSON
    2.    TICKET

         Remarques




39/110
                    CHANGEMENT DE MOYEN DE PAIEMENT –
                       SIGNPAYMENTCORRECTION – M132
La fonctionnalité est disponible sur le POS : ☐ OUI                  ☐ NON
Description : Permet de changer la méthode de paiement utilisée sur un ticket après le paiement initial.


 TESTING du JSON
 Scénario :
 Conditions : par exemple pour le ticket suivant : N14.50 (Total : 52 EUR)

 1. Changez le mode de paiement pour cet event. Il a été effectué par carte bancaire (Mastercard via EFT)
 et non en CASH (comme initialement enregistré)



 Champs                  À vérifier                                                         Résultats
 mutation                signPaymentCorrection                                                 ☐
 fdmref                  ex : fdmRef (fdmId | fdmDateTime | eventLabel : N |                   ☐
                         eventCounter : 14 | totalCounter : 50)
 financials                                                                                    ☐
 Original payment        Id: 1 | type : CASH | amount: -52 | drawer: id+name                   ☐
 Payment change          Id: 2 | type: CARD_CREDIT | amount: 52                                ☐


                 Résultats du POS
1.   REQUÊTE JSON
2.   TICKET

      Remarques




40/110
         SOCIAL (S) – SIGNWORKIN /SIGNWORKOUT - UC140/141
                                TEST 1 : TRAVAIL IN (WORK IN)
                          L’utilisation de cette fonctionnalité est obligatoire.

Description : Permet d’enregistrer l’ouverture d’une session de travail d’un utilisateur.

 Testing du JSON
 Scénario :
 1. Enregistrez le « Travail In » d’un employé.



 Champs                  À vérifier                                                         Résultats
 mutation                signWorkIn                                                            ☐
 employeeId              Présent avec un NISS valide (modulo97)                                ☐
 Pas de lignes de                                                                              ☐
 paiement



                             TEST 2 : TRAVAIL OUT (WORK OUT)
                          L’utilisation de cette fonctionnalité est obligatoire.

: Permet d’enregistrer la fermeture d’une session de travail d’un utilisateur.

 Testing du JSON
 Scénario :
 1. Enregistrez le « Travail Out » d’un employé.



 Champs                  À vérifier                                                         Résultats
 mutation                signWorkOut                                                           ☐
 employeeId              Présent avec un NISS valide (modulo97)                                ☐
 Pas de lignes de                                                                              ☐
 paiement
                Résultats du POS
1.   REQUÊTE JSON
2.   TICKET

      Remarques




41/110
                          FACTURE (I) - SIGNINVOICE – UC150
La fonctionnalité facture est disponible sur le POS : ☐ OUI                    ☐ NON
Description : Une facture ne peut être délivrée qu’à partir d’un ou plusieurs ticket(s) TVA précédemment émis.

Objectif : Vérifier l'exactitude des données enregistrées lors d'une transaction ou d'une facture (JSON, tickets,
document établi).

 TESTING du JSON
 Scénario:
 Conditions : par exemple pour les tickets suivants : N 14/50 ; N16/52

 1. Etablissez une facture pour un client (Id 100- customerVatNo BE0308357159)




 Champs                   À vérifier                                                             Résultats
 mutation                 signInvoice                                                                ☐
 InvoiceNo                Valeur incrémentale                                                        ☐
 costCenter               i.e id: 100 | type: CUSTOMER | reference: FIN                              ☐
 fdmRefs                  Les références sur lesquelles la facture est basée (fdmId,                 ☐
                          FdmDateTime, eventLabel, eventCounter, totalCounter).




                                            Testing de la facture

                                            A vérifier                                               Résultats
La cohérence entre les tickets TVA et la facture, notamment les montants et les numéros de                ☐
ticket.

Chaque facture dispose d'un système de numérotation unique et spécifique.                                 ☐
Une numérotation continue pour toutes les factures.                                                       ☐
La facture peut être exportée en tant que document de sortie via le SCE.                                  ☐
La facture peut être imprimée sous forme papier, numériquement, ou dans les deux formats.                 ☐
Le montant de la facture est présenté hors TVA (applicable uniquement aux tickets N).                     ☐


                  Résultats du POS
    1.   REQUÊTE JSON
    2.   TICKET




42/110
     Remarques




43/110
                              COPIE (C) - SIGNCOPY - M160
La fonctionnalité est disponible sur le POS : ☐ OUI                 ☐ NON
Description : Si vous souhaitez imprimer une copie d'un ticket de caisse TVA précédemment émis (event «
NORMAL ») (par exemple, après une panne d'imprimante ou si un ticket est devenu illisible), vous devez créer
un event « copie ». Les copies d’events N portent toujours la référence du ticket TVA d’origine. L’event peut
également être utilisé pour les events ‘P', 'I', 'S', 'F', ou 'R' précédemment émis.

 TESTING du JSON
 Scénario :
  1. Imprimez la copie d’un ticket N (vente), ex : N 16/52.



 Champs                  À vérifier                                                           Résultats
 mutation                signCopy                                                                ☐
 fdmRef                  fdmId + fdmDateTime + eventLabel : N + eventCounter: 16                 ☐
                         +totalCounter: 52


                   Résultats du POS
    1.    REQUÊTE JSON
    2.    TICKET

         Remarques




44/110
                   TRAINING (T) – ISTRAINING BOOLEAN - M171
La fonctionnalité est disponible sur votre POS :           ☐ OUI            ☐ NON
Description : Transactions effectués en mode école/test/formation.

 Testing du JSON
 Scénario :
 Conditions : En mode entrainement, pour un event « T »
         1. Commandez 2x Perrier et 1x Burger of Chef
         2. Payez cash.


 Champs                  À vérifier                                                      Résultats
 transactions            Les lignes de produit (Products, Quantities, VAT Code, Price)      ☐
                         correspondent au scénario décrit.
 financials                                                                                 ☐
                         type: CASH                                                         ☐
                         amount: 33                                                         ☐
                         amountType: PAYMENT                                                ☐
 isTraining              Valeur: true                                                       ☐


                   Résultats du POS
    1.    REQUÊTE JSON
    2.    TICKET

         Remarques




45/110
                       PRODUIT COMPOSÉ – SIGNSALE – UC200
La fonctionnalité est disponible sur votre POS :             ☐ OUI            ☐ NON
Description : Offre la possibilité de regrouper des produits standards en un seul nouveau produit. Peut être
soumis à plusieurs taux de TVA.

 Testing du JSON
 Scenario :
 Conditions: Vente directe
         1. Enregistrez 1x Business Menu All In
         2. Payez Cash



 Champs                     À vérifier                                                            Résultats
 transactionLines                                                                                    ☐
 lineType                   COMPOSITE_PRODUCT                                                        ☐
 subProducts                Carpaccio Beef: 16 | Burger of the Chef: 24 | Matching Wines: 18         ☐
     priceChanges                                                                                    ☐
               id           5                                                                        ☐
               groupingId   1                                                                        ☐
               name         MenuDiscount                                                             ☐
               type         INTERNAL                                                                 ☐
               scope        LINE                                                                     ☐
               amount       Carpaccio: -4 | Burger of the Chef: -6 | Matching Wines: -4              ☐
 transactionTotal           58                                                                       ☐
 Financials                 type : CASH | amount : 58                                                ☐


            Produits               Quantité       Prix TVA   TVA (%)       TVA        TVA excl.   TVA incl.
                                                   incl. /               (euros)
                                                   unité
 Business Menu All In                         1   € 58,00     12%        € 4,29       € 35,71     € 40,00

                                                              21%        € 3,12       € 14,88     € 18,00
 Total                                                                   € 7,41       € 50,59     € 58,00
                    Résultats du POS
         1.   REQUÊTE JSON
         2.   TICKET

      Remarques




46/110
           MODIFICATION DE PRIX PAR LIGNE - SIGNSALE – UC220
La fonctionnalité est disponible sur votre POS :              ☐ OUI              ☐ NON
Description: modification du prix (en + ou en -) d’une seule ligne de produit.

 TESTING du JSON
 Scénario :
 Conditions : Vente directe
 1. Enregistrez
        1x Burger of the Chef avec 50% de réduction
        1x Dry Martini sans réduction
 2. Payez cash.


 Champs                    A vérifier                                                           Résultats
 transactionLines                                                                                  ☐
           unitPrice       28                                                                      ☐
 Vats / price              28                                                                      ☐
     priceChanges          28Id + grouping id + name + scope + type + amount                       ☐
           name            Par ex. : Discount50p                                                   ☐
           scope           LINE                                                                    ☐
           amount          -14                                                                     ☐
 lineTotal                 Burger of the Chef: 14 | Dry Martini: 12                                ☐
 transactionTotal          26                                                                      ☐


     Produits          Quantité      Prix TVA      Discount     TVA        TVA      TVA excl.   TVA incl.
                                   incl. / unité     50%         (%)     (euros)
 Burger of the                1         € 28,00    -14,00       12%       € 1,50     €12,50      € 14,00
 Chef
 Dry Martini                  1         € 12,00       /         21%       € 2,08     € 9,92      € 12,00
 Total                                                                    € 3,58     € 22,42     € 26,00


                   Résultats du POS
    1.    REQUÊTE JSON
    2.    TICKET

         Remarques




47/110
         MODIFICATION DE PRIX PAR EVENT – SIGNSALE – UC223
La fonctionnalité est disponible sur votre POS :             ☐ OUI    ☐ NON
Description : Modification du prix (en + ou en -) d’une vente entière ou de plusieurs produits (par exemple :
Happy Hour)

 TESTING du JSON
 Scénario :
 Conditions : vente directe
 1. Enregistrez :
          2x Carpaccio Beef
          1x Dry Martini
 2. Appliquez une réduction de 50% sur le ticket
 3. Payez avec une carte VISA



 Champs                    A vérifier                                                           Résultats
 transactionLines                                                                                   ☐
 productName               Carpaccio Beef                                                           ☐
     vats                                                                                           ☐
               Price       40                                                                       ☐
     priceChanges                                                                                   ☐
               name        Par ex : Discount50P                                                     ☐
               scope       EVENT                                                                    ☐
               amount      -20                                                                      ☐
               lineTotal   20                                                                       ☐
 productName               Dry Martini                                                              ☐
     vats                                                                                           ☐
               Price       12                                                                       ☐
     priceChanges                                                                                   ☐
               name        Par ex : Discount50P                                                     ☐
               scope       EVENT                                                                    ☐
               amount      -6                                                                       ☐
               lineTotal   6                                                                        ☐
 transactionTotal          26                                                                       ☐
 Financials                provider : VISA | amount : 26                                            ☐


    Produits           Quantité       Prix TVA      Discount   TVA       TVA       TVA excl.     TVA incl.
                                    incl. / unité     50%       (%)    (euros)
 Carpaccio Beef                 2       € 20,00     -20,00     12%     € 2,14       €17,86        € 20,00
 Dry Martini                    1       € 12,00      -6,00     21%     € 1,04       € 4,96        € 6,00
 Total                                                                 € 3,18       € 22,82       € 26,00




48/110
                 Résultats du POS
         1.   REQUÊTE JSON
         2.   TICKET

     Remarques




49/110
         MODIFICATION DE PRIX MULTIPLE – SIGNSALE – UC224
La fonctionnalité est disponible sur votre POS :            ☐ OUI        ☐ NON
Description : Combinaison des réductions LINE et EVENT.

 Testing du JSON
 Scénario :
 Conditions : vente directe
 1. Enregistrez :
          1x Burger of the Chef avec une réduction de 20%
          1x Perrier
 2. Appliquez une réduction de 20% sur le ticket
 3. Payez avec une carte VISA



 Champs                 A vérifier                                                            Résultats
 mutation               signSale                                                                 ☐
 transactionLines                                                                                ☐

                        Burger of the Chef
 unitPrice              28.00                                                                    ☐
 lineTotal              17.92                                                                    ☐
     priceChanges       Deux réductions : une de type "LINE" et une de type “EVENT".             ☐
         LINE           amount : -5,60                                                           ☐
         EVENT          amount : -4,48                                                           ☐
                              groupingId est obligatoire                                        ☐
                        Perrier
 unitPrice              2,50                                                                     ☐
 lineTotal              2,00                                                                     ☐
     priceChanges       One discount                                                             ☐
         EVENT          amount : -0,5                                                            ☐
         groupingId     1                                                                        ☐
 transactionTotal       19,92                                                                    ☐
 financials             19,92                                                                    ☐


   Produits      Quantité      Prix TVA   Discount   TVA        TVA      TVA      TVA incl.    Produits
                                incl. /     20%       (%)     (euros)    excl.
                                unité
 Burger of the          1      € 28,00       -5,60   -4,48     12%      € 1,92     € 16,00     € 17,92
 Chef
 Perrier                1       € 2,50         /     -0,50     21%      € 0,35     € 1,65      € 2,00
 Total                                                                  € 2,27     € 17,65     € 26,00




50/110
                  Résultats du POS
   1.    REQUÊTE JSON
   2.    TICKET

        Remarques




51/110
                         CORRECTION – SIGNSALE (N) – UC230
La fonctionnalité est disponible sur votre POS :                  ☐ OUI       ☐ NON
Description : Permet de modifier les quantités dans une commande.

 Testing du JSON
 Scénario :
 Conditions : vente directe
 1. Enregistrez :
          3x Dry Martini
 2. Modifiez la quantité vers 2
 3. Payez avec une carte de débit


 Champs                   A vérifier                                                         Résultats
 mutation                 signSale                                                              ☐
 transactionLines                                                                               ☐
  quantity                Enregistrement : 3 | Correction : -1                                  ☐
  unitPrice               Enregistrement : 12 | Correction : 12                                 ☐
  vats : label | price    Enregistrement : 36 | Correction : -12                                ☐
  lineTotal               Enregistrement : 36 | Correction : -12                                ☐
  transactionTotal        24                                                                    ☐
 financials/amount        24                                                                    ☐


              Produits     Quantité          Prix TVA      TVA        TVA     TVA excl.   TVA incl.
                                           incl. / unité    (%)     (euros)
          Dry Martini                  2     € 12,00       21%       € 4,17   € 19,83     € 24,00
          Total                                                      € 4,17   € 19,83     € 24,00


                   Résultats du POS
    1.    REQUÊTE JSON
    2.    TICKET

         Remarques




52/110
           DIVISION DE TABLE – SIGNCOSTCENTERCHANGE – UC240
La fonctionnalité est disponible sur votre POS :                ☐ OUI             ☐ NON
Description : la fonction de division de table (“table split”) permet de séparer les articles commandés à une
table et de les déplacer vers une ou plusieurs autre(s) table(s), avec chacune leur propre Ticket TVA.

 Testing du JSON
 Scénario :
   OPERATION 1 : Commandez 1x Carpaccio Beef, 1x Burger of the Chef et 2x Dry Martini sur la table 1

   OPERATION 2 : Division de la table : 1x Burger of the Chef et 1x Dry Martini sont déplacés vers la
 table 2 (vide),
   OPERATION 3 : Addition provisoire pour la table 1 (1x Carpaccio Beef et 1x Dry Martini),

      OPERATION 4 : Addition provisoire pour la table 2 (1x Burger of the Chef et 1x Dry Martini),

   OPERATION 5 : Finalisez la commande de la table 1 : paiement de 32 EUR en cash. Si votre POS le
 permet, ajoutez un pourboire de 4 EUR cash,

      OPERATION 6 : Finalisez la commande de la table 2 : paiement de 40 EUR par carte Visa.




OPERATION 1 : Commande de 1x Carpaccio Beef, 1x Burger of the Chef et 2x Dry Martini à la table 1

 Champs                    A vérifier                                                          Résultats
 Mutation                  signOrder                                                                 ☐
 costCenter                Id : 1 | type : TABLE | reference: 0158                                   ☐
 transactionTotal          72                                                                        ☐


OPERATION 2 : Division – 1x Burger of the Chef et 1x Dry Martini sont déplacés vers la table 2 (vide)

Champs                     A vérifier                                                          Résultats
Mutation                   signCostCenterChange                                                      ☐
transfer                   from / to                                                                 ☐
 from                      costCenter | id | type | reference                                        ☐
     costCenter            Id : 1 | type : TABLE | reference: 0158                                   ☐
                           Pour les 2 produits : quantity: -1 | negQuantityReason:                   ☐
     transaction           COST_CENTER_CHANGE
                           Pour le Burger : unitPrice: 28 | price: -28 | lineTotal: -28              ☐
                           Pour le Dry Martini : unitPrice: 12 | price: -12 | lineTotal: -12         ☐
     transactionTotal      -40                                                                       ☐
to
     costCenter            id : 2 | type : TABLE | reference: T2_601                                 ☐
     transaction           Pour les 2 produits : quantity: 1                                         ☐
                           Pour le Burger : unitPrice: 28 | price: 28 | lineTotal: 28                ☐


53/110
                          Pour le Dry Martini : unitPrice: 12 | price: 12 | lineTotal: 12            ☐
  transactionTotal        40                                                                         ☐


OPERATION 3 : Addition provisoire pour la table 1 (1x Carpaccio Beef et 1x Dry Martini)

 Champs                   A vérifier                                                              Résultats
 Mutation                 signPrebill                                                                ☐
 costCenter               Id : 1 | type : TABLE | reference: 0158                                    ☐
 transactionTotal         32                                                                         ☐


OPERATION 4 : Addition provisoire pour la table 2 (1x Burger of the Chef et 1x Dry Martini)

 Champs                   A vérifier                                                              Résultats
 Mutation                 signPrebill                                                                ☐
 costCenter               Id : 2 | type : TABLE | reference: T2_601                                  ☐
 transactionTotal         40                                                                         ☐


OPERATION 5 : Finalisez la commande de la table 1 : paiement de 32 EUR en cash. Si votre POS le permet,
ajoutez un tip de 4 EUR cash.

 Champs                   A vérifier                                                              Résultats
 Mutation                 signSale                                                                   ☐
 costCenter               Id : 1 | type : TABLE | reference: 0158                                    ☐
 transactionTotal         32                                                                         ☐
 Financials               Payment type : CASH | amount: 32 | amountType: PAYMENT                     ☐
                          Tip type: CASH | amount: 4 | amountType: TIP                               ☐


   Produits         Quantité         Prix TVA        TVA (%)        TVA (euros)       TVA excl.     TVA incl.
                                   incl. / unité
 Carpaccio                     1       € 20,00         12%             € 2,14          € 17,86       € 20,00
 Beef
 Dry Martini                   1       € 12,00         21%             € 2,08          € 9,92        € 12,00
 Total                                                                 € 4,22          € 27,78       € 32,00


OPERATION 6 : Finalisez la commande de la table 2 : paiement de 40 EUR par carte VISA

 Champs                   A vérifier                                                              Résultats
 Mutation                 signSale                                                                   ☐
 costCenter               Id : 2 | type : TABLE | reference: T2_601                                  ☐
 transactionTotal         40                                                                         ☐
                          Id : 3 | type : CARD_CREDIT | inputMethod: AUTOMATIC                       ☐
 Financials               amount: 40 | provider : VISA | amountType: PAYMENT




54/110
     Produits       Quantité         Prix TVA      TVA (%)   TVA (euros)   TVA excl.   TVA incl.
                                   incl. / unité
 Burger    of                  1     € 28,00        12%        € 3,00      € 25,00     € 28,00
 the Chef
 Dry Martini                   1     € 12,00        21%        € 2,08      € 9,92      € 12,00
 Total                                                         € 5,08      € 34,92     € 40,00


                Résultats du POS
1.   Requête JSON
2.   TICKET


       Remarques




55/110
           DIVISION DE TABLE – SIGNCOSTCENTERCHANGE – UC241

 Testing du JSON
 Scénario :
 OPERATION 1: Commandez 1 Burger of the Chef, 1 Dish of the Day (18 EUR) et 2 Dry Martini's sur la
 table 1.

 OPERATION 2: Commandez 1 Dish of the Day et 1 Perrier sur la table 2.

 OPERATION 3 : Division de la table : 1 Dish of the Day et 1 Dry Martini sont déplacés de la table 1 vers la
 table 2.

 OPERATION 4 : Finalisez la commande sur la table 1 : paiement de 40 EUR en cash. Si votre POS le
 permet, ajoutez un pourboire de 5 EUR.

 OPERATION 5 : Finalisez la commande sur la table 2 : paiement de 50,50 EUR avec VISA.


OPERATION 1 : Commandez 1 Burger of the Chef, 1 Dish of the day (18 EUR) et 2 Dry Martini's sur la table 1

 Champs                  A vérifier                                                            Résultats
 Mutation                signOrder                                                                ☐
 costCenter              Id: 1 | type: TABLE | reference: 20240903_001                            ☐
 transactionTotal        70                                                                       ☐


OPERATION 2 : Commandez 1 Dish of the day et 1 Perrier sur la table 2

 Champs                  A vérifier                                                            Résultats
 Mutation                signOrder                                                                ☐
 costCenter              Id: 2 | type: TABLE | reference: T20240903_002                           ☐
 transactionTotal        20,50                                                                    ☐


OPERATION 3 : Division – 1 Dish of the day et 1 Dry Martini sont déplacés de la table 1 vers la table 2

Champs                   A vérifier                                                            Résultats
Mutation                 signCostCenterChange                                                     ☐
transfer                 from / to                                                                ☐
 from
     costCenter          id : 1 | type: TABLE | reference: 20240903_001                           ☐
                         For both products: quantity: -1 | negQuantityReason:                     ☐
     transaction         COST_CENTER_CHANGE
                         For Dish of the day: unitPrice: 18 | price: -18 | lineTotal: -18         ☐
                         For Dry Martini: unitPrice: 12 | price: -12 | lineTotal: -12             ☐
     transactionTotal    -30                                                                      ☐
to



56/110
  costCenter              id: 2 | type: TABLE | reference: 20240903_002                               ☐
  transaction             For both products: quantity: 1                                              ☐
                          For Dish of the day: unitPrice: 18 | price: 18 | lineTotal: 18              ☐
                          For Dry Martini: unitPrice: 12 | price: 12 | lineTotal: 12                  ☐
  transactionTotal        30                                                                          ☐


OPERATION 4 : Finalisez la commande sur la table 1 : paiement de 40 EUR cash, pourboire de 5 EUR

 Champs                   A vérifier                                                               Résultats
 Mutation                 signSale                                                                    ☐
 costCenter               Id: 1 | type: TABLE | reference: 20240903_001                               ☐
 transactionTotal         40                                                                          ☐
 financials               id: 1 | type: CASH | amount: 40 | amountType: PAYMENT                       ☐
                          id: 1 | type: CASH | amount: 5 | amountType: TIP                            ☐


   Produits         Quantité         Prix TVA        TVA (%)       TVA (euros)         TVA excl.     TVA incl.
                                   incl. / unité
 Burger    of                  1       € 28,00         12%            € 3,00           € 25,00        € 28,00
 the Chef
 Dry Martini                   1       € 12,00         21%            € 2,08           € 9,92         € 12,00
 Total                                                                € 5,08           € 34,92        € 40,00


OPERATION 5 : Finalisez la commande sur la table 2, paiement de 50,50 EUR avec VISA

 Champs                   A vérifier                                                               Résultats
 Mutation                 signSale                                                                    ☐
 costCenter               Id: 2 | type: TABLE | reference: 20240903_002                               ☐
 transactionTotal         50,50                                                                       ☐
                          Id: 1 | type: CARD_CREDIT | amount: 50,50 |                                 ☐
 financials               amountType: PAYMENT



   Produits         Quantité         Prix TVA        TVA (%)       TVA (euros)         TVA excl.     TVA incl.
                                   incl. / unité
 Dish of the                   2       € 18,00         12%            € 3,86           € 32,14        € 36,00
 day
 Perrier                       1       € 2,50          21%            € 0,43           € 2,07         € 2,50
 Dry Martini                   1       € 12,00         21%            € 2,08           € 9,92         € 12,00
 Total                                                                € 6,37           € 44,13        € 50,50


                 Résultats du POS
         1.   Requête JSON
         2.   TICKET


57/110
     Remarques




58/110
         DIVISION DE TABLE – SIGNCOSTCENTERCHANGE – UC243
Objectif: Vérifier que la remise est correctement appliquée après une division en plusieurs tables.

 Testing du JSON
 Scénario :
   OPERATION 1 : Sur la table1, commandez 1x Carpaccio Beef, 1x Burger of the Chef et 2x Dry Martini.
 Appliquez une réduction de 10% sur la table1,

    OPERATION 2 : Division de table : 1x Burger of the Chef et 1x Dry Martini sont déplacés vers la table 2
 (vide).

   OPERATION 3 : Addition provisoire pour la table 1 (1x Carpaccio Beef & 1x Dry Martini avec le
 discount),

   OPERATION 4 : Addition provisoire pour la table 2 (1x Burger of the Chef & 1x Dry Martini avec le
 discount),

    OPERATION 5 : Finalisez la commande sur la table 1 : payez 28,80 EUR cash. Si votre POS le permet,
 ajoutez un pourboire de 3 EUR cash.

   OPERATION 6 : Finalisez la commande sur la table 2 : payez 36 EUR avec une carte Visa.




OPERATION 1 : Commandez 1x Carpaccio Beef, 1x Burger of the Chef et 2x Dry Martini avec une réduction
de 10% sur la table 1

 Champs                   A vérifier                                                            Résultats
 Mutation                 signOrder                                                                   ☐
 costCenter               Id: 1 | type: TABLE | reference: 20240903_001                               ☐
 transactionTotal         64.8                                                                        ☐
      Réduction


OPERATION 2 : Division – 1x Burger of the Chef et 1x Dry Martini sont déplacés vers la table 2 (vide)

 Champs                   A vérifier                                                           Résultats
 Mutation                 signCostCenterChange                                                        ☐
 transfer                 from / to                                                                   ☐
 from                     costCenter | id | type | reference                                          ☐
   costCenter             Id : 1 | type: TABLE | reference: 20240903_001                              ☐
                          Pour les deux produits: quantity: -1 | negQuantityReason:                   ☐
   transaction            COST_CENTER_CHANGE
                          Pour Burger: unitPrice: 28 | price: -25.2 | lineTotal: -25.2                ☐
                          Pour Dry Martini: unitPrice: 12 | price: -10.8 | lineTotal: -10.8           ☐
   transactionTotal       -36                                                                         ☐
 to



59/110
   costCenter               id: 2 | type: TABLE | reference: T20240903_002                                ☐
   transaction              Pour les deux produits: quantity: 1                                           ☐
                            Pour Burger: unitPrice: 28 | price: 25.2 | lineTotal: 25.2                    ☐
                            Pour Dry Martini: unitPrice: 12 | price: 10.8 | lineTotal: 10.8               ☐
   transactionTotal         36                                                                            ☐


OPERATION 3 : Addition provisoire pour la table 1 (1x Carpaccio Beef & 1x Dry Martini)

 Champs                     A vérifier                                                                  Résultats
 Mutation                   signPrebill                                                                    ☐
 costCenter                 Id: 1 | type: TABLE | reference: 20240903_001                                  ☐
 transactionTotal           28.8                                                                           ☐


OPERATION 4 : Addition provisoire pour la table 2 (1x Burger of the Chef & 1x Dry Martini)

 Champs                     A vérifier                                                                  Résultats
 Mutation                   signPrebill                                                                    ☐
 costCenter                 Id: 2 | type: TABLE | reference: T20240903_002                                 ☐
 transactionTotal           36                                                                             ☐


OPERATION 5 : Finalisez la commande sur la table 1: paiement cash 28,8 EUR. Si votre POS le permet,
ajoutez un tip de 3 EUR Cash

 Champs                     A vérifier                                                                  Résultats
 Mutation                   signSale                                                                       ☐
     costCenter             Id: 1 | type: TABLE | reference: 20240903_001                                  ☐
 transactionTotal           28.8                                                                           ☐
 Financials                 Id: 1 | type: CASH | amount: 28.8 | amountType: PAYMENT                        ☐
                            Tip type: CASH | amount: 3 | amountType: TIP                                   ☐


  Produits       Quantité          Prix TVA    TVA (%)         TVA         TVA excl.      TVA incl.        Produits
                                    incl. /                  (euros)
                                    unité
 Carpaccio               1           € 20,00        2,00           12%         € 1,93         € 16,18        € 18,00
 Beef
 Dry                     1           € 12,00        1,20           21%         € 1,87          € 8,93        € 10,80
 Martini
 Total                                                                         € 3,80         € 25,11        € 28,80


OPERATION 6 : Finalisez la commande sur la table 2 : payez 36 EUR avec une carte VISA

 Champs                     A vérifier                                                                  Résultats
 Mutation                   signSale                                                                       ☐


60/110
 costCenter                   Id: 2 | type: TABLE | reference: T20240903_002                    ☐
 transactionTotal             36                                                                ☐
                              Id: 2 | type: CARD_CREDIT | inputMethod: AUTOMATIC                ☐
 Financials                   amount: 36 | provider: VISA | amountType: PAYMENT


  Produits         Quantité       Prix TVA     TVA (%)        TVA       TVA excl.   TVA incl.   Produits
                                   incl. /                  (euros)
                                   unité
 Burger of                1         € 28,00        2,80          12%       € 4,37     € 20,83     € 25,20
 the Chef
 Dry                      1         € 12,00        1,20          21%       € 1,87      € 8,92     € 10,80
 Martini
 Total                                                                     € 6,45     € 30,75     € 36,00


                   Résultats du POS
    1.    Requête JSON
    2.    TICKET


         Remarques




61/110
           FUSION DE TABLES – SINGCOSTCENTERCHANGE – UC250
La fonctionnalité est disponible sur votre POS :               ☐ OUI             ☐ NON
Description : La possibilité de combiner plusieurs commandes en cours avec différents cost centers sur un
seul cost center et de créer un seul ticket final.

 Testing du JSON
 Scénario :
    OPERATION 1 : Commandez 1x Dry Martini, 1x Perrier et 1x Burger of the Chef sur la table1
 (signOrder),

      OPERATION 2 : Commandez 1x Dry Martini, 1x Business menu all-in sur la table2 (signOrder),

      OPERATION 3 : Fusion : La commande de la table 2 est déplacée vers la table 1 (signCostCenterChange),

      OPERATION 4 : Finalisez la commande sur la table1, paiement avec carte de crédit.




OPERATION 1 : Commandez 1x Dry Martini, 1x Perrier et 1x Burger of the Chef sur la table 1

 Champs                    A vérifier                                                         Résultats
 Mutation                  signOrder                                                             ☐
 costCenter                Id: 1 | type: TABLE | reference: T1_501                               ☐
 transactionTotal          42,50                                                                 ☐

OPERATION 2 : Commandez 1x Dry Martini, 1x Business menu all-in sur la table 2

 Champs                    A vérifier                                                         Résultats
 Mutation                  signOrder                                                             ☐
 costCenter                Id: 2 | type: TABLE | reference: T2_601                               ☐
 transactionTotal          70                                                                    ☐

OPERATION 3 : Fusion – La commande de la table 2 est déplacée vers la table 1

Champs                     A vérifier                                                         Résultats
Mutation                   signCostCenterChange                                                  ☐
transfer                   from / to                                                             ☐
 from                                                                                            ☐
     costCenter            id : 2 | type: TABLE | reference: T2_601                              ☐
                           Pour les deux produits: quantity: -1 | negQuantityReason:             ☐
     transaction           COST_CENTER_CHANGE
                           Pour le Dry Martini: unitPrice: 12 | price: -12 | lineTotal: -12      ☐
                           Pour le Business menu all-in: unitPrice: 58 | lineTotal: -58          ☐
     transactionTotal      -70                                                                   ☐
to



62/110
  costCenter               id: 1 | type: TABLE | reference: T1_501                                    ☐
  transaction              Pour les deux produits: quantity: 1                                        ☐
                           Pour le Dry Martini: unitPrice: 12 | price: 12 | lineTotal: 12             ☐
                           Pour le Business menu all-in: unitPrice: 58 | lineTotal: 58                ☐
  transactionTotal       70                                                                           ☐
OPERATION 4 : Finalisez la commande sur la table 1, paiement avec carte de crédit

 Champs                    A vérifier                                                              Résultats
 Mutation                  signSale                                                                   ☐
 costCenter                Id: 1 | type: TABLE | reference: T1_501                                    ☐
 transactionTotal          112,50                                                                     ☐
                           Id: 3 | type: CARD_CREDIT | inputMethod: AUTOMATIC                         ☐
 financials                amount: 112,50 | amountType: PAYMENT



   Produits          Quantité         Prix TVA        TVA (%)       TVA (euros)        TVA excl.     TVA incl.
                                    incl. / unité
 Dry Martini                    2       € 12,00         21%            € 4,17          € 19,83        € 24,00
 Perrier                        1       € 2,50          21%            € 0,43          € 2,07         € 2,50
 Burger    of                   1       € 28,00         12%            € 3,00          € 25,00        € 28,00
 the Chef
 Business                       1       € 58,00         12%            € 4,29          € 35,71        € 40,00
 Menu All In                                            21%            € 3,12          € 14,88        € 18,00
 Total                                                                 € 15,01         € 97,49       € 112,50



                   Résultats du POS
    1.    Requête JSON
    2.    TICKET


         Remarques




63/110
         FUSION DE TABLES – SIGNCOSTCENTERCHANGE – UC251
La fonctionnalité est disponible sur votre POS :             ☐ OUI          ☐ NON
Description: La possibilité de fusionner les commandes de plusieurs costCenter sur un seul costCenter
(existant ou non).

 Testing du JSON
 Scénario :
   OPERATION 1: Commandez 1 Dry Martini, 1 Perrier and 1 Burger of the Chef sur la table 1.

   OPERATION 2: Commandez 1 Dry Martini and 1 Business menu all-in sur la table 2.

   OPERATION 3: Table merge : Les commandes des tables 1 et 2 sont déplacées vers la table 5.

   OPERATION 4: Finalisez la commande de la table 5, paiement avec carte de crédit.


OPERATION 1 : Commandez 1x Dry Martini, 1x Perrier and 1x Burger of the Chef sur la table 1

 Champs                 A vérifier                                                             Résultats
 Mutation               signOrder                                                                   ☐
 costCenter              Id: 1 | type: TABLE | reference: T1_501                                    ☐
 transactionTotal       42.5                                                                        ☐


OPERATION 2 : Commandez 1x Dry Martini and 1x Business menu all-in sur la table 2

 Champs                 A vérifier                                                             Résultats
 Mutation               signOrder                                                                   ☐
 costCenter             Id: 2 | type: TABLE | reference: T2_601                                     ☐
 transactionTotal       70                                                                          ☐


OPERATION 3 : Fusion – Les commandes des tables 1 et 2 sont déplacées vers la table 5

 Champs                 A vérifier                                                      Résultats
 Mutation               signCostCenterChange                                               ☐
 transfer               from / to                                                          ☐
  from                  costCenter | id | type | reference                                 ☐
   costCenter 1         Id: 1 | type: TABLE | reference: T1_501                            ☐
                        Pour tous les produits: quantity: -1 | negQuantityReason:          ☐
   transaction          COST_CENTER_CHANGE
                        Pour Dry Martini: unitPrice: 12 | price: -12 | lineTotal: -12      ☐
                        Pour Perrier: unitPrice: 2.5 | price: -2.5 | lineTotal: -2.5       ☐
                        Pour Burger : unitPrice: 28 | price: -28 | lineTotal: -28
   transactionTotal     -42.5                                                              ☐




64/110
      costCenter 2           Id: 2 | type: TABLE | reference: T2_601                                   ☐
                             Pour tous les produits: quantity: -1 | negQuantityReason:                 ☐
      transaction            COST_CENTER_CHANGE
                             Pour Dry Martini: unitPrice: 12 | price: -12 | lineTotal: -12             ☐
                             Pour Business menu all-in: unitPrice: 58 | lineTotal: - 58                ☐

      transactionTotal       -70                                                                       ☐
 to
      costCenter             Id: 5 | type: TABLE | reference: T5_901                                   ☐
                             Pour Dry Martini: unitPrice: 12 | quantity: 2 | price: 24 |               ☐
      transaction            lineTotal: 24
                             Pour Perrier: unitPrice: 2.5 | quantity: 1 | price: 2.5 |                 ☐
                             lineTotal: 2.5
                             Pour Burger du chef: unitPrice: 28 | quantity: 1 | price: 28              ☐
                             | lineTotal: 28
                             Pour Business menu all-in: unitPrice: 58 | quantity: 1 |
                             lineTotal: 58
      transactionTotal       112.5                                                                     ☐


OPERATION 4 : Finalisez la commande de la table 5, paiement avec carte de crédit

 Champs                      A vérifier                                                                    Résultats
 Mutation                    signSale                                                                         ☐
 costCenter                  Id: 5 | type: TABLE | reference: T5_901                                          ☐
 transactionTotal            112.5                                                                            ☐
                             Id: 1 | name: VISA | type: CARD_CREDIT | inputMethod:
 Financials                  MANUAL | amount: 112.5 | amountType: PAYMENT                                     ☐


       Produits          Quantité      Prix TVA        TVA (%)       TVA (euros)       TVA excl.           TVA incl.
                                     incl. / unité
 Perrier                        1          € 2,50            21%           € 0,43             € 2,07           € 2,50
 Business Menu                  1         € 58,00            12%           € 4,29            € 35,71          € 40,00
 All In                                                      21%           € 3,12            € 14,88          € 18,00

 Burger of the                  1         € 28,00            12%           € 3,00            € 25,00          € 28,00
 Chef
 Dry Martini                    2         € 12,00            21%           € 4,17            € 19,83          € 24,00

 Total                                                                   € 15,01             € 97,49         € 112,50
                     Résultats du POS
      1.   Requête JSON
      2.   TICKET




65/110
     Remarques




66/110
                    FUSION DE TABLES – SIGNSALE (N) – UC252
La fonctionnalité est disponible sur votre POS :              ☐ OUI          ☐ NON
Objectif : Vérifier que les remises sont correctement appliquées après une fusion de tables.

 Testing du JSON
 Scénario :
   OPERATION 1 : Commandez 1x Dry Martini, 1x Perrier and 1x Burger of the Chef sur la table 1,

    OPERATION 2 : Commandez 1x Dry Martini avec une réduction de 20% et 1 Business menu all-in sur
 la table 2.

   OPERATION 3 : Fusion – Les commandes des tables 1 et 2 sont déplacées vers la table 5

   OPERATION 4 : Finalisez la commande de la table 5, paiement avec carte de crédit.


OPERATION 1 : Commandez 1x Dry Martini, 1x Perrier and 1x Burger of the Chef sur la table 1

 Champs                  A vérifier                                                            Résultats
 Mutation                signOrder                                                                ☐
 costCenter               Id: 1 | type: TABLE | reference: T1_501                                 ☐
 transactionTotal        42.5                                                                     ☐


OPERATION 2 : Commandez 1x Dry Martini avec 20% de réduction et 1x Business menu all-in sur la table 2

 Champs                  A vérifier                                                            Résultats
 Mutation                signOrder                                                                ☐
 costCenter              Id: 2 | type: TABLE | reference: T2_601                                  ☐
 transactionTotal        67.6                                                                     ☐


OPERATION 3: Table merge : Les commandes des tables 1 et 2 sont déplacées vers la table 5

 Champs                  A vérifier                                                            Résultats
 Mutation                signCostCenterChange                                                     ☐
 transfer                from / to                                                                ☐
  from                   costCenter | id | type | reference                                       ☐
   costCenter 1          Id: 1 | type: TABLE | reference: T1_501                                  ☐
                         Pour tous les produits: quantity: -1 | negQuantityReason:                ☐
   transaction           COST_CENTER_CHANGE
                         Pour Dry Martini: unitPrice: 12 | price: -12 | lineTotal: -12            ☐
                         Pour Perrier: unitPrice: 2.5 | price: -2.5 | lineTotal: -2.5             ☐
                         Pour Burger : unitPrice: 28 | price: -28 | lineTotal: -28
   transactionTotal      -42.5                                                                    ☐




67/110
      costCenter 2             Id: 2 | type: TABLE | reference: T2_601                                            ☐
                               Pour tous les produits: quantity: -1 | negQuantityReason:                          ☐
      transaction              COST_CENTER_CHANGE
                               Pour Dry Martini: unitPrice: 12 | price: -9.6 | lineTotal: -9.6                    ☐
                               Pour Business menu all-in: unitPrice: 58 | lineTotal: - 58                         ☐

      transactionTotal         -67.6                                                                              ☐

 to
      costCenter               Id: 5 | type: TABLE | reference: T5_901                                            ☐
                               Pour Dry Martini 1: unitPrice: 12 | quantity: 1 | price: 12 |                      ☐
                               lineTotal: 12
                               Pour Dry Martini 2: unitPrice: 12 | quantity: 1 | price: 9.6 |
      transaction              lineTotal: 9.6
                               Pour Perrier: unitPrice: 2.5 | quantity: 1 | price: 2.5 | lineTotal:               ☐
                               2.5
                               Pour Burger du chef: unitPrice: 28 | quantity: 1 | price: 28 |                     ☐
                               lineTotal: 28
                               Pour Business menu all-in: unitPrice: 58 | quantity: 1 |
                               lineTotal: 58
      transactionTotal         110.1                                                                              ☐


OPERATION 4: Finalisez la commande de la table 5, paiement avec carte de crédit

 Champs                        A vérifier                                                                      Résultats
 Mutation                      signSale                                                                           ☐
 costCenter                    Id: 5 | type: TABLE | reference: T5_901                                            ☐
 transactionTotal              110.1                                                                              ☐
                               Id: 1 | name: VISA | type: CARD_CREDIT | inputMethod:                              ☐
 Financials                    MANUAL | amount: 110.1 | amountType: PAYMENT


  Produits          Quantité        Prix TVA      Discount        TVA (%)          TVA           TVA excl.        TVA incl.
                                     incl. /        20%                          (euros)
                                     unité
 Perrier                   1            € 2,50              /          21%          € 0,43          € 2,07           € 2,50
 Business                  1           € 58,00              /          12%          € 4,29         € 35,71          € 40,00
 Menu All                                                              21%          € 3,12         € 14,88          € 18,00
 In
 Burger of                 1           € 28,00              /          12%          € 3,00         € 25,00          € 28,00
 the Chef
 Dry                       1           € 12,00              /          21%          € 2,08            € 9,92        € 12,00
 Martini 1
 Dry                       1           € 12,00          2,40           21%          € 1,67            € 7,93          € 9,60
 Martini 2
 Total                                                                            € 14,59          € 95,51         € 110,10




68/110
                  Résultats du POS
   1.    Requête JSON
   2.    TICKET


        Remarques




69/110
   VENTE D’UN BON À USAGE MULTIPLE – SIGNSALE (N) – UC260
La fonctionnalité est disponible sur votre POS :           ☐ OUI             ☐ NON
Description : La vente d'un bon à usage multiple (MPV). Comme la finalité d’un MPV n'est pas prédéfinie, la
TVA ne peut pas être calculée à l’avance. Ce type de bon est exonéré de TVA.

 TESTING DU JSON
 Scénario :
     1. Achetez un MPV : valeur 50€
     2. Payez par carte.



 Champs                  A vérifier                                                         Résultas
 mutation                signSale                                                              ☐
 transactionLines                                                                              ☐
  productId              40001                                                                 ☐
  departmentId           400                                                                   ☐
  departmentName         Vouchers                                                              ☐
  vats                                                                                         ☐
   label                 X (exempté)                                                           ☐
   price                 50                                                                    ☐
  lineTotal              50                                                                    ☐
 transactionTotal        50                                                                    ☐
 Financials/amount       50                                                                    ☐


           Produits    Quantité       Prix TVA   TVA (%)         TVA         TVA excl.   TVA incl.
                                       incl. /                 (euros)
                                       unité
          MPV                  1       € 50,00             X             X     € 50,00     € 50,00
          Total                                                          X     € 50,00     € 50,00


                   Résultats du POS
    1.    Requête JSON
    2.    TICKET

         Remarques




70/110
      UTILISATION D’UN BON À USAGE MULTIPLE – SIGNSALE (N) –
                              UC261
La fonctionnalité est disponible sur votre POS :           ☐ OUI           ☐ NON

 TESTING DU JSON
 Scenario :

 OPERATION 1 : Commandez 2x Business Menu All-In sur la table 1,

 OPERATION 2 : Finalisez le ticket, payez avec le MPV (50 EUR) et le reste en cash.


OPERATION 1 : Commande de 2x Business menu all-in sur la table 1

 Champs                  A vérifier                                                   Résultats
 mutation                signOrder                                                       ☐
 costCenter                                                                              ☐
            id           1                                                               ☐
            type         TABLE                                                           ☐
            reference    0158                                                            ☐
 lineType                COMPOSITE_PRODUCT                                               ☐
 transactionTotal        116                                                             ☐


OPERATION 2 : Finalisez le ticket, payez avec le MPV et en cash

 Champs                  A vérifier                                                   Results
 mutation                signSale                                                        ☐
 costCenter                                                                              ☐
            id           1                                                               ☐
            type         TABLE                                                           ☐
            reference    0158                                                            ☐
 transactionTotal        116                                                             ☐
 financials
 id                      1                                                               ☐
 name                    MPV                                                             ☐
 type                    VOUCHER_STORE                                                   ☐
 amount                  50                                                              ☐
 …                       …
 id                      5                                                               ☐
 name                    CONTANT                                                         ☐
 type                    CASH                                                            ☐
 amount                  66                                                              ☐




71/110
         Produits      Quantité   Prix TVA    TVA (%)   TVA (euros)   TVA excl.   TVA incl.
                                   incl. /
                                   unité
     Business                 2    € 116,00       12%        € 8,57     € 71,43     € 80,00
     Menu All In                                  21%        € 6,25     € 29,75     € 36,00
     Total                                                  € 14,82    € 101,18       € 116


                    Résultats du POS
   1.    Requête JSON
   2.    TICKET


        Remarques




72/110
      VENTE D’UN BON À USAGE UNIQUE – SIGNSALE (N) – UC270
La fonctionnalité est disponible par votre POS :            ☐ OUI         ☐ NON
Description : La vente d'un bon à usage unique (SPV). Comme la finalité du bon est prédéfinie, la TVA est
calculée sur le bon. Ce type de bon n'est donc pas exonéré de TVA.

 TESTING DU JSON
 Scénario :
     1. Achetez un “Voucher Special Diner Offer” valable pour deux 2x Business menus (TVA B) à 75
           EUR,
     2. Payez par carte.


 Champs                   A vérifier                                                         Résultats
 mutation                 signSale                                                                 ☐
 vats                                                                                              ☐
 label                    B                                                                        ☐
 price                    75                                                                       ☐
 lineTotal                75                                                                       ☐
 transactionTotal         75                                                                       ☐
 productLines                                                                                      ☐
 FINANCIALS                                                                                        ☐
 ID                       5                                                                        ☐
 name                     EFT                                                                      ☐
            type          CARD_DEBIT                                                               ☐
 amount                   75                                                                       ☐
 amountType               PAYMENT                                                                  ☐
            mutation      signSale                                                                 ☐
            vats                                                                                   ☐
            label         B                                                                        ☐


            Produits    Quantité       Prix TVA   TVA (%)        TVA      TVA excl.    TVA incl.
                                        incl. /                (euros)
                                        unité
           SPV                  1       € 75,00       12%        € 8,04     € 66,96      € 75,00
           Total                                                 € 8,04     € 66,96      € 75,00
                    Résultats du POS
      1.   Requête JSON
      2.   TICKET




73/110
     Remarques




74/110
UTILISATION D’UN BON À USAGE UNIQUE – SIGNSALE (N) – UC271
La fonctionnalité est disponible par votre POS :            ☐ OUI   ☐ NON

 TESTING DU JSON
 Scénario :

 1. Commandez sur la table1 : 2 dry Martini
 2. Commandez sur la table1 : 2 business menu all-in
 3. PreBill (facture provisoire)
 4. Finalisez le ticket, paiement par SPV & par carte



OPERATION 1 : Commandez 2x Dry Martini, sur la table1

 Champs                  A vérifier                                         Résultats
 mutation                signOrder                                             ☐
 costCenter              Id: 1 | type: TABLE | reference: 0158                 ☐
 lineType                SINGLE_PRODUCT                                        ☐
 transactionLines        2x Dry Martini                                        ☐
 transactionTotal        24                                                    ☐


OPERATION 2 : Commandez 2x Business Menu All-in, sur la table 1

 Champs                  A vérifier                                         Résultats
 mutation                signOrder                                             ☐
 costCenter              Id: 1 | type: TABLE | reference: 0158                 ☐
 lineType                COMPOSITE_PRODUCT                                     ☐
 transactionLines        2x Business Menu All-In                               ☐
   subproducts           2x Carpaccio Beef + 2x Matching Wines                 ☐
 transactionTotal        116                                                   ☐


OPERATION 3 : PreBill (facture provisoire)

 Champs                  A verifier                                         Résultats
 mutation                signSale                                              ☐
 costCenter              Id: 1 | type: TABLE | reference: 0158                 ☐
 transactionLines        2x Dry Martini + 2x Business Menu All-In              ☐
 transactionTotal        140                                                   ☐




75/110
OPERATION 4 : Utilisation du SPV

 Champs                      A verifier                                           Résultats
 mutation                    signSale                                                ☐
 costCenter                  Id: 1 | type: TABLE | reference: 0158                   ☐
 transaction                                                                         ☐
  transactionLines           2x Dry Martini + 2x Business Menu All-in                ☐
   subproducts               2x Carpaccio Beef + 2x Matching Wines + SP Voucher      ☐
    lineType                 SINGLE_PRODUCT                                          ☐
     productName             SPV Menu all-in                                         ☐
     departmentName          Vouchers                                                ☐
     quantity                -1                                                      ☐
     negQuantityReason       VOUCHER                                                 ☐
     quantityType            PIECE                                                   ☐
     unitPrice               58                                                      ☐
     vats                    Label: A | price: -18 + Label: B | price: -40           ☐
    lineTotal                - 58                                                    ☐
  transactionTotal           82                                                      ☐
                             type: CARD_DEBIT | amount: 82 | amountType:             ☐
 financials                  PAYMENT


                   Résultats du POS
    1.    Requête JSON
    2.    TICKET

         Remarques




76/110
                     DEVISES ÉTRANGÈRES – SIGNSALE – UC296
La fonctionnalité est disponible par votre POS : ☐ OUI              ☐ NON

 TESTING DU JSON
 Scénario :
 Conditions : En vente directe
 1. Enregistrez 2x Carpaccio Beef et 2x “Bottle S”
 2. Payez en USD (€1 = $1.1152), le total à payer est $44,63
 3. Payez 45USD CASH (= 40.35 EUR), la monnaie ($0.37) est laissée en pourboire.



 Champs                  A vérifier                                                        Résultats
 mutation                signSale                                                             ☐
 Financials                                                                                   ☐
  amount                 Pour la vente: 40.02 | Tip: 0.37
  foreignCurrency                                                                             ☐
   amount                Vente: 44.63 | Tip: 0.33                                             ☐
   ISO                   USD                                                                  ☐


          Produits     Quantité       Prix TVA      TVA (%)        TVA      TVA excl.   TVA incl.
                                       incl. /                   (euros)
                                       unité
         Carpaccio              2      € 40,00          12%        € 4,29     € 35,71     € 40,00
         Beef
         Bottle S               2         0,01              0%     € 0,00      € 0,02      € 0,02
         Total                                                     € 4,29     € 35,73     € 40,02


                 Résultats du POS
         1.   Requête JSON
         2.   TICKET

      Remarques




77/110
    RAPPORT FINANCIER X (R) – SIGNREPORTTURNOVERX – M180
                                       Cette mutation est obligatoire

Description: Le rapport financier X est un rapport détaillé qui résume les transactions faites sur une caisse
enregistreuse pour une certaine période (bookingDate). Il peut inclure des informations telles que les ventes
totales, le nombre de transactions, les moyens de paiement utilisés (liquide, cartes de crédit, etc.), les
remboursements, les réductions, etc.

N.B : Pour les producteurs qui n'ont pas les options de remise ou de remboursement, ils peuvent créer leurs
propres tests, qui seront vérifiés ultérieurement. Si le pro forma est absent, seules les données liées au pro
forma manqueront, mais cela n'affectera pas le chiffre d'affaires.

Veuillez vous référer au dossier “Report_Testing” en appendice pour effectuer ce test. Vous y
trouverez les Use Cases relatifs à ce rapport.

 Testing du JSON

 Scénario :
 Ticket 1: Enregistrement “Travail In” de l’utilisateur 1 (employeeId : 84022899837).

 Ticket 2 : L’utilisateur 1 enregistre une vente directe de 1x Dry Martini et 1x Carpaccio Beef. Paiement par
 Bancontact.

 Ticket 3 : L’utilisateur 1 annule le ticket 2. Remboursement en cash.

 Ticket 4 : Enregistrement “Travail In” de l’utilisateur 2 (employeeId : 75061189702).

 Ticket 5 : L’utilisateur 2 enregistre une commande pour la table 1 : 1x Dry Martini, 1x Carpaccio Beef et
 1x Crème Brûlée.

 Ticket 6 : L’utilisateur 2 crée une addition provisoire pour la table 1 (commande du ticket 5)

 Ticket 7 : Le client se plaint, l’utilisateur 2 enregistre une réduction de 10% sur la commande de la table 1
 (commande du ticket 5).

 Ticket 8 : L’utilisateur 2 enregistre la vente de la commande de la table 1 (ticket 7). Le client paie 27.80
 EUR avec un bon et 10 EUR en cash.

 Ticket 9 : L’utilisateur 1 enregistre une commande pour la table 1 : 1x Dry Martini et 1x Coca Cola.

 Ticket 10 : L’utilisateur enregistre la vente du ticket 9. Le client paie 15.05 EUR par cash (avec un arrondi
 de +0.01 EUR).

 Ticket 11 : L’utilisateur 2 enregistre une vente directe : 1x Coca Cola et 1x Tapas Variation. Le client paie
 2.00 EUR par Bancontact et 4.40 EUR en cash (avec un arrondi de +0.02 EUR).

 Ticket 12 : Enregistrement “Travail Out” de l’utilisateur 1 (employeeId : 84022899837).



78/110
Le contenu des rapports JSON pour les utilisateurs (X et Z) ainsi que des rapports
financiers (X et Z) est inclus dans l’appendice de ce PDF.
JSON signReportTurnoverX

           Champs                                         A vérifier                             Résultats
Mutation                   signReportTurnoverX                                                      ☐
bookingDate                Valeur correcte                                                          ☐
PosDevices
                           La date et l’heure correspondent-elles à celles du premier ticket ?
firstPosDateTime           (POS)                                                                    ☐
                           La date et l’heure correspondent-elles à celles du premier ticket ?
lastPosDateTime            (POS)                                                                    ☐
fdmDevices                                                                                          ☐
firstFdmDateTime /         Vérifiez que la date et l’heure de début et de fin du ticket
lastFdmDateTime            correspondent à celles du FDM.                                           ☐
firstTotalCounter /
lastTotalCounter           Vérifiez votre numérotation : correspond-elle à celle du ticket ?        ☐
eventLabel: "N"                                                                                     ☐
ticketCount                5                                                                        ☐
amount                     59,22                                                                    ☐
eventLabel: "P"                                                                                     ☐
ticketCount                3                                                                        ☐
amount                     94,84                                                                    ☐
departmentId               10                                                                       ☐
departmentName             Aperitifs                                                                ☐
Amount                     22,80                                                                    ☐
departmentId               20                                                                       ☐
departmentName             Starters                                                                 ☐
Amount                     18,00                                                                    ☐
departmentId               25                                                                       ☐
departmentName             Desserts                                                                 ☐
amount                     9,00                                                                     ☐
departmentId               12                                                                       ☐
departmentName             Soft Drinks                                                              ☐
amount                     6,08                                                                     ☐
departmentId               28                                                                       ☐
 d epartmentName           Tapas                                                                    ☐
amount                     3,34                                                                     ☐
vatLabel: "A"                                                                                       ☐
taxableAmount              23,87                                                                    ☐
vatAmount                  5,01                                                                     ☐
totalAmount                28,88                                                                    ☐


79/110
vatLabel: "B"                                                          ☐
taxableAmount           27,09                                          ☐
vatAmount               3,25                                           ☐
totalAmount             30,34                                          ☐
payments type CASH                                                     ☐
normalAmount            -2,55                                          ☐
negativeCorrections     0                                              ☐
positiveCorrections     0                                              ☐
correctionCount         0                                              ☐
totalAmount             -2,55                                          ☐
payments type CARD_DEBIT                                               ☐
normalAmount            2,00                                           ☐
negativeCorrections     0                                              ☐
positiveCorrections     0                                              ☐
correctionCount         0                                              ☐
totalAmount             2,00                                           ☐
payments type CHEQUE_MEAL                                              ☐
normalAmount            27,80                                          ☐
negativeCorrections     0                                              ☐
positiveCorrections     0                                              ☐
correctionCount         0                                              ☐
totalAmount             27,80                                          ☐
negQuantityChanges                                                     ☐
negQuantityReason"      REFUND                                         ☐
negQuantityCount        2                                              ☐
ticketCount             1                                              ☐
amount                  -32,00                                         ☐
priceChanges                                                           ☐
name                    Bedragkorting                                  ☐
type                    PUBLIC                                         ☐
Amount                                                                 ☐
label                   A                                              ☐
positive                0                                              ☐
negative                -1,20                                          ☐
vatLabel                B                                              ☐
positive                0                                              ☐
Negative                -3,00                                          ☐
reportBookingDate       Pas de reportBookingDate dans les rapports X   ☐

 Imprimez le ticket X, puis clôturez la journée avec un
                       rapport Z.

80/110
                  Résultats du POS
   1.     Requête JSON
   2.     Rapport (digital ou papier)
        Remarques




81/110
              RAPPORT FINANCIER Z (R) – SIGNREPORTTURNOVERZ
                                         Cette mutation est obligatoire.

Description : le rapport financier Z est un rapport généré par le POS qui résume les transactions faites durant
une BookingDate. Contrairement au rapport X, le rapport Z clôture les transactions en cours, réinitialisant les
totaux afin de commencer une nouvelle bookingDate. Un rapport financier Z est établi pour chaque
BookingDate (s’il n’est pas établi quotidiennement)

Ce rapport est numéroté.

N.B : Pour les producteurs qui n'ont pas les options de remise ou de remboursement, ils peuvent créer leurs
propres tests, qui seront vérifiés ultérieurement. Si le pro forma est absent, seules les données liées au pro
forma manqueront, mais cela n'affectera pas le chiffre d'affaires.

 JSON TESTING : Référez-vous au rapport financier X
 This test is based on the Reports_Testing folder
 Scénario :
 Imprimez le rapport financier Z du scénario que vous venez de faire.



                Le test a été fait et le JSON a bien été ajouté      ☐


JSON signReportTurnoverZ

               Champs                                        A vérifier                      Résultats
Mutation                                 signReportTurnoverZ                                      ☐
reportNo                                 Value incremental                                        ☐
reportBookingDate                        Est obligatoire pour les rapports Z.                     ☐


                  Résultats du POS
         1.   Requête JSON
         2.   Rapport (digital ou papier)

      Remarques




82/110
         RAPPORT UTILISATEUR X (R) – SIGNREPORTUSERX - M182
                                       Cette mutation est obligatoire.

Description : Un « rapport utilisateur X » est un document généré par le POS qui résume les transactions
effectuées par utilisateurs durant la BookingDate en cours.

N.B : Pour les producteurs qui n'ont pas les options de remise ou de remboursement, ils peuvent créer leurs
propres tests, qui seront vérifiés ultérieurement. Si le pro forma est absent, seules les données liées au pro
forma manqueront, mais cela n'affectera pas le chiffre d'affaires.

Veuillez vous référer au dossier “Report_Testing” en appendice pour effectuer ce test. Vous y
trouverez les Use Cases relatifs à ce rapport.


 Testing du JSON

 Scénario :

 Ticket 1: Enregistrement “Travail In” de l’utilisateur 1 (employeeId : 84022899837).

 Ticket 2 : L’utilisateur 1 enregistre une vente directe de 1x Dry Martini et 1x Carpaccio Beef. Paiement par
 Bancontact.

 Ticket 3 : L’utilisateur 1 annule le ticket 2. Remboursement en cash.

 Ticket 4 : Enregistrement “Travail In” de l’utilisateur 2 (employeeId : 75061189702).

 Ticket 5 : L’utilisateur 2 enregistre une commande pour la table 1 : 1x Dry Martini, 1x Carpaccio Beef et
 1x Crème Brûlée.

 Ticket 6 : L’utilisateur 2 crée une addition provisoire pour la table 1 (commande du ticket 5)

 Ticket 7 : Le client se plaint, l’utilisateur 2 enregistre une réduction de 10% sur la commande de la table 1
 (commande du ticket 5).

 Ticket 8 : L’utilisateur 2 enregistre la vente de la commande de la table 1 (ticket 7). Le client paie 27.80
 EUR avec un bon et 10 EUR en cash.

 Ticket 9 : L’utilisateur 1 enregistre une commande pour la table 1 : 1x Dry Martini et 1x Coca Cola.

 Ticket 10 : L’utilisateur enregistre la vente du ticket 9. Le client paie 15.05 EUR par cash (avec un arrondi
 de +0.01 EUR).

 Ticket 11 : L’utilisateur 2 enregistre une vente directe : 1x Coca Cola et 1x Tapas Variation. Le client paie
 2.00 EUR par Bancontact et 4.40 EUR en cash (avec un arrondi de +0.02 EUR).

 Ticket 12 : Enregistrement “Travail Out” de l’utilisateur 1 (employeeId : 84022899837).




83/110
JSON signReportUserX

              Champs                           A vérifier                   Résultats
Mutation                         signReportUserX                               ☐
bookingDate                      Valeur correcte                               ☐
             fdmDevices
                                 Comparez les données avec celles du
terminalId
                                 POS                                           ☐
firstFdmDateTime /               Vérifiez que la date et l’heure de début
lastFdmDateTime                  et de fin du ticket correspondent à
                                 celles du FDM                                 ☐
firstTotalCounter /              Vérifiez votre numérotation :
lastTotalCounter                 correspond-elle à celle du ticket ?           ☐
              USER1
  employeeId       84022899837                                                 ☐
totalAmount                      15.04                                         ☐
Type “CASH”                      PAYMENT: -16,96 | ROUNDING: 0,01              ☐
Type “CARD_DEBIT”                PAYMENT: 32                                   ☐
vatTurnouverAmount                                                             ☐
                                 La date et l’heure correspondent-elles
firstPosDateTime                 à celles du premier ticket ? (POS)            ☐
                                 La date et l’heure correspondent-elles        ☐
lastPosDateTime                  à celles du dernier ticket ? (POS)
inOut                            IN                                            ☐
posDateTime                      Correspond à l’heure à laquelle
                                 l’employé s’est déconnecté ?                  ☐
inOut                            OUT                                           ☐
               USER2
   employeeId 75061189702                                                      ☐
totalAmount                      44.18                                         ☐
Type “CASH”                      PAYMENT: 14.38 | ROUNDING: 0.02               ☐
Type “CARD_DEBIT”                PAYMENT: 2                                    ☐
Type “VOUCHER_STORE”             PAYMENT: 27.8                                 ☐
                                 La date et l’heure correspondent-elles        ☐
firstPosDateTime                 à celles du premier ticket ? (POS)
                                 La date et l’heure correspondent-elles
lastPosDateTime                  à celles du dernier ticket ? (POS)            ☐
inOut                            IN                                            ☐
posDateTime                      Correspond à l’heure à laquelle               ☐
                                 l’employé s’est déconnecté ?
inOut                            OUT                                           ☐
                                 Pas de reportBookingDate pour les
reportBookingDate                rapports X.                                   ☐




84/110
 Imprimez le ticket X, puis clôturez la journée avec un
                       rapport Z.


                  Résultats du POS
         1.   Requête JSON
         2.   Rapport (digital ou papier)



     Remarques




85/110
              RAPPORT UTILISATEUR Z (R) – SIGNREPORTUSERZ
                                        Cette mutation est obligatoire.

Description : Un « rapport utilisateur Z » est un document généré par un POS qui résume les transactions
faites par les utilisateurs par bookingDate. Contrairement au rapport X, le rapport Z clôture les transactions
en cours, réinitialisant les totaux afin de commencer une nouvelle bookingDate. Ce dernier permet un contrôle
final des ventes et permet de s’assurer que toutes les transactions ont été correctement enregistrées avant
la réinitialisation.

N.B : Pour les producteurs qui n'ont pas les options de remise ou de remboursement, ils peuvent créer leurs
propres tests, qui seront vérifiés ultérieurement. Si le pro forma est absent, seules les données liées au pro
forma manqueront, mais cela n'affectera pas le chiffre d'affaires.

Veuillez vous référer au dossier “Report_Testing” en appendice pour effectuer ce test. Vous y
trouverez les Use Cases relatifs à ce rapport.

 Testing du JSON : référez-vous au rapport utilisateur X.



 Scénario :
 Imprimez le rapport utilisateur Z du scénario que vous venez de faire.


                Le test a été effectué et le JSON a été ajouté   ☐


JSON signReportUserZ

          Champs                                          A vérifier                              Résultats
Mutation                       signReportUserZ                                                        ☐
reportNo                       Valeur incrémentale                                                    ☐
reportBookingDate              Est obligatoire pour les rapports Z                                    ☐


                  Résultats du POS
         1.   Requête JSON
         2.   Rapport (digital ou papier)



      Remarques




86/110
              TESTING DES TICKETS DES RAPPORTS : EXEMPLE


                                      Z Financier
SPF Finances
Boulevard Roi Albert II, 33/37 Schaerbeek


Numéro TVA                         BE0000000097
Numéro EST                           2000000042


Rapport n°                                10
Date et heure du rapport             2025-02-11


POS                               CPOS0031234567


                                       FdmIds              TotalCounter

Premier Event                   2025-02-11 12:45:00   72
                                2025-02-11
Dernier Event                   13:41:00              85



Booking period                           dffcd829-a0e5-41ca-a0ae-9eb887f95637
                                        Start                  End
Date/ Heure booking period        2025-02-11 12 :45   2025-02-11 13 :51


"N" Normal Sales
  No N                                    5

   Chiffre d’affaires "N" TVA
inc.                                     59,22 €


  Répartition TVA
             Taux                    Excl. TVA (€)           TVA (€)

  0%                                           - €            - €
  6%                                           - €            - €
  12%                                    27,09 €            3,25 €


 87/110
  21%                                  23,87 €          5,01 €
                                                                        59,22
  TOTAL                                50,96 €          8,26 €      €


  C.A par départements
         Départements              Excl. TVA (€)          TVA (€)
Aperitifs                                3,96 €        18,84 €

Starters                               16,07 €          1,93 €

Desserts                                8,04 €          0,96 €
Soft Drinks                             5,02 €          1,06 €
Tapas                                   2,98 €          0,36 €
                                                                        59,22
              Total                    36,08 €         23,14 €      €


DIMINUTION du C.A
Total réduction                -        4,20 €
Total montant ticket négatif   -       32,00 €


Event "P" Pro forma
Nomb. "P"                                          4
VAT inc.                               94,84 €


Event "T" Training
Nomb. "T"                                          0
Total "T" VAT inc.                                 0


Event "I" (Invoice)
No Invoice                                         0
Total Invoice VAT inc.                             0


Méthodes de paiement
Cash                           -        2,55 €
Bancontact                             34,00 €
Visa                                       - €
Meal Voucher                           27,80 €
Gift card                                  - €
Online payment                             - €
Total                                  59,25 €
Arrondissement                          0,03 €



 88/110
Tiroir-caisse ouvert sans
transaction                                  - €



                               Z UTILISATEURS
SPF Finances
Boulevard Roi Albert II, 33/37, Schaerbeek


Numéro TVA                                           BE0000000097
Numéro EST                                             2000000042


Rapport n°                                                 10
Date et heure du rapport                           2025-02-11 13:53:00


POS                                                 CPOS0031234567




       UTILISATEUR 1
Numéro INSS                                           84022899837
Heure de connexion                                  2025-02-11 12:45
Heure de déconnexion                                2025-02-11 13:41


                                                          Time
Premier Event                                       2025-02-11 12 :45
Dernier Event                                       2025-02-11 13:41


Montant du C.A (TVA inclus)                                15,04 €




       UTILISATEUR 2
Numéro INSS                                           75061189702
Heure de connexion                                 2025-02-11 12:51:00



 89/110
Heure de déconnexion

                                   Time
Premier Event                 2025-02-11 12:51
Dernier Event                 2025-02-11 13:52


                                     44,18 €
Montant du C.A (TVA inclus)




      Remarques




 90/110
             ARRONDISSEMENT DES MÉTHODES DE PAIEMENT

Description : En Belgique, l'arrondi des paiements en espèces est obligatoire au 0,05 EUR le plus proche.
L'arrondi est étendu jusqu'à un maximum de +/- 0,02 EUR.

Préliminaire

Ces directives ne suffisent pas à garantir la réussite du test d'arrondi des paiements. Veuillez prendre en
compte toutes les règles décrites dans la circulaire.

Méthode

    1.   Dans tous les cas : réalisez les tests 1 & 2.
    2.   Si votre POS arrondit seulement les paiements en cash : réalisez en plus le test 3.
    3.   Si votre POS arrondit tous les moyens de paiement : réalisez en plus les tests 4 & 5.
    4.   Ajoutez les résultats en annexe comme pour les autres tests.

                              TEST DE BASE 1 : ARRONDI DU CASH
 SITUATION 1
 Scénario :
 Conditions : En vente directe
 1x Dame Blanche 11,52€ (TVA B)
 Payez seulement en CASH


   Produits        Quantité         Prix TVA       TVA (%)       TVA (euros)      TVA excl.      TVA incl.
                                  incl. / unité
 Dame                         1        € 11,52       12%               € 1,23        € 10,29        € 11,52
 Blanche
 Total                                                                 € 1,23        € 10,29        € 11,52
 À payer                                                                                            € 11,50
 Arrondi                                                                                            € -0,02


 SITUATION 2
 Scénario :
 Conditions : En vente directe
 1x Tapas Variation 3,34€ (TVA B)
 Payez seulement en CASH



   Produits        Quantité         Prix TVA       TVA (%)       TVA (euros)      TVA excl.      TVA incl.
                                  incl. / unité
 Tapas                        1         € 3,34       12%               € 0,36         € 2,98         € 3,34
 Variation
 Total                                                                 € 0,36         € 2,98         € 3,34
 À payer                                                                                             € 3,35
 Arrondi                                                                                             € 0,01



91/110
 SITUATION 3
 Scénario :
 Conditions : En vente directe
 1x Dame Blanche 11,52€ (TVA B)
 1x Tapas Variation 3,34€ (TVA B)
 Payez seulement en CASH.



   Produits        Quantité          Prix TVA      TVA (%)      TVA (euros)    TVA excl.    TVA incl.
                                   incl. / unité
 Dame                         1         € 11,52       12%             € 1,23      € 10,29       € 11,52
 Blanche
 Tapas                        1          € 3,34       12%             € 0,36       € 2,98        € 3,34
 Variation
 Total                                                                € 1,59      € 13,27       € 14,86
 À payer                                                                                        € 14,85
 Arrondi                                                                                        € -0,01




                                  TEST DE BASE 2 : CHÈQUE-REPAS
Rappel : Les chèques-repas ne sont jamais arrondis.

Objectif : vérifier que le payement des chèques-repas n'est pas arrondi.

 SITUATION 4
 Scénario :
 Conditions: En vente directe
 1x Dame Blanche 11,52€ (TVA B)
 1x Tapas Variation 3,34€ (TVA B)
 Payez 11,52€ en CHÈQUES-REPAS
 Payez le solde en CASH.


  Produits         Quantité          Prix TVA      TVA (%)      TVA (euros)    TVA excl.    TVA incl.
                                   incl. / unité
Dame                          1         € 11,52       12%             € 1,23      € 10,29       € 11,52
Blanche
Tapas                         1          € 3,34       12%             € 0,36       € 2,98        € 3,34
Variation
Total                                                                 € 1,59      € 13,27       € 14,86
À payer             CHÈQUE-                                                                     € 11,52
                      REPAS
Reste                                                                                            € 3,34
À payer                 CASH                                                                     € 3,35
Arrondi                                                                                          € 0,01




92/110
                         TEST DE BASE 3 : MULTI-PAIEMENTS 1
Ne faites pas ce test si votre POS arrondit d’autres moyens de payement que le cash

 SITUATION 5
 Scénario :
 Conditions : En vente directe
 1x Dame Blanche 11,52€ (TVA B)
 1x Tapas Variation 3,34€ (TVA B)
 Payez 3,13€ avec des CHÈQUES-REPAS
 Payez 3,97€ par CARTE DE DÉBIT
 Payez le solde restant avec 10€ CASH.


   Produits       Quantité         Prix TVA      TVA (%)      TVA (euros)      TVA excl.       TVA incl.
                                 incl. / unité
 Dame                        1        € 11,52      12%              € 1,23        € 10,29         € 11,52
 Blanche
 Tapas                       1         € 3,34      12%              € 0,36            € 2,98       € 3,34
 Variation
 Total                                                              € 1,59        € 13,27         € 14,86
 À payer          CHÈQUES-                                                                         € 3,13
                     REPAS
 Reste                                                                                            € 11,73
 À payer           CARTE DE                                                                        € 3,97
                      DÉBIT
 Reste                                                                                             € 7,76
 À payer               CASH                                                                       € 10,00
 Rendu                                                                                             € 2,25
 Arrondi                                                                                          € -0,01



                         TEST DE BASE 4 : MULTI-PAIEMENTS 2
Faites ce test seulement si votre POS arrondit tous les moyens de paiement.

 SITUATION 5
 Scénario :
 Conditions : En vente directe
 1x Dame Blanche 11,52€ (TVA B)
 1x Tapas Variation 3,34€ (TVA B)
 Payez 1,85€ avec une CARTE DE DÉBIT/CRÉDIT.
 Payez 0,92€ avec des CHÈQUES-REPAS.
 Payez le solde restant avec 15€ CASH.


   Produits       Quantité         Prix TVA      TVA (%)      TVA (euros)      TVA excl.       TVA incl.
                                 incl. / unité
 Dame                        1        € 11,52      12%              € 1,23        € 10,29         € 11,52
 Blanche




93/110
 Tapas                    1   € 3,34   12%   € 0,36       € 2,98         € 3,34
 Variation
 Total                                       € 1,59      € 13,27       € 14,86
 À payer         CARTE DE                                               € 1,85
                    DÉBIT
 Reste                                                                 € 13,00
 Arrondi                                                                 -0,01
 À payer        CHÈQUES-                                                € 0,92
                   REPAS
 Reste                                                                  € 12,08
 Arrondi                                              Les chèques-repas ne sont
                                                                jamais arrondis
 À payer               CASH                                             € 15,00
 Rendu                                                                   € 2,90
 Arrondi                                                                 € 0,02




 TOTAL des paiements                                                   € 14,87
 TOTAL des arrondis                                                      € 0,01




94/110
   V. COMPORTEMENT DU POS AUX RÉPONSES DU
                     FDM
Les réponses du FDM peuvent contenir des informations, des avertissements et/ou des erreurs.

Dans des cas prédéfinis, le POS doit entreprendre certaines actions. Toutes sont décrites dans le tableau
tb_messages des descriptions détaillées du POS dans la section 2.2.4, objet message.

Certaines d'entre elles doivent obligatoirement être affichées via l'interface utilisateur du POS, d'autres sont
facultatives. Dans tous les cas, effectuez les tests nécessaires et joignez les preuves nécessaires (par exemple
des captures d'écran de l'interface utilisateur).

Dans le tableau suivant, veuillez indiquer les messages affichés.

 Code                                 Message                            Nature          Affichage   Sur le POS
 CLIENT_CERT_NEAR_EXPIRATION          Le certificat du client expire     Avertissement   O           ☐ OUI
                                      dans moins de 4 mois                                           ☐ NON
 CLIENT_CERT_EXPIRED                  Le certificat du client a expiré   Avertissement   M           ☐ OUI

 RTC_SYNC_FAILED                      Le FDM n’a pas pu                  Avertissement   O           ☐ OUI
                                      synchroniser l’horloge en                                      ☐ NON
                                      temps réel
 UPDATE_URLS_FAILED                   Le FDM n’a pas pu mettre à         Avertissement   O           ☐ OUI
                                      jour les URLs                                                  ☐ NON
 UPDATE_TRUST_CERT_FAILED             Le FDM échoue à mettre à           Avertissement   O           ☐ OUI
                                      jour les certificats de                                        ☐ NON
                                      confiance
 UPDATE_TASK_LIST_FAILED              Le FDM n’a pas pu                  Avertissement   O           ☐ OUI
                                      télécharger la liste de tâches                                 ☐ NON
 TASK_FEEDBACK_FAILED                 Le FDM n’a pas pu signaler le      Avertissement   O           ☐ OUI
                                      résultat de la tâche                                           ☐ NON
 NOP_FAILED                           Le FDM n’a pas pu poster un        Avertissement   O           ☐ OUI
                                      message “NOP”                                                  ☐ NON
 BUFFER_NEAR_FULL                     L’utilisation du buffer du FDM     Avertissement   M           ☐ OUI
                                      dépasse 70%
 SERVER_CERT_RESOLVE_FAILED           Le FDM n’a pas pu résoudre         Avertissement   O           ☐ OUI
                                      le certificat du serveur                                       ☐ NON
 TRANSACTION_UPLOAD_FAILED            Le FDM a tenté plus de 10          Avertissement   O           ☐ OUI
                                      fois      de    poster  une                                    ☐ NON
                                      transaction sans succès
 INITIALIZATION_FAILED                L’initialisation du FDM a          Avertissement   O           ☐ OUI
                                      échoué                                                         ☐ NON
 RTC_NOT_INITIALIZED                  L’horloge en temps réel du         Avertissement   N           ☐ NON
                                      FDM n’est pas synchronisée
 CORRUPT_RECORD_ENCOUNTERE            La mémoire du FDM contient         Avertissement   N           ☐ NON
 D                                    des transactions corrompues
 UPDATE_PARAMS_FAILED                 Le FDM n’a pas pu mettre à         Avertissement   N           ☐ NON
                                      jour les paramètres




95/110
 Code                                Message                          Nature            Affichage   Sur le POS
 UPDATE_CLIENT_CERT_FAILED           Le FDM n’a pas pu mettre à       Avertissement     N           ☐ NON
                                     jour le certificat du client
 UPDATE_VAT_RATES_FAILED             Le FDM n’a pas pu mettre à       Avertissement     N           ☐ NON
                                     jour les taux de TVA
 UPDATE_POS_ALLOWLIST_FAILED         Le FDM n’a pas pu mettre à       Avertissement     N           ☐ NON
                                     jour la liste d’autorisation
 DUPLICATE_REQUEST                   Le FDM a reçu un doublon         Avertissement     N           ☐ NON
                                     d’une requête antérieure
 BUFFER_FULL                         Le buffer du FDM est plein       Erreur            M           ☐ OUI
 FDM_LOCKED                          Le FDM est verrouillé            Erreur            M           ☐ OUI
 TOO_MANY_MEMORY_ERRORS              La mémoire du FDM contient       Erreur            N           ☐ NON
                                     trop    de     transactions
                                     corrompues
 UNAUTHORIZED                        Le FDM a reçu des requêtes       Erreur            O           ☐ OUI
                                     non-autorisées
                                                                                                    ☐ NON
 INVALID_REQUEST                     Le FDM n’a pas pu traiter la     Erreur            O           ☐ OUI
                                     requête
                                                                                                    ☐ NON
 INTERNAL_ERROR                      Le FDM a rencontré un            Erreur            O           ☐ OUI
                                     problème technique
                                                                                                    ☐ NON
 UNDEFINED_ERROR                     Le FDM a rencontré une           Erreur            O           ☐ OUI
                                     erreur indéfinie
                                                                                                    ☐ NON
 UNDEFINED_OTHER                     Toute      autre        erreur   Erreur/avertiss   O           ☐ OUI
                                     rencontrée                       ement/
                                                                      Information
                                                                                                    ☐ NON
 FDM_NOT_OPERATIONAL                 Le FDM ne fonctionne pas         Erreur            M           ☐ OUI
 UNKNOWN_POS                         Le FDM a reçu un message         Erreur            M           ☐ OUI
                                     d’un POS non repris dans la
                                     liste d’autorisation




         Le test a été effectué et le JSON ainsi que les captures d’écran ont été ajoutés           ☐




96/110
                                    VI. TESTING DES TICKETS : TICKETS ET FACTURES

                                                                     CONTENU DES TICKETS

                                                                                                                                         AUTRE(S) TICKET(S)
                                                                                                PRO
                        Contenu du ticket                                N        N REFUND                  COPY          TRAINING        (Tickets cuisine,
                                                                                               FORMA
                                                                                                                                          tickets bar, etc.)
                           OBJETS                                     Résultats    Résultats   Résultats   Résultats      Résultats           Résultats
I.   La dénomination complète                                                                   PRO
                                                                                  REMBOURS
                                                                                               FORMA        COPIE
                                                                      TICKET DE   EMENT DU                                TRAINING            PAS DE
                                                                                                 ou           ou
                                                                        CAISSE     TICKET DE                                  ou           DESIGNATION
                                                                                               TICKET       TICKET
                                                                         T.V.A       CAISSE                            TICKET TRAINING      SPECIFIQUE
                                                                                                PRO         COPIE
                                                                                      T.V.A
                                                                                               FORMA
II. Identification
                   Au moyen de son nom ou de sa
                                                                         ☐            ☐           ☐           ☐              ☐
                        dénomination sociale, de son adresse et de
                        son numéro d'identification à la T.V.A
III. La dénomination commerciale de l'exploitation                       ☐            ☐           ☐           ☐              ☐
IV. Le numéro d'unité de l’établissement                                 ☐            ☐           ☐           ☐              ☐
V. Le numéro de fabrication du système POS et le numéro de
                                                                         ☐            ☐           ☐           ☐              ☐
     version du logiciel qui y est installé
VI. Identification du terminal (terminaId)                               ☐            ☐           ☐           ☐              ☐
 VII. L'identification du dispositif d'entrée ; (par ordre de
                                                                         ☐            ☐           ☐           ☐              ☐
      disponibilité : l'adresse MAC, le numéro de série, etc.)



      97/110
VIII. L’identification de l’utilisateur                                ☐   ☐      ☐       ☐     ☐
IX. La date et l’heure de délivrance                                   ☐   ☐      ☐       ☐     ☐
X. Numéro de ticket continu issu d’une série ininterrompue             ☐   ☐      ☐       ☐     ☐
XI. Les opérations ou lignes de produits enregistrées :
                                                                       ☐   ☐      ☐       ☐     ☐
     1. contiennent toujours : la quantité, la description, le prix                                      PAS DE PRIX SUR
     total de la ligne, le code du taux de T.V.A.                                                          CES TICKETS
     2. inclus si nécessaires : les modifications de prix              ☐   ☐      ☐       ☐     ☐
     appliquées et messages ;
     3. peuvent également inclure : GTIN                               ☐   ☐      ☐       ☐     ☐
XII. Le montant total à payer du ticket, T.V.A. comprise               ☐   ☐      ☐       ☐     ☐
XIII. La base imposable par taux de T.V.A. applicable, telle que
      calculée par le FDM                                              ☐   ☐      ☐       ☐     ☐

XIV. Le montant de la T.V.A. due, par taux de T.V.A. applicable, tel
    que calculé par le FDM                                             ☐   ☐      ☐       ☐     ☐
XV. Les modifications de prix non mentionnées dans les lignes
                                                                       ☐   ☐      ☐       ☐     ☐
    de produits ;
XVI. Les données de contrôle créées et transmises par le FDM
                                                                       ☐   ☐      ☐       ☐     ☐
        I.   Le pied de page tel que configuré par le SPF Finance      ☐   ☐      ☐       ☐     ☐
        II. fdmId                                                      ☐   ☐      ☐       ☐     ☐
        III. fdmDateTime                                               ☐   ☐      ☐       ☐     ☐
                                                                       ☐   ☐      ☐       ☐     ☐
        IV. eventLabel
                                                                       ☐   ☐      ☐       ☐     ☐
        V. eventCounter
                                                                       ☐   ☐      ☐       ☐     ☐
        VI. totalCounter                                               ☐   ☐      ☐       ☐     ☐
        VII. shortSignature                                            ☐   ☐   Interdit   ☐   Interdit




      98/110
XVII. Les lignes de paiement applicables, y compris les
    arrondissements éventuels, les pourboires et les autres          ☐             ☐             ☐             ☐                ☐
    paiements (remboursements)
XVIII. Le code QR, qui contient l'URL tel que transmis par le
                                                                     ☐             ☐             ☐             ☐                ☐
    FDM
La mention suivante en cas d’impression : CECI N’EST PAS UN         N.A            N.A           ☐        ☐ si pas N            ☐
                                                                                                                                                       ☐
TICKET DE CAISSE T.V.A VALABLE


                                             Autres tickets : aucun montant ne doit être présent
                                             La mention suivante en cas d'impression : CECI N'EST PAS UN TICKET DE CAISSE DE T.V.A VALABLE

                                             À l'exception de la mention COPIE sur le ticket, il est obligatoire que cette impression (papier/numérique) :
                                             • Soit identique au ticket de TVA original (y compris tous les champs, y compris les données de contrôle) ;
                                             • Mentionne en outre le fdmRef et le posDateTime de l’event de copie (« copie émise le jj.mm.aaaa hh:mm »).

                                             En Belgique, le ticket doit être émis en français, en néerlandais ou en allemand, selon la langue officielle de
                                             la région linguistique où il est émis. Dans les lieux considérés comme des zones touristiques, le ticket peut
                                             également être émis en anglais.




     99/110
                                                  CONTENU DE LA FACTURE IMPRIMÉE
                                                   Objets                            Résultats
    a. Le titre “FACTURE”                                                               ☐
    b. Nom                                                                              ☐
    c. Adresse                                                                          ☐
    d. Numéro d’identification à la T.V.A                                               ☐
    e. Référence au(x) numéro(s) de ticket(s) original/originaux (FdmRefs array)        ☐
    f.    Quantité                                                                      ☐
    g. Nature des bien(s) et service(s)                                                 ☐
    h. Prix TVA incluse, base imposable, taux de TVA appliqué, montant de la TVA        ☐
    i.    Ventilation du montant de la TVA à payer par taux de TVA                      ☐
    j.    Mention “Ceci n’est pas un ticket de caisse T.V.A valable”                    ☐
    k. Le pied de page tel que configuré par le SPF Finance et envoyé par le FDM :      ☐
          1. FdmId                                                                      ☐
          2. FdmDateTime                                                                ☐
          3. eventLabel                                                                 ☐
          4. eventCounter                                                               ☐
          5. totalCounter                                                               ☐
          6. shortSignature                                                             ☐




100/110
101/110
                  VII. ACQUISITION DES DONNÉES

          OBLIGATION DE CONSERVATION (ART. 60 CODE TVA)

                                               LA LOI

L'article 60. § 1 ...
§ 2. ...
§ 3. ...
§ 4. Les livres et autres documents dont la tenue, la rédaction ou l’émission sont prescrits par le
présent Code ou en exécution de celui-ci doivent être conservés par les personnes qui les ont tenus,
dressés, émis ou reçus pendant dix ans à partir du 1er janvier de l’année qui suit leur clôture s’il
s’agit de livres, leur date s’il s’agit d’autres documents ou l’année au cours de laquelle le droit à
déduction a pris naissance dans les situations visées à l’article 58, § 4, 7°, alinéa 2, s’il s’agit des
documents visés à l’article 58, § 4, 7°, alinéa 4.
...
Par dérogation à l’alinéa 2, en ce qui concerne la documentation relative aux analyses, à la
programmation et à l’exploitation de systèmes informatisés, le délai de conservation prend cours à
partir du 1er janvier de l’année qui suit la dernière année pendant laquelle le système décrit dans
cette documentation a été utilisé.
Le Roi peut prolonger le délai de conservation visé à l’alinéa 1er et au paragraphe 3 en vue d’assurer
le contrôle des révisions des déductions qui sont opérées en exécution de l’article 49, 2° et 3°. Dans
les cas qu’Il détermine et selon les modalités qu’Il fixe, Il peut réduire le délai de conservation des
documents autres que les factures et les livres.
§ 5. ...
§ 6. ...


                              CONCRÈTEMENT POUR LES SCE

La responsabilité de la conservation incombe au contribuable-opérateur. Toutefois, le producteur
du système de caisse doit s'assurer qu'il puisse remplir ses obligations.
Dans la demande de certification, le producteur du système de caisse indique comment, combien
de temps et où ces données numériques sont conservées, où les données sont localisées, à quoi
elles ressemblent et quelle est la structure de la base de données. Au cours de la procédure, le
service de certification peut demander le code source du programme utilisé pour l'analyse.




102/110
    OBLIGATION DE SOUMISSION (ARTICLES 61 ET 62 CODE TVA)

                                               LA LOI

Article 61. §1 Toute personne est tenue de communiquer, sans déplacement et sans retard indu, à
toute réquisition de l’administration en charge de la taxe sur la valeur ajoutée, les livres, factures,
copies de factures et autres documents ou leurs copies qu’elle doit conserver conformément à
l’article 60, à l’effet de permettre de vérifier l’exacte perception de la taxe à sa charge ou à la charge
de tiers.
...
Pour les livres, factures et autres documents conservés sous un format électronique, cette
administration a le droit de se faire communiquer les données enregistrées sur des supports
informatiques sous forme lisible et intelligible. Elle peut également requérir la personne visée à
l’alinéa 1er d’effectuer, en présence de ces agents, et sur leur matériel, des copies, dans le format
qu’ils souhaitent, de tout ou partie des données précitées, ainsi que les traitements informatiques
jugés nécessaires à la vérification de l’exacte perception de la taxe.
Sans préjudice du droit du contribuable de demander ou fournir des renseignements verbaux, la
communication des livres, factures, copies de factures et autres documents ou leurs copies visés à
l'alinéa 1er s'applique, pour les personnes physiques et les personnes morales visées à l'alinéa 4,
également à la mise à disposition de ces livres, factures et autres documents via une plateforme
électronique sécurisée du SPF Finances....
§ 2. ...
§ 3. Les obligations mentionnées au présent article sont également d'application lorsque les
données requises par l'administration sont situées digitalement en Belgique ou à l'étranger.
Article 62 § 1. Toute personne est tenue de fournir verbalement ou par écrit, à toute réquisition des
agents de l'administration en charge de la taxe sur la valeur ajoutée, tous renseignements qui lui
sont réclamés aux fins de vérifier l'exacte perception de la taxe à sa charge ou à la charge de tiers.


                              CONCRÈTEMENT POUR LES SCE

            A. LES DONNÉES SONT CONSERVÉES LOCALEMENT

La transmission de certaines données de transaction par le SCE aux serveurs du SPF ne dispense
pas l'opérateur assujetti de l'obligation de soumettre toutes les données numériques relatives à son
système de caisse. La soumission comprend l'inspection et la copie numérique d'une partie ou de
l'ensemble des données du système de caisse, sous la forme souhaitée par l'administration.

Le producteur du système de caisse indique dans la demande de certification où se trouvent ces
données, comment sont-elles accessibles, comment peut-on les copier et/ou comment peut-on
en faire des extraits. Si nécessaire, une procédure utilisable pour chaque client est proposée.




103/110
           B.   LES DONNÉES SONT CONSERVÉES DANS ‘LE CLOUD’

Il en va de même pour les données détenues dans le cloud, quel que soit l'endroit où elles sont
stockées.

Ces données sont la propriété du contribuable-opérateur et sont pleinement soumises à l'obligation
de dépôt, dans les mêmes conditions que si elles étaient stockées localement. Si elle n'est pas en
mesure de les soumettre elle-même, l'administration peut les faire soumettre (de la même
manière) par l'hébergeur des données, en Belgique ou à l'étranger.

Dans la demande de certification, le producteur du système de caisse indique où se trouvent ces
données, comment on peut y accéder, comment on peut les copier et/ou comment on peut en faire
des extraits. Si nécessaire, une procédure utilisable pour chaque client sera proposée.



J’ai pris ceci en considération et j’ai fourni une procédure à l’équipe SCE         ☐




104/110
                            FEUILLE D’ACQUISITION DES DONNÉES

                 A. INFORMATIONS GÉNÉRALES

       Producteur

               Point de contact

               E-Mail

             Numéro de
             téléphone
       Numéro de certificat             A faire compléter par le SPF Finances

       Date de certification            A faire compléter par le SPF Finances

       Nom du logiciel

       Numéro de version du
       logiciel
       Système d’exploitation

       Agent de certification           A faire compléter par le SPF Finances
       Agent de certification
                                        A faire compléter par le SPF Finances
       (reserve)

                 B. RÉSEAUX & APPAREILS



          SCE basé sur un système Cloud                                      ☐ Oui   ☐ Non
          Si non, est-ce qu’une connexion internet est requise ?             ☐ Oui   ☐ Non
          Comment le POS est connecté au FDM ?                               ☐ Ethernet ☐ WiFi

                                                                             ☐ USB     ☐ POE

          Est-ce que le SCE est équipé d’autres périphériques (kiosk, app,   ☐ Oui   ☐ Non
          tablette, réseau...) ?


          Si oui, quelles sont les numéros de version ?


          Quelles sont les fonctionnalités disponibles sur la
          télécommande ?


105/
110
     Est-ce qu’une connexion avec des logiciels externes
     (commande en ligne) ou internes (stats, ERP) est prévue ?




           C. LOGS

     Les logs sont disponibles, lisibles et copiables   ☐ Oui      ☐ Non

     Si oui, fournissez-ici la méthode de lecture/copie de ces logs :




106/110
          Décrivez les étapes de copie de la base de données ci-dessous :




"




107/110
           D. RAPPORTS X/Z


          Comment les rapports financiers X/Z sont-ils visibles/imprimables ? (Veuillez joindre des
          captures d’écran si nécessaire)




          Comment les rapports d’utilisateur X/Z sont-ils visibles/imprimables ? (Veuillez joindre des
          captures d’écran si nécessaire)




108/110
            E. BASE DE DONNÉES

 Logiciel du DBMS (+ n° de version)




 Langage de programmation

 Comment la base de données est sécurisée ?



 Log in pour le SPF Finances

 Mot de passe pour le SPF Finances



            F. VERSION DU LOGICIEL


            N° version                  Date                              SHA1
                                                   EUR-Lex - 6153A6FA0E4880D9B8D0BE4720F78E895265D0A9 -
            ex : v1.XXX               24/01/2025                             NL




109/110
                              VIII. SIGNATURE

 Important :
 Si une des conditions suivantes n'est pas respectée :

          1. Demande complète et signée,
          2. Résultats des tests présents dans le PDF ou dans les annexes,

 Votre demande peut être rejetée par l’Administration.
                                                                             SIGNEZ ICI




110/110

```
