# 20260903_Intro_certification_procédure

> 1:1 text extraction of [`20260903_Intro_certification_procédure.pdf`](<original_pdfs/20260903_Intro_certification_procédure.pdf>) (via `pdftotext -layout`). Diagrams, images, and exact PDF visual layout are not preserved - refer to the source PDF for those. Text content is verbatim.

```text
Procédure de certification SCE 2.0 – POS

1. Introduction
Le présent document décrit la procédure de certification à l’intention des producteurs souhaitant
faire certifier un système POS. Il fournit un aperçu succinct des étapes, des documents requis et
des points d’attention afin de préparer et d’introduire correctement une demande de certification.




2. Aperçu de la procédure




                                              1/4
   Étape 1 — Entretien technique préparatoire Q&A
   (facultatif)
Les producteurs qui souhaitent obtenir des précisions complémentaires avant l’introduction de
leur demande peuvent solliciter une session de questions-réponses auprès du secrétariat
(secr.gksce@minfin.fed.be). Lors de cette session virtuelle, ils peuvent poser des questions
relatives à la préparation de leur demande de certification. La session dure en moyenne une
heure et est assurée par un membre de l’équipe de certification.



   Étape 2 — Introduction de la demande
La demande est introduite au moyen du formulaire de demande, accompagné de la
documentation requise et des résultats des tests (voir point 3). Ces documents doivent être
transmis à secr.gksce@minfin.fed.be. Une demande n’est considérée comme complète que
lorsque toutes les informations requises ont été fournies.



   Étape 3 — Pré-intake (virtuel)
Après réception du dossier, une réunion de pré-intake en ligne est organisée. Lors de cette
réunion, le producteur a l’occasion de présenter brièvement son système, tandis que l’équipe de
certification formule un premier retour sur les documents et résultats de tests soumis. Sur la base
de ce retour, des adaptations peuvent être nécessaires. Dans ce cas, les tests modifiés devront
être soumis à nouveau. Si le dossier n’est pas suffisamment avancé, un second pré-intake sera
organisé. Lorsque l’ensemble des documents et résultats de tests est conforme, le producteur
peut, après approbation explicite du certificateur, installer au maximum cinq environnements de
test chez des clients afin de poursuivre les essais dans des conditions réelles.



   Étape 4 — Intake
Lors de l’intake, la caisse est physiquement installée dans le laboratoire de l’équipe de
certification à Bruxelles. Une alimentation électrique ainsi qu’une connexion Internet ouverte via
Wi‑Fi ou câble UTP sont mises à disposition. Dans le cas d’une caisse cloud, le producteur est
également invité à être physiquement présent afin de présenter le système. Lors de cette étape,
l'équipe de certification vérifie, sur la base des résultats de tests précédemment soumis, le bon
fonctionnement du système et des fonctionnalités de base. Un nouveau rapport est ensuite
donné, afin que les éventuels points bloquants puissent être résolus avant le début de la
certification.


   Étape 5 — Certification
Lorsque le certificateur entame les tests, il prend contact avec le producteur. Le système est
soumis à des tests approfondis afin de vérifier le respect de toutes les exigences et la conformité
des structures JSON. Sur base de ces tests, des rapports de feedback peuvent être établis afin
d’indiquer les adaptations nécessaires pour rendre le système conforme.




                                               2/4
   Étape 6 — Tests de deuxième ligne
À titre de contrôle complémentaire, le système de caisse est testé soit par le chef de service, soit
par un certificateur autre que celui ayant suivi le dossier. Cette seconde vérification indépendante
confirme les résultats obtenus avant la délivrance du certificat.



   Étape 7 — Certificat
Après l'aboutissement de toutes les étapes, le certificat est établi et publié sur le site web.


   Après la certification
Les producteurs qui souhaitent distribuer eux-mêmes leur système certifié doivent s'enregistrer
en tant que distributeur.
L'ensemble du processus de certification, y compris les autorisations associées, est
gratuit.



3. Documents requis
Tous les documents ci-dessous se trouvent sur https://www.systemedecaisseenregistreuse.be/fr,
dans la section Certification SCE 2.0, dont le Manuel de certification POS (FR).


   3.1 Configuration de test
Avant le lancement des tests, le producteur doit configurer sur son propre système POS
l’environnement fictif de test décrit dans le document UC_TestSetup_20240806.pdf. Ce
document constitue la référence pour l’ensemble de la configuration de test : établissement
horeca fictif, zones et tables, équipements POS et FDM associés, employés de test ainsi que la
liste complète des produits de test avec leurs identifiants, prix, tarifs et libellés TVA. Les listes de
prix et les tarifs doivent être repris exactement tels quels. Tous les résultats de tests soumis
doivent être générés à partir de cette configuration.



   3.2 Résultats des tests (dossier Report_Testing)
Le producteur exécute les tests correspondant aux fonctionnalités qu’il souhaite faire certifier. Par
exemple, si le système POS prend en charge le remboursement partiel et que cette fonctionnalité
doit être certifiée, les tests correspondants doivent être réalisés et soumis. Pour chaque test, la
requête JSON et la réponse JSON doivent être fournies. Lorsqu’un ticket est imprimé, comme un
ticket TVA ou un ticket de bar, une photographie ou une version PDF doit également être
transmise.



   3.3 Introduire une demande
Lors de l'introduction de la demande de certification, les informations suivantes doivent être
transmises :




                                                 3/4
   •   Le formulaire de demande signé : 20250115_FORMULAIRE DEMANDE_CERTIF_POS-
       FDM_FR
   •   Le manuel de certification POS entièrement complété : 20250307_FR - Manuel de
       certification_POS_V11
   •   Un aperçu des distributeurs potentiels
   •   Le ou les manuels destinés au distributeur ou à l’utilisateur final
   •   Une description technique du processus de production de votre équipement
   •   Une description technique détaillée de l’intégration de toutes les prescriptions techniques
   •   La description complète des données utilisées et générées par l’équipement
   •   L’ensemble des résultats de tests réalisés sur la base de la configuration de test

Une demande n'est valable que lorsque l’ensemble de ces éléments a été transmis
intégralement.
Les documents et résultats de tests peuvent être transmis sous la forme d’un fichier ZIP. Les
producteurs qui préfèrent utiliser une plateforme de téléchargement peuvent prendre contact
avec l'équipe de certification ; dans ce cas, une plateforme est mise à disposition pour y déposer
les fichiers.



4. Mises à jour et nouvelles versions
Le producteur est tenu de transmettre toute nouvelle version ou mise à jour du système certifié.
Les mises à jour ayant un impact sur le volet SCE ne peuvent être installées chez les clients
qu'après avoir été certifiées par nos soins. Les corrections urgentes de bugs ou les solutions à
des blocages chez des clients constituent une exception à cette règle, mais doivent tout de
même nous être signalées.




                                                4/4
```
