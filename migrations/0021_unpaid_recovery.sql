-- S11 / IMPAYES & RECOUVREMENT — detection, historique, promesses et actions.
-- Additif, non destructif. Toute action juridique garde une validation humaine explicite.

CREATE TABLE IF NOT EXISTS arrears_cases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    lease_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    invoice_id UUID,
    detected_at DATE NOT NULL DEFAULT CURRENT_DATE,
    due_date DATE NOT NULL,
    expected_cents BIGINT NOT NULL DEFAULT 0,
    paid_cents BIGINT NOT NULL DEFAULT 0,
    outstanding_cents BIGINT NOT NULL DEFAULT 0,
    delay_days INTEGER NOT NULL DEFAULT 0,
    detection_type TEXT NOT NULL CHECK (detection_type IN ('ABSENCE','RETARD','PARTIAL','UNDERPAYMENT','RECURRENCE')),
    reason TEXT NOT NULL DEFAULT '',
    recurrence_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'OPEN' CHECK (status IN ('OPEN','PROMISED','PARTIAL','RESOLVED','DISPUTED','LEGAL_PREPARATION','CLOSED')),
    last_checked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id,invoice_id,detection_type)
);
CREATE INDEX IF NOT EXISTS idx_arrears_scope_s11 ON arrears_cases(legal_entity_id,status,detected_at DESC,due_date);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_arrears_lease_entity_s11') THEN
        ALTER TABLE arrears_cases ADD CONSTRAINT fk_arrears_lease_entity_s11 FOREIGN KEY(lease_id,legal_entity_id) REFERENCES leases(id,legal_entity_id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_arrears_invoice_entity_s11') THEN
        ALTER TABLE arrears_cases ADD CONSTRAINT fk_arrears_invoice_entity_s11 FOREIGN KEY(invoice_id,legal_entity_id) REFERENCES invoices(id,legal_entity_id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS arrears_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    arrears_case_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    event_date DATE NOT NULL DEFAULT CURRENT_DATE,
    amount_cents BIGINT,
    actor TEXT NOT NULL DEFAULT 'SYSTEM',
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_arrears_events_scope_s11 ON arrears_events(legal_entity_id,arrears_case_id,event_date DESC,created_at DESC);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_arrears_events_case_entity_s11') THEN
        ALTER TABLE arrears_events ADD CONSTRAINT fk_arrears_events_case_entity_s11 FOREIGN KEY(arrears_case_id) REFERENCES arrears_cases(id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS payment_promises (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    arrears_case_id UUID,
    tenant_id UUID NOT NULL,
    lease_id UUID NOT NULL,
    promised_date DATE NOT NULL,
    promised_amount_cents BIGINT NOT NULL CHECK (promised_amount_cents > 0),
    state TEXT NOT NULL DEFAULT 'PENDING' CHECK (state IN ('PENDING','HELD','PARTIALLY_HELD','BROKEN','CANCELLED')),
    source TEXT NOT NULL DEFAULT 'MANAGER',
    response_note TEXT NOT NULL DEFAULT '',
    checked_at TIMESTAMPTZ,
    checked_paid_cents BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_payment_promises_scope_s11 ON payment_promises(legal_entity_id,promised_date,state);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_payment_promise_case_entity_s11') THEN
        ALTER TABLE payment_promises ADD CONSTRAINT fk_payment_promise_case_entity_s11 FOREIGN KEY(arrears_case_id) REFERENCES arrears_cases(id) ON DELETE SET NULL;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_payment_promise_lease_entity_s11') THEN
        ALTER TABLE payment_promises ADD CONSTRAINT fk_payment_promise_lease_entity_s11 FOREIGN KEY(lease_id,legal_entity_id) REFERENCES leases(id,legal_entity_id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS collection_actions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    arrears_case_id UUID NOT NULL,
    action_level TEXT NOT NULL CHECK (action_level IN ('AMIABLE_REMINDER','REMINDER','FORMAL_NOTICE_PREPARATION','CASE_PREPARATION','COURT_HUISSIER_PREPARATION','FOLLOW_UP')),
    status TEXT NOT NULL DEFAULT 'PREPARED' CHECK (status IN ('PREPARED','VALIDATION_REQUIRED','VALIDATED','EXECUTED','CANCELLED')),
    content_snapshot JSONB NOT NULL DEFAULT '{}'::jsonb,
    approval_required BOOLEAN NOT NULL DEFAULT true,
    approved_by TEXT,
    approved_at TIMESTAMPTZ,
    executed_at TIMESTAMPTZ,
    notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id,arrears_case_id,action_level)
);
CREATE INDEX IF NOT EXISTS idx_collection_actions_scope_s11 ON collection_actions(legal_entity_id,status,action_level,created_at DESC);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_collection_actions_case_entity_s11') THEN
        ALTER TABLE collection_actions ADD CONSTRAINT fk_collection_actions_case_entity_s11 FOREIGN KEY(arrears_case_id) REFERENCES arrears_cases(id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE OR REPLACE FUNCTION guard_collection_legal_execution_s11() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.action_level IN ('FORMAL_NOTICE_PREPARATION','CASE_PREPARATION','COURT_HUISSIER_PREPARATION')
       AND NEW.status='EXECUTED' AND (NEW.approval_required OR NEW.approved_by IS NULL OR NEW.approved_at IS NULL) THEN
        RAISE EXCEPTION 'Une action juridique ne peut être exécutée automatiquement : validation humaine requise';
    END IF;
    IF NEW.status='EXECUTED' AND NEW.executed_at IS NULL THEN NEW.executed_at:=now(); END IF;
    NEW.updated_at:=now();
    RETURN NEW;
END; $$;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname='trg_guard_collection_legal_execution_s11') THEN
        CREATE TRIGGER trg_guard_collection_legal_execution_s11 BEFORE INSERT OR UPDATE ON collection_actions
        FOR EACH ROW EXECUTE FUNCTION guard_collection_legal_execution_s11();
    END IF;
END $$;

COMMENT ON TABLE payment_promises IS 'Promesses de paiement vérifiées automatiquement, sans transformation implicite en encaissement.';
COMMENT ON TABLE collection_actions IS 'Préparation des relances; toute mesure juridique exige une validation humaine explicite.';
