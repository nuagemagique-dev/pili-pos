# 20260717_FAQ_PRODUCTEURS_FR

> 1:1 text extraction of [`20260717_FAQ_PRODUCTEURS_FR.pdf`](<original_pdfs/20260717_FAQ_PRODUCTEURS_FR.pdf>) (via `pdftotext -layout`). Diagrams, images, and exact PDF visual layout are not preserved - refer to the source PDF for those. Text content is verbatim.

```text
                                                                                                                                                                                                                               CNR / GKS
                                                                                                                                                                                               Bd du Roi Albert II, 33 à 1030 Schaerbeek
                                                                                                                                                                                                        mail: secr.gksce@minfin.fed.be




                                         FOIRE AUX QUESTIONS RÉGLEMENTATION GKS-SCE 2.0
                                                                                                                                                                                                                     version 1.0


CATÉGORIE                               QUESTION                                                                                            RÉPONSE                                                                        ID

Généralités    Quelle est la date limite de validité d'une caisse certifiée en    a) La date de validité dépend de la date d'installation de votre dernière caisse certifiée en 1.0. Attention, la date                    101
                version 1.0 avant le passage obligatoire à la version 2.0 ?       d'installation des FDM n'est pas prise en compte.



                                                                                  b) Conformément aux dispositions transitoires, une période de validité spécifique s'applique à compter de cette date.




                                                                                  c) Veuillez consulter l'Arrêté royal du 24 avril 2024 (disponible sur la page « Législation » du site officiel
                                                                                  www.systemedecaisseenregistreuse.be) pour connaître ces périodes.




                                                                                 d) Vous pouvez vérifier la date limite applicable à votre entreprise via l'e-service SCE, sous l'onglet « Mon profil » > «
                                                                                 Unités d'établissement connues ». L'accès à l'e-service requiert d'être représentant légal de l'entreprise, ou de disposer
                                                                                 du rôle « SPFFin – Exploitants de restaurant (SCE) » attribué par le représentant légal.
Généralités   L'obligation d'utilisation d'un SCE certifié s'applique-t-elle aux a) Non, l'obligation d'utiliser un SCE ne s'applique pas si le chiffre d'affaires « repas sur place » est strictement inférieur à         102
              établissements dont le chiffre d'affaires « repas sur place » est ce seuil.
                         inférieur au seuil de 25 000,00 € par an ?

                                                                                  b) L'établissement reste toutefois soumis aux obligations fiscales classiques.




                                                                                  c) Une veille législative régulière est nécessaire, les seuils ou conditions étant susceptibles d'évoluer.




                                                                                                                                                                                                                                   1/12
Ventes & Commandes      Comment enregistrer un produit pesé en temps réel (par          a) Le produit pesé est enregistré avec `quantityType: KILOGRAM`. La ligne de produit contient le prix au kg comme                  201
                                     exemple un salade-bar) ?                           `unitPrice` et le poids en kg comme `quantity`.



                                                                                        b) Une option ou un accompagnement payant (sauce, riz, nouilles, etc.) est enregistré sur une ligne de produit distincte,
                                                                                        avec son propre prix.



                                                                                        c) Une réduction sur le prix au kg (par exemple lorsque le client apporte son contenant) est enregistrée via un
                                                                                        `priceChange` appliqué à la ligne du produit pesé.



                                                                                        d) Une variante de format ou de contenant (S / M / L) qui entraîne un surcoût est enregistrée sur une ligne de produit
                                                                                        distincte. Elle ne modifie pas le prix au kg du produit pesé.



                                                                                        e) Le type `PIECE` est réservé aux articles pré-emballés vendus à un prix fixe à l'unité. Il ne s'applique pas à un produit
                                                                                        pesé, même si une option payante y est ajoutée.



Ventes & Commandes   Quand utiliser `signPreBill` (addition / aperçu d'addition) et faut- a) Le `signPreBill` est utilisé pour émettre un aperçu d'addition (addition provisoire) au cours d'une vente interrompue,        202
                                  il toujours un `signOrder` préalable ?                  sans clôture fiscale. Il doit être envoyé au moment où l'utilisateur demande l'addition, afin de refléter l'état courant de la
                                                                                          commande.

                                                                                        b) Un `signPreBill` ne nécessite pas systématiquement un nouveau `signOrder` immédiatement avant lui : si rien n'a
                                                                                        changé dans la commande depuis le dernier `signOrder`, le `signPreBill` est envoyé seul.



                                                                                        c) Il ne faut pas générer un `signOrder` artificiel au seul motif que l'addition est imprimée. Un enchaînement `signOrder`
                                                                                        puis `signPreBill` au même moment, sans modification réelle de la commande entre les deux, n'est pas conforme.



                                                                                        d) Une seule exception : lors de la toute première commande de la vente interrompue, un `signOrder` est obligatoire
                                                                                        avant le premier `signPreBill`, faute de quoi il n'y aurait rien à reprendre dans l'aperçu d'addition.



                                                                                        e) Il n'est pas autorisé d'ajouter ou de modifier des lignes de produits lors de la création de l'aperçu d'addition.




                                                                                                                                                                                                                             Page 2/12
Ventes & Commandes    Webshop — commande avec enlèvement ou livraison le jour       a) Toute commande webshop doit aboutir à un event N (`signSale`) dans le SCE, même lorsque le paiement a eu lieu en            203
                              même : comment enregistrer la vente ?                 ligne.



                                                                                    b) Si la commande est envoyée automatiquement vers la caisse, un event P ou N est créé directement. Si un event P
                                                                                    existait déjà pour cette commande, c'est un event N qui clôture.



                                                                                    c) Si la commande est saisie manuellement dans la caisse, un event N est créé manuellement.




                                                                                    d) Pour une commande en ligne avec enlèvement ou livraison à une date ultérieure (traiteur, banquets, etc.), voir Q204.




Ventes & Commandes   Webshop — commande en ligne avec enlèvement ou livraison à a) Paiement sur place le jour de l'enlèvement : `signOrder` le jour de la commande ; event N (`signSale`) le jour de               204
                        une date ultérieure (traiteur, banquets, etc.) : comment l'enlèvement.
                                 enregistrer la vente et le paiement ?

                                                                                    b) Paiement en ligne — option 1 : event N (`signSale`) immédiatement à la commande, avec délivrance du ticket de
                                                                                    caisse TVA par e-mail ou QR code.



                                                                                    c) Paiement en ligne — option 2 (tolérance administrative) : à la commande, `signOrder` + `signPreBill` mentionnant
                                                                                    l'acompte ; à l'enlèvement, event N (`signSale`) avec délivrance du ticket TVA.



                                                                                    d) Paiement en ligne — option 3 : à la commande, `signOrder` + event N (`signSale`) avec une ligne de produit « acompte
                                                                                    » au taux de 21 % ; à l'enlèvement, event N (`signSale`) `REFUND` de cet acompte au taux de 21 %, puis event N
                                                                                    (`signSale`) de la commande complète avec délivrance du ticket TVA.

                                                                                    e) Pour les transactions hors webshop ou kiosque (traiteurs, salles de banquets dont la commande n'est pas saisie via
                                                                                    webshop), l'acompte est saisi dans le SCE au moment du paiement.



                                                                                    f) Rapports : (i) le jour de la commande, le rapport Z reprend les `signOrder` et `signPreBill`, mais pas le paiement reçu ;
                                                                                    (ii) le jour de l'enlèvement, le rapport Z reprend l'event N (`signSale`) et la totalité des paiements (acompte en ligne +
                                                                                    complément éventuel).




                                                                                                                                                                                                                     Page 3/12
                                                                                        Exemple : commande le 12/3 de 2 burgers à 56,00 €, acompte de 30,00 €, enlèvement le 14/3.




                                                                                        — 12/3 : `signOrder` de 56,00 € + `signPreBill` de 56,00 € avec financial line « paiement en ligne » de 30,00 €. Rapport Z
                                                                                        du 12/3 : 2 events P, total 112,00 €, sans paiement.



                                                                                        — 14/3 : event N (`signSale`) de 56,00 € avec deux lignes de paiement (30,00 € en ligne, 26,00 € en espèces). Rapport Z
                                                                                        du 14/3 : 1 event N de 56,00 €, deux paiements.



Ventes & Commandes   Une commande en cours doit-elle obligatoirement donner lieu a) Toute transaction entamée en vente directe doit être finalisée par un event N (`signSale`).                                      205
                                à la création d'un event P ou N ?



                                                                                        b) Toute transaction entamée en gestion de table doit aboutir à la création d'un event P ou N selon la situation
                                                                                        (enregistrement d'une commande ou création d'un ticket de caisse TVA). Exception : en cas de transfert de la table vers
                                                                                        une chambre d'hôtel (compte global), il suffit d'émettre un `signCostCenterChange` pour ramener le total de la table à 0 ;
                                                                                        aucun event N n'est nécessaire pour cette table.
Ventes & Commandes   Lorsqu'en vente directe une vente est soldée à 0,00 €, à la suite a) Un event N (`signSale`) doit être créé (ticket de caisse TVA à 0,00 €).                                                    206
                            d'une remise de 100 %, comment l'enregistrer ?



                                                                                        b) Il doit comporter les valeurs : `name: "PAY 0"`, `type: "CASH"`, `amount: 0`. Il est également possible d'envoyer un
                                                                                        array financials vide.



                                                                                        c) Cela confirme que la transaction est close fiscalement même sans échange monétaire.




Ventes & Commandes      Quelles sont les règles d'utilisation du `signPreBill` et du    a) `signOrder` est requis à chaque création ou modification de commande, en ne contenant que les lignes ajoutées ou          207
                                   `signOrder` en gestion de table ?                    modifiées.



                                                                                        b) `signPreBill` est requis lors de la demande d'une addition provisoire.




                                                                                                                                                                                                                       Page 4/12
                                                                                     c) Il est interdit de générer un `signOrder` automatiquement au seul motif d'imprimer un `signPreBill`, si le contenu de la
                                                                                     commande n'a pas changé.



Ventes & Commandes   Que renseigner dans le champ obligatoire `customerVatNo`        Vous pouvez utiliser la valeur `NA` (Not Applicable) dans ce champ.                                                           208
                     (event I, Invoice) pour un client privé sans numéro de TVA ?



Ventes & Commandes   Pour une vente directe soldée à 0,00 € (par exemple à la suite a) Oui, un ticket de caisse TVA doit être délivré pour toute transaction enregistrée, y compris lorsque le montant total est   209
                      d'une remise de 100 %), un ticket de caisse TVA doit-il être  égal à 0,00 €.
                                          délivré au client ?

                                                                                     b) Le ticket peut être délivré au format papier ou au format numérique. La valeur `ticketMedium: NONE` n'est pas
                                                                                     autorisée dans ce cas.



                                                                                     c) L'event N (`signSale`) correspondant doit être créé selon les modalités décrites à la Q206.




Ventes & Commandes     Quand utiliser le type de ligne `COMPOSITE_PRODUCT` ?         a) `COMPOSITE_PRODUCT` est réservé aux produits composés de plusieurs sous-produits ayant chacun leur prix et leur            210
                                                                                     code TVA (par exemple un menu, un magazine + CD).



                                                                                     b) Une option sans surcoût (chaud / froid, pain blanc / brun) n'est pas un sous-produit. Elle ne doit pas être enregistrée
                                                                                     en `COMPOSITE_PRODUCT` ni transmise au FDM. Le produit est enregistré comme une ligne standard ; l'option
                                                                                     éventuelle est gérée au niveau du POS pour l'affichage du ticket cuisine ou bar.

Ventes & Commandes      La `reference` d'un `costCenter` doit-elle être unique ?     Oui. Chaque `costCenter` doit avoir une `reference` unique. Réutiliser systématiquement une même valeur (par exemple          211
                                                                                     « T1 ») pour des sessions de tables différentes n'est pas conforme.



Ventes & Commandes      Lors d'un changement de `costCenter` (transfert d'une        La `costCenter.reference` doit rester la même tout au long de la vie d'une commande, pour permettre de suivre son flux        212
                       commande), comment évolue la `costCenter.reference` ?         complet. En cas de transfert :



                                                                                     a) Transfert complet d'une table vers une table vide : la `reference` de la table d'origine est transférée vers la table de
                                                                                     destination.




                                                                                                                                                                                                                     Page 5/12
                                                                                      b) Transfert partiel vers une table vide : la partie transférée reçoit une nouvelle `reference` sur la table de destination.




                                                                                      c) Transfert (complet ou partiel) vers une table déjà occupée : la commande transférée reprend la `reference` de la table
                                                                                      de destination.



Ventes & Commandes   Comment gérer les corrections lors d'une commande passée         a) Les corrections ne doivent être enregistrées que lorsqu'elles sont effectuées par un employé. Les corrections faites par    213
                       par le client lui-même via un kiosque ou un webshop ?          le client lui-même sur un kiosque ou un webshop ne doivent pas être enregistrées.



                                                                                      b) Pour les commandes passées au kiosque ou au webshop, la consolidation des lignes est autorisée à la fois dans le
                                                                                      message JSON envoyé au FDM et sur le ticket de caisse TVA.



    Corrections        Quelles sont les règles de distinction entre `REFUND` et       a) `REFUND` : utilisé lors d'un retour client ou d'une annulation après émission du ticket de caisse TVA. Lorsque le ticket    301
                        `CORRECTION` dans le champ `negQuantityReason` ?              comprend uniquement des lignes négatives, il porte la mention « REFUND ». Lorsque le ticket créé correspond au
                                                                                      remboursement complet d'un ticket TVA, la `fdmRef` du ticket d'origine doit être ajoutée.

                                                                                      b) `CORRECTION` : utilisé pour corriger une erreur de saisie interne avant paiement définitif.




                                                                                      c) Le nombre de remboursements et le nombre de corrections sont repris dans les rapports sous « negQuantities »




    Corrections       Quelles règles utiliser pour les corrections spécifiques (ex:   a) `VOUCHER` : utilisé lorsqu'une ligne négative correspond à l'usage d'un bon à usage unique (SPV).                           302
                         voucher, remplacement d'articles) dans le champ
                                        `negQuantityReason` ?

                                                                                      b) `PRODUCT_SUBSTITUTION` : utilisé lors du remplacement d'un sous-article par un autre sous-article dans un produit
                                                                                      composite (`COMPOSITE_PRODUCT`) (ex: frites au lieu de purée au sein d'un menu).



                                                                                      c) L'usage du motif générique `CORRECTION` est interdit pour ces cas d'usage spécifiques afin d'assurer une traçabilité
                                                                                      précise.




                                                                                                                                                                                                                       Page 6/12
Corrections        Sur quels messages se baser pour comptabiliser les           a) En gestion de table : sur les `signOrder`.                                                                                303
              `negQuantityReason` dans le rapport de chiffre d'affaires (X / Z)
                                            ?

                                                                                 b) En vente directe : sur les `signSale`.




                                                                                 c) Les `signPreBill` ne sont jamais comptabilisés.




                                                                                 d) Cas particulier : lorsque le `priceChange` est utilisé pour annuler un article déjà commandé afin de lui appliquer une
                                                                                 remise, il n'apparaît que dans la commande et est donc compté sur le `signOrder`, quel que soit le contexte de vente.



Corrections     Comment gérer l'utilisation du `groupingId` lors de remises      a) Le « groupingId » indique l'ordre dans lequel les remises ont été appliquées. Si une remise est appliquée                304
                                       successives ?                             simultanément à plusieurs produits, ceux-ci se voient attribuer le même « groupingId ».



                                                                                 b) Pour des remises successives, l'approche chronologique est requise : chaque nouvelle remise incrémente l'ID (1, 2,
                                                                                 3…).



                                                                                 c) Cette méthode permet de distinguer si les remises ont été appliquées globalement (1 ID unique) ou ligne par ligne
                                                                                 (plusieurs IDs) et dans quel ordre.



                                                                                 d) En cas de `costCenterChange`, le `groupingId` reste inchangé.




                                                                                 e) Il est donc possible que lors du `signSale` final, plusieurs remises portent le même `groupingId`.




Paiements      Si le système ne dispose pas de tiroir-caisse, les fonctions de   a) C'est une règle de « tout ou rien » : si le système permet l'encaissement en espèces (`CASH`), ces fonctions sont        401
                 gestion des espèces (Money In/Out) sont-elles requises ?        obligatoires.




                                                                                                                                                                                                               Page 7/12
                                                                          b) Les `signMoneyInOut` doivent tracer tout mouvement de fond, même sans tiroir physique.




                                                                          c) Seuls les systèmes strictement cashless (sans option espèce) sont dispensés de cette obligation.




Paiements   Comment enregistrer le rendu de monnaie et le cashback dans a) Paiement en espèces : une seule ligne positive (montant dû). Le rendu de monnaie n'est pas enregistré.                  402
                              les objets `payment` ?



                                                                          b) Cashback (carte) : une ligne positive pour le montant carte prélevé, et une ligne négative de type `CASH` pour le
                                                                          montant rendu au client.



                                                                          c) Aucune monnaie ne peut être rendue sur un chèque repas.




Paiements        Comment encoder les pourboires (`TIP`) et arrondis       a) Ces montants ne doivent jamais être inclus dans le `totalAmount` du chiffre d'affaires utilisateur.                   403
                 (`ROUNDING`) dans la communication vers le FDM ?



                                                                          b) Ils doivent apparaître distinctement dans le tableau `payments` avec leur propre type (`TIP` ou `ROUNDING`).




                                                                          c) Si la fonction pourboire est prévue dans la caisse : l'intégration du pourboire dans le même objet `payment` que la
                                                                          vente (sous un `amount` séparé) est obligatoire.



Paiements       Les pièces de 1 et 2 centimes doivent-elles être gérées   a) Non, ces pièces constituent de la monnaie légale.                                                                     404
                      différemment dans les flux Money In/Out ?



                                                                          b) Elles doivent être comptabilisées dans le montant total des espèces (`CASH`) sans distinction particulière dans le
                                                                          journal fiscal.




                                                                                                                                                                                                     Page 8/12
Paiements      Tiroir-caisse : quand utiliser `signDrawerOpen` et faut-il     a) `signDrawerOpen` est utilisé uniquement lorsque le tiroir-caisse est ouvert sans aucune autre opération enregistrée         405
                        l'envoyer avant un `signMoneyInOut` ?                 (par exemple pour récupérer une clé).



                                                                              b) Il n'est pas nécessaire d'envoyer un `signDrawerOpen` avant un `signMoneyInOut`.




Technique     Un rapport Z Chiffre d'Affaires peut-il être généré avec une a) Le rapport Z doit être établi (au moins) une fois par jour. Il est par exemple possible de générer un rapport de chiffre       501
            périodicité supérieure à 24 heures ? Est-il possible de générer d'affaires Z par BookingPeriod.
            plus d'un rapport de chiffre d'affaires par jour (bookingDate) ?

                                                                              b) Une tolérance existe pour une création différée si elle est justifiée (ex: fermeture hebdomadaire). Cf. Circulaire C70 du
                                                                              6 novembre 2017.



                                                                              c) En cas de dépassement, la caisse créera automatiquement un event R par période de booking date manquante (24 h).
                                                                              c)



Technique   Les events R (Chiffre d'Affaires et Utilisateurs) doivent-ils être Non. Les `timestamps` ne doivent pas inclure l'event R du rapport lui-même : seuls les events de la période effective sont    502
            inclus pour déterminer les `timestamps` de début et de fin de pris en compte.
                                       période ?

Technique      Un event Copy (`signCopy`) doit-il être généré lors de la      a) Oui, l'event Copy est obligatoire pour toute réimpression via le système de caisse d'un event signé, qu'il s'agisse d'un    503
                          réimpression d'une facture A4 ?                     ticket de caisse TVA ou d'une facture (event I), quel que soit son format (ticket ou A4, papier ou numérique via Peppol).



                                                                              b) Le champ `copyOfEvent fdmRef` doit contenir la référence de l'event original.




Technique      Le rapport Chiffre d'Affaires doit-il inclure tous les types   a) Non, le message JSON du rapport Chiffre d'Affaires ne doit pas contenir l'intégralité des events.                           504
                                 d'events (ex: COPY) ?



                                                                              b) Les events de type Copy (duplicatas) ne doivent pas y figurer.




                                                                                                                                                                                                               Page 9/12
                                                                              c) Seuls les events impactant les données financières ou de contrôle doivent être consolidés dans ce rapport.




Technique   Identification du `deviceId` : comment doit être identifié chaque a) En général, l'identification se fait via l'adresse MAC propre de chaque device.                                      505
                                          device ?



                                                                              b) D'autres types d'identification sont possibles via un identifiant hardware unique (ex: numéro de série).




                                                                              c) Pour un webshop, qui ne dispose pas d'identifiant matériel, l'URL du site web peut être utilisée comme `deviceId`.




                                                                              d) L'identifiant doit être différent pour chaque type de device (POS, handheld, kiosque).




Technique     La query `Status` prévue pour le FDM est-elle optionnelle ?     Oui. Cette query est mise à disposition par le FDM mais est facultative.                                                506




Technique   La mention « TICKET DE CAISSE TVA. » doit-elle figurer sur tous a) Non, cette mention doit être imprimée exclusivement sur le ticket de caisse TVA.                                       507
                                  les documents ?



                                                                              b) Il ne doit pas apparaître sur les duplicatas non fiscaux ni sur les notes internes.




                                                                              c) Il doit être imprimé dans une police de taille normale et lisible.




Technique      Les rapports internes (non fiscaux) du système doivent-ils     a) Oui, tout rapport généré par la caisse (hebdomadaire, mensuel) doit être cohérent avec les données des rapports Z    508
                        correspondre aux rapports Z fiscaux ?                 fiscaux.




                                                                                                                                                                                                       Page 10/12
                                                                          b) L'administration exige que les données de gestion correspondent aux données fiscales signées.




Technique    Dans le rapport Z (objet `users`), que représente le champ   a) Le champ `totalAmount` correspond à la somme des `transactionTotal` de tous les events N (`signSale`) enregistrés par    509
                `totalAmount` ? Inclut-il l'arrondi et le pourboire ?     l'utilisateur concerné durant la période. Il représente donc le chiffre d'affaires réalisé par cet utilisateur.



                                                                          b) Le pourboire n'est pas inclus, car il n'entre pas dans le `transactionTotal` d'une vente.




                                                                          c) Par conséquent, la somme des montants figurant dans l'objet `payments` du même utilisateur peut être supérieure ou
                                                                          inférieure à `totalAmount'.



Technique   FDM d'urgence (Emergency FDM) : comment l'enregistrer et      a) Un Emergency FDM est enregistré dans le profil d'un distributeur et ne peut pas être converti en FDM permanent pour      511
                                  l'utiliser ?                            être revendu.



                                                                          b) Tant que l'enregistrement via l'application n'est pas possible, le numéro de série du FDM doit être communiqué au
                                                                          secrétariat (secr.gksce@minfin.fed.be).



                                                                          c) Le distributeur peut utiliser un Emergency FDM dans deux cas : (i) défaut présumé d'un FDM en attente de réparation,
                                                                          pour quelques jours seulement ; (ii) en attente de la livraison d'un nouveau FDM qui ne peut pas être enregistré
                                                                          immédiatement dans l'application d'enregistrement.

                                                                          d) Pour programmer la caisse pour un nouveau client en l'absence de FDM test, le numéro d'entreprise et le numéro
                                                                          d'unité d'établissement du distributeur doivent être programmés dans la caisse.



                                                                          e) Avant de basculer temporairement sur un Emergency FDM, vérifier : (i) les messages d'erreur spécifiques de la caisse ;
                                                                          (ii) les voyants du FDM ; (iii) que le numéro d'entreprise et le numéro d'unité d'établissement programmés dans la caisse
                                                                          correspondent bien à l'entreprise pour laquelle les transactions sont enregistrées.

                                                                          f) Lorsqu'un Emergency FDM est utilisé, les données de transaction sont transmises au cloud sous le numéro d'entreprise
                                                                          et l'unité d'établissement programmés dans la caisse.




                                                                                                                                                                                                       Page 11/12
Technique   FDM test : qu'est-ce que c'est et dans quels cas peut-il être   a) Un FDM test est un FDM réservé à l'environnement d'acceptation. Il ne fonctionne pas en environnement de                 512
                           utilisé chez un restaurateur ?                   production et ne peut pas être converti en FDM permanent pour être revendu à un assujetti-exploitant.



                                                                            b) Le fabricant ou le distributeur peut s'en servir pour préparer la mise en service d'une caisse avant qu'un FDM
                                                                            permanent ne soit installé : tests internes chez le fabricant, programmation de la caisse pour un nouveau client,
                                                                            démonstrations en salle d'exposition ou en salon.

                                                                            c) Lorsqu'un FDM test est en service, les transactions sont transmises au cloud d'acceptation sous le numéro d'entreprise
                                                                            et l'unité d'établissement programmés dans la caisse — elles ne sont pas considérées comme des données fiscales
                                                                            réelles.

                                                                            d) Le numéro de série du FDM test doit être communiqué au secrétariat (secr.gksce@minfin.fed.be).




Technique   Remplacement d'un FDM : quelles vérifications effectuer ?       a) Vérifier dans l'application que le numéro de série du FDM a été correctement enregistré.                                 513




                                                                            b) Vérifier que le numéro d'entreprise et le numéro d'unité d'établissement sont correctement programmés dans la
                                                                            caisse.




                                                                                                                                                                                                         Page 12/12
```
