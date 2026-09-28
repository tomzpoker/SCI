use crate::dolibarr::models::{
    DolibarrBankAccount, DolibarrBankLine, DolibarrBankLineDraft, DolibarrInvoice,
    DolibarrInvoiceLine, DolibarrPayment, DolibarrThirdParty,
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

    fn extract_id(resp: &serde_json::Value) -> Option<String> {
        if let Some(n) = resp.as_i64() {
            return Some(n.to_string());
        }
        if let Some(s) = resp.as_str() {
            return Some(s.to_string());
        }
        if let Some(id) = resp.get("id") {
            if let Some(s) = id.as_str() {
                return Some(s.to_string());
            }
            if let Some(n) = id.as_i64() {
                return Some(n.to_string());
            }
        }
        if let Some(arr) = resp.as_array() {
            if let Some(first) = arr.first() {
                if let Some(id) = first.get("id") {
                    if let Some(s) = id.as_str() {
                        return Some(s.to_string());
                    }
                    if let Some(n) = id.as_i64() {
                        return Some(n.to_string());
                    }
                }
                if let Some(n) = first.as_i64() {
                    return Some(n.to_string());
                }
                if let Some(s) = first.as_str() {
                    return Some(s.to_string());
                }
            }
        }
        None
    }

    fn json_to_string(v: Option<&serde_json::Value>) -> String {
        match v {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::Bool(b)) => b.to_string(),
            _ => String::new(),
        }
    }

    fn json_to_i64(v: Option<&serde_json::Value>) -> i64 {
        match v {
            Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(0),
            Some(serde_json::Value::String(s)) => {
                if s.is_empty() {
                    0
                } else {
                    s.parse::<i64>().unwrap_or_else(|_| {
                        s.parse::<f64>().map(|f| f as i64).unwrap_or(0)
                    })
                }
            }
            _ => 0,
        }
    }

    // FACTURES - lecture

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

    // FACTURES - ecriture

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
        Self::extract_id(&resp)
            .ok_or_else(|| format!("Reponse Dolibarr inattendue : {resp}"))
    }

    pub async fn validate_invoice(&self, id: &str) -> Result<(), String> {
        let _: serde_json::Value = self
            .post_json(&format!("invoices/{}/validate", id), &serde_json::json!({}))
            .await?;
        Ok(())
    }

    // PAIEMENTS

    pub async fn list_payments_for_invoice(
        &self,
        invoice_id: &str,
    ) -> Result<Vec<DolibarrPayment>, String> {
        self.get_json(&format!("invoices/{}/payments", invoice_id))
            .await
    }

    pub async fn create_payment(
        &self,
        invoice_id: &str,
        date: i64,
        amount: f64,
        payment_id: i32,
        account_id: i32,
    ) -> Result<String, String> {
        #[derive(serde::Serialize)]
        struct CreatePaymentBody {
            datepaye: i64,
            paymentid: i32,
            closepaidinvoices: &'static str,
            accountid: i32,
            amount: f64,
        }
        let body = CreatePaymentBody {
            datepaye: date,
            paymentid: payment_id,
            closepaidinvoices: "yes",
            accountid: account_id,
            amount,
        };
        let path = format!("invoices/{}/payments", invoice_id);
        let resp: serde_json::Value = self.post_json(&path, &body).await?;
        Ok(Self::extract_id(&resp).unwrap_or_else(|| resp.to_string()))
    }

    // TIERS

    pub async fn list_third_parties(&self, limit: u32) -> Result<Vec<DolibarrThirdParty>, String> {
        let path = format!("thirdparties?limit={}&sortfield=t.nom&sortorder=ASC", limit);
        self.get_json(&path).await
    }

    pub async fn get_third_party(&self, id: &str) -> Result<DolibarrThirdParty, String> {
        self.get_json(&format!("thirdparties/{}", id)).await
    }

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
        Self::extract_id(&resp)
            .ok_or_else(|| format!("Reponse Dolibarr inattendue : {resp}"))
    }

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

    pub async fn delete_third_party(&self, id: &str) -> Result<(), String> {
        self.delete_json(&format!("thirdparties/{}", id)).await
    }

    // BANQUE

    pub async fn list_bank_accounts(&self) -> Result<Vec<DolibarrBankAccount>, String> {
        let url = format!("{}/api/index.php/bankaccounts", self.base_url);
        let resp = self.http
            .get(&url)
            .header("DOLAPIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("HTTP {status}: {body}"));
        }

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("JSON parse error: {e}. Body: {}", &body[..body.len().min(500)]))?;

        let arr = if let Some(v) = json.get("value").and_then(|v| v.as_array()) {
            v.clone()
        } else if let Some(v) = json.as_array() {
            v.clone()
        } else {
            return Err(format!("Reponse inattendue : {}", &body[..body.len().min(500)]));
        };

        let mut result = Vec::new();
        for item in arr {
            result.push(Self::parse_bank_account(&item));
        }
        Ok(result)
    }

    fn parse_bank_account(item: &serde_json::Value) -> DolibarrBankAccount {
        DolibarrBankAccount {
            id: Self::json_to_string(item.get("id")),
            r#ref: Self::json_to_string(item.get("ref")),
            label: Self::json_to_string(item.get("label")),
            bank: Self::json_to_string(item.get("bank")),
            currency_code: Self::json_to_string(item.get("currency_code")),
            iban: Self::json_to_string(item.get("iban")),
            bic: Self::json_to_string(item.get("bic")),
            clos: Self::json_to_i64(item.get("clos")) as i32,
            courant: Self::json_to_i64(item.get("courant")) as i32,
            solde: Self::json_to_i64(item.get("solde")),
        }
    }

    pub async fn list_bank_lines(&self, account_id: &str) -> Result<Vec<DolibarrBankLine>, String> {
        let url = format!(
            "{}/api/index.php/bankaccounts/{}/lines",
            self.base_url, account_id
        );
        let resp = self.http
            .get(&url)
            .header("DOLAPIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("HTTP {status}: {body}"));
        }

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("JSON parse error: {e}. Body: {}", &body[..body.len().min(500)]))?;

        let arr = if let Some(v) = json.get("value").and_then(|v| v.as_array()) {
            v.clone()
        } else if let Some(v) = json.as_array() {
            v.clone()
        } else {
            return Ok(Vec::new());
        };

        let mut result = Vec::new();
        for item in arr {
            result.push(Self::parse_bank_line(&item));
        }
        Ok(result)
    }

    fn parse_bank_line(item: &serde_json::Value) -> DolibarrBankLine {
        DolibarrBankLine {
            id: Self::json_to_string(item.get("id")),
            dateo: Self::json_to_i64(item.get("dateo")),
            datev: Self::json_to_i64(item.get("datev")),
            amount: Self::json_to_string(item.get("amount")),
            label: Self::json_to_string(item.get("label")),
            r#type: Self::json_to_string(item.get("type")),
            num_releve: Self::json_to_string(item.get("num_releve")),
            rappro: Self::json_to_i64(item.get("rappro")) as i32,
            fk_bordereau: Self::json_to_i64(item.get("fk_bordereau")) as i32,
        }
    }

    pub async fn create_bank_line(
        &self,
        account_id: &str,
        draft: &DolibarrBankLineDraft,
    ) -> Result<String, String> {
        let path = format!("bankaccounts/{}/lines", account_id);
        let resp: serde_json::Value = self.post_json(&path, draft).await?;
        Ok(Self::extract_id(&resp).unwrap_or_else(|| resp.to_string()))
    }

    /// Supprime une ligne bancaire dans Dolibarr.
    /// Essaie plusieurs endpoints car Dolibarr n'a pas de chemin standard.
    pub async fn delete_bank_line(
        &self,
        account_id: &str,
        line_id: &str,
    ) -> Result<(), String> {
        // Essai 1 : /banklines/{id}
        let r1 = self.delete_json(&format!("banklines/{}", line_id)).await;
        if r1.is_ok() {
            return Ok(());
        }

        // Essai 2 : /bankaccounts/{id}/lines/{line_id}
        let r2 = self.delete_json(&format!("bankaccounts/{}/lines/{}", account_id, line_id)).await;
        if r2.is_ok() {
            return Ok(());
        }

        // Essai 3 : /bankaccounts/{id}/line/{line_id}
        let r3 = self.delete_json(&format!("bankaccounts/{}/line/{}", account_id, line_id)).await;
        if r3.is_ok() {
            return Ok(());
        }

        // Aucun endpoint n'a marché, on remonte l'erreur la plus parlante
        let e2 = r2.err().unwrap_or_default();
        Err(format!(
            "Aucun endpoint DELETE n'a fonctionne. Derniere erreur : {}",
            e2
        ))
    }
}

#[cfg(not(feature = "server"))]
pub struct DolibarrClient;

#[cfg(not(feature = "server"))]
impl DolibarrClient {
    pub fn from_env() -> Result<Self, String> {
        Err("DolibarrClient est execute cote serveur".into())
    }
}