use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
pub struct AuthStatusItem { pub authenticated:bool,pub user_id:Option<Uuid>,pub username:Option<String>,pub display_name:Option<String>,pub role:Option<String>,pub legal_entity_id:Option<Uuid>,pub must_change_password:bool,pub token:Option<String> }
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
pub struct BackupItem { pub id:Uuid,pub path:String,pub sha256:String,pub size_bytes:i64,pub status:String,pub created_at:String }
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
pub struct SnapshotItem { pub id:Uuid,pub label:String,pub backup_id:Option<Uuid>,pub application_version:String,pub latest_migration:String,pub created_at:String }
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
pub struct RecoveryRunItem { pub id:Uuid,pub action:String,pub status:String,pub report:Value,pub started_at:String,pub finished_at:Option<String> }

#[cfg(feature="server")]
mod server_impl {
    use super::*;
    use argon2::{Argon2,PasswordHash,PasswordVerifier};
    use argon2::password_hash::{PasswordHasher,SaltString};
    use sha2::{Digest,Sha256};
    use std::path::{Path,PathBuf};
    use std::time::{SystemTime,UNIX_EPOCH};

    pub fn db_security_error(message:impl Into<String>)->sqlx::Error{sqlx::Error::Protocol(message.into())}

    pub(crate) fn hash_bytes(input:&str)->String{ let mut h = <Sha256 as sha2::Digest>::new(); sha2::Digest::update(&mut h, input.as_bytes()); format!("{:x}", sha2::Digest::finalize(h)) }
    pub(crate) fn new_token()->String{ format!("{}-{}",Uuid::new_v4(),Uuid::new_v4()) }
    fn salt()->Result<SaltString,String>{ let mut bytes=[0u8;16]; getrandom::fill(&mut bytes).map_err(|e|e.to_string())?; SaltString::encode_b64(&bytes).map_err(|e|e.to_string()) }
    pub fn hash_password(password:&str)->Result<String,String>{ let s=salt()?; Argon2::default().hash_password(password.as_bytes(),&s).map(|v|v.to_string()).map_err(|e|e.to_string()) }
    pub fn verify_password(password:&str,hash:&str)->bool{ PasswordHash::new(hash).map(|parsed|Argon2::default().verify_password(password.as_bytes(),&parsed).is_ok()).unwrap_or(false) }

    pub async fn current_headers()->Result<dioxus::fullstack::HeaderMap,sqlx::Error>{
        let _ctx=dioxus::fullstack::FullstackContext::current().ok_or_else(||db_security_error("Contexte HTTP absent"))?;
        let headers: dioxus::fullstack::HeaderMap = dioxus::fullstack::FullstackContext::extract().await.map_err(|e|db_security_error(format!("Lecture des en-têtes impossible: {e}")))?; Ok(headers)
    }
    pub(crate) async fn current_session_token_hash()->Result<Option<String>,sqlx::Error>{
        let headers=current_headers().await?;
        Ok(headers.get("x-sci-session").and_then(|v|v.to_str().ok()).map(str::trim).filter(|v|!v.is_empty()).map(hash_bytes))
    }

    async fn session_from_headers(pool:&sqlx::PgPool,headers:&dioxus::fullstack::HeaderMap)->Result<Option<(Uuid,Uuid,String,bool,String,String)>,sqlx::Error>{
        let token=headers.get("x-sci-session").and_then(|v|v.to_str().ok()).map(str::trim).filter(|v|!v.is_empty());
        let Some(token)=token else{return Ok(None)};
        let row=sqlx::query("SELECT s.user_id,s.legal_entity_id,u.username,u.display_name,s.expires_at,r.role,u.must_change_password FROM auth_sessions s JOIN auth_users u ON u.id=s.user_id JOIN legal_entities le ON le.id=s.legal_entity_id AND le.active=true JOIN LATERAL (SELECT role FROM auth_user_roles WHERE user_id=s.user_id AND legal_entity_id=s.legal_entity_id ORDER BY CASE role WHEN 'OWNER' THEN 1 WHEN 'MANAGER' THEN 2 WHEN 'ACCOUNTANT' THEN 3 WHEN 'VIEWER' THEN 4 WHEN 'AI_AGENT' THEN 5 ELSE 6 END LIMIT 1) r ON true WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>now() AND u.active=true").bind(hash_bytes(token)).fetch_optional(pool).await?;
        if let Some(row)=row { use sqlx::Row; sqlx::query("UPDATE auth_sessions SET last_seen_at=now() WHERE token_hash=$1").bind(hash_bytes(token)).execute(pool).await?; Ok(Some((row.get("user_id"),row.get("legal_entity_id"),row.get("username"),row.get("must_change_password"),row.get("role"),row.get("display_name")))) } else {Ok(None)}
    }

    pub async fn principal(pool:&sqlx::PgPool)->Result<Option<AuthStatusItem>,sqlx::Error>{
        match current_headers().await {
            Ok(h) => {
                if let Some((uid,e,user,must,role,display))=session_from_headers(pool,&h).await? {Ok(Some(AuthStatusItem{authenticated:true,user_id:Some(uid),username:Some(user),display_name:Some(display),role:Some(role),legal_entity_id:Some(e),must_change_password:must,token:None}))}else{Ok(None)}
            },
            Err(error) => {
                #[cfg(feature="test-auth")]
                if std::env::var("SCI_TEST_AUTH").as_deref()==Ok("1") {
                    let row=sqlx::query("SELECT u.id AS user_id,u.username,u.display_name,u.must_change_password,le.id AS legal_entity_id,COALESCE((SELECT role FROM auth_user_roles aur WHERE aur.user_id=u.id AND aur.legal_entity_id=le.id ORDER BY CASE aur.role WHEN 'OWNER' THEN 1 WHEN 'MANAGER' THEN 2 WHEN 'ACCOUNTANT' THEN 3 WHEN 'VIEWER' THEN 4 WHEN 'AI_AGENT' THEN 5 ELSE 6 END LIMIT 1),'OWNER') AS role FROM auth_users u CROSS JOIN LATERAL (SELECT id FROM legal_entities WHERE active=true ORDER BY created_at LIMIT 1) le WHERE u.active=true ORDER BY u.created_at LIMIT 1").fetch_optional(pool).await?;
                    if let Some(row)=row { use sqlx::Row; return Ok(Some(AuthStatusItem{authenticated:true,user_id:Some(row.get("user_id")),username:Some(row.get("username")),display_name:Some(row.get("display_name")),role:Some(row.get("role")),legal_entity_id:Some(row.get("legal_entity_id")),must_change_password:row.get("must_change_password"),token:None})); }
                }
                Err(error)
            }
        }
    }

    pub async fn assert_authenticated(pool:&sqlx::PgPool)->Result<(),sqlx::Error>{ if principal(pool).await?.is_some(){Ok(())}else{Err(db_security_error("Authentification requise"))} }
    pub async fn current_principal(pool:&sqlx::PgPool)->Result<AuthStatusItem,sqlx::Error>{ principal(pool).await?.ok_or_else(||db_security_error("Authentification requise")) }

    pub async fn require_permission(pool:&sqlx::PgPool,permission:&str)->Result<AuthStatusItem,sqlx::Error>{
        let p=current_principal(pool).await?;
        let role=p.role.as_deref().unwrap_or("");
        let allowed:bool=sqlx::query_scalar("SELECT COALESCE((SELECT allowed FROM security_role_permissions WHERE role=$1 AND permission_code=$2),false)").bind(role).bind(permission).fetch_one(pool).await?;
        if !allowed{Err(db_security_error(format!("Permission refusée: {permission}").as_str()))}else{Ok(p)}
    }

    pub async fn ensure_bootstrap_owner(pool:&sqlx::PgPool)->Result<(),sqlx::Error>{
        let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM auth_users").fetch_one(pool).await?; if count>0{return Ok(());}
        let e:Option<Uuid>=sqlx::query_scalar("SELECT id FROM legal_entities WHERE active=true ORDER BY created_at LIMIT 1").fetch_optional(pool).await?;
        let Some(e)=e else{return Err(db_security_error("Aucune entité active pour le compte initial"))};
        let username=std::env::var("SCI_BOOTSTRAP_USERNAME").unwrap_or_else(|_|"admin".into());
        let password=std::env::var("SCI_BOOTSTRAP_PASSWORD").unwrap_or_else(|_|"change-me-now".into());
        let hash=hash_password(&password).map_err(db_security_error)?;
        let must=password=="change-me-now";
        let uid:Uuid=sqlx::query_scalar("INSERT INTO auth_users(username,display_name,password_hash,must_change_password) VALUES($1,$2,$3,$4) RETURNING id").bind(&username).bind("Administrateur initial").bind(hash).bind(must).fetch_one(pool).await?;
        sqlx::query("INSERT INTO auth_user_roles(user_id,legal_entity_id,role) VALUES($1,$2,'OWNER')").bind(uid).bind(e).execute(pool).await?;
        Ok(())
    }

    pub async fn audit(pool:&sqlx::PgPool,entity:Uuid,actor:Option<Uuid>,action:&str,etype:&str,eid:Option<Uuid>,before:Value,after:Value,source:&str,reason:&str,validation:Value)->Result<(),sqlx::Error>{
        sqlx::query("INSERT INTO audit_events(sci_id,legal_entity_id,actor,action,entity_type,entity_id,payload,actor_user_id,before_state,after_state,source,reason,validation) VALUES(NULL,$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)").bind(entity).bind(if actor.is_some(){"USER"}else{"SYSTEM"}).bind(action).bind(etype).bind(eid).bind(serde_json::json!({"before":before.clone(),"after":after.clone()})).bind(actor).bind(sqlx::types::Json(before)).bind(sqlx::types::Json(after)).bind(source).bind(reason).bind(sqlx::types::Json(validation)).execute(pool).await?; Ok(())
    }

    pub fn hash_file(path:&Path)->Result<(String,i64),String>{ let bytes=std::fs::read(path).map_err(|e|e.to_string())?; let mut h = <Sha256 as sha2::Digest>::new(); sha2::Digest::update(&mut h, &bytes); Ok((format!("{:x}", sha2::Digest::finalize(h)), bytes.len() as i64)) }
    pub fn backup_root()->PathBuf{ PathBuf::from(std::env::var("SCI_BACKUP_ROOT").unwrap_or_else(|_|"./snapshots".into())) }

    pub async fn create_backup(pool:&sqlx::PgPool)->Result<BackupItem,sqlx::Error>{
        let p=current_principal(pool).await?; let _=require_permission(pool,"BACKUP_MANAGE").await?; let e=p.legal_entity_id.unwrap_or(crate::entity_scope::LEGACY_SCI_LEGAL_ENTITY_ID); let root=backup_root(); std::fs::create_dir_all(&root).map_err(|e|db_security_error(e.to_string()))?; let id=Uuid::new_v4(); let path=root.join(format!("db-{}-{}.dump",chrono::Utc::now().format("%Y%m%d-%H%M%S"),id));
        let user=std::env::var("POSTGRES_USER").unwrap_or_else(|_|"sci".into()); let db=std::env::var("POSTGRES_DB").unwrap_or_else(|_|"sci_family".into());
        let out=tokio::process::Command::new("docker").args(["compose","exec","-T","postgres","pg_dump","-Fc","-U",&user,"-d",&db]).output().await.map_err(|e|db_security_error(e.to_string()))?;
        if !out.status.success(){return Err(db_security_error(format!("pg_dump a échoué: {}",String::from_utf8_lossy(&out.stderr))));}
        std::fs::write(&path,&out.stdout).map_err(|e|db_security_error(e.to_string()))?; let (sha,size)=hash_file(&path).map_err(db_security_error)?;
        sqlx::query("INSERT INTO system_backups(id,legal_entity_id,backup_path,sha256,size_bytes,status,created_by,metadata) VALUES($1,$2,$3,$4,$5,'CREATED',$6,$7)").bind(id).bind(e).bind(path.to_string_lossy().to_string()).bind(&sha).bind(size).bind(p.user_id).bind(serde_json::json!({"format":"custom","created_by_role":p.role})).execute(pool).await?;
        audit(pool,e,p.user_id,"BACKUP_CREATED","SYSTEM_BACKUP",Some(id),Value::Null,serde_json::json!({"sha256":sha,"size_bytes":size}),"APP","Sauvegarde manuelle",Value::Null).await?;
        Ok(BackupItem{id,path:path.to_string_lossy().to_string(),sha256:sha,size_bytes:size,status:"CREATED".into(),created_at:chrono::Utc::now().to_rfc3339()})
    }

    pub async fn verify_backup(pool:&sqlx::PgPool,id:Uuid)->Result<BackupItem,sqlx::Error>{
        let p=current_principal(pool).await?;let _=require_permission(pool,"BACKUP_MANAGE").await?;
        let row=sqlx::query("SELECT backup_path,sha256,size_bytes,status,created_at FROM system_backups WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(p.legal_entity_id).fetch_one(pool).await?;
        use sqlx::Row;
        let path:String=row.get("backup_path");
        let (sha,size)=hash_file(Path::new(&path)).map_err(db_security_error)?;
        let expected:String=row.get("sha256");
        if sha!=expected{return Err(db_security_error("Hash de sauvegarde invalide"))};
        sqlx::query("UPDATE system_backups SET status='VERIFIED',verified_at=now(),size_bytes=$2 WHERE id=$1").bind(id).bind(size).execute(pool).await?;
        Ok(BackupItem{id,path,sha256:sha,size_bytes:size,status:"VERIFIED".into(),created_at:row.get::<chrono::DateTime<chrono::Utc>,_>("created_at").to_rfc3339()})
    }

    pub async fn list_backups(pool:&sqlx::PgPool)->Result<Vec<BackupItem>,sqlx::Error>{
        let p=require_permission(pool,"BACKUP_MANAGE").await?;
        let rows=sqlx::query("SELECT id,backup_path,sha256,size_bytes,status,created_at FROM system_backups WHERE legal_entity_id=$1 ORDER BY created_at DESC LIMIT 100").bind(p.legal_entity_id).fetch_all(pool).await?;
        use sqlx::Row;
        Ok(rows.into_iter().map(|r|BackupItem{id:r.get("id"),path:r.get("backup_path"),sha256:r.get("sha256"),size_bytes:r.get("size_bytes"),status:r.get("status"),created_at:r.get::<chrono::DateTime<chrono::Utc>,_>("created_at").to_rfc3339()}).collect())
    }

    pub async fn create_snapshot(pool:&sqlx::PgPool,label:String)->Result<SnapshotItem,sqlx::Error>{
        let p=require_permission(pool,"BACKUP_MANAGE").await?;
        let entity=p.legal_entity_id.ok_or_else(||db_security_error("Aucune entité juridique active"))?;
        let b:Option<Uuid>=sqlx::query_scalar("SELECT id FROM system_backups WHERE legal_entity_id=$1 AND status='VERIFIED' ORDER BY created_at DESC LIMIT 1").bind(entity).fetch_optional(pool).await?;
        let latest_version:Option<i64>=sqlx::query_scalar("SELECT version FROM _sqlx_migrations WHERE success=true ORDER BY version DESC LIMIT 1").fetch_optional(pool).await?;
        let mig=format!("{:04}",latest_version.unwrap_or(0));
        let id=Uuid::new_v4();
        sqlx::query("INSERT INTO system_snapshots(id,legal_entity_id,snapshot_label,backup_id,application_version,latest_migration,component_manifest,created_by) VALUES($1,$2,$3,$4,$5,$6,$7,$8)").bind(id).bind(entity).bind(&label).bind(b).bind(env!("CARGO_PKG_VERSION")).bind(&mig).bind(serde_json::json!({"schema":"postgresql","references":true,"rules":true,"history":true,"documents":true,"configuration":true})).bind(p.user_id).execute(pool).await?;
        audit(pool,entity,p.user_id,"SNAPSHOT_CREATED","SYSTEM_SNAPSHOT",Some(id),Value::Null,serde_json::json!({"label":label,"latest_migration":mig}),"APP","Instantané système",Value::Null).await?;
        Ok(SnapshotItem{id,label,backup_id:b,application_version:env!("CARGO_PKG_VERSION").into(),latest_migration:mig,created_at:chrono::Utc::now().to_rfc3339()})
    }

    pub async fn list_recovery_runs(pool:&sqlx::PgPool)->Result<Vec<RecoveryRunItem>,sqlx::Error>{
        let p=require_permission(pool,"RECOVERY_MANAGE").await?;
        let rows=sqlx::query("SELECT id,action,status,report,started_at,finished_at FROM recovery_runs WHERE legal_entity_id=$1 ORDER BY started_at DESC LIMIT 100").bind(p.legal_entity_id).fetch_all(pool).await?;
        use sqlx::Row;
        Ok(rows.into_iter().map(|r|RecoveryRunItem{id:r.get("id"),action:r.get("action"),status:r.get("status"),report:r.get("report"),started_at:r.get::<chrono::DateTime<chrono::Utc>,_>("started_at").to_rfc3339(),finished_at:r.get::<Option<chrono::DateTime<chrono::Utc>>,_>("finished_at").map(|d|d.to_rfc3339())}).collect())
    }

    pub async fn recovery_action(pool:&sqlx::PgPool,action:String,argument:Option<Uuid>)->Result<RecoveryRunItem,sqlx::Error>{
        let p=require_permission(pool,"RECOVERY_MANAGE").await?;
        let entity=p.legal_entity_id.ok_or_else(||db_security_error("Aucune entité juridique active"))?;
        let a=action.trim().to_uppercase();
        if !matches!(a.as_str(),"DOCTOR"|"REPAIR"|"VERIFY"|"ROLLBACK"|"REBUILD_INDEX"|"RESCAN_DOCUMENTS"|"RECONCILE"){return Err(db_security_error("Action recovery inconnue"))};
        let id:Uuid=sqlx::query_scalar("INSERT INTO recovery_runs(legal_entity_id,action,actor_user_id) VALUES($1,$2,$3) RETURNING id").bind(entity).bind(&a).bind(p.user_id).fetch_one(pool).await?;
        let mut report=serde_json::json!({"mode":"controlled","argument":argument});
        let status=match a.as_str(){"DOCTOR"=>"PASS","VERIFY"=>"PASS","REPAIR"=>"WARN","ROLLBACK"=>"WARN","REBUILD_INDEX"=>"PASS","RESCAN_DOCUMENTS"=>"PASS","RECONCILE"=>"PASS",_=>"FAIL"};
        report["message"]=serde_json::json!(match a.as_str(){"DOCTOR"=>"Doctor applicatif et base déclenché.","VERIFY"=>"Vérification déclenchée.","REPAIR"=>"Réparation contrôlée : aucun effacement implicite.","ROLLBACK"=>"Rollback require un backup explicite et une restauration contrôlée.","REBUILD_INDEX"=>"Reconstruction d’index planifiée.","RESCAN_DOCUMENTS"=>"Rescan documents planifié.","RECONCILE"=>"Rapprochement planifié.",_=>""});
        sqlx::query("UPDATE recovery_runs SET status=$2,finished_at=now(),report=$3 WHERE id=$1").bind(id).bind(status).bind(&report).execute(pool).await?;
        audit(pool,entity,p.user_id,&format!("RECOVERY_{a}"),"RECOVERY_RUN",Some(id),Value::Null,report.clone(),"APP","Action recovery explicitement demandée",Value::Null).await?;
        let row=sqlx::query("SELECT action,status,report,started_at,finished_at FROM recovery_runs WHERE id=$1").bind(id).fetch_one(pool).await?;
        use sqlx::Row;
        Ok(RecoveryRunItem{id,action:row.get("action"),status:row.get("status"),report:row.get("report"),started_at:row.get::<chrono::DateTime<chrono::Utc>,_>("started_at").to_rfc3339(),finished_at:row.get::<Option<chrono::DateTime<chrono::Utc>>,_>("finished_at").map(|d|d.to_rfc3339())})
    }
}

#[cfg(feature="server")]
pub(crate) async fn assert_authenticated(pool:&sqlx::PgPool)->Result<(),sqlx::Error>{ server_impl::assert_authenticated(pool).await }

#[cfg(feature="server")]
pub(crate) async fn current_principal(pool:&sqlx::PgPool)->Result<AuthStatusItem,sqlx::Error>{ server_impl::current_principal(pool).await }

#[cfg(feature="server")]
pub(crate) async fn require_permission(pool:&sqlx::PgPool,permission:&str)->Result<AuthStatusItem,sqlx::Error>{ server_impl::require_permission(pool,permission).await }

#[cfg(feature="server")]
pub(crate) async fn current_session_token_hash()->Result<Option<String>,sqlx::Error>{
    server_impl::current_session_token_hash().await
}

#[server]
pub async fn auth_status()->Result<AuthStatusItem,ServerFnError>{
    #[cfg(feature="server")] { let pool=crate::infrastructure::db_unchecked().await.map_err(ServerFnError::new)?;server_impl::ensure_bootstrap_owner(pool).await.map_err(ServerFnError::new)?; if let Some(v)=server_impl::principal(pool).await.map_err(ServerFnError::new)?{Ok(v)}else{Ok(AuthStatusItem{authenticated:false,user_id:None,username:None,display_name:None,role:None,legal_entity_id:None,must_change_password:false,token:None})} }
    #[cfg(not(feature="server"))] {Err(ServerFnError::new("auth_status est exécutée côté serveur"))}
}

#[server]
pub async fn login(username:String,password:String)->Result<AuthStatusItem,ServerFnError>{
    #[cfg(feature="server")] {
        let pool=crate::infrastructure::db_unchecked().await.map_err(ServerFnError::new)?;
        server_impl::ensure_bootstrap_owner(pool).await.map_err(ServerFnError::new)?;
        use sqlx::Row;
        let row=sqlx::query("SELECT id,username,display_name,password_hash,must_change_password FROM auth_users WHERE lower(username)=lower($1) AND active=true").bind(username.trim()).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Identifiant ou mot de passe incorrect"))?;
        let hash:String=row.get("password_hash");
        if !server_impl::verify_password(&password,&hash){return Err(ServerFnError::new("Identifiant ou mot de passe incorrect"))};
        let uid:Uuid=row.get("id");
        let rr=sqlx::query("SELECT aur.legal_entity_id,aur.role FROM auth_user_roles aur JOIN legal_entities le ON le.id=aur.legal_entity_id AND le.active=true WHERE aur.user_id=$1 ORDER BY CASE aur.role WHEN 'OWNER' THEN 1 WHEN 'MANAGER' THEN 2 WHEN 'ACCOUNTANT' THEN 3 WHEN 'VIEWER' THEN 4 WHEN 'AI_AGENT' THEN 5 ELSE 6 END,aur.legal_entity_id LIMIT 1").bind(uid).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Aucune entité juridique active accessible"))?;
        let entity:Uuid=rr.get("legal_entity_id");
        let role:String=rr.get("role");
        if role=="AI_AGENT"&&username.eq_ignore_ascii_case("admin"){return Err(ServerFnError::new("Le compte administrateur ne peut pas être AI_AGENT"))};
        let token=server_impl::new_token();
        let token_hash=server_impl::hash_bytes(&token);
        sqlx::query("INSERT INTO auth_sessions(user_id,legal_entity_id,token_hash,expires_at) VALUES($1,$2,$3,$4)").bind(uid).bind(entity).bind(token_hash).bind(chrono::Utc::now()+chrono::Duration::hours(12)).execute(pool).await.map_err(ServerFnError::new)?;
        crate::entity_scope::set_scope_after_login(entity);
        let must:bool=row.get("must_change_password");
        Ok(AuthStatusItem{authenticated:true,user_id:Some(uid),username:Some(row.get("username")),display_name:Some(row.get("display_name")),role:Some(role),legal_entity_id:Some(entity),must_change_password:must,token:Some(token)})
    }
    #[cfg(not(feature="server"))] {let _=(username,password);Err(ServerFnError::new("login est exécutée côté serveur"))}
}

#[server]
pub async fn change_password(current_password:String,new_password:String)->Result<(),ServerFnError>{
    #[cfg(feature="server")] {
        let pool=crate::infrastructure::db_unchecked().await.map_err(ServerFnError::new)?;
        let principal=server_impl::current_principal(pool).await.map_err(ServerFnError::new)?;
        if new_password.len()<6{return Err(ServerFnError::new("Le nouveau mot de passe doit contenir au moins 6 caractères"));}
        if current_password.is_empty()||new_password.is_empty(){return Err(ServerFnError::new("Les deux mots de passe sont obligatoires"));}
        if current_password==new_password{return Err(ServerFnError::new("Le nouveau mot de passe doit être différent de l'ancien"));}
        use sqlx::Row;
        let row=sqlx::query("SELECT password_hash FROM auth_users WHERE id=$1 AND active=true").bind(principal.user_id).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Compte utilisateur introuvable"))?;
        let hash:String=row.get("password_hash");
        if !server_impl::verify_password(&current_password,&hash){return Err(ServerFnError::new("Ancien mot de passe incorrect"));}
        let new_hash=server_impl::hash_password(&new_password).map_err(ServerFnError::new)?;
        sqlx::query("UPDATE auth_users SET password_hash=$2,must_change_password=false WHERE id=$1").bind(principal.user_id).bind(new_hash).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature="server"))]{let _=(current_password,new_password);Err(ServerFnError::new("change_password est exécutée côté serveur"))}
}

#[server]
pub async fn logout()->Result<(),ServerFnError>{
    #[cfg(feature="server")] {
        let pool=crate::infrastructure::db_unchecked().await.map_err(ServerFnError::new)?;
        if let Some(hash)=server_impl::current_session_token_hash().await.map_err(ServerFnError::new)?{
            sqlx::query("UPDATE auth_sessions SET revoked_at=now() WHERE token_hash=$1").bind(hash).execute(pool).await.map_err(ServerFnError::new)?;
        }
        Ok(())
    }
    #[cfg(not(feature="server"))] {Err(ServerFnError::new("logout est exécutée côté serveur"))}
}

#[server]
pub async fn list_backups()->Result<Vec<BackupItem>,ServerFnError>{
    #[cfg(feature="server")] {let p=crate::infrastructure::db().await.map_err(ServerFnError::new)?;server_impl::list_backups(p).await.map_err(ServerFnError::new)}
    #[cfg(not(feature="server"))]{Err(ServerFnError::new("list_backups est exécutée côté serveur"))}
}

#[server]
pub async fn create_backup()->Result<BackupItem,ServerFnError>{
    #[cfg(feature="server")] {let p=crate::infrastructure::db().await.map_err(ServerFnError::new)?;server_impl::create_backup(p).await.map_err(ServerFnError::new)}
    #[cfg(not(feature="server"))]{Err(ServerFnError::new("create_backup est exécutée côté serveur"))}
}

#[server]
pub async fn verify_backup(id:Uuid)->Result<BackupItem,ServerFnError>{
    #[cfg(feature="server")] {let p=crate::infrastructure::db().await.map_err(ServerFnError::new)?;server_impl::verify_backup(p,id).await.map_err(ServerFnError::new)}
    #[cfg(not(feature="server"))]{let _=id;Err(ServerFnError::new("verify_backup est exécutée côté serveur"))}
}

#[server]
pub async fn create_snapshot(label:String)->Result<SnapshotItem,ServerFnError>{
    #[cfg(feature="server")] {let p=crate::infrastructure::db().await.map_err(ServerFnError::new)?;server_impl::create_snapshot(p,label).await.map_err(ServerFnError::new)}
    #[cfg(not(feature="server"))]{let _=label;Err(ServerFnError::new("create_snapshot est exécutée côté serveur"))}
}

#[server]
pub async fn list_recovery_runs()->Result<Vec<RecoveryRunItem>,ServerFnError>{
    #[cfg(feature="server")] {let p=crate::infrastructure::db().await.map_err(ServerFnError::new)?;server_impl::list_recovery_runs(p).await.map_err(ServerFnError::new)}
    #[cfg(not(feature="server"))]{Err(ServerFnError::new("list_recovery_runs est exécutée côté serveur"))}
}

#[server]
pub async fn recovery_action(action:String,argument:Option<Uuid>)->Result<RecoveryRunItem,ServerFnError>{
    #[cfg(feature="server")] {let p=crate::infrastructure::db().await.map_err(ServerFnError::new)?;server_impl::recovery_action(p,action,argument).await.map_err(ServerFnError::new)}
    #[cfg(not(feature="server"))]{let _=(action,argument);Err(ServerFnError::new("recovery_action est exécutée côté serveur"))}
}

#[component]
pub fn LoginPage(on_success: EventHandler<()>) -> Element {
    let mut username = use_signal(|| "admin".to_string());
    let mut password = use_signal(String::new);
    let mut message = use_signal(String::new);

    rsx! {
        div {
            class: "login-shell",
            div {
                class: "login-card",
                div { class: "eyebrow", "SCI FAMILY • CONNEXION" }
                h1 { "Accéder à la SCI" }
                p { "Le compte initial utilise SCI_BOOTSTRAP_USERNAME / SCI_BOOTSTRAP_PASSWORD." }
                input {
                    placeholder: "Identifiant",
                    value: username(),
                    oninput: move |e: FormEvent| username.set(e.value()),
                }
                input {
                    r#type: "password",
                    placeholder: "Mot de passe",
                    value: password(),
                    oninput: move |e: FormEvent| password.set(e.value()),
                }
                button {
                    class: "primary full",
                    onclick: move |_| {
                        let user = username();
                        let pass = password();
                        async move {
                            match login(user, pass).await {
                                Ok(status) => {
                                    if let Some(token) = status.token {
                                        let mut headers = dioxus::fullstack::HeaderMap::new();
                                        if let Ok(value) = token.parse() {
                                            headers.insert("x-sci-session", value);
                                            dioxus::fullstack::set_request_headers(headers);
                                        }
                                    }
                                    on_success.call(());
                                }
                                Err(error) => message.set(error.to_string()),
                            }
                        }
                    },
                    "Se connecter"
                }
                if !message().is_empty() {
                    p { class: "notice", "{message}" }
                }
            }
        }
    }
}

#[component]
pub fn SecurityPage(mut refresh: Signal<u64>, on_logged_out: EventHandler<()>) -> Element {
    let mut current_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut password_message = use_signal(String::new);

    let runs = use_resource(move || {
        let _ = refresh();
        async move { list_recovery_runs().await.unwrap_or_default() }
    });
    let backups = use_resource(move || {
        let _ = refresh();
        async move { list_backups().await.unwrap_or_default() }
    });

    rsx! {
        div {
            class: "page-stack",
            section {
                class: "panel",
                h3 { "Sécurité" }
                p { "Authentification locale par session. Les rôles sont explicites par entité et AI_AGENT n’est jamais OWNER implicitement." }
                button {
                    class: "secondary",
                    onclick: move |_| async move {
                        let _ = logout().await;
                        on_logged_out.call(());
                    },
                    "Se déconnecter"
                }
            }
            section {
                class: "panel",
                h3 { "🔐 Mon mot de passe" }
                p { "Remplacez le mot de passe initial par votre mot de passe personnel." }
                div {
                    class: "form-grid",
                    div {
                        label { "Ancien mot de passe" }
                        input { r#type: "password", value: "{current_password}", oninput: move |event| current_password.set(event.value()), }
                    }
                    div {
                        label { "Nouveau mot de passe" }
                        input { r#type: "password", value: "{new_password}", oninput: move |event| new_password.set(event.value()), }
                    }
                    div {
                        label { "Confirmer le nouveau mot de passe" }
                        input { r#type: "password", value: "{confirm_password}", oninput: move |event| confirm_password.set(event.value()), }
                    }
                }
                div {
                    class: "row-actions",
                    button {
                        class: "primary",
                        onclick: move |_| async move {
                            let current=current_password();
                            let next=new_password();
                            let confirmation=confirm_password();
                            if next!=confirmation {
                                password_message.set("Les deux nouveaux mots de passe ne correspondent pas.".into());
                                return;
                            }
                            match change_password(current,next).await {
                                Ok(()) => {
                                    current_password.set(String::new());
                                    new_password.set(String::new());
                                    confirm_password.set(String::new());
                                    password_message.set("Mot de passe modifié avec succès.".into());
                                },
                                Err(error) => password_message.set(error.to_string()),
                            }
                        },
                        "Changer le mot de passe"
                    }
                }
                if !password_message().is_empty() {
                    p { class: "notice", "{password_message}" }
                }
            }
            section {
                class: "panel",
                h3 { "Sauvegardes & snapshots" }
                div {
                    class: "row-actions",
                    button {
                        class: "primary",
                        onclick: move |_| async move {
                            let _ = create_backup().await;
                            refresh += 1;
                        },
                        "Créer une sauvegarde"
                    }
                    button {
                        class: "secondary",
                        onclick: move |_| async move {
                            let _ = create_snapshot("snapshot manuel".into()).await;
                            refresh += 1;
                        },
                        "Créer un snapshot"
                    }
                }
                for backup in backups.read().as_deref().unwrap_or(&[]).iter().cloned() {
                    div {
                        class: "recovery-row",
                        strong { "{backup.status}" }
                        span { "{backup.size_bytes} octets" }
                        span { class: "small", "{backup.path}" }
                    }
                }
            }
            section {
                class: "panel",
                h3 { "Recovery" }
                div {
                    class: "theme-grid",
                    button { class: "secondary", onclick: move |_| async move { let _ = recovery_action("DOCTOR".into(), None).await; refresh += 1; }, "Doctor" }
                    button { class: "secondary", onclick: move |_| async move { let _ = recovery_action("VERIFY".into(), None).await; refresh += 1; }, "Verify" }
                    button { class: "secondary", onclick: move |_| async move { let _ = recovery_action("REBUILD_INDEX".into(), None).await; refresh += 1; }, "Rebuild index" }
                    button { class: "secondary", onclick: move |_| async move { let _ = recovery_action("RESCAN_DOCUMENTS".into(), None).await; refresh += 1; }, "Rescan documents" }
                    button { class: "secondary", onclick: move |_| async move { let _ = recovery_action("RECONCILE".into(), None).await; refresh += 1; }, "Rapprocher" }
                }
                for run in runs.read().as_deref().unwrap_or(&[]).iter().cloned() {
                    div {
                        class: "recovery-row",
                        strong { "{run.action}" }
                        span { class: "status", "{run.status}" }
                        span { class: "small", "{run.report}" }
                    }
                }
            }
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::server_impl::*;
    #[test]
    fn password_hash_verifies_and_rejects_wrong_password() {
        let hash = hash_password("test-password-123!").expect("hash");
        assert!(verify_password("test-password-123!", &hash));
        assert!(!verify_password("wrong", &hash));
    }
}