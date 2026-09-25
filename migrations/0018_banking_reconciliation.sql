-- S08 — BANQUE & RAPPROCHEMENT
-- Migration additive, idempotente et non destructive.

CREATE TABLE IF NOT EXISTS bank_import_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    bank_account_id UUID,
    source_type TEXT NOT NULL CHECK (source_type IN ('CSV','OFX','PDF_OCR','MANUAL')),
    source_name TEXT NOT NULL DEFAULT '',
    imported_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    rows_seen INTEGER NOT NULL DEFAULT 0,
    rows_imported INTEGER NOT NULL DEFAULT 0,
    rows_skipped INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'COMPLETED' CHECK (status IN ('RUNNING','COMPLETED','FAILED','PARTIAL')),
    error_message TEXT NOT NULL DEFAULT '',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);
CREATE INDEX IF NOT EXISTS idx_bank_import_batches_entity ON bank_import_batches(legal_entity_id,imported_at DESC);

CREATE TABLE IF NOT EXISTS bank_account_profiles (
    id UUID PRIMARY KEY REFERENCES legal_entity_bank_accounts(id) ON DELETE RESTRICT,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    account_type TEXT NOT NULL DEFAULT 'CURRENT',
    currency_code TEXT NOT NULL DEFAULT 'EUR',
    opening_balance_cents BIGINT NOT NULL DEFAULT 0,
    opening_balance_date DATE,
    bank_name TEXT NOT NULL DEFAULT '',
    account_holder TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT true,
    notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(account_type) <> '')
);
CREATE INDEX IF NOT EXISTS idx_bank_account_profiles_entity ON bank_account_profiles(legal_entity_id,active);

ALTER TABLE bank_import_batches
    ADD COLUMN IF NOT EXISTS bank_account_id UUID;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_bank_import_batch_account_s08') THEN
        ALTER TABLE bank_import_batches ADD CONSTRAINT fk_bank_import_batch_account_s08
            FOREIGN KEY (bank_account_id) REFERENCES legal_entity_bank_accounts(id) ON DELETE RESTRICT;
    END IF;
END $$;

ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS bank_account_id UUID;
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS transaction_date DATE;
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS balance_cents BIGINT;
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS reference TEXT NOT NULL DEFAULT '';
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'MANUAL';
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS transaction_hash TEXT NOT NULL DEFAULT '';
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS debit_credit TEXT NOT NULL DEFAULT 'CREDIT';
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS import_batch_id UUID;
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS raw_payload JSONB NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS reconciliation_level TEXT NOT NULL DEFAULT 'UNMATCHED';
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS reconciliation_type TEXT NOT NULL DEFAULT 'UNKNOWN';
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS reconciliation_confidence_bp INTEGER NOT NULL DEFAULT 0;
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS reconciled_at TIMESTAMPTZ;

UPDATE bank_transactions
SET transaction_date=COALESCE(transaction_date,booked_at::date),
    debit_credit=CASE WHEN amount_cents < 0 THEN 'DEBIT' ELSE 'CREDIT' END,
    source=CASE WHEN btrim(source)='' THEN 'MANUAL' ELSE source END,
    reconciliation_level=CASE reconciliation_status
        WHEN 'MATCHED' THEN 'MATCHED'
        WHEN 'PROBABLE' THEN 'PROBABLE'
        WHEN 'TO_VALIDATE' THEN 'TO_VALIDATE'
        WHEN 'ANOMALY' THEN 'ANOMALY'
        ELSE 'UNMATCHED' END
WHERE transaction_date IS NULL OR btrim(source)='' OR debit_credit IS NULL OR reconciliation_level='UNMATCHED';

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_bank_transactions_account_s08') THEN
        ALTER TABLE bank_transactions ADD CONSTRAINT fk_bank_transactions_account_s08
            FOREIGN KEY (bank_account_id) REFERENCES legal_entity_bank_accounts(id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_bank_transactions_batch_s08') THEN
        ALTER TABLE bank_transactions ADD CONSTRAINT fk_bank_transactions_batch_s08
            FOREIGN KEY (import_batch_id) REFERENCES bank_import_batches(id) ON DELETE SET NULL;
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='chk_bank_tx_debit_credit_s08') THEN
        ALTER TABLE bank_transactions ADD CONSTRAINT chk_bank_tx_debit_credit_s08
            CHECK (debit_credit IN ('DEBIT','CREDIT'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='chk_bank_tx_reconciliation_level_s08') THEN
        ALTER TABLE bank_transactions ADD CONSTRAINT chk_bank_tx_reconciliation_level_s08
            CHECK (reconciliation_level IN ('MATCHED','PROBABLE','TO_VALIDATE','UNMATCHED','ANOMALY'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='chk_bank_tx_confidence_s08') THEN
        ALTER TABLE bank_transactions ADD CONSTRAINT chk_bank_tx_confidence_s08
            CHECK (reconciliation_confidence_bp BETWEEN 0 AND 10000);
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_bank_tx_account_s08 ON bank_transactions(legal_entity_id,bank_account_id,booked_at DESC);
CREATE INDEX IF NOT EXISTS idx_bank_tx_hash_s08 ON bank_transactions(legal_entity_id,transaction_hash);
CREATE INDEX IF NOT EXISTS idx_bank_tx_reconciliation_s08 ON bank_transactions(legal_entity_id,reconciliation_level,booked_at DESC);
CREATE INDEX IF NOT EXISTS idx_bank_tx_source_s08 ON bank_transactions(legal_entity_id,source,booked_at DESC);

CREATE TABLE IF NOT EXISTS bank_reconciliation_matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    bank_transaction_id UUID NOT NULL,
    target_type TEXT NOT NULL CHECK (target_type IN ('INVOICE','PAYMENT','LEASE','DEPOSIT','INTERNAL_TRANSFER','FEE','REFUND','IMPAYE','UNKNOWN')),
    target_id UUID,
    invoice_id UUID,
    payment_id UUID,
    matched_amount_cents BIGINT NOT NULL,
    match_type TEXT NOT NULL,
    confidence_bp INTEGER NOT NULL DEFAULT 0 CHECK (confidence_bp BETWEEN 0 AND 10000),
    status TEXT NOT NULL DEFAULT 'PROPOSED' CHECK (status IN ('PROPOSED','CONFIRMED','REJECTED')),
    reason TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    confirmed_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_bank_matches_tx_s08 ON bank_reconciliation_matches(legal_entity_id,bank_transaction_id,created_at DESC);
CREATE INDEX IF NOT EXISTS idx_bank_matches_invoice_s08 ON bank_reconciliation_matches(legal_entity_id,invoice_id,status);
CREATE UNIQUE INDEX IF NOT EXISTS uq_payments_id_legal_entity
    ON payments(id,legal_entity_id);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_bank_matches_tx_s08') THEN
        ALTER TABLE bank_reconciliation_matches ADD CONSTRAINT fk_bank_matches_tx_s08
            FOREIGN KEY (bank_transaction_id) REFERENCES bank_transactions(id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_bank_matches_invoice_s08') THEN
        ALTER TABLE bank_reconciliation_matches ADD CONSTRAINT fk_bank_matches_invoice_s08
            FOREIGN KEY (invoice_id, legal_entity_id) REFERENCES invoices(id,legal_entity_id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_bank_matches_payment_s08') THEN
        ALTER TABLE bank_reconciliation_matches ADD CONSTRAINT fk_bank_matches_payment_s08
            FOREIGN KEY (payment_id, legal_entity_id) REFERENCES payments(id,legal_entity_id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS bank_reconciliation_events (
    id BIGSERIAL PRIMARY KEY,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    bank_transaction_id UUID NOT NULL,
    from_level TEXT,
    to_level TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    actor TEXT NOT NULL DEFAULT 'SYSTEM',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_bank_reconciliation_events_s08 ON bank_reconciliation_events(legal_entity_id,bank_transaction_id,created_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS uq_bank_tx_external_s08
    ON bank_transactions(legal_entity_id,bank_account_id,external_id)
    WHERE bank_account_id IS NOT NULL AND external_id IS NOT NULL AND btrim(external_id)<>'';

-- Compatibilité : les comptes existants disposent désormais d'un profil métier minimal.
INSERT INTO bank_account_profiles(id,legal_entity_id)
SELECT a.id,a.legal_entity_id
FROM legal_entity_bank_accounts a
WHERE NOT EXISTS (SELECT 1 FROM bank_account_profiles p WHERE p.id=a.id);

COMMENT ON TABLE bank_reconciliation_matches IS 'Rapprochement explicite et traçable, y compris paiements partiels et multiples.';
COMMENT ON COLUMN bank_transactions.transaction_hash IS 'Empreinte stable du mouvement source pour détecter les doublons d import.';
COMMENT ON COLUMN bank_transactions.reconciliation_level IS 'MATCHED / PROBABLE / TO_VALIDATE / UNMATCHED / ANOMALY.';
