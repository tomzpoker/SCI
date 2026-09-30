-- Historique des relances clients envoyées.
-- Une ligne par tentative d'envoi (succès ou échec).
CREATE TABLE IF NOT EXISTS email_relances (
    id                  uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id     uuid NOT NULL REFERENCES legal_entities(id) ON DELETE CASCADE,
    dolibarr_invoice_id text NOT NULL,
    invoice_ref         text NOT NULL,
    client_name         text NOT NULL,
    client_email        text NOT NULL,
    level               integer NOT NULL CHECK (level BETWEEN 1 AND 3),
    subject             text NOT NULL,
    body                text NOT NULL,
    sent_at             timestamptz NOT NULL DEFAULT now(),
    sent_via            text NOT NULL CHECK (sent_via IN ('dolibarr', 'smtp', 'manual')),
    status              text NOT NULL CHECK (status IN ('sent', 'failed')),
    error_message       text,
    metadata            jsonb NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX IF NOT EXISTS idx_email_relances_invoice
    ON email_relances (legal_entity_id, dolibarr_invoice_id, sent_at DESC);

COMMENT ON TABLE email_relances IS
    'Historique des relances impayés (source de vérité = Dolibarr pour l''envoi, on log ici)';