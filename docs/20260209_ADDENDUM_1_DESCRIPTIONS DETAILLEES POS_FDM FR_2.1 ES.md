# 20260209_ADDENDUM_1_DESCRIPTIONS DETAILLEES POS_FDM FR_2.1 ES

> 1:1 text extraction of [`20260209_ADDENDUM_1_DESCRIPTIONS DETAILLEES POS_FDM FR_2.1 ES.pdf`](<original_pdfs/20260209_ADDENDUM_1_DESCRIPTIONS DETAILLEES POS_FDM FR_2.1 ES.pdf>) (via `pdftotext -layout`). Diagrams, images, and exact PDF visual layout are not preserved — refer to the source PDF for those. Text content is verbatim.

```text
                           AM 29/04/2024
 SYSTÈME DE CAISSE ENREGISTREUSE -
         RÈGLEMENTATION TECHNIQUE -


   ADDENDUM 1 AUX DESCRIPTIONS
 DÉTAILLÉES DU FONCTIONNEMENT ET
  DE LA COMMUNICATION ENTRE LE
SYSTÈME DE CAISSE ET LE FISCAL DATA
             MODULE




VERSION 1.0 – 16/02/2026
CHANGELOG


Version   Date         Résumé
1.0       10/02/2026   Traduction en FR




2/64
 INTRODUCTION ............................................................................................................... 4
 Chapitre 1 – Raison de la création de cet addendum ......................................................... 5
 Chapitre 2 – Conséquences............................................................................................... 6
 Chapitre 3 – Dispositions techniques ................................................................................. 7
    3.1. Dispositions générales ............................................................................................ 7
    3.2. Conditions techniques de la fonctionnalité FDM BII pour la connexion POS-FDM ... 7
    3.3. Étappes de la communication .................................................................................. 8
 Chapitre 4 – Restriction du groupe cible ............................................................................ 9
    4.1 Définition d’un POS entièrement basé sur le cloud ................................................... 9
    4.2 Portée de cette définition .......................................................................................... 9




3/64
INTRODUCTION

Les descriptions détaillées du POS et du FDM reprennent de manière plus technique et plus détaillée
les dispositions techniques de l'arrêté ministériel du 29/04/2024 relatives aux aspects techniques et à
la certification du système de caisse.

Ces documents peuvent faire l'objet de modifications mineures, en raison d'éventuelles décisions
réglementaires et afin de corriger d'éventuelles erreurs.

Toutefois, si les modifications sont telles qu'elles nécessitent une certification distincte, un addendum
sera publié à cet effet.

Des informations complémentaires peuvent être obtenues auprès du service compétent, CNR, division
SCE, via secr.gksce@minfin.fed.be .




 4/64
CHAPITRE 1 – RAISON DE LA CREATION DE CET ADDENDUM

Au chapitre 2 – Interaction POS – FDM, la section 1 traite de la connexion physique entre POS et FDM.

Le deuxième paragraphe contient une remarque importante selon laquelle cette connexion sans fil via
WIFI peut également passer par Internet.

Plusieurs intégrateurs de POS basés sur le cloud utilisent un matériel intermédiaire, tel qu'un Raspberry
Pi. Une fois que ce POS est certifié, le SPF Finances n'a plus aucun contrôle sur les implémentations
correctes sur place (mises à jour logicielles, adaptations logicielles, etc.).

Afin d'éviter ces risques, la préférence est donnée à un FDM bridge internet interface (ci-après
dénommée FDM BII), qui est décritplus en détail ci-dessous.




 5/64
CHAPITRE 2 – CONSEQUENCES

Comme mentionné plus loin, cela s’applique uniquement aux POS entièrement basés sur le cloud. Afin
de garantir la traçabilité des installations, les FDM agréés recevront un nouveau certificat à cet effet.

Cela signifie également que toute modification apportée à la solution certifiée, tout comme les
adaptations du firmware du FDM lui-même, conformément à la loi du 30 juillet 2013 relative à la
certification d'un système de caisse enregistreuse, doit être communiquée et approuvée avant d'être
mise en œuvre chez les utilisateurs finaux. Les fonctionnalités non décrites peuvent entraîner le retrait
du certificat.




 6/64
CHAPITRE 3 – DISPOSITIONS TECHNIQUES

Le FDM BII proposé n'est pas géré par le SPF Finances. Le SPF Finances met toutefois à disposition,
via cet addendum, un cadre générique et contraignant à l'intention des fabricants. Les fabricants de
FDM savent en effet parfaitement, grâce à son réseau, où se trouve chaque FDM (notamment pour les
mises à jour du firmware, la console d'administration, etc.).

La fonctionnalité proposée n'est pas obligatoire, mais peut être proposée à leurs clients, pour autant
qu'elle soit soumise au service compétent du SPF Finances dans le cadre (ou la mise à jour) de leur
certification.

À cette fin, le fabricant de FDM doit fournir une description complète et détaillée au SPF Finances,
accompagnée du produit, à des fins de vérification et de test. Compte tenu de sa nature, la procédure
de certification peut inclure une visite sur place de l'environnement serveur du fabricant de FDM.

    3.1. DISPOSITIONS GENERALES
       Chaque fabricant est libre de développer sa solution, sous sa propre responsabilité.
       La responsabilité de la sécurité de la connexion incombe au fabricant, qui doit toutefois se
        conformer aux exigences décrites ci-dessous.
       En cas de perte de connexion, le POS ne peut plus atteindre le FDM et cesse de fonctionner.
       Le fabricant du FDM ne stocke en aucun cas les données qui transitent par ses serveurs via
        cette connexion.
       La solution proposée est désactivée par défaut. Elle ne peut être activée que si le client final en
        fait la demande, en utilisant un système basé sur le cloud qui répond à la définition élaborée au
        chapitre 4.
       Si le FDM BII est hors ligne, aucune communication entre le POS et le FDM ne doit être
        possible.
       La solution proposée doit être totalement transparente pour le SPF Finances. Cela implique
        notamment :
              o un accès permanent et la possibilité de se connecter,
              o la possibilité de suivre quels FDM utilisent activement la solution.


    3.2. CONDITIONS TECHNIQUES DE LA FONCTIONNALITE FDM BII
    POUR LA CONNEXION POS-FDM

       Le POS envoie sa mutation GraphQL à l'adresse réseau du FDM BII du fabricant du FDM
        plutôt qu'à l'adresse réseau directe du FDM, via une connexion TLS sécurisée (c'est-à-dire
        avec cryptage) ;
       Le FDM BII vérifie à quel FDM la demande doit être envoyée ; le fdmId doit donc être ajouté
        dans le header de la demande GraphQL;
       Le FDM BII vérifie également si l'utilisateur final (vatNo, estNo & posId) est autorisé à utiliser
        cette fonctionnalité ;
       Le FDBM BII transmet la demande via une connexion TLS sécurisée (cryptée) au FDM
        concerné.
       La réponse du FDM suit le chemin inverse.
       Si le FDM n'est pas disponible, la procédure est exactement la même que pour une connexion
        physique (retries, etc.).




 7/64
     3.3. ÉTAPES DE LA COMMUNICATION

1.   Le FDM BII reçoit une demande d'un POS

        a. La demande ne comporte pas les en-têtes nécessaires : message d'erreur «
        INTERNAL_FDM_BII_SERVER_ERROR »

2. Le FDM BII vérifie si la combinaison vatNo, estNo et posId est autorisée à utiliser la fonctionnalité
FDM BII à chaque réception d'une demande

        a. pas autorisé : message d'erreur « FDM BII_NOT_ALLOWD »

        b. autorisé : l'application cloud transfère vers le FDM appelé

3.   Le FDM n'est pas actif : message d'erreur « FDM_NOT_OPERATIONAL »

4.   Le FDM est actif, mais n’est pas joignable : message d'erreur « FDM_NOT_CONNECTED »

5.   Le FDM est actif, mais aucune réponse n'est reçue après 5 secondes : « TIMEOUT »

6.   Le FDM est actif et répond à la demande : le FDM BII transmet la réponse au POS demandeur.




 8/64
CHAPITRE 4 – RESTRICTION DU GROUPE CIBLE

Cette fonctionnalité est réservée aux installations dans lesquelles un POS entièrement basé sur le cloud
est connecté au FDM.



    4.1 DEFINITION D’UN POS ENTIEREMENT BASE SUR LE CLOUD

Un système de caisse dans lequel toutes les fonctions essentielles – telles que le traitement des
transactions, la gestion des stocks, le reporting, la gestion des clients et les intégrations avec des
services externes – fonctionnent entièrement sur des serveurs dans le cloud, plutôt que sur un serveur
local ou uniquement sur la caisse elle-même. L'accès se fait via internet, ce qui permet de synchroniser
les données en temps réel et de les rendre disponibles sur n'importe quel appareil connecté (PC,
tablette, smartphone, terminal de paiement). Un tel POS ne peut fonctionner sans connexion internet
(l'ensemble du backend fonctionne dans le cloud).



    4.2 PORTEE DE CETTE DEFINITION

Un POS entièrement basé sur le cloud répond aux cinq caractéristiques suivantes :

• Aucun serveur local ni installation de logiciel n'est nécessaire : seule une connexion Internet et
un appareil compatible sont requis.

• Toutes les fonctionnalités sont gérées de manière centralisée dans le cloud, y compris les mises
à jour, la sécurité et les sauvegardes.

• Une synchronisation en temps réel existe entre plusieurs succursales, caisses ou canaux de vente
(par exemple, boutique en ligne + magasin physique).

• Il permet des intégrations flexibles avec des applications externes (comptabilité, commerce
électronique, programmes de fidélité, prestataires de paiement).

• L'accessibilité est garantie partout et à tout moment, via une connexion sécurisée.




 9/64

```
