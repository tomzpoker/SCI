use crate::dolibarr::models::{DolibarrInvoice, DolibarrPayment, DolibarrThirdParty};

#[cfg(feature = "server")]
pub struct DolibarrClient {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
}

#[cfg(feature = "server")]
impl DolibarrClient {
    pub fn from_env() -> Result<Self, String> {
        let base_url = std::env::var("DOLIBARR_URL")
            .unwrap_or_else(|_| "http://localhost:8081".to_string());
        let api_key = std::env::var("DOLIBARR_API_KEY")
            .map_err(|_| "DOLIBARR_API_KEY manquant dans .env".to_string())?;

        Ok(Self {
            base_url,
            api_key,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|e| e.to_string())?,
        })
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let url = format!("{}/api/index.php/{}", self.base_url, path);
        let resp = self.http
            .get(&url)
            .header("DOLAPIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }

        resp.json::<T>().await.map_err(|e| format!("JSON error: {e}"))
    }

    pub async fn list_invoices(&self, limit: u32) -> Result<Vec<DolibarrInvoice>, String> {
        let path = format!("invoices?limit={}&sortfield=t.datec&sortorder=DESC", limit);
        self.get_json(&path).await
    }

    pub async fn get_invoice(&self, id: &str) -> Result<DolibarrInvoice, String> {
        self.get_json(&format!("invoices/{}", id)).await
    }

    /// Récupère la liste des paiements (encaissements) liés à une facture.
    /// Endpoint Dolibarr : GET /invoices/{id}/payments
    pub async fn list_payments_for_invoice(
        &self,
        invoice_id: &str,
    ) -> Result<Vec<DolibarrPayment>, String> {
        self.get_json(&format!("invoices/{}/payments", invoice_id))
            .await
    }

    pub async fn list_third_parties(&self, limit: u32) -> Result<Vec<DolibarrThirdParty>, String> {
        let path = format!("thirdparties?limit={}&sortfield=t.nom&sortorder=ASC", limit);
        self.get_json(&path).await
    }

    pub async fn get_third_party(&self, id: &str) -> Result<DolibarrThirdParty, String> {
        self.get_json(&format!("thirdparties/{}", id)).await
    }
}

#[cfg(not(feature = "server"))]
pub struct DolibarrClient;

#[cfg(not(feature = "server"))]
impl DolibarrClient {
    pub fn from_env() -> Result<Self, String> {
        Err("DolibarrClient est exécuté côté serveur".into())
    }
}