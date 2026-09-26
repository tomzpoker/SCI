-- S12 / COURRIERS, EMAILS, BAILS & PDF — templates versionnes et generation tracable.
-- Additif, non destructif.

CREATE TABLE IF NOT EXISTS document_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    document_kind TEXT NOT NULL CHECK (document_kind IN ('COURRIER','EMAIL','LEASE','JUSTIFICATIF','FACTURE','RELANCE','MISE_EN_DEMEURE','REGULARISATION','REVISION','TAXE','ECHEANCIER','OTHER')),
    version_no INTEGER NOT NULL DEFAULT 1,
    subject_template TEXT NOT NULL DEFAULT '',
    body_template TEXT NOT NULL,
    variables JSONB NOT NULL DEFAULT '[]'::jsonb,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id,code,version_no)
);
CREATE INDEX IF NOT EXISTS idx_document_templates_scope_s12 ON document_templates(legal_entity_id,document_kind,active,code);

INSERT INTO document_templates(legal_entity_id,code,name,document_kind,version_no,subject_template,body_template,variables)
SELECT e.id, v.code, v.name, v.kind, 1, v.subject, v.body, v.variables::jsonb
FROM legal_entities e
CROSS JOIN (VALUES
 ('JUSTIFICATIF_STANDARD','Justificatif standard','JUSTIFICATIF','Justificatif {{reference}}','Nous certifions les éléments suivants :\n{{details}}.', '["reference","details"]'),
 ('COURRIER_LOYER','Courrier loyer','COURRIER','Loyer {{reference}}','Objet : loyer {{reference}}\n\nMontant : {{amount}} EUR\nPériode : {{period}}\n\nCordialement,\n{{entity_name}}.', '["reference","amount","period","entity_name"]'),
 ('RELANCE_LOYER','Relance amiable','RELANCE','Relance — facture {{invoice_number}}','Bonjour {{tenant_name}},\n\nNous constatons un solde restant dû de {{outstanding}} EUR pour la facture {{invoice_number}}.\n\nMerci de régulariser votre situation ou de nous contacter.', '["tenant_name","invoice_number","outstanding"]'),
 ('MISE_EN_DEMEURE','Préparation mise en demeure','MISE_EN_DEMEURE','Projet — mise en demeure {{invoice_number}}','PROJET SOUMIS À VALIDATION HUMAINE\n\nClient : {{tenant_name}}\nFacture : {{invoice_number}}\nMontant : {{outstanding}} EUR\nÉchéance : {{due_date}}', '["tenant_name","invoice_number","outstanding","due_date"]'),
 ('REGULARISATION_CHARGES','Régularisation de charges','REGULARISATION','Régularisation {{period}}','Régularisation des charges : provisions {{provision}} EUR, réel {{actual}} EUR, écart {{difference}} EUR.', '["period","provision","actual","difference"]'),
 ('REVISION_LOYER','Révision de loyer','REVISION','Révision {{reference}}','Révision contractuelle : ancien loyer {{old_rent}} EUR, nouvel indice {{new_index}}, nouveau loyer calculé {{new_rent}} EUR.', '["reference","old_rent","new_index","new_rent"]'),
 ('TAXE_RECUPERABLE','Taxe récupérable','TAXE','Taxe récupérable {{tax_label}}','Taxe : {{tax_label}}\nBase juridique : {{legal_basis}}\nMontant : {{amount}} EUR.', '["tax_label","legal_basis","amount"]'),
 ('ECHEANCIER','Échéancier','ECHEANCIER','Échéancier — {{tenant_name}}','Échéancier proposé pour {{tenant_name}} :\n{{schedule}}.', '["tenant_name","schedule"]'),
 ('EMAIL_RELANCE_LOYER','Email de relance loyer','EMAIL','Relance — facture {{invoice_number}}','Bonjour {{tenant_name}},\n\nVotre facture {{invoice_number}} présente un solde de {{outstanding}} EUR. Merci de nous indiquer la date prévue de règlement.\n\nCordialement,\n{{entity_name}}.', '["tenant_name","invoice_number","outstanding","entity_name"]'),
 ('EMAIL_FACTURE_LOYER','Email d’envoi de loyer','EMAIL','Facture {{invoice_number}}','Bonjour {{tenant_name}},\n\nVeuillez trouver votre facture {{invoice_number}} pour la période {{period}}, d’un montant de {{amount}} EUR.\n\nCordialement,\n{{entity_name}}.', '["tenant_name","invoice_number","period","amount","entity_name"]'),
 ('BAIL_STANDARD','Bail commercial standard','LEASE','Bail {{reference}}','BAIL {{reference}}\n\nPreneur : {{tenant_name}}\nImmeuble : {{property_name}}\nLot : {{unit_label}}\nDate de début : {{start_date}}\nDate de fin : {{end_date}}\nLoyer : {{rent}} EUR\nCharges : {{charges}}\nTVA : {{vat_mode}}\nIndexation : {{index_code}}\n\nClauses :\n{{clauses}}', '["reference","tenant_name","property_name","unit_label","start_date","end_date","rent","charges","vat_mode","index_code","clauses"]')
) AS v(code,name,kind,subject,body,variables)
WHERE NOT EXISTS (SELECT 1 FROM document_templates t WHERE t.legal_entity_id=e.id AND t.code=v.code AND t.version_no=1);

CREATE TABLE IF NOT EXISTS generated_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    template_id UUID,
    document_kind TEXT NOT NULL,
    reference TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT','PREVIEWED','READY','SENT','CANCELLED')),
    subject TEXT NOT NULL DEFAULT '',
    recipient TEXT NOT NULL DEFAULT '',
    body_text TEXT NOT NULL DEFAULT '',
    body_html TEXT NOT NULL DEFAULT '',
    source_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    validation_required BOOLEAN NOT NULL DEFAULT false,
    validation_status TEXT NOT NULL DEFAULT 'NOT_REQUIRED' CHECK (validation_status IN ('NOT_REQUIRED','PENDING','APPROVED','REJECTED')),
    validation_request_id UUID,
    pdf_path TEXT,
    pdf_watermark TEXT NOT NULL DEFAULT '',
    pdf_logo_path TEXT NOT NULL DEFAULT '',
    pdf_generated_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_generated_documents_scope_s12 ON generated_documents(legal_entity_id,document_kind,status,created_at DESC);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_generated_document_template_entity_s12') THEN
        ALTER TABLE generated_documents ADD CONSTRAINT fk_generated_document_template_entity_s12 FOREIGN KEY(template_id) REFERENCES document_templates(id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS document_generation_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    generated_document_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    actor TEXT NOT NULL DEFAULT 'SYSTEM',
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_document_generation_events_s12 ON document_generation_events(legal_entity_id,generated_document_id,occurred_at DESC);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_document_generation_event_doc_entity_s12') THEN
        ALTER TABLE document_generation_events ADD CONSTRAINT fk_document_generation_event_doc_entity_s12 FOREIGN KEY(generated_document_id) REFERENCES generated_documents(id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS pdf_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    code TEXT NOT NULL DEFAULT 'DEFAULT_A4',
    page_size TEXT NOT NULL DEFAULT 'A4' CHECK (page_size='A4'),
    header_text TEXT NOT NULL DEFAULT '',
    footer_text TEXT NOT NULL DEFAULT '',
    logo_path TEXT NOT NULL DEFAULT '',
    margin_top_mm NUMERIC(8,2) NOT NULL DEFAULT 18,
    margin_right_mm NUMERIC(8,2) NOT NULL DEFAULT 18,
    margin_bottom_mm NUMERIC(8,2) NOT NULL DEFAULT 18,
    margin_left_mm NUMERIC(8,2) NOT NULL DEFAULT 18,
    active BOOLEAN NOT NULL DEFAULT true,
    UNIQUE(legal_entity_id,code)
);
INSERT INTO pdf_profiles(legal_entity_id,code,page_size,header_text,footer_text)
SELECT id,'DEFAULT_A4','A4','SCI FAMILY','SCI FAMILY — {reference} — {status} — Page {page}/{pages}' FROM legal_entities
WHERE NOT EXISTS(SELECT 1 FROM pdf_profiles p WHERE p.legal_entity_id=legal_entities.id AND p.code='DEFAULT_A4');

COMMENT ON TABLE document_templates IS 'Templates metier versionnes et remplacables sans modifier les documents deja generes.';
COMMENT ON TABLE generated_documents IS 'Courriers, emails et baux generes avec preview, validation, PDF et historique.';
COMMENT ON TABLE pdf_profiles IS 'Presentation PDF A4 configurable : en-tete, pied de page, pagination, logo.';
