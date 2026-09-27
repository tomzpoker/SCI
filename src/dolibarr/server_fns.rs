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