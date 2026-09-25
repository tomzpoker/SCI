-- S02 / US-0203 — Cloisonnement des données par entité juridique.
-- Migration additive et compatible avec le vertical slice historique.
--
-- Principe : legal_entity_id devient la portée canonique des flux métier.
-- sci_id reste conservé pour compatibilité avec les données SCI historiques.
-- Pour les tables génériques, sci_id peut désormais être NULL afin qu'une SARL
-- puisse posséder ses propres factures, paiements, banque, règles, documents,
-- échéances et paramètres sans fabriquer une seconde SCI.
--
-- Cette migration ne supprime ni donnée ni objet.

-- 1. Ajouter la portée juridique sur les tables existantes.
ALTER TABLE associates ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE properties ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE units ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE leases ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE payments ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE bank_transactions ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE automation_rules ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE tax_deadlines ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE forecast_snapshots ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE app_settings ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE storage_locations ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE llm_accounts ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE email_accounts ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE document_inbox ADD COLUMN IF NOT EXISTS legal_entity_id UUID;
ALTER TABLE assistant_proposals ADD COLUMN IF NOT EXISTS legal_entity_id UUID;

-- 2. Reprendre automatiquement l'entité des données existantes.
UPDATE associates a SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE a.legal_entity_id IS NULL AND a.sci_id = s.id;

UPDATE properties p SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE p.legal_entity_id IS NULL AND p.sci_id = s.id;

UPDATE units u SET legal_entity_id = p.legal_entity_id
FROM properties p
WHERE u.legal_entity_id IS NULL AND u.property_id = p.id;

UPDATE tenants t SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE t.legal_entity_id IS NULL AND t.sci_id = s.id;

UPDATE leases l SET legal_entity_id = u.legal_entity_id
FROM units u
WHERE l.legal_entity_id IS NULL AND l.unit_id = u.id;

UPDATE invoices i SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE i.legal_entity_id IS NULL AND i.sci_id = s.id;

UPDATE payments p SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE p.legal_entity_id IS NULL AND p.sci_id = s.id;

UPDATE bank_transactions b SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE b.legal_entity_id IS NULL AND b.sci_id = s.id;

UPDATE automation_rules r SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE r.legal_entity_id IS NULL AND r.sci_id = s.id;

UPDATE tasks t SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE t.legal_entity_id IS NULL AND t.sci_id = s.id;

UPDATE tax_deadlines d SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE d.legal_entity_id IS NULL AND d.sci_id = s.id;

UPDATE documents d SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE d.legal_entity_id IS NULL AND d.sci_id = s.id;

UPDATE forecast_snapshots f SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE f.legal_entity_id IS NULL AND f.sci_id = s.id;

UPDATE app_settings a SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE a.legal_entity_id IS NULL AND a.sci_id = s.id;

UPDATE storage_locations l SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE l.legal_entity_id IS NULL AND l.sci_id = s.id;

UPDATE llm_accounts l SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE l.legal_entity_id IS NULL AND l.sci_id = s.id;

UPDATE email_accounts e SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE e.legal_entity_id IS NULL AND e.sci_id = s.id;

UPDATE document_inbox d SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE d.legal_entity_id IS NULL AND d.sci_id = s.id;

UPDATE assistant_proposals a SET legal_entity_id = s.legal_entity_id
FROM scis s
WHERE a.legal_entity_id IS NULL AND a.sci_id = s.id;

-- 3. Les tables génériques ne dépendent plus obligatoirement d'une SCI.
-- Cela autorise les flux propres à une SARL tout en conservant sci_id pour
-- l'historique SCI et les compatibilités existantes.
ALTER TABLE invoices ALTER COLUMN sci_id DROP NOT NULL;
ALTER TABLE payments ALTER COLUMN sci_id DROP NOT NULL;
ALTER TABLE bank_transactions ALTER COLUMN sci_id DROP NOT NULL;
ALTER TABLE automation_rules ALTER COLUMN sci_id DROP NOT NULL;
ALTER TABLE tasks ALTER COLUMN sci_id DROP NOT NULL;
ALTER TABLE tax_deadlines ALTER COLUMN sci_id DROP NOT NULL;
ALTER TABLE documents ALTER COLUMN sci_id DROP NOT NULL;
ALTER TABLE forecast_snapshots ALTER COLUMN sci_id DROP NOT NULL;
ALTER TABLE app_settings ALTER COLUMN sci_id DROP NOT NULL;
ALTER TABLE storage_locations ALTER COLUMN sci_id DROP NOT NULL;
ALTER TABLE llm_accounts ALTER COLUMN sci_id DROP NOT NULL;
ALTER TABLE email_accounts ALTER COLUMN sci_id DROP NOT NULL;
ALTER TABLE document_inbox ALTER COLUMN sci_id DROP NOT NULL;
ALTER TABLE assistant_proposals ALTER COLUMN sci_id DROP NOT NULL;

-- 4. Les écritures déjà présentes doivent toutes avoir une portée juridique.
DO $$
DECLARE
    missing_count BIGINT;
BEGIN
    SELECT
        (SELECT COUNT(*) FROM associates WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM properties WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM units WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM tenants WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM leases WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM invoices WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM payments WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM bank_transactions WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM automation_rules WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM tasks WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM tax_deadlines WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM documents WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM forecast_snapshots WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM app_settings WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM storage_locations WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM llm_accounts WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM email_accounts WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM document_inbox WHERE legal_entity_id IS NULL)
        +(SELECT COUNT(*) FROM assistant_proposals WHERE legal_entity_id IS NULL)
    INTO missing_count;

    IF missing_count > 0 THEN
        RAISE EXCEPTION 'US-0203: % ligne(s) sans legal_entity_id après reprise', missing_count;
    END IF;
END $$;

-- 5. Une ligne métier ne peut exister sans portée juridique.
ALTER TABLE associates ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE properties ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE units ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE tenants ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE leases ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE invoices ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE payments ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE bank_transactions ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE automation_rules ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE tasks ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE tax_deadlines ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE documents ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE forecast_snapshots ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE app_settings ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE storage_locations ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE llm_accounts ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE email_accounts ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE document_inbox ALTER COLUMN legal_entity_id SET NOT NULL;
ALTER TABLE assistant_proposals ALTER COLUMN legal_entity_id SET NOT NULL;

-- 6. Contraintes de référence vers l'entité juridique.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_associates_legal_entity') THEN
        ALTER TABLE associates ADD CONSTRAINT fk_associates_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_properties_legal_entity') THEN
        ALTER TABLE properties ADD CONSTRAINT fk_properties_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_units_legal_entity') THEN
        ALTER TABLE units ADD CONSTRAINT fk_units_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_tenants_legal_entity') THEN
        ALTER TABLE tenants ADD CONSTRAINT fk_tenants_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_leases_legal_entity') THEN
        ALTER TABLE leases ADD CONSTRAINT fk_leases_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_invoices_legal_entity') THEN
        ALTER TABLE invoices ADD CONSTRAINT fk_invoices_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_payments_legal_entity') THEN
        ALTER TABLE payments ADD CONSTRAINT fk_payments_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_bank_transactions_legal_entity') THEN
        ALTER TABLE bank_transactions ADD CONSTRAINT fk_bank_transactions_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_automation_rules_legal_entity') THEN
        ALTER TABLE automation_rules ADD CONSTRAINT fk_automation_rules_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_tasks_legal_entity') THEN
        ALTER TABLE tasks ADD CONSTRAINT fk_tasks_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_tax_deadlines_legal_entity') THEN
        ALTER TABLE tax_deadlines ADD CONSTRAINT fk_tax_deadlines_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_documents_legal_entity') THEN
        ALTER TABLE documents ADD CONSTRAINT fk_documents_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_forecast_snapshots_legal_entity') THEN
        ALTER TABLE forecast_snapshots ADD CONSTRAINT fk_forecast_snapshots_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_app_settings_legal_entity') THEN
        ALTER TABLE app_settings ADD CONSTRAINT fk_app_settings_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_storage_locations_legal_entity') THEN
        ALTER TABLE storage_locations ADD CONSTRAINT fk_storage_locations_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_llm_accounts_legal_entity') THEN
        ALTER TABLE llm_accounts ADD CONSTRAINT fk_llm_accounts_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_email_accounts_legal_entity') THEN
        ALTER TABLE email_accounts ADD CONSTRAINT fk_email_accounts_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_document_inbox_legal_entity') THEN
        ALTER TABLE document_inbox ADD CONSTRAINT fk_document_inbox_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_assistant_proposals_legal_entity') THEN
        ALTER TABLE assistant_proposals ADD CONSTRAINT fk_assistant_proposals_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
END $$;

-- 7. Les paires sci_id/legal_entity_id sont cohérentes lorsqu'un sci_id existe.
CREATE UNIQUE INDEX IF NOT EXISTS uq_scis_id_legal_entity
    ON scis(id, legal_entity_id);

DO $$
DECLARE
    target_table TEXT;
BEGIN
    FOREACH target_table IN ARRAY ARRAY[
        'associates','properties','tenants','invoices','payments',
        'bank_transactions','automation_rules','tasks','tax_deadlines','documents',
        'forecast_snapshots','app_settings','storage_locations','llm_accounts',
        'email_accounts','document_inbox','assistant_proposals'
    ] LOOP
        IF NOT EXISTS (
            SELECT 1 FROM pg_constraint WHERE conname = 'fk_' || target_table || '_sci_entity'
        ) THEN
            EXECUTE format(
                'ALTER TABLE %I ADD CONSTRAINT %I FOREIGN KEY (sci_id, legal_entity_id) REFERENCES scis(id, legal_entity_id)',
                target_table,
                'fk_' || target_table || '_sci_entity'
            );
        END IF;
    END LOOP;
END $$;

-- 8. Empêcher un objet parent d'être utilisé dans une autre entité.
CREATE UNIQUE INDEX IF NOT EXISTS uq_properties_id_legal_entity
    ON properties(id, legal_entity_id);
CREATE UNIQUE INDEX IF NOT EXISTS uq_units_id_legal_entity
    ON units(id, legal_entity_id);
CREATE UNIQUE INDEX IF NOT EXISTS uq_tenants_id_legal_entity
    ON tenants(id, legal_entity_id);
CREATE UNIQUE INDEX IF NOT EXISTS uq_leases_id_legal_entity
    ON leases(id, legal_entity_id);
CREATE UNIQUE INDEX IF NOT EXISTS uq_invoices_id_legal_entity
    ON invoices(id, legal_entity_id);
CREATE UNIQUE INDEX IF NOT EXISTS uq_automation_rules_id_legal_entity
    ON automation_rules(id, legal_entity_id);
CREATE UNIQUE INDEX IF NOT EXISTS uq_storage_locations_id_legal_entity
    ON storage_locations(id, legal_entity_id);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_units_property_entity') THEN
        ALTER TABLE units ADD CONSTRAINT fk_units_property_entity
            FOREIGN KEY (property_id, legal_entity_id) REFERENCES properties(id, legal_entity_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_leases_unit_entity') THEN
        ALTER TABLE leases ADD CONSTRAINT fk_leases_unit_entity
            FOREIGN KEY (unit_id, legal_entity_id) REFERENCES units(id, legal_entity_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_leases_tenant_entity') THEN
        ALTER TABLE leases ADD CONSTRAINT fk_leases_tenant_entity
            FOREIGN KEY (tenant_id, legal_entity_id) REFERENCES tenants(id, legal_entity_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_invoices_lease_entity') THEN
        ALTER TABLE invoices ADD CONSTRAINT fk_invoices_lease_entity
            FOREIGN KEY (lease_id, legal_entity_id) REFERENCES leases(id, legal_entity_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_payments_invoice_entity') THEN
        ALTER TABLE payments ADD CONSTRAINT fk_payments_invoice_entity
            FOREIGN KEY (invoice_id, legal_entity_id) REFERENCES invoices(id, legal_entity_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_tasks_rule_entity') THEN
        ALTER TABLE tasks ADD CONSTRAINT fk_tasks_rule_entity
            FOREIGN KEY (automation_rule_id, legal_entity_id) REFERENCES automation_rules(id, legal_entity_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_inbox_storage_entity') THEN
        ALTER TABLE document_inbox ADD CONSTRAINT fk_inbox_storage_entity
            FOREIGN KEY (selected_storage_location_id, legal_entity_id) REFERENCES storage_locations(id, legal_entity_id);
    END IF;
END $$;

-- 9. Les contraintes d'unicité critiques portent désormais sur l'entité,
--    notamment pour les données SARL dont sci_id est NULL.
CREATE UNIQUE INDEX IF NOT EXISTS uq_bank_transactions_entity_external
    ON bank_transactions(legal_entity_id, external_id)
    WHERE external_id IS NOT NULL AND external_id <> '';
CREATE UNIQUE INDEX IF NOT EXISTS uq_automation_rules_entity_code
    ON automation_rules(legal_entity_id, code);
CREATE UNIQUE INDEX IF NOT EXISTS uq_tasks_entity_occurrence
    ON tasks(legal_entity_id, code, occurrence_key);
CREATE UNIQUE INDEX IF NOT EXISTS uq_tax_deadlines_entity_code_date
    ON tax_deadlines(legal_entity_id, code, deadline_date);
CREATE UNIQUE INDEX IF NOT EXISTS uq_app_settings_entity_key
    ON app_settings(legal_entity_id, key);
CREATE UNIQUE INDEX IF NOT EXISTS uq_llm_accounts_entity_name
    ON llm_accounts(legal_entity_id, name);
CREATE UNIQUE INDEX IF NOT EXISTS uq_email_accounts_entity_mailbox
    ON email_accounts(legal_entity_id, address, imap_host, mailbox);
CREATE UNIQUE INDEX IF NOT EXISTS uq_document_inbox_entity_source
    ON document_inbox(legal_entity_id, source_type, source_ref, filename);

-- 10. Index de lecture par portée canonique.
CREATE INDEX IF NOT EXISTS idx_associates_entity ON associates(legal_entity_id, active);
CREATE INDEX IF NOT EXISTS idx_properties_entity ON properties(legal_entity_id, active);
CREATE INDEX IF NOT EXISTS idx_units_entity ON units(legal_entity_id, active);
CREATE INDEX IF NOT EXISTS idx_tenants_entity ON tenants(legal_entity_id, active);
CREATE INDEX IF NOT EXISTS idx_leases_entity ON leases(legal_entity_id, active);
CREATE INDEX IF NOT EXISTS idx_invoices_entity ON invoices(legal_entity_id, issue_date DESC);
CREATE INDEX IF NOT EXISTS idx_payments_entity ON payments(legal_entity_id, received_at DESC);
CREATE INDEX IF NOT EXISTS idx_bank_transactions_entity ON bank_transactions(legal_entity_id, booked_at DESC);
CREATE INDEX IF NOT EXISTS idx_automation_rules_entity ON automation_rules(legal_entity_id, enabled, priority DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_entity ON tasks(legal_entity_id, state, due_at);
CREATE INDEX IF NOT EXISTS idx_tax_deadlines_entity ON tax_deadlines(legal_entity_id, deadline_date);
CREATE INDEX IF NOT EXISTS idx_documents_entity ON documents(legal_entity_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_forecast_snapshots_entity ON forecast_snapshots(legal_entity_id, generated_at DESC);
CREATE INDEX IF NOT EXISTS idx_app_settings_entity ON app_settings(legal_entity_id, key);
CREATE INDEX IF NOT EXISTS idx_storage_locations_entity ON storage_locations(legal_entity_id, is_active);
CREATE INDEX IF NOT EXISTS idx_llm_accounts_entity ON llm_accounts(legal_entity_id, enabled, is_free);
CREATE INDEX IF NOT EXISTS idx_email_accounts_entity ON email_accounts(legal_entity_id, enabled, auto_scan);
CREATE INDEX IF NOT EXISTS idx_document_inbox_entity ON document_inbox(legal_entity_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_assistant_proposals_entity ON assistant_proposals(legal_entity_id, status, created_at DESC);
