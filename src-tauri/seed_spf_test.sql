-- ── pili-pos - Seed pour la machine de test SPF Finances ───────────
--
-- Reference : §III "Lignes de produit predefinies" du manuel de
-- certification (page 29-30 de
-- 20250307_FR - Manuel de certification_POS_V11.md).
--
-- Cette base de donnees contient EXACTEMENT les produits, productId,
-- departments et prix du jeu de test SPF (UC_TestSetup_20240806,
-- docs/20260821_UC_Reports_1.3/) attendus par les UC du Basic Set v1.8
-- et de l'Advanced Set v1.6, afin que les agents SPF Finances puissent
-- executer les scenarios sans translation entre le catalogue d'un
-- etablissement et les fixtures du manuel.
--
-- Applique a la construction de l'image d'admission, sur une base
-- vierge (refuse si des evenements fiscaux existent deja) :
--   pili-pos --init-db --seed seed_spf_test.sql
--
-- VAT codes utilises (col vat_code) :
--   A = 21% (alcools, boissons rafraichissantes)
--   B = 12% (services restauration sur place)
--   C = 6%  (vente a emporter)
--   D = 0%  (cigarettes)
--   X = N/A (vouchers et consignes - hors champ TVA)
--
-- Pour les UC du manuel, le scenario indique "DINING ROOM" ou "TAKEAWAY".
-- menu_items.vat_code porte le label "Sur place" (A/B) ; le POS applique
-- le label "A emporter" (colonne menu_items.vat_code_takeaway, voir fin
-- de fichier) aux commandes a emporter.

PRAGMA foreign_keys = ON;

-- ── Reset (idempotent pour reseed) ───────────────────────────────
DELETE FROM order_lines;
DELETE FROM payments;
DELETE FROM orders;
-- Les lignes systeme du bon cadeau (migration 63) restent : categorie
-- 'pili:system' et produit 'pili:gift-voucher' (inactifs, hors menu).
DELETE FROM translations WHERE entity_type IN ('category', 'menu_item')
    AND entity_id NOT IN (990000, 990001);
DELETE FROM menu_items WHERE external_id IS NOT 'pili:gift-voucher';
DELETE FROM categories WHERE external_id IS NOT 'pili:system';

-- ── Langues supportees ───────────────────────────────────────────
-- Le schema initialise fr/nl/de/en/zh ('de' depuis la migration v7).
-- Repete ici pour qu'une base plus ancienne accepte les traductions
-- allemandes ci-dessous.
INSERT OR IGNORE INTO languages (code, name) VALUES
    ('de', 'Deutsch');

-- ── Utilisateurs de test ────────────────────────────────────────
-- Le personnel du jeu de test SPF (UC_TestSetup_20240806) : John
-- (barman), Elisa (chef de salle) et Sammy (sommelier), avec leurs
-- employeeId ; ils enregistrent commandes et ventes (role cashier).
-- S'y ajoutent un profil par role de PROFILS_UTILISATEURS_FR.md
-- (identifiants remis aux agents, REPONSES_MANUEL_CERT_FR.md,
-- "Informations supplementaires"). PIN hashes bcrypt (cout 12) :
-- Technicien 0097, Gerant 1111, Elisa 2222, Apprenti 3333, John 4444,
-- Sammy 5555. Le robot (00000000029) n'a pas de PIN.
INSERT OR REPLACE INTO users (insz, name, role, pin_hash) VALUES
    ('00000000029', 'Robot / Kiosk', 'admin', NULL),
    ('00000000097', 'Technicien SPF', 'admin', '$2y$12$biMjvYoNKdT/dWj40tTyTuO3u/zvao.kwyU01SdyFPwIoxGg6UZR2'),
    ('94060242576', 'Gerant Test', 'admin', '$2y$12$XHH6GSKAOw3OBnHXvDlXjeBkBosOS2xDU4xu31O7yUUf6tfiO2ZRy'),
    ('75061189702', 'John', 'cashier', '$2y$12$J.LaAI2LXHL.3Sn/RxaizO06h/PaFlS4TDN.W67AP/CpJoBRMQ0Ne'),
    ('84022899837', 'Elisa', 'cashier', '$2y$12$7X94vlUi08Hlaid/ZV2vb.FM3QPpjKHW1vqEXxppYfANN7HCKuKD6'),
    ('80113078968', 'Sammy', 'cashier', '$2y$12$SBshMmwu2WPvEtC0A7ldpei7uKtYigjta7y6FsmHI8FFnWkQetp5W'),
    ('99012345667', 'Apprenti Test', 'waiter', '$2y$12$FxE17eeRfr94ie9p28GF1eD0cB8o5dwy1pNsvKGf3oOq2oE81XqPi');

-- ── Categories (= departments du manuel) ────────────────────────
INSERT INTO categories (id, sort_order, color, active) VALUES
    (10,  1, '#E74C3C', 1),  -- Aperitifs
    (12,  2, '#E67E22', 1),  -- Soft Drinks
    (15,  3, '#F39C12', 1),  -- Wines
    (20,  4, '#F1C40F', 1),  -- Starters
    (22,  5, '#2ECC71', 1),  -- Main Dishes
    (25,  6, '#27AE60', 1),  -- Desserts
    (28,  7, '#1ABC9C', 1),  -- Tapas
    (30,  8, '#3498DB', 1),  -- Menus
    (90,  9, '#9B59B6', 1),  -- Bottle Deposits
    (400, 10, '#E91E63', 1), -- Vouchers
    (500, 11, '#34495E', 1); -- Cigarettes

-- Noms des departements : ceux du jeu de test SPF, dans toutes les
-- langues de l'ecran (les UC envoient departmentName au FDM).
INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
    ('category', 10, 'name', 'fr', 'Aperitifs'),
    ('category', 10, 'name', 'en', 'Aperitifs'),
    ('category', 10, 'name', 'nl', 'Aperitifs'),
    ('category', 10, 'name', 'de', 'Aperitifs'),
    ('category', 12, 'name', 'fr', 'Soft Drinks'),
    ('category', 12, 'name', 'en', 'Soft Drinks'),
    ('category', 12, 'name', 'nl', 'Soft Drinks'),
    ('category', 12, 'name', 'de', 'Soft Drinks'),
    ('category', 15, 'name', 'fr', 'Wines'),
    ('category', 15, 'name', 'en', 'Wines'),
    ('category', 15, 'name', 'nl', 'Wines'),
    ('category', 15, 'name', 'de', 'Wines'),
    ('category', 20, 'name', 'fr', 'Starters'),
    ('category', 20, 'name', 'en', 'Starters'),
    ('category', 20, 'name', 'nl', 'Starters'),
    ('category', 20, 'name', 'de', 'Starters'),
    ('category', 22, 'name', 'fr', 'Main Dishes'),
    ('category', 22, 'name', 'en', 'Main Dishes'),
    ('category', 22, 'name', 'nl', 'Main Dishes'),
    ('category', 22, 'name', 'de', 'Main Dishes'),
    ('category', 25, 'name', 'fr', 'Desserts'),
    ('category', 25, 'name', 'en', 'Desserts'),
    ('category', 25, 'name', 'nl', 'Desserts'),
    ('category', 25, 'name', 'de', 'Desserts'),
    ('category', 28, 'name', 'fr', 'Tapas'),
    ('category', 28, 'name', 'en', 'Tapas'),
    ('category', 28, 'name', 'nl', 'Tapas'),
    ('category', 28, 'name', 'de', 'Tapas'),
    ('category', 30, 'name', 'fr', 'Menus'),
    ('category', 30, 'name', 'en', 'Menus'),
    ('category', 30, 'name', 'nl', 'Menus'),
    ('category', 30, 'name', 'de', 'Menus'),
    ('category', 90, 'name', 'fr', 'Bottle Deposits'),
    ('category', 90, 'name', 'en', 'Bottle Deposits'),
    ('category', 90, 'name', 'nl', 'Bottle Deposits'),
    ('category', 90, 'name', 'de', 'Bottle Deposits'),
    ('category', 400, 'name', 'fr', 'Vouchers'),
    ('category', 400, 'name', 'en', 'Vouchers'),
    ('category', 400, 'name', 'nl', 'Vouchers'),
    ('category', 400, 'name', 'de', 'Vouchers'),
    ('category', 500, 'name', 'fr', 'Cigarettes'),
    ('category', 500, 'name', 'en', 'Cigarettes'),
    ('category', 500, 'name', 'nl', 'Cigarettes'),
    ('category', 500, 'name', 'de', 'Cigarettes');

-- ── Produits predefinis selon manuel cert §III ───────────────────
-- ID, category, prix en cents (12 EUR = 1200), VAT dining-room.
-- vat_category mirrors HubRise sync logic : food | beverage | zero_rated | out_of_scope.
INSERT INTO menu_items (id, category_id, price_cents, vat_code, vat_category, kitchen_printer, sort_order, active) VALUES
    (10006,  10,  1200,  'A', 'beverage',     0, 1, 1),  -- Dry Martini, 12,00 EUR
    (12007,  12,   250,  'A', 'beverage',     0, 1, 1),  -- Perrier, 2,50 EUR
    (12009,  12,   304,  'A', 'beverage',     0, 2, 1),  -- Coca Cola, 3,04 EUR
    (15001,  15,  3500,  'A', 'beverage',     0, 1, 1),  -- Bourgogne Red, 35,00 EUR
    (15002,  15,  4000,  'A', 'beverage',     0, 2, 1),  -- Chardonnay White, 40,00 EUR
    (15003,  15,   800,  'A', 'beverage',     0, 3, 1),  -- Wine (glass), 8,00 EUR
    (15023,  15,  2400,  'A', 'beverage',     0, 4, 1),  -- Matching Wines, 24,00 EUR
    (20001,  20,  2000,  'B', 'food',         1, 1, 1),  -- Carpaccio Beef, 20,00 EUR
    (22001,  22,  2800,  'B', 'food',         1, 1, 1),  -- Burger of the Chef, 28,00 EUR
    (22005,  22,  4500,  'B', 'food',         1, 2, 1),  -- Catch of the day, 45,00 EUR par KG (vente au poids non supportee)
    (22010,  22,     0,  'B', 'food',         1, 3, 1),  -- Dish of the day (prix libre)
    (25003,  25,  1152,  'B', 'food',         0, 1, 1),  -- Dame Blanche, 11,52 EUR
    (25008,  25,  1000,  'B', 'food',         0, 2, 1),  -- Creme brulee, 10,00 EUR
    (28007,  28,   334,  'B', 'food',         1, 1, 1),  -- Tapas variation, 3,34 EUR
    (30001,  30,  3800,  'B', 'food',         1, 1, 1),  -- Business menu, 38,00 EUR (composite)
    (30010,  30,  5800,  'B', 'food',         1, 2, 1),  -- Business menu all-in, 58,00 EUR, labels A&B (produit compose non supporte)
    (30020,  30,  1200,  'A', 'beverage',     0, 3, 1),  -- Menu drink, 12,00 EUR
    (40001, 400,  5000,  'X', 'out_of_scope', 0, 1, 1),  -- Gift Voucher 50 EUR
    (40002, 400, 10000,  'X', 'out_of_scope', 0, 2, 1),  -- Gift Voucher 100 EUR
    (40003, 400, 15000,  'X', 'out_of_scope', 0, 3, 1),  -- Gift Voucher 150 EUR
    (40004, 400, 25000,  'X', 'out_of_scope', 0, 4, 1),  -- Gift Voucher 250 EUR
    (40020, 400,  7500,  'B', 'food',         0, 5, 1),  -- Voucher Special Diner Offer 75 EUR
    (90001,  90,     1,  'X', 'out_of_scope', 0, 1, 1),  -- Bottle S deposit 0,01 EUR
    (90100,  90,   449,  'X', 'out_of_scope', 0, 2, 1),  -- Beer bin / keg 4,49 EUR
    (500001, 500,  840,  'D', 'zero_rated',   0, 1, 1);  -- Cigarettes (20 pcs), 8,40 EUR

-- Noms des produits : ceux du jeu de test SPF, a l'identique, dans
-- toutes les langues de l'ecran (les UC envoient ces noms au FDM).
INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
    ('menu_item', 10006, 'name', 'fr', 'Dry Martini'),
    ('menu_item', 10006, 'name', 'en', 'Dry Martini'),
    ('menu_item', 10006, 'name', 'nl', 'Dry Martini'),
    ('menu_item', 10006, 'name', 'de', 'Dry Martini'),
    ('menu_item', 12007, 'name', 'fr', 'Perrier'),
    ('menu_item', 12007, 'name', 'en', 'Perrier'),
    ('menu_item', 12007, 'name', 'nl', 'Perrier'),
    ('menu_item', 12007, 'name', 'de', 'Perrier'),
    ('menu_item', 12009, 'name', 'fr', 'Coca Cola'),
    ('menu_item', 12009, 'name', 'en', 'Coca Cola'),
    ('menu_item', 12009, 'name', 'nl', 'Coca Cola'),
    ('menu_item', 12009, 'name', 'de', 'Coca Cola'),
    ('menu_item', 15001, 'name', 'fr', 'Bourgogne Red'),
    ('menu_item', 15001, 'name', 'en', 'Bourgogne Red'),
    ('menu_item', 15001, 'name', 'nl', 'Bourgogne Red'),
    ('menu_item', 15001, 'name', 'de', 'Bourgogne Red'),
    ('menu_item', 15002, 'name', 'fr', 'Chardonnay White'),
    ('menu_item', 15002, 'name', 'en', 'Chardonnay White'),
    ('menu_item', 15002, 'name', 'nl', 'Chardonnay White'),
    ('menu_item', 15002, 'name', 'de', 'Chardonnay White'),
    ('menu_item', 15003, 'name', 'fr', 'Wine (glass)'),
    ('menu_item', 15003, 'name', 'en', 'Wine (glass)'),
    ('menu_item', 15003, 'name', 'nl', 'Wine (glass)'),
    ('menu_item', 15003, 'name', 'de', 'Wine (glass)'),
    ('menu_item', 15023, 'name', 'fr', 'Matching Wines'),
    ('menu_item', 15023, 'name', 'en', 'Matching Wines'),
    ('menu_item', 15023, 'name', 'nl', 'Matching Wines'),
    ('menu_item', 15023, 'name', 'de', 'Matching Wines'),
    ('menu_item', 20001, 'name', 'fr', 'Carpaccio Beef'),
    ('menu_item', 20001, 'name', 'en', 'Carpaccio Beef'),
    ('menu_item', 20001, 'name', 'nl', 'Carpaccio Beef'),
    ('menu_item', 20001, 'name', 'de', 'Carpaccio Beef'),
    ('menu_item', 22001, 'name', 'fr', 'Burger of the CHEF'),
    ('menu_item', 22001, 'name', 'en', 'Burger of the CHEF'),
    ('menu_item', 22001, 'name', 'nl', 'Burger of the CHEF'),
    ('menu_item', 22001, 'name', 'de', 'Burger of the CHEF'),
    ('menu_item', 22005, 'name', 'fr', 'Catch of the day'),
    ('menu_item', 22005, 'name', 'en', 'Catch of the day'),
    ('menu_item', 22005, 'name', 'nl', 'Catch of the day'),
    ('menu_item', 22005, 'name', 'de', 'Catch of the day'),
    ('menu_item', 22010, 'name', 'fr', 'Dish of the day'),
    ('menu_item', 22010, 'name', 'en', 'Dish of the day'),
    ('menu_item', 22010, 'name', 'nl', 'Dish of the day'),
    ('menu_item', 22010, 'name', 'de', 'Dish of the day'),
    ('menu_item', 25003, 'name', 'fr', 'Dame Blanche'),
    ('menu_item', 25003, 'name', 'en', 'Dame Blanche'),
    ('menu_item', 25003, 'name', 'nl', 'Dame Blanche'),
    ('menu_item', 25003, 'name', 'de', 'Dame Blanche'),
    ('menu_item', 25008, 'name', 'fr', 'crème brûlée'),
    ('menu_item', 25008, 'name', 'en', 'crème brûlée'),
    ('menu_item', 25008, 'name', 'nl', 'crème brûlée'),
    ('menu_item', 25008, 'name', 'de', 'crème brûlée'),
    ('menu_item', 28007, 'name', 'fr', 'Tapas variation'),
    ('menu_item', 28007, 'name', 'en', 'Tapas variation'),
    ('menu_item', 28007, 'name', 'nl', 'Tapas variation'),
    ('menu_item', 28007, 'name', 'de', 'Tapas variation'),
    ('menu_item', 30001, 'name', 'fr', 'Business menu'),
    ('menu_item', 30001, 'name', 'en', 'Business menu'),
    ('menu_item', 30001, 'name', 'nl', 'Business menu'),
    ('menu_item', 30001, 'name', 'de', 'Business menu'),
    ('menu_item', 30010, 'name', 'fr', 'Business menu all-in'),
    ('menu_item', 30010, 'name', 'en', 'Business menu all-in'),
    ('menu_item', 30010, 'name', 'nl', 'Business menu all-in'),
    ('menu_item', 30010, 'name', 'de', 'Business menu all-in'),
    ('menu_item', 30020, 'name', 'fr', 'Menu drink'),
    ('menu_item', 30020, 'name', 'en', 'Menu drink'),
    ('menu_item', 30020, 'name', 'nl', 'Menu drink'),
    ('menu_item', 30020, 'name', 'de', 'Menu drink'),
    ('menu_item', 40001, 'name', 'fr', 'Gift Voucher General'),
    ('menu_item', 40001, 'name', 'en', 'Gift Voucher General'),
    ('menu_item', 40001, 'name', 'nl', 'Gift Voucher General'),
    ('menu_item', 40001, 'name', 'de', 'Gift Voucher General'),
    ('menu_item', 40002, 'name', 'fr', 'Gift Voucher General'),
    ('menu_item', 40002, 'name', 'en', 'Gift Voucher General'),
    ('menu_item', 40002, 'name', 'nl', 'Gift Voucher General'),
    ('menu_item', 40002, 'name', 'de', 'Gift Voucher General'),
    ('menu_item', 40003, 'name', 'fr', 'Gift Voucher General'),
    ('menu_item', 40003, 'name', 'en', 'Gift Voucher General'),
    ('menu_item', 40003, 'name', 'nl', 'Gift Voucher General'),
    ('menu_item', 40003, 'name', 'de', 'Gift Voucher General'),
    ('menu_item', 40004, 'name', 'fr', 'Gift Voucher General'),
    ('menu_item', 40004, 'name', 'en', 'Gift Voucher General'),
    ('menu_item', 40004, 'name', 'nl', 'Gift Voucher General'),
    ('menu_item', 40004, 'name', 'de', 'Gift Voucher General'),
    ('menu_item', 40020, 'name', 'fr', 'Voucher Special Diner Offer'),
    ('menu_item', 40020, 'name', 'en', 'Voucher Special Diner Offer'),
    ('menu_item', 40020, 'name', 'nl', 'Voucher Special Diner Offer'),
    ('menu_item', 40020, 'name', 'de', 'Voucher Special Diner Offer'),
    ('menu_item', 90001, 'name', 'fr', 'Bottle S'),
    ('menu_item', 90001, 'name', 'en', 'Bottle S'),
    ('menu_item', 90001, 'name', 'nl', 'Bottle S'),
    ('menu_item', 90001, 'name', 'de', 'Bottle S'),
    ('menu_item', 90100, 'name', 'fr', 'Beer bin/ keg beer'),
    ('menu_item', 90100, 'name', 'en', 'Beer bin/ keg beer'),
    ('menu_item', 90100, 'name', 'nl', 'Beer bin/ keg beer'),
    ('menu_item', 90100, 'name', 'de', 'Beer bin/ keg beer'),
    ('menu_item', 500001, 'name', 'fr', 'Cigarettes (20 pcs)'),
    ('menu_item', 500001, 'name', 'en', 'Cigarettes (20 pcs)'),
    ('menu_item', 500001, 'name', 'nl', 'Cigarettes (20 pcs)'),
    ('menu_item', 500001, 'name', 'de', 'Cigarettes (20 pcs)');

-- ── Produits presents dans la liste du manuel mais non disponibles ─
-- Ils necessitent des fonctionnalites declarees non supportees en
-- v1.0.0 (REPONSES_MANUEL_CERT_FR.md §III.F) : prix au poids (22005),
-- produits composes (30001, 30010, 30020) et
-- vouchers (40001-40004, 40020). Ils restent en base avec leur
-- productId pour la tracabilite mais ne sont pas proposes a la vente.
UPDATE menu_items SET active = 0
WHERE id IN (22005, 30001, 30010, 30020, 40001, 40002, 40003, 40004, 40020);
-- Departements sans aucun produit disponible : masques dans la grille.
UPDATE categories SET active = 0 WHERE id IN (30, 400);

-- Plat du jour (22010) : prix libre, saisi par l'operateur a l'ajout.
UPDATE menu_items SET open_price = 1 WHERE id = 22010;

-- Label TVA "A emporter" (manuel, lignes de produit) quand il differe
-- du label "Sur place". NULL = meme label qu'en salle (A->A, X->X).
UPDATE menu_items SET vat_code_takeaway = 'C'
WHERE id IN (12007, 12009, 20001, 22001, 22005, 22010, 25003, 25008, 28007, 30001);

-- ── Compteurs fiscaux remis a 0 pour la machine de test ──────────
DELETE FROM fiscal_counters;
INSERT INTO fiscal_counters (event_label, current_value) VALUES
    ('N', 0), ('P', 0), ('T', 0), ('C', 0),
    ('I', 0), ('F', 0), ('S', 0), ('R', 0);

-- ── Note finale ──────────────────────────────────────────────────
-- Apres execution de ce seed, le POS est pret pour les UC du manuel :
-- - M110 : 2x Dry Martini (10006) + 1x Burger of the Chef (22001), payment CASH
-- - M111 : refund du M110
-- - M121 : commande sur table sans paiement (P event)
-- - etc.
--
-- Une image de production est construite avec le catalogue de
-- l'etablissement a la place de ce seed (meme binaire pili-pos).
