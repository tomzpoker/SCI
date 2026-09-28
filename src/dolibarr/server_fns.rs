use dioxus::prelude::*;
use crate::dolibarr::models::{
    DolibarrBankAccount, DolibarrBankLine, DolibarrBankLineDraft, DolibarrInvoice,
    DolibarrPayment, DolibarrThirdParty,
};

#[server]
pub async fn dolibarr_list_invoices(limit: u32) -> Result<Vec<DolibarrInvoice>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client.list_invoices(limit).await.map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_list_invoices est executee cote serveur"))
}

#[server]
pub async fn dolibarr_list_third_parties(limit: u32) -> Result<Vec<DolibarrThirdParty>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client.list_third_parties(limit).await.map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_list_third_parties est executee cote serveur"))
}

#[server]
pub async fn dolibarr_list_invoice_payments(
    invoice_id: String,
) -> Result<Vec<DolibarrPayment>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .list_payments_for_invoice(&invoice_id)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_list_invoice_payments est executee cote serveur"))
}

#[server]
pub async fn dolibarr_create_invoice(
    socid: String,
    date: i64,
    lines: Vec<crate::dolibarr::models::DolibarrInvoiceLine>,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .create_invoice(&socid, date, &lines)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_create_invoice est executee cote serveur"))
}

#[server]
pub async fn dolibarr_validate_invoice(invoice_id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .validate_invoice(&invoice_id)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_validate_invoice est executee cote serveur"))
}

#[server]
pub async fn dolibarr_get_invoice_lines(
    invoice_id: String,
) -> Result<Vec<crate::dolibarr::models::DolibarrInvoiceLine>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .get_invoice_lines(&invoice_id)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_get_invoice_lines est executee cote serveur"))
}

#[server]
pub async fn dolibarr_create_payment(
    invoice_id: String,
    date: i64,
    amount: f64,
    payment_id: i32,
    account_id: i32,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .create_payment(&invoice_id, date, amount, payment_id, account_id)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_create_payment est executee cote serveur"))
}

#[server]
pub async fn dolibarr_create_third_party(
    name: String,
    email: String,
    phone: String,
    address: String,
    zip: String,
    town: String,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .create_third_party(&name, &email, &phone, &address, &zip, &town)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_create_third_party est executee cote serveur"))
}

#[server]
pub async fn dolibarr_update_third_party(
    id: String,
    name: String,
    email: String,
    phone: String,
    address: String,
    zip: String,
    town: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .update_third_party(&id, &name, &email, &phone, &address, &zip, &town)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_update_third_party est executee cote serveur"))
}

#[server]
pub async fn dolibarr_delete_third_party(id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .delete_third_party(&id)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_delete_third_party est executee cote serveur"))
}

#[server]
pub async fn dolibarr_list_bank_accounts() -> Result<Vec<DolibarrBankAccount>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client.list_bank_accounts().await.map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_list_bank_accounts est executee cote serveur"))
}

#[server]
pub async fn dolibarr_list_bank_lines(
    account_id: String,
) -> Result<Vec<DolibarrBankLine>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client.list_bank_lines(&account_id).await.map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_list_bank_lines est executee cote serveur"))
}

#[server]
pub async fn dolibarr_create_bank_line(
    account_id: String,
    dateo: i64,
    amount: f64,
    label: String,
    line_type: String,
    num_releve: String,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        let draft = DolibarrBankLineDraft {
            date: dateo,
            datev: dateo,
            amount,
            label,
            line_type,
            num_releve,
        };
        client
            .create_bank_line(&account_id, &draft)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_create_bank_line est executee cote serveur"))
}

/// Verifie qu'une chaine ne contient que des chiffres (protection injection SQL).
#[cfg(feature = "server")]
fn sanitize_id(s: &str) -> Result<String, ServerFnError> {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
        Ok(s.to_string())
    } else {
        Err(ServerFnError::new(format!("ID invalide : {}", s)))
    }
}

/// Supprime une ou plusieurs lignes bancaires de Dolibarr (SQL direct via docker exec).
/// Refuse de supprimer les lignes rapprochees (rappro = 1).
#[server]
pub async fn dolibarr_delete_bank_lines(
    account_id: String,
    line_ids: Vec<String>,
) -> Result<usize, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if line_ids.is_empty() {
            return Ok(0);
        }

        let acc_clean = sanitize_id(&account_id)?;
        let ids_clean: Vec<String> = line_ids
            .iter()
            .map(|id| sanitize_id(id))
            .collect::<Result<Vec<_>, _>>()?;

        let ids_join = ids_clean.join(",");
        let sql = format!(
            "DELETE FROM llx_bank WHERE rowid IN ({}) AND fk_account = {} AND rappro = 0;",
            ids_join, acc_clean
        );

        let output = tokio::process::Command::new("docker")
            .args([
                "exec",
                "sci-family-dolibarr-db",
                "psql",
                "-U",
                "dolibarr",
                "-d",
                "dolibarr",
                "-c",
                &sql,
            ])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur exec docker : {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ServerFnError::new(format!("Erreur SQL : {}", stderr)));
        }

        // psql ecrit "DELETE N" sur stdout pour nous dire combien de lignes ont ete supprimees
        let stdout = String::from_utf8_lossy(&output.stdout);
        let count = stdout
            .lines()
            .find(|l| l.trim_start().starts_with("DELETE"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(ids_clean.len());

        Ok(count)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "dolibarr_delete_bank_lines est executee cote serveur",
    ))
}