# 20250115_Manual_Intro_ToRead_FR

> 1:1 text extraction of [`20250115_Manual_Intro_ToRead_FR.pdf`](<../original_pdfs/20250307_POS_CERTIFICATION_MANUEAL_FRV1.1/20250115_Manual_Intro_ToRead_FR.pdf>) (via `pdftotext -layout`). Diagrams, images, and exact PDF visual layout are not preserved — refer to the source PDF for those. Text content is verbatim.

```text
       INTRODUCTION AU MANUEL DE
          CERTIFICATION SCE 2.0




1/12
                                                   TABLE DES MATIÈRES

  LEXIQUE ................................................................................................................................................. 3
  PREAMBULE........................................................................................................................................... 4
       A.      QU’EST-CE QU’UN SCE ? ......................................................................................................... 4
       B.      LIRE L'ARRÊTÉ MINISTÉRIEL ................................................................................................... 4
       C.      L'ESPRIT DU PROCESSUS DE CERTIFICATION .................................................................... 5
  LE PROCESSUS DE CERTIFICATION .................................................................................................. 5
       A.      PRE-INTAKE ............................................................................................................................... 6
            I. CRITÈRES DE CERTIFICATION ................................................................................................. 6
            II. FONCTIONNALITÉS DU POS ..................................................................................................... 6
            III. LIGNES DE PRODUITS PRÉDÉFINIES .................................................................................... 7
            IV. MATÉRIEL ET MACHINE DE TEST .......................................................................................... 8
            V. VÉRIFICATION DES DOCUMENTS ........................................................................................... 8
       B.      INTAKE MEETING (rendez-vous d’admission) ......................................................................... 10
       C.      TESTS ....................................................................................................................................... 11
       D.      CONCLUSION ........................................................................................................................... 11
  MISE À JOUR APRÈS CERTIFICATION .............................................................................................. 12




2/12
                                  Manuel de Certification
                                  pour le Système de Caisse Enregistreuse 2.0

  Version Date               Journal des modifications
  1.0     10/01/2025         aucun


                                              LEXIQUE
  TERME           Signification
  Agent de        Un agent désigné par le SPF Finances chargé de superviser la certification de
  certification   chaque POS.
  DAS             Fiche de collecte de données, un document utilisé pour capturer et enregistrer les
                  données fiscales pour les audits.
  DBMS            Database Management System, Système de Gestion de Base de Données
  Distributeur    L'entité responsable de l'installation des SCE chez les utilisateurs finaux.
  EFT             Transfert électronique de fonds, le transfert numérique d'argent entre comptes.
  End user        L’utilisateur final, le professionnel de l’HoReCa qui utilisera le SCE.
  Events          Catégorie de transaction fiscale se produisant entre le SCE et le FDM
  FDM             Module de Données Fiscales, un dispositif signant et transmettant les transactions
                  fiscales.
  SPF             Service Public Fédéral (anglais : FPS ; néerlandais : FOD)
  Intake          Présentation formelle d'un POS au Service SCE.
  JSON            JavaScript Object Notation, une syntaxe pour stocker et échanger des données.
  LAN             Réseau Local, un réseau qui connecte des ordinateurs dans une zone limitée.
  MPV             Multi-Purpose Voucher, but non décidé au moment de la vente (tva % inconnu)
  Mutation        Une opération utilisant GraphQL pour modifier des données côté serveur, souvent
                  utilisée pour gérer des fichiers JSON.
  NISS            Numéro d'Identification de Sécurité Sociale (néerlandais : INSZ)
  Product line    La description d'un produit en vente.
  POS             Point de Vente, un système de caisse non certifié.
  Producteur      L'entreprise fabriquant les systèmes POS à certifier.
  RCRS            Registered Cash Register System. Une caisse enregistreuse certifiée.
  SCE             Système de Caisse Enregistreuse (néerlandais : GKS), une caisse enregistreuse
                  certifiée.
  SHA 1           Algorithme de Hachage Sécurisé. Un processus d'authentification électronique.
  Split bill      Une méthode de division d'une facture en montants égaux ou non égaux pour des
                  paiements séparés.
  Split table     La pratique de diviser les commandes entre groupe de clients.
  SPV          Single Purpose Voucher, but décidé au moment de la vente (tva % connu)
  Machine test Un système POS entièrement fonctionnel fourni au SPF pour les tests.
  Ticket       Un résumé formaté d'une transaction. Il peut être imprimé sur papier ou de manière
              numérique.
  Transaction L'exécution d'opérations de données fiscales, généralement classées par type
              d’event.
  TVA         Taxe sur la Valeur Ajoutée (TVA), un type de taxe appliquée aux biens et services.
  WIFI            Wireless Fidelity, une technologie de réseau local sans fil.

3/12
                                         PREAMBULE

  Ce guide offre aux fabricants de POS un cadre clair pour comprendre et appliquer les
  exigences belges du Système de Caisse Enregistreuse (SCE) afin d’obtenir la certification
  officielle. Cette certification confirme que le système POS est conforme à la législation fiscale.
  L'utilisation correcte (taux de TVA, fonctionnalités) reste la seule responsabilité de l’utilisateur.
  Obtenir cette certification est essentiel pour garantir la fiabilité du système et l’engagement du
  fabricant envers la conformité légale.



   A. QU’EST-CE QU’UN SCE ?

  Un Système de Caisse Enregistreuse (SCE), intègre un système POS standard avec un
  Module de Données Fiscales (FDM), créant une solution unifiée où les deux composants sont
  essentiels et interdépendants. Le POS, conçu par les fabricants et distribué aux utilisateurs
  finaux, doit fonctionner en tandem avec le FDM, qui est développé selon les spécifications
  définies par le Service Public Fédéral des Finances (SPF Finances). Les fabricants ont la
  flexibilité de produire à la fois des systèmes POS et des FDM, et peuvent également les
  distribuer. Il est important de noter que le SPF Finances maintient un registre détaillé de toutes
  les installations SCE, suivant quels systèmes et FDM sont utilisés par chaque utilisateur final.

  L’introduction du SCE 2.0 représente une amélioration significative en répondant aux
  contraintes techniques antérieures et en facilitant une connexion numérique directe entre le
  FDM et les serveurs du SPF Finances. Cette avancée limite la nécessité pour les agents
  fiscaux de récupérer physiquement les données sur les sites commerciaux, simplifiant ainsi
  tout le processus.



   B. LIRE L'ARRÊTÉ MINISTÉRIEL

  En mai 2024, une nouvelle directive de certification a été établie par un arrêté ministériel,
  définissant le cadre des normes du Système de Caisse Enregistreuse (SCE), applicable aux
  nouvelles demandes de certification soumises après la date de publication. Cet arrêté précise
  les fonctionnalités, obligations et limitations d’un SCE.

  De plus, le SPF Finances a publié une documentation technique supplémentaire (en anglais,
  français et néerlandais) sur le site SCE : https://www.systemedecaisseenregistreuse.be
  (descriptions détaillées, Uses cases, etc.). Comprendre cet arrêté ainsi que les informations
  supplémentaires constitue la première étape pour obtenir la certification SCE.

  Nous attendons des fabricants qu’ils mettent en œuvre les exigences décrites dans cette
  documentation. Si vous avez des questions ou besoin de clarifications, n’hésitez pas à
  contacter le service SCE du SPF Finances à l’adresse suivante : secr.gksce@minfin.fed.be .



4/12
   C. L'ESPRIT DU PROCESSUS DE CERTIFICATION

  Le principe fondamental de la certification est clair : chaque donnée fiscale doit être entièrement traçable et inaltérable, depuis la saisie initiale
  jusqu’au rapport quotidien final. Le FDM (Fiscal Data Module) reçoit toutes les données dans un format prédéfini, met à jour les compteurs et
  signe le contenu. Ces données sont ensuite transmises au SPF Finances par le FDM. Le processus de certification vérifie ainsi la traduction
  correcte des fonctionnalités du POS (Point of Sale) dans la communication prédéfinie, ainsi que le traitement adéquat des messages FDM par le
  POS.



                                              LE PROCESSUS DE CERTIFICATION
  Obtenir la certification SCE nécessite de passer par plusieurs étapes techniques et administratives. Ces étapes peuvent être regroupées en trois
  phases principales : Pré-Intake (Pré-admission), certification complète et certification des mises à jour.




5/12
   A. PRE-INTAKE

  La phase de pre-intake (pré-admission) commence dès le premier contact entre un fabricant
  de POS et le service SCE. Elle se termine par la présentation officielle lors de la réunion
  d’intake (admission), dont l’objectif est de vérifier que le système POS respecte les niveaux
  de conformité requis pour entamer la procédure officielle de certification. Au cours de cette
  étape, des points clés seront abordés.



       I. CRITÈRES DE CERTIFICATION
  Pour déterminer si un système peut être certifié, plusieurs critères sont pris en compte :

       •   Gère-t-il les transactions ?

       •   En quoi se distingue-t-il d’un serveur prenant des commandes ?

       •   Quelles sont les fonctionnalités du logiciel et quelles sont ses limitations ?

       •   Peut-il annuler ou modifier des commandes ?

  Une discussion préliminaire avec un agent de certification au sujet du système
  permettra de gagner un temps précieux.



       II. FONCTIONNALITÉS DU POS
  Après avoir confirmé l’éligibilité d’un système à la certification, l’étape suivante consiste à
  répertorier toutes les fonctionnalités du POS. L’Arrêté Ministériel et les descriptions détaillées
  mentionnent de nombreuses possibilités, qui peuvent être soit optionnelles, soit
  personnalisables. Créer un résumé de ces fonctionnalités permet de définir les priorités dès le
  départ. Étant donné qu’il peut exister différentes versions d’un même SCE, il est essentiel de
  détailler les fonctionnalités propres à chaque version.



  Pas toutes les fonctionnalités n’influencent la certification, comme la gestion des stocks.
  Cependant, certaines fonctionnalités, telles que les options pour la consommation sur place
  ou à emporter, modifient considérablement la fonctionnalité du POS et sont cruciales pour le
  testing.



  Veuillez noter que des fonctionnalités non documentées pourraient compliquer le processus
  de certification. Elles sont interdites !!!!




6/12
       III. LIGNES DE PRODUITS PRÉDÉFINIES

  Les lignes de produits prédéfinies sont configurées pour une vérification rapide des résultats
  de test, comme la TVA et la numérotation. Le logiciel POS doit inclure ces produits avec leur
  libellé, leur prix, leur catégorie et leur taux de TVA. Cependant, certains produits sont
  nécessaires uniquement pour tester des fonctionnalités avancées, qui peuvent ne pas être
  présentes dans tous les SCE. Si ces fonctionnalités avancées sont absentes, ces produits ne
  sont pas obligatoires.
  Rappel rapide sur la TVA dans le secteur HoReCA:

  Taux de TVA A = 21 % :

       •   Produits alimentaires de luxe (homard, caviar, huîtres…) mais uniquement en vente à
           emporter

       •   Tous les alcools.

       •   Boissons non alcoolisées consommées sur place

       •   Voucher (bon) uniquement pour des boissons

  • Taux de TVA B = 12 % :

       •   Nourriture consommée sur place

       •   Voucher uniquement pour de la nourriture

  • Taux de TVA C = 6 % :

       •   Nourriture et boissons non alcoolisées en vente à emporter

  • Taux de TVA D = 0 % :

       •   Produits du tabac

  • X (hors TVA) = exempté :

       •   Consignes (bouteilles, fûts…)

       •   Multipurpose vouchers (nourriture et boissons), considérés comme un moyen de
           paiement




  Menu All In (Menu Tout Inclus) : Un menu combinant nourriture et boissons, on peut soit
  appliquer le taux de TVA le plus élevé (21 %), soit répartir les deux taux de TVA appropriés
  dans un prix global.




7/12
       IV. MATÉRIEL ET MACHINE DE TEST
  En général, le type de matériel utilisé n’est pas déterminant. Le logiciel peut fonctionner sur
  une caisse enregistreuse traditionnelle, une tablette, un téléphone mobile, ou même en mode
  SAS via un navigateur web. Cependant, certains équipements spécifiques sont importants.
  Par exemple, si le système POS inclut un paiement au poids, une balance connectée est
  indispensable pour les vérifications. De même, si un appareil portable est utilisé, nous aurons
  besoin de précisions sur ses fonctionnalités : s’agit-il uniquement de prise de commandes à
  distance, d’un système complet ou d’une solution intermédiaire ?

  Il vous sera demandé d’apporter un POS entièrement fonctionnel en tant que machine de test
  lors de la réunion d’intake. Cependant, durant le pré-intake, nous nous concentrons sur le
  matériel afin de prévenir d’éventuels problèmes qui pourraient survenir lors de l’intake.



       V. VÉRIFICATION DES DOCUMENTS
  La certification implique de rassembler divers documents pour identifier le fabricant et
  expliquer le fonctionnement du système POS. Essentiellement, cette étape consiste à
  organiser toutes les informations nécessaires sur le POS sous forme de documents. Cette
  approche présente deux principaux avantages :

       •   Elle permet aux agents de certification d’accéder à tous les détails techniques
           nécessaires sans avoir à contacter le fabricant à plusieurs reprises, ce qui rend le
           processus plus rapide et efficace.

       •   La documentation sert également de référence pour une utilisation future, tant pour les
           agents de certification que pour le personnel de terrain du SPF Finances. Elle peut
           également constituer une preuve importante dans des situations juridiques

  Les documents requis sont les suivants :

       •   Demande officielle de certification

  Document formel notifiant au SPF Finances que le POS est prêt à être testé. Il doit inclure les
  détails du produit et les informations de l’entreprise demandeuse, daté et signé.

       •   Liste des importateurs et/ou distributeurs

  Document précisant les entreprises impliquées dans la fabrication, l’importation et l’installation
  du SCE en Belgique, car la chaîne d’approvisionnement peut être complexe.

       •   Manuel d’utilisation

  Ce manuel doit couvrir les opérations de base de la caisse, telles que les ventes,
  remboursements, remises, annulations, gestion des tickets et rapports légaux, ainsi que les
  fonctionnalités optionnelles comme la gestion des commandes, des tables et la création de
  factures.
8/12
       •   Manuel d’installation

  Document détaillant l’installation du POS, incluant la gestion des produits et des prix, la gestion
  des comptes employés et clients, ainsi que les options de personnalisation telles que
  l’impression de tickets et les droits d’accès.

       •   Analyses, documentations de conception et code source

  Explique les choix de conception et les processus derrière le développement du SCE, souvent
  en lien avec les versions précédentes, afin de justifier certaines fonctionnalités.

       •   Acquisition des données

  Détaille comment le SPF Finances peut copier et accéder à la base de données et aux fichiers
  journaux pour des audits, tout en garantissant un accès en lecture seule pour protéger les
  données des contribuables. Sur la machine de test, un accès permettant de visualiser la base
  de données est attendu (cfr le document DAS dans le pdf).

       •   Rapport de tests et de certification

  Les tests de certification pour les fabricants de POS sont compilés dans un fichier PDF,
  essentiel pour le processus de certification. Ce document comprend :

       •   Liste des fonctionnalités POS : Détails des fonctionnalités, des lignes de produits,
           des documents nécessaires et des composants de la machine de test, comme
           mentionné précédemment.
       •   Détails du matériel : Liste du matériel fourni par le fabricant pour les tests, essentielle
           pour tout échange, mise à jour ou retour, car la machine de test reste la propriété du
           fabricant.
       •   Spécifications de connexion : Informations sur les configurations spécifiques
           nécessaires pour connecter le FDM au SCE, car tous les systèmes ne sont pas
           immédiatement prêts à l’usage sans équipements ou configurations supplémentaires.
       •   Mots de passe : Comprend les mots de passe nécessaires pour le système
           d’exploitation, les rôles des utilisateurs et l’accès au backend, si applicable. Des mots
           de passe simples sont acceptés pour les tests. Si l’accès nécessite d’autres méthodes
           (par exemple, clé Dallas, carte à puce), celles-ci doivent également être fournies avec
           la machine de test.
       •   Suite de tests : Conçue pour évaluer les fonctionnalités obligatoires et les options
           courantes des SCE. Ces tests initiaux sont généraux, mais les agent de certification
           mèneront des évaluations plus détaillées en fonction des performances de chaque
           SCE. Chaque test est décrit dans le fichier PDF, incluant ce qu’il faut tester, les
           données à utiliser, les informations TVA attendues, les fichiers JSON associés et les
           uses cases pertinents.

  Cette approche vise à simplifier le processus, en offrant aux fabricants une orientation
  claire et en réduisant le besoin pour les agents de certification d’interrompre la
  certification pour des problèmes mineurs.


9/12
   B. INTAKE MEETING (RENDEZ-VOUS D’ADMISSION)



  PRINCIPE GÉNÉRAL
  Le rendez-vous d’intake est une étape clé où les fabricants présentent leur système POS à un
  agent de certification désigné. Cette réunion constitue la dernière occasion de vérifier que la
  machine de test fonctionne parfaitement, que toute la documentation est complète, que les
  fonctionnalités sont correctement répertoriées et que les tests de base ont été réalisés avec
  succès.




  INTAKE
  Lors de la réunion d’ intake (admission), le fabricant installe la machine de test dans notre
  laboratoire et la présente à l’agent de certification assigné (ou désigné ?)

  Les points suivants seront pris en compte :

  Documents administratifs

  Cette réunion constitue la dernière opportunité de soumettre les documents administratifs
  manquants. Sans ces documents, le processus de certification ne peut pas commencer. Le
  fichier PDF mentionné précédemment est essentiel, car il détaille les capacités et les
  fonctionnalités du POS.

  • Machine de test

  La phase de pre-intake doit avoir confirmé que la machine de test est prête, complète et
  opérationnelle. Si ce n’est pas le cas, le processus de certification ne pourra pas avancer.
  Nous vérifierons le bon fonctionnement de tous les composants matériels listés (ordinateurs,
  câbles, imprimantes, scanners, routeurs, appareils Wi-Fi, terminaux portables, balances, etc.).
  Ces composants seront ensuite enregistrés comme faisant partie du SCE et répertoriés avec
  leurs numéros de série. Un accusé de réception sera délivré.

  Un schéma d’assemblage doit être fourni (cela permettra un réassemblage pour des tests
  dans le future).

  Il est recommandé d’inclure un FDM avec la machine de test pour accélérer le processus de
  certification. Si un simulateur de FDM est utilisé, sa méthode de connexion et ses ports doivent
  être clairement documentés.




10/12
        •   Profils utilisateur

  La machine de test doit être configurée avec des profils utilisateur ayant des niveaux d’accès
  distincts, notamment :

        •   Manager/Technicien : Accès complet à toutes les fonctionnalités.
        •   Serveur : Accès uniquement aux fonctions de vente.
        •   Apprenti : Accès au mode formation (T-Ticket), si disponible.




        •   Calendrier des étapes

  La procédure s’étend sur maximum trois mois. Elle commence à la date de l’acceptation du
  formulaire de demande à la fin de réunion intake, attestée par la signature de l’agent de
  certification. Pendant cette période, l’agent de certification peut contacter le fabricant pour
  clarifier certains points techniques ou proposer des modifications mineures au système.




   C. TESTS

  L’agent de certification prend désormais en charge le processus, en vérifiant les détails fournis
  dans le fichier PDF. Il testera le POS en fonction des fonctionnalités répertoriées, des events
  et mutations de GraphQL, ainsi que du bon fonctionnement global du logiciel.

  Après les premiers tests, l’agent de certification discutera des éventuels problèmes avec le
  fabricant. Un premier débriefing aura lieu à un moment clé des tests, afin de résoudre les
  problèmes jugés rectifiables. Un dernier débriefing permettra de rendre la décision finale. La
  certification pourra être déclarée réussie ou échouée.



   D. CONCLUSION

        •   En cas de succès
  Une certification réussie signifie que le POS est officiellement certifié, répertorié comme SCE,
  et disponible pour installation. À partir de ce moment, toute modification sera évaluée via le
  processus de mise à jour (voir point 6).

        •   En cas d’échec
  Le SPF Finances peut demander des modifications finales ou mettre fin au processus de
  certification. En cas d’échec, le fabricant doit attendre trois mois avant de pouvoir soumettre
  une nouvelle demande, conformément à l’article 109 de l’Arrêté Ministériel.


11/12
                     MISE À JOUR APRÈS CERTIFICATION

  Une mise à jour consiste à certifier les modifications apportées à un système POS déjà certifié.
  Ces mises à jour peuvent corriger des problèmes techniques, introduire de nouvelles
  fonctionnalités ou apporter des améliorations visuelles. Nous évaluons leurs impacts sur les
  fonctions existantes, ce qui signifie que l’ensemble du logiciel n’a pas besoin d’être recertifié.
  Par conséquent, le processus de mise à jour est simplifié et ne dure que 15 jours ouvrables.



  Les documents requis sont :

        •   Un journal des modifications détaillant tous les changements ;
        •   Une déclaration signée confirmant qu’aucune autre modification n’a été effectuée ;
        •   Les signatures SHA1 mises à jour pour la vérification de l’intégrité.




12/12

```
