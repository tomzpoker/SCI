use crate::domain::{BusinessProfileCatalogItem, LegalEntityBusinessProfileDraft, LegalEntityBusinessProfileItem};
use dioxus::prelude::*;
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::entity_scope::current_legal_entity_id;
#[cfg(feature = "server")]
use crate::infrastructure::db;
#[cfg(feature = "server")]
use sqlx::Row;

fn validate_profile_draft(draft: &LegalEntityBusinessProfileDraft) -> Result<(), String> {
    if draft.legal_entity_id == Uuid::nil() { return Err("Entité juridique requise".into()); }
    if draft.profile_code.trim().is_empty() { return Err("Profil d'exploitation requis".into()); }
    Ok(())
}

#[server]
pub async fn list_business_profile_catalog() -> Result<Vec<BusinessProfileCatalogItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query(
            "SELECT code,label,description,legal_form_code,COALESCE(default_tax_regime,'') AS default_tax_regime,module_key,capabilities,active FROM business_profile_catalog WHERE active ORDER BY legal_form_code, label",
        ).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| BusinessProfileCatalogItem {
            code:r.get("code"), label:r.get("label"), description:r.get("description"),
            legal_form_code:r.get("legal_form_code"), default_tax_regime:r.get("default_tax_regime"),
            module_key:r.get("module_key"), capabilities:r.get("capabilities"), active:r.get("active")
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_business_profile_catalog est exécutée côté serveur"))
}

#[server]
pub async fn list_entity_business_profiles(legal_entity_id: Option<Uuid>) -> Result<Vec<LegalEntityBusinessProfileItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let entity = legal_entity_id.unwrap_or_else(current_legal_entity_id);
        let rows = sqlx::query(
            r#"SELECT p.legal_entity_id,p.profile_code,c.label,c.description,c.legal_form_code,
                      COALESCE(c.default_tax_regime,'') AS default_tax_regime,c.module_key,c.capabilities,
                      p.is_primary,p.active,p.configuration,p.notes
               FROM legal_entity_business_profiles p
               JOIN business_profile_catalog c ON c.code=p.profile_code
               WHERE p.legal_entity_id=$1
               ORDER BY p.active DESC,p.is_primary DESC,c.label"#,
        ).bind(entity).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| LegalEntityBusinessProfileItem {
            legal_entity_id:r.get("legal_entity_id"), profile_code:r.get("profile_code"),
            label:r.get("label"), description:r.get("description"), legal_form_code:r.get("legal_form_code"),
            default_tax_regime:r.get("default_tax_regime"), module_key:r.get("module_key"),
            capabilities:r.get("capabilities"), is_primary:r.get("is_primary"), active:r.get("active"),
            configuration:r.get("configuration"), notes:r.get("notes")
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    { let _ = legal_entity_id; Err(ServerFnError::new("list_entity_business_profiles est exécutée côté serveur")) }
}

#[server]
pub async fn set_entity_business_profile(draft: LegalEntityBusinessProfileDraft) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        validate_profile_draft(&draft).map_err(ServerFnError::new)?;
        let pool = db().await.map_err(ServerFnError::new)?;
        let _principal = crate::security::require_permission(pool, "DATA_WRITE").await.map_err(ServerFnError::new)?;
        let entity=draft.legal_entity_id;
        let legal_form:String = sqlx::query_scalar("SELECT legal_form_code FROM legal_entities WHERE id=$1 AND active=true")
            .bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?
            .ok_or_else(||ServerFnError::new("Entité juridique introuvable ou inactive"))?;
        let profile=sqlx::query("SELECT legal_form_code,COALESCE(default_tax_regime,'') AS default_tax_regime FROM business_profile_catalog WHERE code=$1 AND active=true")
            .bind(draft.profile_code.trim().to_uppercase()).fetch_optional(pool).await.map_err(ServerFnError::new)?
            .ok_or_else(||ServerFnError::new("Profil d'exploitation introuvable"))?;
        let profile_form:String=profile.get("legal_form_code");
        let default_tax:String=profile.get("default_tax_regime");
        if legal_form != profile_form { return Err(ServerFnError::new("Le profil choisi ne correspond pas à la forme juridique de l'entité")); }
        if draft.active && draft.profile_code.trim().eq_ignore_ascii_case("SARL_IS_GARAGE_SANS_SAV") {
            let tax:String=sqlx::query_scalar("SELECT tax_regime FROM legal_entities WHERE id=$1").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
            if tax!="IS" { return Err(ServerFnError::new("Le profil Garage sans SAV exige une SARL à l'IS")); }
            let _=default_tax;
        }
        sqlx::query("INSERT INTO legal_entity_business_profiles(legal_entity_id,profile_code,is_primary,active,configuration,notes) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(legal_entity_id,profile_code) DO UPDATE SET is_primary=EXCLUDED.is_primary,active=EXCLUDED.active,configuration=EXCLUDED.configuration,notes=EXCLUDED.notes,updated_at=now()")
            .bind(entity).bind(draft.profile_code.trim().to_uppercase()).bind(draft.is_primary).bind(draft.active)
            .bind(draft.configuration).bind(draft.notes.trim()).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO audit_events(sci_id,legal_entity_id,actor,action,entity_type,entity_id,payload) VALUES(NULL,$1,'MANAGER','SET_BUSINESS_PROFILE','BUSINESS_PROFILE',$1,$2)")
            .bind(entity).bind(serde_json::json!({"profile_code":draft.profile_code.trim().to_uppercase(),"active":draft.active,"is_primary":draft.is_primary})).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    { let _=draft; Err(ServerFnError::new("set_entity_business_profile est exécutée côté serveur")) }
}

#[server]
pub async fn current_entity_business_profile() -> Result<Option<LegalEntityBusinessProfileItem>, ServerFnError> {
    let list=list_entity_business_profiles(None).await?;
    Ok(list.into_iter().find(|p|p.active && p.is_primary))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_nil_entity() {
        let d=LegalEntityBusinessProfileDraft{legal_entity_id:Uuid::nil(),profile_code:"SARL_IS_GARAGE_SANS_SAV".into(),is_primary:true,active:true,configuration:serde_json::json!({}),notes:String::new()};
        assert!(validate_profile_draft(&d).is_err());
    }
}
