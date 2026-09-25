-- Seed data for Layer 3 integration tests.
-- Applied to a fresh SQLite DB before starting the real Tauri binary.
-- The schema is created by run_migrations() at POS boot.

-- Test operator (no PIN for CI speed - PIN tests are covered in Layer 2).
INSERT OR IGNORE INTO users (insz, name, role, active)
VALUES ('00000000029', 'CI Robot', 'admin', 1);

-- Two categories
INSERT OR IGNORE INTO categories (id, sort_order, color, active)
VALUES (1, 0, '#e94560', 1), (2, 1, '#9b59b6', 1);

INSERT OR IGNORE INTO translations (entity_type, entity_id, field, lang, value) VALUES
  ('category', 1, 'name', 'fr', 'Sushi'),
  ('category', 1, 'name', 'en', 'Sushi'),
  ('category', 1, 'name', 'zh', '寿司'),
  ('category', 2, 'name', 'fr', 'Boissons'),
  ('category', 2, 'name', 'en', 'Drinks'),
  ('category', 2, 'name', 'zh', '饮料');

-- Three menu items
INSERT OR IGNORE INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active)
VALUES
  (1, 1, 1200, 'C', 1, 0, 1),
  (2, 1, 1300, 'C', 1, 1, 1),
  (3, 2, 300,  'A', 0, 0, 1);

INSERT OR IGNORE INTO translations (entity_type, entity_id, field, lang, value) VALUES
  ('menu_item', 1, 'name', 'fr', 'Maki saumon x8'),
  ('menu_item', 1, 'name', 'en', 'Salmon Roll x8'),
  ('menu_item', 1, 'name', 'zh', '三文鱼卷 ×8'),
  ('menu_item', 2, 'name', 'fr', 'Maki thon x8'),
  ('menu_item', 2, 'name', 'en', 'Tuna Roll x8'),
  ('menu_item', 2, 'name', 'zh', '金枪鱼卷 ×8'),
  ('menu_item', 3, 'name', 'fr', 'Coca-Cola 33cl'),
  ('menu_item', 3, 'name', 'en', 'Coca-Cola 33cl'),
  ('menu_item', 3, 'name', 'zh', '可口可乐 33cl');
