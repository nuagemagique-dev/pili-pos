-- Seed for the online-order suite (e2e/real-binary/hubrise.test.mjs):
-- a manager with a PIN (1234), who connects HubRise, takes over the
-- menu and confirms refunds. The menu itself comes from HubRise.
INSERT OR IGNORE INTO users (insz, name, role, pin_hash, active) VALUES
  ('94060242576', 'Gerant E2E', 'admin', '$2y$10$B.uGSApY8BU1alj3OYiFBeu3yXs8YO3e6/MtGAg7DFTX93jdKUpj2', 1);
