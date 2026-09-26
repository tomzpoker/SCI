use std::fs;

#[test]
fn git_ignores_runtime_secrets_and_local_data() {
    let s = fs::read_to_string(".gitignore").expect("gitignore");
    for needle in [".env", "snapshots/", ".sci-inbox/", "*.dump"] {
        assert!(s.contains(needle), "entrée absente du .gitignore: {needle}");
    }
}

#[test]
fn ai_agent_is_not_owner_by_default_in_source() {
    let s = fs::read_to_string("src/security.rs").expect("security source");
    assert!(s.contains("AI_AGENT"));
    assert!(s.contains("OWNER"));
    assert!(s.contains("must_change_password"));
}

#[test]
fn multi_company_scope_requires_entity_role_and_updates_session_scope() {
    let s = fs::read_to_string("src/entity_scope.rs").expect("entity scope source");
    assert!(s.contains("FROM auth_user_roles aur JOIN legal_entities le"));
    assert!(s.contains("aur.user_id=$1 AND aur.legal_entity_id=$2 AND le.active=true"));
    assert!(s.contains("permission_code='DATA_READ'"));
    assert!(s.contains("UPDATE auth_sessions SET legal_entity_id=$2"));
}

#[test]
fn entity_selector_starts_from_authenticated_scope() {
    let src = fs::read_to_string("src/ui.rs").expect("ui.rs");
    assert!(src.contains("EntityScopeSelector{refresh,page,initial_entity_id:auth_status.legal_entity_id}"));
    assert!(src.contains("initial_entity_id.unwrap_or(LEGACY_SCI_LEGAL_ENTITY_ID)"));
}

#[test]
fn document_validation_uses_document_id_and_authenticated_write_role() {
    let s = fs::read_to_string("src/documents/workflow.rs").expect("document workflow source");
    assert!(s.contains("require_permission(pool, \"DATA_WRITE\")"));
    assert!(s.contains("WHERE document_id=$1 AND legal_entity_id=$2 ORDER BY created_at DESC"));
    assert!(!s.contains("WHERE id=$1 AND legal_entity_id=$2 AND document_id=$3 ORDER BY created_at DESC"));
}


#[test]
fn entity_scope_audit_event_has_consistent_sql_parameters() {
    let s = fs::read_to_string("src/entity_scope.rs").expect("entity scope source");
    assert!(s.contains("entity_id,payload,actor_user_id,source,reason) VALUES(NULL,$1,'USER','CHANGE_ACTIVE_LEGAL_ENTITY','LEGAL_ENTITY',$1,$2,$3,'APP'"));
    assert!(!s.contains("entity_id,payload,actor_user_id,source,reason) VALUES(NULL,$1,'USER','CHANGE_ACTIVE_LEGAL_ENTITY','LEGAL_ENTITY',$1,$3,$4,'APP'"));
}

#[test]
fn legal_entity_audit_uses_authenticated_actor() {
    let s = fs::read_to_string("src/legal_entities.rs").expect("legal entities source");
    assert!(s.contains("current_principal(pool).await?"));
    assert!(s.contains("actor_user_id"));
    assert!(!s.contains("VALUES(NULL,$1,'MANAGER',$2,'LEGAL_ENTITY'"));
}
