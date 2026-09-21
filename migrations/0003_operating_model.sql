ALTER TABLE units ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE leases ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE documents ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE automation_rules ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

CREATE INDEX IF NOT EXISTS idx_units_property ON units(property_id, active, code);
CREATE INDEX IF NOT EXISTS idx_leases_active ON leases(active, start_date, end_date);
CREATE INDEX IF NOT EXISTS idx_invoices_status_due ON invoices(sci_id, status, due_date);
CREATE INDEX IF NOT EXISTS idx_documents_expiry ON documents(sci_id, expires_at);
CREATE INDEX IF NOT EXISTS idx_deadlines_status_date ON tax_deadlines(sci_id, status, deadline_date);
CREATE INDEX IF NOT EXISTS idx_bank_reconciliation ON bank_transactions(sci_id, reconciliation_status, booked_at);
