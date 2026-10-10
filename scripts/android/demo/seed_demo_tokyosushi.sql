-- pili-pos: DEMO seed (Tokyo Sushi Bar menu) for the Android emulator demo.
-- Not a certification fixture. Menu from src-tauri/seed.sql, staff and PINs from
-- src-tauri/seed_spf_test.sql: Mengmeng Z. (manager, PIN 1111), Sa X. (server, PIN 2222).

-- ── Reset (idempotent pour reseed) ───────────────────────────────
DELETE FROM order_lines;
DELETE FROM payments;
DELETE FROM orders;
DELETE FROM translations WHERE entity_type IN ('category', 'menu_item');
DELETE FROM menu_items;
DELETE FROM categories;

-- ── Langues supportees ───────────────────────────────────────────
-- Le schema initialise fr/nl/de/en/zh ('de' depuis la migration v7).
-- Repete ici pour qu'une base plus ancienne accepte les traductions
-- allemandes ci-dessous.
INSERT OR IGNORE INTO languages (code, name) VALUES
    ('de', 'Deutsch');

-- ── Demo staff (PIN hashes of the SPF test seed: 1111 and 2222) ──
DELETE FROM users;
INSERT OR REPLACE INTO users (insz, name, role, pin_hash) VALUES
    ('00000000029', 'Robot / Kiosk', 'admin', NULL),
    ('94060242576', 'Mengmeng Z.', 'admin', '$2y$12$XHH6GSKAOw3OBnHXvDlXjeBkBosOS2xDU4xu31O7yUUf6tfiO2ZRy'),
    ('84022899837', 'Sa X.', 'cashier', '$2y$12$7X94vlUi08Hlaid/ZV2vb.FM3QPpjKHW1vqEXxppYfANN7HCKuKD6');


-- ── Categories ─────────────────────────────────────────────────
INSERT OR REPLACE INTO categories (id, sort_order, color) VALUES
    (1,  1,  '#E74C3C'), -- Menu plateau
    (2,  2,  '#E67E22'), -- Menu bento box
    (3,  3,  '#F39C12'), -- Sushi
    (4,  4,  '#F1C40F'), -- Maki
    (5,  5,  '#2ECC71'), -- Gunkan
    (6,  6,  '#27AE60'), -- Spring roll
    (7,  7,  '#1ABC9C'), -- California roll
    (8,  8,  '#16A085'), -- Temaki
    (9,  9,  '#3498DB'), -- Masago roll
    (10, 10, '#2980B9'), -- Spécial roll
    (11, 11, '#9B59B6'), -- Chirashi
    (12, 12, '#8E44AD'), -- Sashimi
    (13, 13, '#E91E63'), -- Poke bowl
    (14, 14, '#D35400'), -- Tokyo hot
    (15, 15, '#95A5A6'), -- Accompagnement
    (16, 16, '#34495E'), -- Boisson
    (17, 17, '#C0392B'); -- Dessert

-- Category translations (fr, en, zh)
INSERT OR REPLACE INTO translations (entity_type, entity_id, field, lang, value) VALUES
    ('category', 1,  'name', 'fr', 'Menu plateau'),
    ('category', 1,  'name', 'en', 'Platter menu'),
    ('category', 1,  'name', 'zh', '拼盘套餐'),
    ('category', 2,  'name', 'fr', 'Menu bento box'),
    ('category', 2,  'name', 'en', 'Bento box menu'),
    ('category', 2,  'name', 'zh', '便当套餐'),
    ('category', 3,  'name', 'fr', 'Sushi'),
    ('category', 3,  'name', 'en', 'Sushi'),
    ('category', 3,  'name', 'zh', '寿司'),
    ('category', 4,  'name', 'fr', 'Maki'),
    ('category', 4,  'name', 'en', 'Maki'),
    ('category', 4,  'name', 'zh', '卷寿司'),
    ('category', 5,  'name', 'fr', 'Gunkan'),
    ('category', 5,  'name', 'en', 'Gunkan'),
    ('category', 5,  'name', 'zh', '军舰寿司'),
    ('category', 6,  'name', 'fr', 'Spring roll'),
    ('category', 6,  'name', 'en', 'Spring roll'),
    ('category', 6,  'name', 'zh', '春卷'),
    ('category', 7,  'name', 'fr', 'California roll'),
    ('category', 7,  'name', 'en', 'California roll'),
    ('category', 7,  'name', 'zh', '加州卷'),
    ('category', 8,  'name', 'fr', 'Temaki'),
    ('category', 8,  'name', 'en', 'Temaki'),
    ('category', 8,  'name', 'zh', '手卷'),
    ('category', 9,  'name', 'fr', 'Masago roll'),
    ('category', 9,  'name', 'en', 'Masago roll'),
    ('category', 9,  'name', 'zh', '鱼子卷'),
    ('category', 10, 'name', 'fr', 'Spécial roll'),
    ('category', 10, 'name', 'en', 'Special roll'),
    ('category', 10, 'name', 'zh', '特色卷'),
    ('category', 11, 'name', 'fr', 'Chirashi'),
    ('category', 11, 'name', 'en', 'Chirashi'),
    ('category', 11, 'name', 'zh', '散寿司'),
    ('category', 12, 'name', 'fr', 'Sashimi'),
    ('category', 12, 'name', 'en', 'Sashimi'),
    ('category', 12, 'name', 'zh', '刺身'),
    ('category', 13, 'name', 'fr', 'Poke bowl'),
    ('category', 13, 'name', 'en', 'Poke bowl'),
    ('category', 13, 'name', 'zh', '夏威夷鱼生饭'),
    ('category', 14, 'name', 'fr', 'Tokyo hot'),
    ('category', 14, 'name', 'en', 'Tokyo hot'),
    ('category', 14, 'name', 'zh', '东京热食'),
    ('category', 15, 'name', 'fr', 'Accompagnement'),
    ('category', 15, 'name', 'en', 'Side dish'),
    ('category', 15, 'name', 'zh', '配菜'),
    ('category', 16, 'name', 'fr', 'Boisson'),
    ('category', 16, 'name', 'en', 'Drink'),
    ('category', 16, 'name', 'zh', '饮品'),
    ('category', 17, 'name', 'fr', 'Dessert'),
    ('category', 17, 'name', 'en', 'Dessert'),
    ('category', 17, 'name', 'zh', '甜点');

-- ── Menu Items ─────────────────────────────────────────────────
-- VAT codes: B = 12% restaurant services (dine-in food)
--            A = 21% beverages (all drinks incl. soft)
--            C = 6% takeaway food (not used in dine-in POS)

-- Cat 1: Menu plateau (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (1, 1, 1480, 'B', 1), (2, 1, 1700, 'B', 2), (3, 1, 1790, 'B', 3),
    (4, 1, 1850, 'B', 4), (5, 1, 1880, 'B', 5), (6, 1, 2180, 'B', 6),
    (7, 1, 2180, 'B', 7), (8, 1, 2480, 'B', 8), (9, 1, 2480, 'B', 9),
    (10, 1, 2480, 'B', 10), (11, 1, 2680, 'B', 11), (12, 1, 3080, 'B', 12),
    (13, 1, 4280, 'B', 13), (14, 1, 4980, 'B', 14), (15, 1, 5080, 'B', 15),
    (16, 1, 5080, 'B', 16), (17, 1, 5250, 'B', 17), (18, 1, 5680, 'B', 18),
    (19, 1, 9280, 'B', 19), (20, 1, 9800, 'B', 20);

-- Cat 2: Menu bento box (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (21, 2, 1580, 'B', 1), (22, 2, 1780, 'B', 2),
    (23, 2, 1880, 'B', 3), (24, 2, 1880, 'B', 4);

-- Cat 3: Sushi - pièce (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (25, 3, 180, 'B', 1), (26, 3, 180, 'B', 2), (27, 3, 180, 'B', 3),
    (28, 3, 190, 'B', 4), (29, 3, 200, 'B', 5), (30, 3, 200, 'B', 6),
    (31, 3, 200, 'B', 7), (32, 3, 200, 'B', 8), (33, 3, 200, 'B', 9),
    (34, 3, 220, 'B', 10), (35, 3, 230, 'B', 11), (36, 3, 230, 'B', 12),
    (37, 3, 250, 'B', 13), (38, 3, 250, 'B', 14), (39, 3, 250, 'B', 15),
    (40, 3, 260, 'B', 16), (41, 3, 260, 'B', 17), (42, 3, 280, 'B', 18),
    (43, 3, 350, 'B', 19), (44, 3, 360, 'B', 20);

-- Cat 4: Maki x6 (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (45, 4, 420, 'B', 1), (46, 4, 420, 'B', 2), (47, 4, 420, 'B', 3),
    (48, 4, 430, 'B', 4), (49, 4, 450, 'B', 5), (50, 4, 470, 'B', 6),
    (51, 4, 480, 'B', 7), (52, 4, 480, 'B', 8), (53, 4, 500, 'B', 9),
    (54, 4, 500, 'B', 10), (55, 4, 500, 'B', 11), (56, 4, 520, 'B', 12),
    (57, 4, 550, 'B', 13), (58, 4, 550, 'B', 14), (59, 4, 560, 'B', 15),
    (60, 4, 600, 'B', 16), (61, 4, 600, 'B', 17), (62, 4, 750, 'B', 18);

-- Cat 5: Gunkan - pièce (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (63, 5, 200, 'B', 1), (64, 5, 230, 'B', 2), (65, 5, 230, 'B', 3),
    (66, 5, 240, 'B', 4), (67, 5, 240, 'B', 5), (68, 5, 260, 'B', 6),
    (69, 5, 260, 'B', 7), (70, 5, 280, 'B', 8);

-- Cat 6: Spring roll x6 (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (71, 6, 590, 'B', 1), (72, 6, 590, 'B', 2), (73, 6, 600, 'B', 3),
    (74, 6, 630, 'B', 4), (75, 6, 650, 'B', 5), (76, 6, 690, 'B', 6),
    (77, 6, 720, 'B', 7), (78, 6, 720, 'B', 8), (79, 6, 920, 'B', 9);

-- Cat 7: California roll x6 (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (80, 7, 520, 'B', 1), (81, 7, 560, 'B', 2), (82, 7, 570, 'B', 3),
    (83, 7, 580, 'B', 4), (84, 7, 580, 'B', 5), (85, 7, 600, 'B', 6),
    (86, 7, 600, 'B', 7), (87, 7, 600, 'B', 8), (88, 7, 610, 'B', 9),
    (89, 7, 620, 'B', 10), (90, 7, 700, 'B', 11), (91, 7, 710, 'B', 12),
    (92, 7, 780, 'B', 13), (93, 7, 780, 'B', 14), (94, 7, 780, 'B', 15),
    (95, 7, 980, 'B', 16);

-- Cat 8: Temaki - pièce (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (96, 8, 450, 'B', 1), (97, 8, 480, 'B', 2),
    (98, 8, 490, 'B', 3), (99, 8, 600, 'B', 4);

-- Cat 9: Masago roll x6 (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (100, 9, 600, 'B', 1), (101, 9, 600, 'B', 2), (102, 9, 650, 'B', 3);

-- Cat 10: Spécial roll x10 (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (103, 10, 800, 'B', 1), (104, 10, 980, 'B', 2), (105, 10, 980, 'B', 3),
    (106, 10, 980, 'B', 4), (107, 10, 1000, 'B', 5), (108, 10, 1000, 'B', 6),
    (109, 10, 1050, 'B', 7), (110, 10, 1080, 'B', 8), (111, 10, 1080, 'B', 9),
    (112, 10, 1080, 'B', 10), (113, 10, 1080, 'B', 11), (114, 10, 1180, 'B', 12),
    (115, 10, 1280, 'B', 13), (116, 10, 1480, 'B', 14);

-- Cat 11: Chirashi (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (117, 11, 1480, 'B', 1), (118, 11, 1580, 'B', 2), (119, 11, 1780, 'B', 3);

-- Cat 12: Sashimi (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (120, 12, 860, 'B', 1), (121, 12, 1080, 'B', 2),
    (122, 12, 1780, 'B', 3), (123, 12, 2080, 'B', 4);

-- Cat 13: Poke bowl (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (124, 13, 1180, 'B', 1), (125, 13, 1330, 'B', 2),
    (126, 13, 1330, 'B', 3), (127, 13, 1380, 'B', 4), (128, 13, 1580, 'B', 5);

-- Cat 14: Tokyo hot (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (129, 14, 400, 'B', 1), (130, 14, 450, 'B', 2), (131, 14, 550, 'B', 3),
    (132, 14, 550, 'B', 4), (133, 14, 600, 'B', 5), (134, 14, 650, 'B', 6),
    (135, 14, 660, 'B', 7), (136, 14, 680, 'B', 8), (137, 14, 750, 'B', 9),
    (138, 14, 750, 'B', 10), (139, 14, 850, 'B', 11), (140, 14, 980, 'B', 12),
    (141, 14, 1050, 'B', 13), (142, 14, 1150, 'B', 14), (143, 14, 1380, 'B', 15),
    (144, 14, 1580, 'B', 16), (145, 14, 1680, 'B', 17);

-- Cat 15: Accompagnement (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (146, 15, 300, 'B', 1), (147, 15, 350, 'B', 2),
    (148, 15, 350, 'B', 3), (149, 15, 400, 'B', 4),
    (150, 15, 400, 'B', 5), (151, 15, 400, 'B', 6);

-- Cat 16: Boisson (A - 21% TVA alcool+soft)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (152, 16, 250, 'A', 1), (153, 16, 250, 'A', 2),
    (154, 16, 380, 'A', 3), (155, 16, 380, 'A', 4),
    (156, 16, 380, 'A', 5), (157, 16, 380, 'A', 6),
    (158, 16, 380, 'A', 7), (159, 16, 380, 'A', 8),
    (160, 16, 390, 'A', 9);

-- Cat 17: Dessert (B)
INSERT OR REPLACE INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
    (161, 17, 500, 'B', 1), (162, 17, 500, 'B', 2),
    (163, 17, 550, 'B', 3), (164, 17, 750, 'B', 4);

-- ── Menu Item Translations ─────────────────────────────────────
-- Cat 1: Menu plateau
INSERT OR REPLACE INTO translations (entity_type, entity_id, field, lang, value) VALUES
    ('menu_item', 1, 'name', 'fr', 'Sushi saumon'), ('menu_item', 1, 'name', 'en', 'Salmon sushi'), ('menu_item', 1, 'name', 'zh', '三文鱼寿司拼盘'),
    ('menu_item', 2, 'name', 'fr', 'Saumon Time'), ('menu_item', 2, 'name', 'en', 'Salmon Time'), ('menu_item', 2, 'name', 'zh', '三文鱼时光'),
    ('menu_item', 3, 'name', 'fr', 'M1'), ('menu_item', 3, 'name', 'en', 'M1'), ('menu_item', 3, 'name', 'zh', 'M1'),
    ('menu_item', 4, 'name', 'fr', 'Plateau 16'), ('menu_item', 4, 'name', 'en', 'Platter 16'), ('menu_item', 4, 'name', 'zh', '拼盘16'),
    ('menu_item', 5, 'name', 'fr', 'M2'), ('menu_item', 5, 'name', 'en', 'M2'), ('menu_item', 5, 'name', 'zh', 'M2'),
    ('menu_item', 6, 'name', 'fr', 'Plateau saumon'), ('menu_item', 6, 'name', 'en', 'Salmon platter'), ('menu_item', 6, 'name', 'zh', '三文鱼拼盘'),
    ('menu_item', 7, 'name', 'fr', 'M3'), ('menu_item', 7, 'name', 'en', 'M3'), ('menu_item', 7, 'name', 'zh', 'M3'),
    ('menu_item', 8, 'name', 'fr', 'Plateau sushi grillé'), ('menu_item', 8, 'name', 'en', 'Grilled Sushi Platter'), ('menu_item', 8, 'name', 'zh', '烤寿司拼盘'),
    ('menu_item', 9, 'name', 'fr', 'Plateau California 24'), ('menu_item', 9, 'name', 'en', 'California platter 24'), ('menu_item', 9, 'name', 'zh', '加州卷拼盘24'),
    ('menu_item', 10, 'name', 'fr', 'Plateau sushi mix'), ('menu_item', 10, 'name', 'en', 'Mixed sushi platter'), ('menu_item', 10, 'name', 'zh', '什锦寿司拼盘'),
    ('menu_item', 11, 'name', 'fr', 'M4'), ('menu_item', 11, 'name', 'en', 'M4'), ('menu_item', 11, 'name', 'zh', 'M4'),
    ('menu_item', 12, 'name', 'fr', 'M5'), ('menu_item', 12, 'name', 'en', 'M5'), ('menu_item', 12, 'name', 'zh', 'M5'),
    ('menu_item', 13, 'name', 'fr', 'Plateau 36'), ('menu_item', 13, 'name', 'en', 'Platter 36'), ('menu_item', 13, 'name', 'zh', '拼盘36'),
    ('menu_item', 14, 'name', 'fr', 'Plateau 38 Pièces'), ('menu_item', 14, 'name', 'en', '38 Piece Platter'), ('menu_item', 14, 'name', 'zh', '拼盘38'),
    ('menu_item', 15, 'name', 'fr', 'Plateau 46'), ('menu_item', 15, 'name', 'en', 'Platter 46'), ('menu_item', 15, 'name', 'zh', '拼盘46'),
    ('menu_item', 16, 'name', 'fr', 'M6'), ('menu_item', 16, 'name', 'en', 'M6'), ('menu_item', 16, 'name', 'zh', 'M6'),
    ('menu_item', 17, 'name', 'fr', 'Plateau 42 Pièces'), ('menu_item', 17, 'name', 'en', '42 Piece Platter'), ('menu_item', 17, 'name', 'zh', '拼盘42'),
    ('menu_item', 18, 'name', 'fr', 'M7'), ('menu_item', 18, 'name', 'en', 'M7'), ('menu_item', 18, 'name', 'zh', 'M7'),
    ('menu_item', 19, 'name', 'fr', 'Plateau 80'), ('menu_item', 19, 'name', 'en', 'Platter 80'), ('menu_item', 19, 'name', 'zh', '拼盘80'),
    ('menu_item', 20, 'name', 'fr', 'Plateau Tokyo'), ('menu_item', 20, 'name', 'en', 'Platter Menu'), ('menu_item', 20, 'name', 'zh', '东京拼盘'),
    -- Cat 2: Menu bento box
    ('menu_item', 21, 'name', 'fr', 'Bento végétarien'), ('menu_item', 21, 'name', 'en', 'Vegetarian bento'), ('menu_item', 21, 'name', 'zh', '素食便当'),
    ('menu_item', 22, 'name', 'fr', 'Bento box 1'), ('menu_item', 22, 'name', 'en', 'Bento box 1'), ('menu_item', 22, 'name', 'zh', '便当1'),
    ('menu_item', 23, 'name', 'fr', 'Bento box 2'), ('menu_item', 23, 'name', 'en', 'Bento box 2'), ('menu_item', 23, 'name', 'zh', '便当2'),
    ('menu_item', 24, 'name', 'fr', 'Bento royal'), ('menu_item', 24, 'name', 'en', 'Royal bento'), ('menu_item', 24, 'name', 'zh', '皇家便当'),
    -- Cat 3: Sushi
    ('menu_item', 25, 'name', 'fr', 'Surimi'), ('menu_item', 25, 'name', 'en', 'Surimi'), ('menu_item', 25, 'name', 'zh', '蟹肉棒'),
    ('menu_item', 26, 'name', 'fr', 'Omelette'), ('menu_item', 26, 'name', 'en', 'Omelette'), ('menu_item', 26, 'name', 'zh', '玉子'),
    ('menu_item', 27, 'name', 'fr', 'Avocat'), ('menu_item', 27, 'name', 'en', 'Avocado'), ('menu_item', 27, 'name', 'zh', '牛油果'),
    ('menu_item', 28, 'name', 'fr', 'Crevette'), ('menu_item', 28, 'name', 'en', 'Shrimp'), ('menu_item', 28, 'name', 'zh', '虾'),
    ('menu_item', 29, 'name', 'fr', 'Saumon'), ('menu_item', 29, 'name', 'en', 'Salmon'), ('menu_item', 29, 'name', 'zh', '三文鱼'),
    ('menu_item', 30, 'name', 'fr', 'Maquereau'), ('menu_item', 30, 'name', 'en', 'Mackerel'), ('menu_item', 30, 'name', 'zh', '鲭鱼'),
    ('menu_item', 31, 'name', 'fr', 'Dorade'), ('menu_item', 31, 'name', 'en', 'Sea bream'), ('menu_item', 31, 'name', 'zh', '鲷鱼'),
    ('menu_item', 32, 'name', 'fr', 'Tofu'), ('menu_item', 32, 'name', 'en', 'Tofu'), ('menu_item', 32, 'name', 'zh', '豆腐'),
    ('menu_item', 33, 'name', 'fr', 'Poulpe'), ('menu_item', 33, 'name', 'en', 'Octopus'), ('menu_item', 33, 'name', 'zh', '章鱼'),
    ('menu_item', 34, 'name', 'fr', 'Thon'), ('menu_item', 34, 'name', 'en', 'Tuna'), ('menu_item', 34, 'name', 'zh', '金枪鱼'),
    ('menu_item', 35, 'name', 'fr', 'Saumon cheese'), ('menu_item', 35, 'name', 'en', 'Salmon cheese'), ('menu_item', 35, 'name', 'zh', '三文鱼芝士'),
    ('menu_item', 36, 'name', 'fr', 'Saumon mi-cuit'), ('menu_item', 36, 'name', 'en', 'Semi-cooked salmon'), ('menu_item', 36, 'name', 'zh', '半熟三文鱼'),
    ('menu_item', 37, 'name', 'fr', 'Saumon cuit caramélisé'), ('menu_item', 37, 'name', 'en', 'Caramelized cooked salmon'), ('menu_item', 37, 'name', 'zh', '焦糖熟三文鱼'),
    ('menu_item', 38, 'name', 'fr', 'Dorade grillée'), ('menu_item', 38, 'name', 'en', 'Grilled sea bream'), ('menu_item', 38, 'name', 'zh', '烤鲷鱼'),
    ('menu_item', 39, 'name', 'fr', 'Saumon avocat'), ('menu_item', 39, 'name', 'en', 'Salmon avocado'), ('menu_item', 39, 'name', 'zh', '三文鱼牛油果'),
    ('menu_item', 40, 'name', 'fr', 'Thon mi-cuit'), ('menu_item', 40, 'name', 'en', 'Semi-cooked tuna'), ('menu_item', 40, 'name', 'zh', '半熟金枪鱼'),
    ('menu_item', 41, 'name', 'fr', 'Anguille'), ('menu_item', 41, 'name', 'en', 'Eel'), ('menu_item', 41, 'name', 'zh', '鳗鱼'),
    ('menu_item', 42, 'name', 'fr', 'Maquereau grillé'), ('menu_item', 42, 'name', 'en', 'Grilled mackerel'), ('menu_item', 42, 'name', 'zh', '烤鲭鱼'),
    ('menu_item', 43, 'name', 'fr', 'Foie gras'), ('menu_item', 43, 'name', 'en', 'Foie gras'), ('menu_item', 43, 'name', 'zh', '鹅肝'),
    ('menu_item', 44, 'name', 'fr', 'Saint-Jacques mi-cuit'), ('menu_item', 44, 'name', 'en', 'Semi-cooked Saint-Jacques'), ('menu_item', 44, 'name', 'zh', '半熟扇贝'),
    -- Cat 4: Maki
    ('menu_item', 45, 'name', 'fr', 'Avocat'), ('menu_item', 45, 'name', 'en', 'Avocado'), ('menu_item', 45, 'name', 'zh', '牛油果'),
    ('menu_item', 46, 'name', 'fr', 'Radis japonais'), ('menu_item', 46, 'name', 'en', 'Japanese radish'), ('menu_item', 46, 'name', 'zh', '日式萝卜'),
    ('menu_item', 47, 'name', 'fr', 'Concombre sésame'), ('menu_item', 47, 'name', 'en', 'Cucumber sesame'), ('menu_item', 47, 'name', 'zh', '芝麻黄瓜'),
    ('menu_item', 48, 'name', 'fr', 'Surimi'), ('menu_item', 48, 'name', 'en', 'Surimi'), ('menu_item', 48, 'name', 'zh', '蟹肉棒'),
    ('menu_item', 49, 'name', 'fr', 'Cheese concombre'), ('menu_item', 49, 'name', 'en', 'Cucumber cheese'), ('menu_item', 49, 'name', 'zh', '黄瓜芝士'),
    ('menu_item', 50, 'name', 'fr', 'Saumon'), ('menu_item', 50, 'name', 'en', 'Salmon'), ('menu_item', 50, 'name', 'zh', '三文鱼'),
    ('menu_item', 51, 'name', 'fr', 'Saumon spicy'), ('menu_item', 51, 'name', 'en', 'Spicy salmon'), ('menu_item', 51, 'name', 'zh', '辣三文鱼'),
    ('menu_item', 52, 'name', 'fr', 'Cheese avocat'), ('menu_item', 52, 'name', 'en', 'Avocado cheese'), ('menu_item', 52, 'name', 'zh', '牛油果芝士'),
    ('menu_item', 53, 'name', 'fr', 'Thon cuit spicy'), ('menu_item', 53, 'name', 'en', 'Spicy cooked tuna'), ('menu_item', 53, 'name', 'zh', '辣熟金枪鱼'),
    ('menu_item', 54, 'name', 'fr', 'Mangue'), ('menu_item', 54, 'name', 'en', 'Mango'), ('menu_item', 54, 'name', 'zh', '芒果'),
    ('menu_item', 55, 'name', 'fr', 'Ciboulette cheese'), ('menu_item', 55, 'name', 'en', 'Chives cheese'), ('menu_item', 55, 'name', 'zh', '香葱芝士'),
    ('menu_item', 56, 'name', 'fr', 'Thon'), ('menu_item', 56, 'name', 'en', 'Tuna'), ('menu_item', 56, 'name', 'zh', '金枪鱼'),
    ('menu_item', 57, 'name', 'fr', 'Dorade mangue'), ('menu_item', 57, 'name', 'en', 'Sea bream mango'), ('menu_item', 57, 'name', 'zh', '鲷鱼芒果'),
    ('menu_item', 58, 'name', 'fr', 'Tempura crevette'), ('menu_item', 58, 'name', 'en', 'Shrimp tempura'), ('menu_item', 58, 'name', 'zh', '天妇罗虾'),
    ('menu_item', 59, 'name', 'fr', 'Anguille'), ('menu_item', 59, 'name', 'en', 'Eel'), ('menu_item', 59, 'name', 'zh', '鳗鱼'),
    ('menu_item', 60, 'name', 'fr', 'Saumon roll cheese'), ('menu_item', 60, 'name', 'en', 'Salmon roll cheese'), ('menu_item', 60, 'name', 'zh', '三文鱼芝士卷'),
    ('menu_item', 61, 'name', 'fr', 'Tartare thon ciboulette'), ('menu_item', 61, 'name', 'en', 'Tuna and chives tartare'), ('menu_item', 61, 'name', 'zh', '金枪鱼香葱他他'),
    ('menu_item', 62, 'name', 'fr', 'Foie gras'), ('menu_item', 62, 'name', 'en', 'Foie gras'), ('menu_item', 62, 'name', 'zh', '鹅肝'),
    -- Cat 5: Gunkan
    ('menu_item', 63, 'name', 'fr', 'Avocat cheese'), ('menu_item', 63, 'name', 'en', 'Cheese avocado'), ('menu_item', 63, 'name', 'zh', '芝士牛油果'),
    ('menu_item', 64, 'name', 'fr', 'Saumon avocat'), ('menu_item', 64, 'name', 'en', 'Salmon avocado'), ('menu_item', 64, 'name', 'zh', '三文鱼牛油果'),
    ('menu_item', 65, 'name', 'fr', 'Tartare saumon cheese'), ('menu_item', 65, 'name', 'en', 'Salmon-cheese tartare'), ('menu_item', 65, 'name', 'zh', '三文鱼芝士他他'),
    ('menu_item', 66, 'name', 'fr', 'Oeufs de poisson'), ('menu_item', 66, 'name', 'en', 'Fish eggs'), ('menu_item', 66, 'name', 'zh', '鱼子'),
    ('menu_item', 67, 'name', 'fr', 'Dorade mangue'), ('menu_item', 67, 'name', 'en', 'Mango bream'), ('menu_item', 67, 'name', 'zh', '芒果鲷鱼'),
    ('menu_item', 68, 'name', 'fr', 'Thon avocat'), ('menu_item', 68, 'name', 'en', 'Tuna avocado'), ('menu_item', 68, 'name', 'zh', '金枪鱼牛油果'),
    ('menu_item', 69, 'name', 'fr', 'Tartare thon ciboulette'), ('menu_item', 69, 'name', 'en', 'Tuna-chives tartare'), ('menu_item', 69, 'name', 'zh', '金枪鱼香葱他他'),
    ('menu_item', 70, 'name', 'fr', 'Oeufs de saumon'), ('menu_item', 70, 'name', 'en', 'Salmon eggs'), ('menu_item', 70, 'name', 'zh', '三文鱼子'),
    -- Cat 6: Spring roll
    ('menu_item', 71, 'name', 'fr', 'Saumon cheese'), ('menu_item', 71, 'name', 'en', 'Salmon cheese'), ('menu_item', 71, 'name', 'zh', '三文鱼芝士'),
    ('menu_item', 72, 'name', 'fr', 'Saumon avocat'), ('menu_item', 72, 'name', 'en', 'Salmon avocado'), ('menu_item', 72, 'name', 'zh', '三文鱼牛油果'),
    ('menu_item', 73, 'name', 'fr', 'Saumon mangue'), ('menu_item', 73, 'name', 'en', 'Salmon mango'), ('menu_item', 73, 'name', 'zh', '三文鱼芒果'),
    ('menu_item', 74, 'name', 'fr', 'Thon avocat'), ('menu_item', 74, 'name', 'en', 'Tuna avocado'), ('menu_item', 74, 'name', 'zh', '金枪鱼牛油果'),
    ('menu_item', 75, 'name', 'fr', 'Poulet pané mayo'), ('menu_item', 75, 'name', 'en', 'Breaded chicken mayo'), ('menu_item', 75, 'name', 'zh', '炸鸡蛋黄酱'),
    ('menu_item', 76, 'name', 'fr', 'Tempura crevette oignons'), ('menu_item', 76, 'name', 'en', 'Tempura shrimp fried onions'), ('menu_item', 76, 'name', 'zh', '天妇罗虾炸洋葱'),
    ('menu_item', 77, 'name', 'fr', 'Poulet mangue menthe'), ('menu_item', 77, 'name', 'en', 'Chicken mango mint'), ('menu_item', 77, 'name', 'zh', '鸡肉芒果薄荷'),
    ('menu_item', 78, 'name', 'fr', 'Saumon fumé cheese'), ('menu_item', 78, 'name', 'en', 'Smoked salmon cheese'), ('menu_item', 78, 'name', 'zh', '烟熏三文鱼芝士香葱'),
    ('menu_item', 79, 'name', 'fr', 'Foie gras mangue'), ('menu_item', 79, 'name', 'en', 'Foie gras mango'), ('menu_item', 79, 'name', 'zh', '鹅肝芒果'),
    -- Cat 7: California roll
    ('menu_item', 80, 'name', 'fr', 'Végétarien'), ('menu_item', 80, 'name', 'en', 'Vegetarian'), ('menu_item', 80, 'name', 'zh', '素食'),
    ('menu_item', 81, 'name', 'fr', 'Saumon avocat'), ('menu_item', 81, 'name', 'en', 'Salmon avocado'), ('menu_item', 81, 'name', 'zh', '三文鱼牛油果'),
    ('menu_item', 82, 'name', 'fr', 'Thon cuit pomme spicy'), ('menu_item', 82, 'name', 'en', 'Cooked tuna spicy apple'), ('menu_item', 82, 'name', 'zh', '辣熟金枪鱼苹果'),
    ('menu_item', 83, 'name', 'fr', 'Saumon mangue'), ('menu_item', 83, 'name', 'en', 'Salmon mango'), ('menu_item', 83, 'name', 'zh', '三文鱼芒果'),
    ('menu_item', 84, 'name', 'fr', 'Saumon cheese'), ('menu_item', 84, 'name', 'en', 'Salmon cheese'), ('menu_item', 84, 'name', 'zh', '三文鱼芝士'),
    ('menu_item', 85, 'name', 'fr', 'Tempura crevette cheese'), ('menu_item', 85, 'name', 'en', 'Tempura shrimp cheese'), ('menu_item', 85, 'name', 'zh', '天妇罗虾芝士'),
    ('menu_item', 86, 'name', 'fr', 'Tempura crevette avocat'), ('menu_item', 86, 'name', 'en', 'Tempura shrimp avocado'), ('menu_item', 86, 'name', 'zh', '天妇罗虾牛油果'),
    ('menu_item', 87, 'name', 'fr', 'Surimi avocat'), ('menu_item', 87, 'name', 'en', 'Surimi avocado'), ('menu_item', 87, 'name', 'zh', '蟹肉棒牛油果'),
    ('menu_item', 88, 'name', 'fr', 'Crevette avocat cheese'), ('menu_item', 88, 'name', 'en', 'Shrimp avocado cheese'), ('menu_item', 88, 'name', 'zh', '虾牛油果芝士'),
    ('menu_item', 89, 'name', 'fr', 'Thon avocat'), ('menu_item', 89, 'name', 'en', 'Tuna avocado'), ('menu_item', 89, 'name', 'zh', '金枪鱼牛油果'),
    ('menu_item', 90, 'name', 'fr', 'Oignons frits poulet'), ('menu_item', 90, 'name', 'en', 'Fried onions chicken'), ('menu_item', 90, 'name', 'zh', '炸洋葱鸡肉'),
    ('menu_item', 91, 'name', 'fr', 'Poulet frit roquette miel'), ('menu_item', 91, 'name', 'en', 'Fried chicken salad honey'), ('menu_item', 91, 'name', 'zh', '炸鸡沙拉蜂蜜'),
    ('menu_item', 92, 'name', 'fr', 'Tempura crevette fraise'), ('menu_item', 92, 'name', 'en', 'Tempura shrimp strawberry'), ('menu_item', 92, 'name', 'zh', '天妇罗虾牛油果芝士草莓'),
    ('menu_item', 93, 'name', 'fr', 'Poulet piquant'), ('menu_item', 93, 'name', 'en', 'Spicy chicken'), ('menu_item', 93, 'name', 'zh', '辣鸡肉'),
    ('menu_item', 94, 'name', 'fr', 'Tempura crevette coriandre'), ('menu_item', 94, 'name', 'en', 'Tempura shrimp coriander'), ('menu_item', 94, 'name', 'zh', '辣天妇罗虾香菜'),
    ('menu_item', 95, 'name', 'fr', 'Foie gras miel'), ('menu_item', 95, 'name', 'en', 'Foie gras honey'), ('menu_item', 95, 'name', 'zh', '炸洋葱鹅肝蜂蜜'),
    -- Cat 8: Temaki
    ('menu_item', 96, 'name', 'fr', 'Saumon avocat concombre'), ('menu_item', 96, 'name', 'en', 'Salmon avocado cucumber'), ('menu_item', 96, 'name', 'zh', '三文鱼牛油果黄瓜'),
    ('menu_item', 97, 'name', 'fr', 'Tempura crevette avocat'), ('menu_item', 97, 'name', 'en', 'Shrimp tempura avocado'), ('menu_item', 97, 'name', 'zh', '天妇罗虾牛油果黄瓜'),
    ('menu_item', 98, 'name', 'fr', 'Thon avocat concombre'), ('menu_item', 98, 'name', 'en', 'Tuna avocado cucumber'), ('menu_item', 98, 'name', 'zh', '金枪鱼牛油果黄瓜'),
    ('menu_item', 99, 'name', 'fr', 'Oeufs de saumon avocat'), ('menu_item', 99, 'name', 'en', 'Salmon eggs avocado'), ('menu_item', 99, 'name', 'zh', '三文鱼子牛油果黄瓜'),
    -- Cat 9: Masago roll
    ('menu_item', 100, 'name', 'fr', 'Tempura crevette'), ('menu_item', 100, 'name', 'en', 'Shrimp tempura'), ('menu_item', 100, 'name', 'zh', '天妇罗虾'),
    ('menu_item', 101, 'name', 'fr', 'Saumon avocat'), ('menu_item', 101, 'name', 'en', 'Salmon avocado'), ('menu_item', 101, 'name', 'zh', '三文鱼牛油果'),
    ('menu_item', 102, 'name', 'fr', 'Thon mangue'), ('menu_item', 102, 'name', 'en', 'Tuna mango'), ('menu_item', 102, 'name', 'zh', '金枪鱼芒果'),
    -- Cat 10: Spécial roll
    ('menu_item', 103, 'name', 'fr', 'Oignon rolls'), ('menu_item', 103, 'name', 'en', 'Onion rolls'), ('menu_item', 103, 'name', 'zh', '洋葱卷'),
    ('menu_item', 104, 'name', 'fr', 'Spicy poulet'), ('menu_item', 104, 'name', 'en', 'Spicy chicken'), ('menu_item', 104, 'name', 'zh', '辣鸡肉'),
    ('menu_item', 105, 'name', 'fr', 'Miel rolls'), ('menu_item', 105, 'name', 'en', 'Honey rolls'), ('menu_item', 105, 'name', 'zh', '蜂蜜卷'),
    ('menu_item', 106, 'name', 'fr', 'Chips rolls'), ('menu_item', 106, 'name', 'en', 'Chips rolls'), ('menu_item', 106, 'name', 'zh', '薯片卷'),
    ('menu_item', 107, 'name', 'fr', 'Saumon royal'), ('menu_item', 107, 'name', 'en', 'Royal salmon'), ('menu_item', 107, 'name', 'zh', '皇家三文鱼'),
    ('menu_item', 108, 'name', 'fr', 'Mangue rolls'), ('menu_item', 108, 'name', 'en', 'Mango rolls'), ('menu_item', 108, 'name', 'zh', '芒果卷'),
    ('menu_item', 109, 'name', 'fr', 'Délice mangue'), ('menu_item', 109, 'name', 'en', 'Mango delight'), ('menu_item', 109, 'name', 'zh', '芒果乐'),
    ('menu_item', 110, 'name', 'fr', 'Crispy saumon'), ('menu_item', 110, 'name', 'en', 'Crispy salmon'), ('menu_item', 110, 'name', 'zh', '脆三文鱼'),
    ('menu_item', 111, 'name', 'fr', 'Anguille rolls'), ('menu_item', 111, 'name', 'en', 'Eel rolls'), ('menu_item', 111, 'name', 'zh', '鳗鱼卷'),
    ('menu_item', 112, 'name', 'fr', 'Avocat rolls'), ('menu_item', 112, 'name', 'en', 'Avocado rolls'), ('menu_item', 112, 'name', 'zh', '牛油果卷'),
    ('menu_item', 113, 'name', 'fr', 'Assortiment rolls'), ('menu_item', 113, 'name', 'en', 'Mixed rolls'), ('menu_item', 113, 'name', 'zh', '什锦卷'),
    ('menu_item', 114, 'name', 'fr', 'Spicy saumon'), ('menu_item', 114, 'name', 'en', 'Spicy salmon'), ('menu_item', 114, 'name', 'zh', '辣三文鱼'),
    ('menu_item', 115, 'name', 'fr', 'Osaka rolls'), ('menu_item', 115, 'name', 'en', 'Osaka rolls'), ('menu_item', 115, 'name', 'zh', '大阪卷'),
    ('menu_item', 116, 'name', 'fr', 'Foie gras fraise'), ('menu_item', 116, 'name', 'en', 'Foie gras strawberry'), ('menu_item', 116, 'name', 'zh', '鹅肝草莓卷'),
    -- Cat 11: Chirashi
    ('menu_item', 117, 'name', 'fr', 'Chirashi saumon avocat'), ('menu_item', 117, 'name', 'en', 'Salmon avocado chirashi'), ('menu_item', 117, 'name', 'zh', '三文鱼牛油果切片'),
    ('menu_item', 118, 'name', 'fr', 'Chirashi thon avocat'), ('menu_item', 118, 'name', 'en', 'Tuna avocado chirashi'), ('menu_item', 118, 'name', 'zh', '金枪鱼牛油果切片'),
    ('menu_item', 119, 'name', 'fr', 'Chirashi assortiment'), ('menu_item', 119, 'name', 'en', 'Mixed chirashi'), ('menu_item', 119, 'name', 'zh', '什锦'),
    -- Cat 12: Sashimi
    ('menu_item', 120, 'name', 'fr', 'Sashimi saumon'), ('menu_item', 120, 'name', 'en', 'Salmon sashimi'), ('menu_item', 120, 'name', 'zh', '三文鱼'),
    ('menu_item', 121, 'name', 'fr', 'Sashimi thon'), ('menu_item', 121, 'name', 'en', 'Tuna sashimi'), ('menu_item', 121, 'name', 'zh', '金枪鱼'),
    ('menu_item', 122, 'name', 'fr', 'Sashimi saumon thon'), ('menu_item', 122, 'name', 'en', 'Salmon tuna sashimi'), ('menu_item', 122, 'name', 'zh', '三文鱼金枪鱼'),
    ('menu_item', 123, 'name', 'fr', 'Sashimi assortiment'), ('menu_item', 123, 'name', 'en', 'Mixed sashimi'), ('menu_item', 123, 'name', 'zh', '什锦'),
    -- Cat 13: Poke bowl
    ('menu_item', 124, 'name', 'fr', 'Poke végétarien tofu'), ('menu_item', 124, 'name', 'en', 'Vegetarian tofu poke'), ('menu_item', 124, 'name', 'zh', '素食豆腐'),
    ('menu_item', 125, 'name', 'fr', 'Poke gyoza poulet frit'), ('menu_item', 125, 'name', 'en', 'Gyoza fried chicken poke'), ('menu_item', 125, 'name', 'zh', '煎饺炸鸡'),
    ('menu_item', 126, 'name', 'fr', 'Poke poulet'), ('menu_item', 126, 'name', 'en', 'Chicken poke'), ('menu_item', 126, 'name', 'zh', '鸡肉'),
    ('menu_item', 127, 'name', 'fr', 'Poke saumon'), ('menu_item', 127, 'name', 'en', 'Salmon poke'), ('menu_item', 127, 'name', 'zh', '三文鱼'),
    ('menu_item', 128, 'name', 'fr', 'Poke thon'), ('menu_item', 128, 'name', 'en', 'Tuna poke'), ('menu_item', 128, 'name', 'zh', '金枪鱼'),
    -- Cat 14: Tokyo hot
    ('menu_item', 129, 'name', 'fr', 'Soupe miso'), ('menu_item', 129, 'name', 'en', 'Miso soup'), ('menu_item', 129, 'name', 'zh', '味噌汤'),
    ('menu_item', 130, 'name', 'fr', 'Soupe nouilles poulet'), ('menu_item', 130, 'name', 'en', 'Chicken Noodle Soup'), ('menu_item', 130, 'name', 'zh', '鸡肉面汤'),
    ('menu_item', 131, 'name', 'fr', 'Soupe piquante'), ('menu_item', 131, 'name', 'en', 'Spicy soup'), ('menu_item', 131, 'name', 'zh', '辣汤'),
    ('menu_item', 132, 'name', 'fr', 'Brochette poulet x2'), ('menu_item', 132, 'name', 'en', 'Chicken skewer x2'), ('menu_item', 132, 'name', 'zh', '鸡肉串'),
    ('menu_item', 133, 'name', 'fr', 'Brochette tofu x2'), ('menu_item', 133, 'name', 'en', 'Tofu skewers x2'), ('menu_item', 133, 'name', 'zh', '豆腐串'),
    ('menu_item', 134, 'name', 'fr', 'Tempura scampis x3'), ('menu_item', 134, 'name', 'en', 'Tempura scampi x3'), ('menu_item', 134, 'name', 'zh', '天妇罗虾'),
    ('menu_item', 135, 'name', 'fr', 'Boulettes poulpe x4'), ('menu_item', 135, 'name', 'en', 'Octopus balls x4'), ('menu_item', 135, 'name', 'zh', '章鱼小丸子'),
    ('menu_item', 136, 'name', 'fr', 'Raviolis poulet x6'), ('menu_item', 136, 'name', 'en', 'Chicken dumplings x6'), ('menu_item', 136, 'name', 'zh', '日式鸡肉饺子'),
    ('menu_item', 137, 'name', 'fr', 'Raviolis scampis vapeur x5'), ('menu_item', 137, 'name', 'en', 'Steamed scampi ravioli x5'), ('menu_item', 137, 'name', 'zh', '蒸虾饺'),
    ('menu_item', 138, 'name', 'fr', 'Brochette boeuf fromage x3'), ('menu_item', 138, 'name', 'en', 'Beef cheese skewer x3'), ('menu_item', 138, 'name', 'zh', '芝士牛肉串'),
    ('menu_item', 139, 'name', 'fr', 'Raviolis crevette x6'), ('menu_item', 139, 'name', 'en', 'Shrimp dumplings x6'), ('menu_item', 139, 'name', 'zh', '日式虾饺'),
    ('menu_item', 140, 'name', 'fr', 'Katsu poulet x6'), ('menu_item', 140, 'name', 'en', 'Chicken katsu x6'), ('menu_item', 140, 'name', 'zh', '香辣蛋黄酱炸鸡排'),
    ('menu_item', 141, 'name', 'fr', 'Tataki saumon x6'), ('menu_item', 141, 'name', 'en', 'Salmon tataki x6'), ('menu_item', 141, 'name', 'zh', '三文鱼他他'),
    ('menu_item', 142, 'name', 'fr', 'Nouilles sautées légumes'), ('menu_item', 142, 'name', 'en', 'Stir-fried veggie noodles'), ('menu_item', 142, 'name', 'zh', '蔬菜炒面'),
    ('menu_item', 143, 'name', 'fr', 'Udon poulet croustillant'), ('menu_item', 143, 'name', 'en', 'Crispy chicken udon'), ('menu_item', 143, 'name', 'zh', '脆鸡炒乌冬面'),
    ('menu_item', 144, 'name', 'fr', 'Nouilles sautées scampis'), ('menu_item', 144, 'name', 'en', 'Stir-fried scampi noodles'), ('menu_item', 144, 'name', 'zh', '虾炒面'),
    ('menu_item', 145, 'name', 'fr', 'Katsu poulet curry'), ('menu_item', 145, 'name', 'en', 'Chicken katsu curry'), ('menu_item', 145, 'name', 'zh', '日式炸鸡咖喱'),
    -- Cat 15: Accompagnement
    ('menu_item', 146, 'name', 'fr', 'Riz blanc chaud'), ('menu_item', 146, 'name', 'en', 'Hot white rice'), ('menu_item', 146, 'name', 'zh', '热白饭'),
    ('menu_item', 147, 'name', 'fr', 'Chou vinaigré'), ('menu_item', 147, 'name', 'en', 'Pickled cabbage'), ('menu_item', 147, 'name', 'zh', '腌卷心菜'),
    ('menu_item', 148, 'name', 'fr', 'Edamame'), ('menu_item', 148, 'name', 'en', 'Edamame'), ('menu_item', 148, 'name', 'zh', '毛豆'),
    ('menu_item', 149, 'name', 'fr', 'Riz vinaigré'), ('menu_item', 149, 'name', 'en', 'Vinegar rice'), ('menu_item', 149, 'name', 'zh', '醋饭'),
    ('menu_item', 150, 'name', 'fr', 'Kimchi carottes maison'), ('menu_item', 150, 'name', 'en', 'Homemade carrot kimchi'), ('menu_item', 150, 'name', 'zh', '自制胡萝卜泡菜'),
    ('menu_item', 151, 'name', 'fr', 'Wakame'), ('menu_item', 151, 'name', 'en', 'Wakame'), ('menu_item', 151, 'name', 'zh', '裙带菜'),
    -- Cat 16: Boisson (TVA A = 21%)
    ('menu_item', 152, 'name', 'fr', 'Coca-Cola'), ('menu_item', 152, 'name', 'en', 'Coca-Cola'), ('menu_item', 152, 'name', 'zh', '可口可乐'),
    ('menu_item', 153, 'name', 'fr', 'Coca-Cola zéro'), ('menu_item', 153, 'name', 'en', 'Coca-Cola Zero'), ('menu_item', 153, 'name', 'zh', '零度可乐'),
    ('menu_item', 154, 'name', 'fr', 'Thé glacé melon'), ('menu_item', 154, 'name', 'en', 'Melon Iced Tea'), ('menu_item', 154, 'name', 'zh', '蜜瓜冰茶'),
    ('menu_item', 155, 'name', 'fr', 'Bière Tsingtao'), ('menu_item', 155, 'name', 'en', 'Tsingtao Beer'), ('menu_item', 155, 'name', 'zh', '青岛啤酒'),
    ('menu_item', 156, 'name', 'fr', 'Bière Kirin'), ('menu_item', 156, 'name', 'en', 'Kirin Beer'), ('menu_item', 156, 'name', 'zh', '麒麟啤酒'),
    ('menu_item', 157, 'name', 'fr', 'Bière Sapporo'), ('menu_item', 157, 'name', 'en', 'Sapporo Beer'), ('menu_item', 157, 'name', 'zh', '札幌啤酒'),
    ('menu_item', 158, 'name', 'fr', 'Thé glacé litchi'), ('menu_item', 158, 'name', 'en', 'Lychee Iced Tea'), ('menu_item', 158, 'name', 'zh', '荔枝冰茶'),
    ('menu_item', 159, 'name', 'fr', 'Bière Iki Yuzu'), ('menu_item', 159, 'name', 'en', 'Iki Yuzu Beer'), ('menu_item', 159, 'name', 'zh', '柚子啤酒'),
    ('menu_item', 160, 'name', 'fr', 'Bière Asahi'), ('menu_item', 160, 'name', 'en', 'Asahi Beer'), ('menu_item', 160, 'name', 'zh', '朝日啤酒'),
    -- Cat 17: Dessert
    ('menu_item', 161, 'name', 'fr', 'Cheesecake Yuzu'), ('menu_item', 161, 'name', 'en', 'Yuzu Cheesecake'), ('menu_item', 161, 'name', 'zh', '柚子芝士蛋糕'),
    ('menu_item', 162, 'name', 'fr', 'Cheesecake matcha'), ('menu_item', 162, 'name', 'en', 'Matcha Cheesecake'), ('menu_item', 162, 'name', 'zh', '抹茶芝士蛋糕'),
    ('menu_item', 163, 'name', 'fr', 'Boule sésame haricots'), ('menu_item', 163, 'name', 'en', 'Sesame ball red beans'), ('menu_item', 163, 'name', 'zh', '红豆芝麻球'),
    ('menu_item', 164, 'name', 'fr', 'Mochi glacé x3'), ('menu_item', 164, 'name', 'en', 'Frozen mochi x3'), ('menu_item', 164, 'name', 'zh', '冰麻糬');

-- Drinks: beverage VAT category, no kitchen ticket.
UPDATE menu_items SET vat_category = 'beverage', kitchen_printer = 0 WHERE category_id = 16;

-- Fiscal counters from zero for the demo till.
DELETE FROM fiscal_counters;
INSERT INTO fiscal_counters (event_label, current_value) VALUES
    ('N', 0), ('P', 0), ('T', 0), ('C', 0),
    ('I', 0), ('F', 0), ('S', 0), ('R', 0);
-- Takeaway VAT labels: food 6 % (C) instead of 12 % (B) on site; soft drinks 6 % (C)
-- instead of 21 % (A); beers stay 21 % (A).
UPDATE menu_items SET vat_code_takeaway = 'C' WHERE category_id <> 16;
UPDATE menu_items SET vat_code_takeaway = 'A' WHERE category_id = 16;
UPDATE menu_items SET vat_code_takeaway = 'C'
WHERE category_id = 16 AND id IN (
    SELECT entity_id FROM translations
    WHERE entity_type = 'menu_item' AND field = 'name' AND lang = 'fr'
      AND (value LIKE 'Coca%' OR value LIKE 'Th%'));

-- Demo menu codes: one letter per category (A = first category), the product's place
-- in it as the number (Sushi saumon = A1). The real codes come from the catalog in
-- production (HubRise SKU custom_fields.code).
UPDATE menu_items SET code = (
    SELECT char(64 + c.sort_order) FROM categories c WHERE c.id = menu_items.category_id
) || sort_order;
