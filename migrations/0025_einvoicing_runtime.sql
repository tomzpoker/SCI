-- S13: runtime e-facturation: providers/adapters, événements, réception/émission et e-reporting.
-- Extension additive; aucun fournisseur n'est imposé au coeur métier.

INSERT INTO einvoice_providers(legal_entity_id,provider_code,provider_name,adapter_kind,format_code,capabilities,active)
SELECT e.id,'DISABLED','Aucun fournisseur','DISABLED','FACTUR_X',jsonb_build_object('inbound',false,'outbound',false,'ereporting',false),false
FROM legal_entities e
WHERE NOT EXISTS (SELECT 1 FROM einvoice_providers p WHERE p.legal_entity_id=e.id AND p.provider_code='DISABLED');

CREATE UNIQUE INDEX IF NOT EXISTS uq_einvoice_active_provider_s13
    ON einvoice_providers(legal_entity_id) WHERE active;

COMMENT ON COLUMN einvoice_providers.credential_reference IS 'Référence opaque vers un secret externe; aucun secret fournisseur n est stocké en clair.';
COMMENT ON COLUMN einvoice_providers.endpoint_reference IS 'Référence d endpoint/connexion, remplaçable sans modifier le coeur métier.';
