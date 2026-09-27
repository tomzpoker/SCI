use dioxus::prelude::*;
use crate::dolibarr::models::{DolibarrInvoice, DolibarrPayment, DolibarrThirdParty};

#[server]
pub async fn dolibarr_list_invoices(limit: u32) -> Result<Vec<DolibarrInvoice>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client.list_invoices(limit).await.map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_list_invoices est exécutée côté serveur"))
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
    Err(ServerFnError::new("dolibarr_list_third_parties est exécutée côté serveur"))
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
    Err(ServerFnError::new("dolibarr_list_invoice_payments est exécutée côté serveur"))
}
// ============================================================
//  Sprint 2 - Factures (crÃ©ation + validation + lignes)
// ============================================================

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
    Err(ServerFnError::new("dolibarr_create_invoice est exÃ©cutÃ©e cÃ´tÃ© serveur"))
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
    Err(ServerFnError::new("dolibarr_validate_invoice est exÃ©cutÃ©e cÃ´tÃ© serveur"))
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
    Err(ServerFnError::new("dolibarr_get_invoice_lines est exÃ©cutÃ©e cÃ´tÃ© serveur"))
}

// ============================================================
//  Sprint 2 - Paiements
// ============================================================

#[server]
pub async fn dolibarr_create_payment(
    invoice_id: String,
    date: i64,
    amount: f64,
    payment_id: i32,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .create_payment(&invoice_id, date, amount, payment_id)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_create_payment est exÃ©cutÃ©e cÃ´tÃ© serveur"))
}

// ============================================================
//  Sprint 2 - Tiers (crÃ©ation, mise Ã  jour, suppression)
// ============================================================

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
    Err(ServerFnError::new("dolibarr_create_third_party est exÃ©cutÃ©e cÃ´tÃ© serveur"))
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
    Err(ServerFnError::new("dolibarr_update_third_party est exÃ©cutÃ©e cÃ´tÃ© serveur"))
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
    Err(ServerFnError::new("dolibarr_delete_third_party est exÃ©cutÃ©e cÃ´tÃ© serveur"))
}