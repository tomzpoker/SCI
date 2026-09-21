ALTER TABLE scis
  ADD COLUMN IF NOT EXISTS onboarding_completed_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_associates_sci_active ON associates(sci_id, active);
CREATE INDEX IF NOT EXISTS idx_properties_sci_active ON properties(sci_id, active);
CREATE INDEX IF NOT EXISTS idx_tenants_sci_active ON tenants(sci_id, active);
CREATE INDEX IF NOT EXISTS idx_units_property_active ON units(property_id, active);

INSERT INTO app_settings (sci_id, key, value)
VALUES
  ('00000000-0000-0000-0000-000000000010', 'onboarding.version', '1'::jsonb),
  ('00000000-0000-0000-0000-000000000010', 'onboarding.mode', '"SCI_IR_TVA_COLLECTION"'::jsonb)
ON CONFLICT (sci_id, key) DO NOTHING;
