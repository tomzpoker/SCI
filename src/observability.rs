use std::fmt;

use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogDomain {
    Application,
    Security,
    Business,
    Fiscal,
    Automation,
    Ocr,
    Bank,
    Ai,
    Migration,
}

impl LogDomain {
    pub const ALL: [Self; 9] = [
        Self::Application,
        Self::Security,
        Self::Business,
        Self::Fiscal,
        Self::Automation,
        Self::Ocr,
        Self::Bank,
        Self::Ai,
        Self::Migration,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::Security => "security",
            Self::Business => "business",
            Self::Fiscal => "fiscal",
            Self::Automation => "automation",
            Self::Ocr => "ocr",
            Self::Bank => "bank",
            Self::Ai => "ai",
            Self::Migration => "migration",
        }
    }
}

impl fmt::Display for LogDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

pub fn new_error_id() -> String {
    format!("ERR-{}", Uuid::new_v4())
}

pub fn record_error<E>(domain: LogDomain, error: &E, message: &'static str) -> String
where
    E: fmt::Debug + ?Sized,
{
    let error_id = new_error_id();
    tracing::error!(
        error_id = %error_id,
        domain = %domain,
        error = ?error,
        event = "error",
        "{message}"
    );
    error_id
}

pub fn record_warning(message: &'static str, domain: LogDomain, detail: impl fmt::Debug) -> String {
    let error_id = new_error_id();
    tracing::warn!(
        error_id = %error_id,
        domain = %domain,
        detail = ?detail,
        event = "warning",
        "{message}"
    );
    error_id
}

#[cfg(feature = "server")]
pub fn init() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .try_init();
}

#[cfg(not(feature = "server"))]
pub fn init() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_id_has_stable_prefix_and_uuid_shape() {
        let id = new_error_id();
        assert!(id.starts_with("ERR-"));
        let uuid_part = id.strip_prefix("ERR-").expect("prefix");
        assert!(Uuid::parse_str(uuid_part).is_ok());
    }

    #[test]
    fn all_required_domains_are_declared() {
        let expected = [
            "application",
            "security",
            "business",
            "fiscal",
            "automation",
            "ocr",
            "bank",
            "ai",
            "migration",
        ];
        let actual: Vec<_> = LogDomain::ALL.iter().map(|d| d.as_str()).collect();
        assert_eq!(actual, expected);
    }
}
