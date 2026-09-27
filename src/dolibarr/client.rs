use crate::dolibarr::models::{
    DolibarrInvoice, DolibarrInvoiceLine, DolibarrPayment, DolibarrThirdParty,
};

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
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| e.to_string())?,
        })
    }

    // ------------------------------------------------------------------------
    //  Helpers HTTP
    // ------------------------------------------------------------------------

    async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let url = format!("{}/api/index.php/{}", self.base_url, path);
        let resp = self.http
            .get(&url)
            .header("DOLAPIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;
        Self::handle_response(resp).await
    }

    async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let url = format!("{}/api/index.php/{}", self.base_url, path);
        let resp = self.http
            .post(&url)
            .header("DOLAPIKEY", &self.api_key)
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;
        Self::handle_response(resp).await
    }

    async fn put_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let url = format!("{}/api/index.php/{}", self.base_url, path);
        let resp = self.http
            .put(&url)
            .header("DOLAPIKEY", &self.api_key)
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;
        Self::handle_response(resp).await
    }

    async fn delete_json(&self, path: &str) -> Result<(), String> {
        let url = format!("{}/api/index.php/{}", self.base_url, path);
        let resp = self.http
            .delete(&url)
            .header("DOLAPIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }
        Ok(())
    }

    async fn handle_response<T: serde::de::DeserializeOwned>(
        resp: reqwest::Response,
    ) -> Result<T, String> {
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }
        resp.json::<T>().await.map_err(|e| format!("JSON error: {e}"))
    }

    // ------------------------------------------------------------------------
    //  FACTURES - lecture
    // ------------------------------------------------------------------------

    pub async fn list_invoices(&self, limit: u32) -> Result<Vec<DolibarrInvoice>, String> {
        let path = format!("invoices?limit={}&sortfield=t.datec&sortorder=DESC", limit);
        self.get_json(&path).await
    }

    pub async fn get_invoice(&self, id: &str) -> Result<DolibarrInvoice, String> {
        self.get_json(&format!("invoices/{}", id)).await
    }

    pub async fn get_invoice_lines(&self, id: &str) -> Result<Vec<DolibarrInvoiceLine>, String> {
        self.get_json(&format!("invoices/{}/lines", id)).await
    }

    // ------------------------------------------------------------------------
    //  FACTURES - Ã©criture (Sprint 4)
    // ------------------------------------------------------------------------

    /// CrÃ©e une facture brouillon dans Dolibarr.
    /// - `socid` : ID du tiers client
    /// - `date` : timestamp Unix (seconds)
    /// - `lines` : lignes de facture
    pub async fn create_invoice(
        &self,
        socid: &str,
        date: i64,
        lines: &[DolibarrInvoiceLine],
    ) -> Result<String, String> {
        #[derive(serde::Serialize)]
        struct CreateInvoiceBody<'a> {
            socid: &'a str,
            date: i64,
            lines: &'a [DolibarrInvoiceLine],
        }
        let body = CreateInvoiceBody { socid, date, lines };
        let resp: serde_json::Value = self.post_json("invoices", &body).await?;
        resp.get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| format!("RÃ©ponse Dolibarr inattendue : {resp}"))
    }

    /// Valide une facture brouillon (statut passe Ã  "impayÃ©e").
    pub async fn validate_invoice(&self, id: &str) -> Result<(), String> {
        let _: serde_json::Value = self
            .post_json(&format!("invoices/{}/validate", id), &serde_json::json!({}))
            .await?;
        Ok(())
    }

    // ------------------------------------------------------------------------
    //  PAIEMENTS
    // ------------------------------------------------------------------------

    /// Liste les paiements (encaissements) liÃ©s Ã  une facture.
    pub async fn list_payments_for_invoice(
        &self,
        invoice_id: &str,
    ) -> Result<Vec<DolibarrPayment>, String> {
        self.get_json(&format!("invoices/{}/payments", invoice_id))
            .await
    }

    /// Enregistre un paiement sur une facture.
    /// - `date` : timestamp Unix (seconds)
    /// - `amount` : montant en unitÃ©s (pas en cents)
    /// - `payment_id` : ID du mode de paiement (1 = virement, 2 = chÃ¨que, 3 = espÃ¨ces, etc.)
    pub async fn create_payment(
        &self,
        invoice_id: &str,
        date: i64,
        amount: f64,
        payment_id: i32,
    ) -> Result<String, String> {
        #[derive(serde::Serialize)]
        struct CreatePaymentBody {
            datepaye: i64,
            paymentid: i32,
            closepaidinvoices: &'static str,
            chid: i32,
            amount: f64,
        }
        let body = CreatePaymentBody {
            datepaye: date,
            paymentid: payment_id,
            closepaidinvoices: "yes",
            chid: 1,
            amount,
        };
        let path = format!("invoices/{}/payments", invoice_id);
        let resp: serde_json::Value = self.post_json(&path, &body).await?;
        Ok(resp.to_string())
    }

    // ------------------------------------------------------------------------
    //  TIERS
    // ------------------------------------------------------------------------

    pub async fn list_third_parties(&self, limit: u32) -> Result<Vec<DolibarrThirdParty>, String> {
        let path = format!("thirdparties?limit={}&sortfield=t.nom&sortorder=ASC", limit);
        self.get_json(&path).await
    }

    pub async fn get_third_party(&self, id: &str) -> Result<DolibarrThirdParty, String> {
        self.get_json(&format!("thirdparties/{}", id)).await
    }

    /// CrÃ©e un tiers (client). Retourne l'ID Dolibarr.
    pub async fn create_third_party(
        &self,
        name: &str,
        email: &str,
        phone: &str,
        address: &str,
        zip: &str,
        town: &str,
    ) -> Result<String, String> {
        #[derive(serde::Serialize)]
        struct CreateThirdPartyBody<'a> {
            name: &'a str,
            email: &'a str,
            phone: &'a str,
            address: &'a str,
            zip: &'a str,
            town: &'a str,
            client: i32,
            code_client: &'static str,
        }
        let body = CreateThirdPartyBody {
            name,
            email,
            phone,
            address,
            zip,
            town,
            client: 1,
            code_client: "-1",
        };
        let resp: serde_json::Value = self.post_json("thirdparties", &body).await?;
        resp.get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| resp.get("id").and_then(|v| v.as_i64()).map(|n| n.to_string()))
            .ok_or_else(|| format!("RÃ©ponse Dolibarr inattendue : {resp}"))
    }

    /// Met Ã  jour un tiers existant.
    pub async fn update_third_party(
        &self,
        id: &str,
        name: &str,
        email: &str,
        phone: &str,
        address: &str,
        zip: &str,
        town: &str,
    ) -> Result<(), String> {
        #[derive(serde::Serialize)]
        struct UpdateThirdPartyBody<'a> {
            name: &'a str,
            email: &'a str,
            phone: &'a str,
            address: &'a str,
            zip: &'a str,
            town: &'a str,
        }
        let body = UpdateThirdPartyBody { name, email, phone, address, zip, town };
        let _: serde_json::Value = self
            .put_json(&format!("thirdparties/{}", id), &body)
            .await?;
        Ok(())
    }

    /// Supprime un tiers.
    pub async fn delete_third_party(&self, id: &str) -> Result<(), String> {
        self.delete_json(&format!("thirdparties/{}", id)).await
    }
}

#[cfg(not(feature = "server"))]
pub struct DolibarrClient;

#[cfg(not(feature = "server"))]
impl DolibarrClient {
    pub fn from_env() -> Result<Self, String> {
        Err("DolibarrClient est exÃ©cutÃ© cÃ´tÃ© serveur".into())
    }
}