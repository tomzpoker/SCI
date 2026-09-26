use dioxus::prelude::*;
use uuid::Uuid;

pub const LEGACY_SCI_LEGAL_ENTITY_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000010);

#[cfg(feature = "server")]
use std::sync::{OnceLock, RwLock};

#[cfg(feature = "server")]
static ACTIVE_LEGAL_ENTITY_ID: OnceLock<RwLock<Uuid>> = OnceLock::new();

#[cfg(feature = "server")]
fn active_cell() -> &'static RwLock<Uuid> {
    ACTIVE_LEGAL_ENTITY_ID.get_or_init(|| RwLock::new(LEGACY_SCI_LEGAL_ENTITY_ID))
}

/// Portée métier courante du processus local.
///
/// Le runtime actuel est volontairement single-user/local : le sélecteur d'entité
/// modifie cette portée, puis les fonctions serveur relisent ce même contexte.
/// Une session/authentification future pourra remplacer ce stockage global par
/// un contexte de requête sans changer le modèle SQL.
pub fn current_legal_entity_id() -> Uuid {
    #[cfg(feature = "server")]
    {
        *active_cell().read().expect("scope lock poisoned")
    }
    #[cfg(not(feature = "server"))]
    {
        LEGACY_SCI_LEGAL_ENTITY_ID
    }
}

#[cfg(feature="server")]
pub fn set_scope_after_login(id: Uuid) {
    *active_cell().write().expect("scope lock poisoned") = id;
}

#[server]
pub async fn set_active_legal_entity(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let principal = crate::security::require_permission(pool, "DATA_READ").await.map_err(ServerFnError::new)?;
        let user_id = principal.user_id.ok_or_else(|| ServerFnError::new("Utilisateur courant introuvable"))?;
        let target_role: Option<String> = sqlx::query_scalar(
            "SELECT aur.role FROM auth_user_roles aur JOIN legal_entities le ON le.id=aur.legal_entity_id WHERE aur.user_id=$1 AND aur.legal_entity_id=$2 AND le.active=true ORDER BY CASE aur.role WHEN 'OWNER' THEN 1 WHEN 'MANAGER' THEN 2 WHEN 'ACCOUNTANT' THEN 3 WHEN 'VIEWER' THEN 4 WHEN 'AI_AGENT' THEN 5 ELSE 6 END LIMIT 1",
        )
        .bind(user_id)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(ServerFnError::new)?;
        let Some(target_role) = target_role else {
            return Err(ServerFnError::new("Accès refusé à cette entité juridique"));
        };
        let target_can_read: bool = sqlx::query_scalar(
            "SELECT COALESCE((SELECT allowed FROM security_role_permissions WHERE role=$1 AND permission_code='DATA_READ'),false)",
        )
        .bind(&target_role)
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;
        if !target_can_read {
            return Err(ServerFnError::new("Permission DATA_READ refusée pour cette entité juridique"));
        }
        let previous = current_legal_entity_id();
        if let Ok(Some(token_hash)) = crate::security::current_session_token_hash().await {
            sqlx::query("UPDATE auth_sessions SET legal_entity_id=$2,last_seen_at=now() WHERE token_hash=$1 AND user_id=$3 AND revoked_at IS NULL")
                .bind(token_hash)
                .bind(id)
                .bind(user_id)
                .execute(pool)
                .await
                .map_err(ServerFnError::new)?;
        }
        *active_cell().write().expect("scope lock poisoned") = id;
        sqlx::query(
            "INSERT INTO audit_events(sci_id,legal_entity_id,actor,action,entity_type,entity_id,payload,actor_user_id,source,reason) VALUES(NULL,$1,'USER','CHANGE_ACTIVE_LEGAL_ENTITY','LEGAL_ENTITY',$1,$2,$3,'APP','Changement d’entité autorisé')",
        )
        .bind(id)
        .bind(serde_json::json!({
            "previous_legal_entity_id": previous,
            "current_legal_entity_id": id,
            "target_role": target_role,
        }))
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = id;
        Err(ServerFnError::new(
            "set_active_legal_entity est exécutée côté serveur",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_scope_is_stable() {
        assert_eq!(LEGACY_SCI_LEGAL_ENTITY_ID.to_string(), "00000000-0000-0000-0000-000000000010");
    }
}
