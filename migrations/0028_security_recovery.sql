-- S16 / AUDIT / SECURITE / RECOVERY. Additif et non destructif.
CREATE TABLE IF NOT EXISTS auth_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), username TEXT NOT NULL UNIQUE, display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL, active BOOLEAN NOT NULL DEFAULT true, must_change_password BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS auth_user_roles (
    user_id UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    role TEXT NOT NULL CHECK (role IN ('OWNER','MANAGER','ACCOUNTANT','VIEWER','AI_AGENT','SYSTEM')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY(user_id,legal_entity_id,role)
);
CREATE INDEX IF NOT EXISTS idx_auth_user_roles_entity_s16 ON auth_user_roles(legal_entity_id,role,user_id);
CREATE TABLE IF NOT EXISTS auth_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), user_id UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT, token_hash TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(), expires_at TIMESTAMPTZ NOT NULL, revoked_at TIMESTAMPTZ,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_auth_sessions_lookup_s16 ON auth_sessions(token_hash,expires_at,revoked_at);

CREATE TABLE IF NOT EXISTS security_permission_catalog (
    code TEXT PRIMARY KEY, label_fr TEXT NOT NULL, description TEXT NOT NULL DEFAULT '',
    risk_level TEXT NOT NULL CHECK (risk_level IN ('LOW','MEDIUM','HIGH','CRITICAL'))
);
INSERT INTO security_permission_catalog(code,label_fr,description,risk_level) VALUES
('DASHBOARD_READ','Voir le tableau de bord','Accéder aux indicateurs.','LOW'),
('UX_CONFIGURE','Configurer l’interface','Modifier thèmes et onboarding.','LOW'),
('DATA_READ','Lire les données métier','Consulter les données de l’entité.','LOW'),
('DATA_WRITE','Modifier les données métier','Créer ou modifier des données métier.','HIGH'),
('FINANCE_WRITE','Modifier les données financières','Opérations financières sensibles.','HIGH'),
('FISCAL_WRITE','Modifier les préparations fiscales','Préparer/valider les éléments fiscaux.','HIGH'),
('LEGAL_PREPARE','Préparer une action juridique','Préparer sans exécution externe.','HIGH'),
('LEGAL_EXECUTE','Exécuter une action juridique','Exécution d’une procédure juridique.','CRITICAL'),
('AI_USE','Utiliser l’assistant IA','Interroger l’assistant et tools autorisés.','MEDIUM'),
('SECURITY_MANAGE','Gérer la sécurité','Utilisateurs, rôles et permissions.','CRITICAL'),
('BACKUP_MANAGE','Gérer les sauvegardes','Backup/verify/restore/snapshot.','CRITICAL'),
('RECOVERY_MANAGE','Gérer la récupération','Doctor/repair/rollback/rebuild/rescan/reconcile.','CRITICAL')
ON CONFLICT(code) DO NOTHING;
CREATE TABLE IF NOT EXISTS security_role_permissions (
    role TEXT NOT NULL CHECK (role IN ('OWNER','MANAGER','ACCOUNTANT','VIEWER','AI_AGENT','SYSTEM')),
    permission_code TEXT NOT NULL REFERENCES security_permission_catalog(code) ON DELETE RESTRICT,
    allowed BOOLEAN NOT NULL DEFAULT false, PRIMARY KEY(role,permission_code)
);
INSERT INTO security_role_permissions(role,permission_code,allowed)
SELECT r.role,p.code,CASE
 WHEN r.role='OWNER' THEN true
 WHEN r.role='MANAGER' AND p.code IN ('DASHBOARD_READ','UX_CONFIGURE','DATA_READ','DATA_WRITE','FINANCE_WRITE','FISCAL_WRITE','LEGAL_PREPARE','AI_USE') THEN true
 WHEN r.role='ACCOUNTANT' AND p.code IN ('DASHBOARD_READ','DATA_READ','FINANCE_WRITE','FISCAL_WRITE','AI_USE') THEN true
 WHEN r.role='VIEWER' AND p.code IN ('DASHBOARD_READ','DATA_READ') THEN true
 WHEN r.role='AI_AGENT' AND p.code IN ('DASHBOARD_READ','DATA_READ','AI_USE','LEGAL_PREPARE') THEN true
 WHEN r.role='SYSTEM' THEN true ELSE false END
FROM (VALUES('OWNER'),('MANAGER'),('ACCOUNTANT'),('VIEWER'),('AI_AGENT'),('SYSTEM')) r(role)
CROSS JOIN security_permission_catalog p
ON CONFLICT(role,permission_code) DO NOTHING;

ALTER TABLE audit_events ADD COLUMN IF NOT EXISTS actor_user_id UUID REFERENCES auth_users(id) ON DELETE SET NULL;
ALTER TABLE audit_events ADD COLUMN IF NOT EXISTS before_state JSONB NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE audit_events ADD COLUMN IF NOT EXISTS after_state JSONB NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE audit_events ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'APP';
ALTER TABLE audit_events ADD COLUMN IF NOT EXISTS reason TEXT NOT NULL DEFAULT '';
ALTER TABLE audit_events ADD COLUMN IF NOT EXISTS validation JSONB NOT NULL DEFAULT '{}'::jsonb;
CREATE INDEX IF NOT EXISTS idx_audit_events_actor_s16 ON audit_events(legal_entity_id,actor_user_id,occurred_at DESC);

CREATE TABLE IF NOT EXISTS system_backups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    backup_path TEXT NOT NULL, backup_format TEXT NOT NULL DEFAULT 'CUSTOM', sha256 TEXT NOT NULL DEFAULT '',
    size_bytes BIGINT NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT 'CREATED' CHECK(status IN('CREATED','VERIFIED','FAILED','RESTORED')),
    created_by UUID REFERENCES auth_users(id) ON DELETE SET NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    verified_at TIMESTAMPTZ, metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);
CREATE TABLE IF NOT EXISTS system_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    snapshot_label TEXT NOT NULL, backup_id UUID REFERENCES system_backups(id) ON DELETE RESTRICT,
    application_version TEXT NOT NULL, latest_migration TEXT NOT NULL, component_manifest JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by UUID REFERENCES auth_users(id) ON DELETE SET NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS recovery_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    action TEXT NOT NULL CHECK(action IN('DOCTOR','REPAIR','VERIFY','ROLLBACK','REBUILD_INDEX','RESCAN_DOCUMENTS','RECONCILE')),
    status TEXT NOT NULL DEFAULT 'STARTED' CHECK(status IN('STARTED','PASS','WARN','FAIL','CANCELLED')),
    actor_user_id UUID REFERENCES auth_users(id) ON DELETE SET NULL, started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ, report JSONB NOT NULL DEFAULT '{}'::jsonb
);
CREATE INDEX IF NOT EXISTS idx_recovery_runs_scope_s16 ON recovery_runs(legal_entity_id,started_at DESC,action,status);
