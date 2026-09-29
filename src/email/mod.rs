//! Client IMAP pour recuperer les factures recues par email.
//! Connexion a Gmail via mot de passe d'application.
//!
//! Variables d'environnement requises (voir .env.example) :
//!   GMAIL_IMAP_HOST        (defaut: imap.gmail.com)
//!   GMAIL_IMAP_PORT        (defaut: 993)
//!   GMAIL_USER             (ex: scidegrandebretagne@gmail.com)
//!   GMAIL_APP_PASSWORD     (16 caracteres, sans espaces)
//!   GMAIL_FACTURE_FOLDER   (defaut: Facture)

use serde::{Deserialize, Serialize};

/// Resume d'un email contenant une facture (sans le PDF lui-meme).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailFacture {
    pub uid: u32,
    pub from: String,
    pub subject: String,
    pub date: String,
}

#[cfg(feature = "server")]
mod imp {
    use super::*;
    use imap::Session;
    use mail_parser::MimeHeaders;
    use native_tls::TlsConnector;

    fn read_env(key: &str, default: Option<&str>) -> Result<String, String> {
        match std::env::var(key) {
            Ok(v) if !v.trim().is_empty() => Ok(v.trim().to_string()),
            _ => match default {
                Some(d) => Ok(d.to_string()),
                None => Err(format!("Variable d'environnement manquante : {}", key)),
            },
        }
    }

    fn connect_session(
    ) -> Result<Session<native_tls::TlsStream<std::net::TcpStream>>, String> {
        let host = read_env("GMAIL_IMAP_HOST", Some("imap.gmail.com"))?;
        let port: u16 = read_env("GMAIL_IMAP_PORT", Some("993"))?
            .parse()
            .map_err(|_| "GMAIL_IMAP_PORT invalide".to_string())?;
        let user = read_env("GMAIL_USER", None)?;
        let password = read_env("GMAIL_APP_PASSWORD", None)?;

        let tls = TlsConnector::builder()
            .build()
            .map_err(|e| format!("Erreur initialisation TLS : {}", e))?;

        let client = imap::connect((host.as_str(), port), host.as_str(), &tls)
            .map_err(|e| format!("Erreur connexion IMAP {}:{} : {}", host, port, e))?;

        let session = client.login(&user, &password).map_err(|(e, _)| {
            format!(
                "Authentification Gmail refusee : {} (verifie GMAIL_USER et GMAIL_APP_PASSWORD)",
                e
            )
        })?;

        Ok(session)
    }

    /// Liste les emails non lus du dossier "Facture" (parametrable).
    pub fn scan_facture_folder() -> Result<Vec<EmailFacture>, String> {
        let mut session = connect_session()?;
        let folder = read_env("GMAIL_FACTURE_FOLDER", Some("Facture"))?;

        session.select(&folder).map_err(|e| {
            format!(
                "Dossier '{}' introuvable ou inaccessible dans Gmail : {}",
                folder, e
            )
        })?;

        let uids = session
            .uid_search("UNSEEN")
            .map_err(|e| format!("Erreur recherche IMAP : {}", e))?;

        let mut factures = Vec::new();
        for uid in uids.iter() {
            let fetch = match session.uid_fetch(uid.to_string(), "(ENVELOPE)") {
                Ok(f) => f,
                Err(_) => continue,
            };
            let Some(msg) = fetch.iter().next() else { continue };

            let (from, subject, date) = if let Some(env) = msg.envelope() {
                let from = env
                    .from
                    .as_ref()
                    .and_then(|v| v.first())
                    .map(|addr| {
                        let mb = addr
                            .mailbox
                            .as_ref()
                            .map(|m| String::from_utf8_lossy(m).to_string())
                            .unwrap_or_default();
                        let host = addr
                            .host
                            .as_ref()
                            .map(|h| String::from_utf8_lossy(h).to_string())
                            .unwrap_or_default();
                        if host.is_empty() {
                            mb
                        } else {
                            format!("{}@{}", mb, host)
                        }
                    })
                    .unwrap_or_default();
                let subject = env
                    .subject
                    .as_ref()
                    .map(|s| String::from_utf8_lossy(s).to_string())
                    .unwrap_or_default();
                let date = env
                    .date
                    .as_ref()
                    .map(|d| String::from_utf8_lossy(d).to_string())
                    .unwrap_or_default();
                (from, subject, date)
            } else {
                (String::new(), String::new(), String::new())
            };

            factures.push(EmailFacture {
                uid: *uid,
                from,
                subject,
                date,
            });
        }

        session.logout().ok();
        Ok(factures)
    }

    /// Recupere la premiere piece jointe PDF d'un email donne.
    pub fn fetch_attachment(uid: u32) -> Result<(String, Vec<u8>), String> {
        let mut session = connect_session()?;
        let folder = read_env("GMAIL_FACTURE_FOLDER", Some("Facture"))?;
        session
            .select(&folder)
            .map_err(|e| format!("Dossier '{}' introuvable : {}", folder, e))?;

        let fetch = session
            .uid_fetch(uid.to_string(), "(RFC822)")
            .map_err(|e| format!("Erreur fetch email {} : {}", uid, e))?;

        let msg = fetch
            .iter()
            .next()
            .ok_or_else(|| format!("Email {} introuvable", uid))?;

        let body = msg
            .body()
            .ok_or_else(|| format!("Corps de l'email {} absent", uid))?;

        let parsed = mail_parser::MessageParser::default()
            .parse(body)
            .ok_or_else(|| format!("Impossible de parser l'email {}", uid))?;

        let mut result: Option<(String, Vec<u8>)> = None;
        for part in parsed.attachments() {
            let name = part
                .attachment_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("facture_{}.pdf", uid));
            if name.to_lowercase().ends_with(".pdf") {
                result = Some((name, part.contents().to_vec()));
                break;
            }
        }

        session.logout().ok();

        result.ok_or_else(|| format!("Aucun PDF joint trouve dans l'email {}", uid))
    }
}

#[cfg(feature = "server")]
pub use imp::{fetch_attachment, scan_facture_folder};