use crate::documents::workflow::sha256_hex;
use crate::entity_scope::current_legal_entity_id;
use crate::ui::{FormField, InfoTileOwned, ModuleHeader};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankAccountManagementItem {
    pub id: Uuid,
    pub label: String,
    pub iban: String,
    pub bic: String,
    pub account_type: String,
    pub currency_code: String,
    pub opening_balance_cents: i64,
    pub opening_balance_date: Option<NaiveDate>,
    pub bank_name: String,
    pub is_primary: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankImportResult {
    pub batch_id: Uuid,
    pub source_type: String,
    pub rows_seen: i32,
    pub rows_imported: i32,
    pub rows_skipped: i32,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankTransactionManagementItem {
    pub id: Uuid,
    pub transaction_date: NaiveDate,
    pub value_date: Option<NaiveDate>,
    pub amount_cents: i64,
    pub debit_credit: String,
    pub label: String,
    pub counterparty: String,
    pub reference: String,
    pub balance_cents: Option<i64>,
    pub source: String,
    pub transaction_hash: String,
    pub reconciliation_level: String,
    pub reconciliation_type: String,
    pub reconciliation_confidence_bp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankReconciliationSummary {
    pub scanned: i32,
    pub matched: i32,
    pub probable: i32,
    pub to_validate: i32,
    pub anomalous: i32,
    pub unmatched: i32,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CashPositionsView {
    pub imported_bank_cents: i64,
    pub reconciled_cents: i64,
    pub theoretical_cents: i64,
    pub forecast_cents: i64,
    pub available_cents: i64,
}

#[derive(Debug, Clone)]
struct ParsedBankRecord {
    booked_date: NaiveDate,
    value_date: Option<NaiveDate>,
    label: String,
    amount_cents: i64,
    balance_cents: Option<i64>,
    reference: String,
    external_id: String,
    counterparty: String,
    raw_payload: Value,
}

fn parse_money(raw: &str) -> Option<i64> {
    let mut s = raw.trim().replace('\u{00a0}', "").replace(' ', "");
    if s.is_empty() { return None; }
    let negative = s.starts_with('(') && s.ends_with(')') || s.ends_with('-');
    s = s.trim_matches(|c| c == '(' || c == ')' || c == '-').to_string();
    let euro = s.replace("EUR", "").replace("€", "");
    let normalized = if euro.contains(',') && euro.contains('.') {
        if euro.rfind(',') > euro.rfind('.') { euro.replace('.', "").replace(',', ".") } else { euro.replace(',', "") }
    } else if euro.contains(',') { euro.replace(',', ".") } else { euro };
    let value = normalized.parse::<f64>().ok()?;
    let cents = (value * 100.0).round() as i64;
    Some(if negative { -cents } else { cents })
}

fn parse_date(raw: &str) -> Option<NaiveDate> {
    let s = raw.trim();
    for fmt in ["%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%Y%m%d", "%d.%m.%Y"] {
        if let Ok(d) = NaiveDate::parse_from_str(s, fmt) { return Some(d); }
    }
    None
}

fn split_delimited_line(line: &str, delimiter: char) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            if quoted && i + 1 < chars.len() && chars[i + 1] == '"' { current.push('"'); i += 1; }
            else { quoted = !quoted; }
        } else if c == delimiter && !quoted { values.push(current.trim().to_string()); current.clear(); }
        else { current.push(c); }
        i += 1;
    }
    values.push(current.trim().to_string());
    values
}

fn detect_delimiter(header: &str) -> char {
    [';', '\t', ','].into_iter().max_by_key(|d| header.matches(*d).count()).unwrap_or(';')
}

fn norm_header(v: &str) -> String {
    v.trim().to_lowercase().replace([' ', '_', '-', '.', '/'], "")
}

fn find_col(headers: &[String], names: &[&str]) -> Option<usize> {
    headers.iter().position(|h| {
        let n = norm_header(h);
        names.iter().any(|target| n == norm_header(target) || n.contains(&norm_header(target)))
    })
}

fn parse_csv_records(csv: &str) -> Result<(Vec<ParsedBankRecord>, usize), String> {
    let lines: Vec<&str> = csv.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() { return Ok((Vec::new(), 0)); }
    let delimiter = detect_delimiter(lines[0]);
    let first = split_delimited_line(lines[0], delimiter);
    let first_len = first.len();
    let header_like = first.iter().any(|x| {
        let n = norm_header(x);
        ["date", "datedebut", "dateoperation", "datetransaction", "montant", "libelle", "description", "debit", "credit", "fitid", "reference"]
            .iter().any(|h| n.contains(h))
    });
    let (headers, data_start) = if header_like {
        (first, 1)
    } else {
        ((0..first.len()).map(|i| format!("col{i}")).collect::<Vec<_>>(), 0)
    };
    let date_col = find_col(&headers, &["date", "date opération", "dateoperation", "booking date"]).unwrap_or(0);
    let value_col = find_col(&headers, &["date valeur", "datevaleur", "value date"]);
    let amount_col = find_col(&headers, &["montant", "amount", "transaction amount"]).or_else(|| if !header_like && first_len>1 {Some(1)} else {None});
    let debit_col = find_col(&headers, &["débit", "debit"]);
    let credit_col = find_col(&headers, &["crédit", "credit"]);
    let balance_col = find_col(&headers, &["solde", "balance"]);
    let label_col = find_col(&headers, &["libellé", "libelle", "description", "memo", "name"]).or_else(|| if !header_like && first_len>2 {Some(2)} else {None});
    let ref_col = find_col(&headers, &["reference", "ref", "référence", "refnum"]);
    let id_col = find_col(&headers, &["id", "fitid", "identifiant", "external id"]).or_else(|| if !header_like && first_len>3 {Some(3)} else {None});
    let cp_col = find_col(&headers, &["contrepartie", "counterparty", "tiers"]);
    let mut out = Vec::new();
    for (line_no, line) in lines.iter().enumerate().skip(data_start) {
        let parts = split_delimited_line(line, delimiter);
        let Some(booked_date) = parts.get(date_col).and_then(|v| parse_date(v)) else { continue };
        let amount = if let Some(c) = amount_col { parts.get(c).and_then(|v| parse_money(v)) }
            else {
                let credit = credit_col.and_then(|c| parts.get(c)).and_then(|v| parse_money(v));
                let debit = debit_col.and_then(|c| parts.get(c)).and_then(|v| parse_money(v)).unwrap_or(0).abs();
                Some(credit.unwrap_or(0) - debit)
            };
        let Some(amount_cents) = amount else { continue };
        let label = label_col.and_then(|c| parts.get(c)).cloned().filter(|v| !v.is_empty()).unwrap_or_else(|| parts.get(2).cloned().unwrap_or_default());
        let external_id = id_col.and_then(|c| parts.get(c)).cloned().unwrap_or_else(|| format!("CSV:{}:{}", booked_date, line_no + 1));
        let reference = ref_col.and_then(|c| parts.get(c)).cloned().unwrap_or_default();
        let counterparty = cp_col.and_then(|c| parts.get(c)).cloned().unwrap_or_default();
        let balance = balance_col.and_then(|c| parts.get(c)).and_then(|v| parse_money(v));
        let value_date = value_col.and_then(|c| parts.get(c)).and_then(|v| parse_date(v));
        out.push(ParsedBankRecord {
            booked_date,
            value_date,
            label,
            amount_cents,
            balance_cents: balance,
            reference,
            external_id,
            counterparty,
            raw_payload: serde_json::json!({"source":"CSV","line":line_no + 1,"raw":line}),
        });
    }
    Ok((out, lines.len()))
}

fn ofx_value(block: &str, tag: &str) -> String {
    let open = format!("<{}>", tag);
    let upper = block.to_uppercase();
    let Some(start) = upper.find(&open) else { return String::new(); };
    let tail = &block[start + open.len()..];
    tail.split('<').next().unwrap_or("").trim().to_string()
}

fn parse_ofx_date(raw: &str) -> Option<NaiveDate> { parse_date(&raw.chars().take(8).collect::<String>()) }

fn parse_ofx_records(ofx: &str) -> Result<(Vec<ParsedBankRecord>, usize), String> {
    let upper = ofx.to_uppercase();
    let mut pos = 0usize;
    let mut records = Vec::new();
    while let Some(rel) = upper[pos..].find("<STMTTRN>") {
        let start = pos + rel;
        let end = upper[start..].find("</STMTTRN>").map(|v| start + v).unwrap_or_else(|| {
            upper[start + 9..].find("<STMTTRN>").map(|v| start + 9 + v).unwrap_or(ofx.len())
        });
        let block = &ofx[start..end];
        let date = parse_ofx_date(&ofx_value(block, "DTPOSTED"));
        let Some(booked_date) = date else { pos = end.max(start + 9); continue };
        let amount = parse_money(&ofx_value(block, "TRNAMT"));
        let Some(amount_cents) = amount else { pos = end.max(start + 9); continue };
        let name = ofx_value(block, "NAME");
        let memo = ofx_value(block, "MEMO");
        let label = if memo.is_empty() { name.clone() } else if name.is_empty() { memo.clone() } else { format!("{} — {}", name, memo) };
        let refnum = ofx_value(block, "REFNUM");
        let fitid = ofx_value(block, "FITID");
        let bankref = ofx_value(block, "BANKREF");
        let external_id = if !fitid.is_empty() { fitid.clone() } else if !bankref.is_empty() { bankref.clone() } else { format!("OFX:{}:{}", booked_date, records.len() + 1) };
        let value_date = parse_ofx_date(&ofx_value(block, "DTAVAIL"));
        records.push(ParsedBankRecord {
            booked_date, value_date, label, amount_cents, balance_cents: None,
            reference: if !refnum.is_empty() { refnum } else { bankref },
            external_id, counterparty: name,
            raw_payload: serde_json::json!({"source":"OFX","fitid":fitid,"trntype":ofx_value(block,"TRNTYPE")}),
        });
        pos = end.max(start + 9);
    }
    if records.is_empty() { return Err("Aucune transaction OFX <STMTTRN> exploitable".to_owned()); }
    Ok((records, upper.matches("<STMTTRN>").count()))
}

fn parse_ocr_records(text: &str) -> Result<(Vec<ParsedBankRecord>, usize), String> {
    let mut out = Vec::new();
    let mut seen = 0usize;
    for (line_no, line) in text.lines().enumerate() {
        seen += 1;
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let date_pos = tokens.iter().position(|t| parse_date(t).is_some());
        let Some(dp) = date_pos else { continue };
        let Some(booked_date) = parse_date(tokens[dp]) else { continue };
        let money_positions: Vec<(usize, i64)> = tokens.iter().enumerate().filter_map(|(i,t)| parse_money(t).map(|v| (i,v))).collect();
        if money_positions.is_empty() { continue; }
        let amount = money_positions.iter().find(|(i,_)| *i > dp).map(|(_,v)| *v).unwrap_or(money_positions[0].1);
        let balance = money_positions.iter().filter(|(i,_)| *i > dp).nth(1).map(|(_,v)| *v);
        let label_start = dp + 1;
        let label_end = money_positions.iter().find(|(i,_)| *i >= label_start).map(|(i,_)| *i).unwrap_or(tokens.len());
        let label = tokens[label_start..label_end].join(" ");
        if label.is_empty() { continue; }
        out.push(ParsedBankRecord {
            booked_date, value_date: None, label, amount_cents: amount, balance_cents: balance,
            reference: String::new(), external_id: format!("PDFOCR:{}:{}:{}", booked_date, line_no + 1, sha256_hex(line.as_bytes())),
            counterparty: String::new(), raw_payload: serde_json::json!({"source":"PDF_OCR","line":line_no + 1,"raw":line}),
        });
    }
    if out.is_empty() { return Err("Aucune ligne bancaire exploitable dans le texte OCR".to_owned()); }
    Ok((out, seen))
}

#[server]
pub async fn list_bank_transactions_management() -> Result<Vec<BankTransactionManagementItem>, ServerFnError> {
    #[cfg(feature="server")]
    {
        use sqlx::Row;
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows=sqlx::query("SELECT id,transaction_date,value_date,amount_cents,debit_credit,label,COALESCE(counterparty,'') counterparty,COALESCE(reference,'') reference,balance_cents,source,transaction_hash,reconciliation_level,reconciliation_type,reconciliation_confidence_bp FROM bank_transactions WHERE legal_entity_id=$1 ORDER BY transaction_date DESC,booked_at DESC LIMIT 500").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| BankTransactionManagementItem{id:r.get("id"),transaction_date:r.get("transaction_date"),value_date:r.get("value_date"),amount_cents:r.get("amount_cents"),debit_credit:r.get("debit_credit"),label:r.get("label"),counterparty:r.get("counterparty"),reference:r.get("reference"),balance_cents:r.get("balance_cents"),source:r.get("source"),transaction_hash:r.get("transaction_hash"),reconciliation_level:r.get("reconciliation_level"),reconciliation_type:r.get("reconciliation_type"),reconciliation_confidence_bp:r.get("reconciliation_confidence_bp")}).collect())
    }
    #[cfg(not(feature="server"))]
    Err(ServerFnError::new("list_bank_transactions_management est exécutée côté serveur"))
}

#[server]
pub async fn record_bank_transaction_management(account_id: Option<Uuid>, booked_date: NaiveDate, value_date: Option<NaiveDate>, amount_cents: i64, label: String, counterparty: String, reference: String, balance_cents: Option<i64>, external_id: String) -> Result<BankImportResult, ServerFnError> {
    #[cfg(feature="server")]
    {
        if label.trim().is_empty(){return Err(ServerFnError::new("Libellé bancaire requis"));}
        let rec=ParsedBankRecord{booked_date,value_date,label,counterparty,reference,amount_cents,balance_cents,external_id:if external_id.trim().is_empty(){format!("MANUAL:{}:{}",booked_date,Uuid::new_v4())}else{external_id},raw_payload:json!({"source":"MANUAL"})};
        import_records(account_id,"MANUAL","saisie manuelle",vec![rec],1).await
    }
    #[cfg(not(feature="server"))]
    { let _=(account_id,booked_date,value_date,amount_cents,label,counterparty,reference,balance_cents,external_id); Err(ServerFnError::new("record_bank_transaction_management est exécutée côté serveur")) }
}

#[server]
pub async fn list_bank_accounts_management() -> Result<Vec<BankAccountManagementItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let rows = sqlx::query(r#"SELECT a.id,a.label,a.iban,a.bic,a.is_primary,a.active,
                    COALESCE(p.account_type,'CURRENT') account_type,COALESCE(p.currency_code,'EUR') currency_code,
                    COALESCE(p.opening_balance_cents,0) opening_balance_cents,p.opening_balance_date,
                    COALESCE(p.bank_name,'') bank_name
                    FROM legal_entity_bank_accounts a
                    LEFT JOIN bank_account_profiles p ON p.id=a.id AND p.legal_entity_id=a.legal_entity_id
                    WHERE a.legal_entity_id=$1 ORDER BY a.active DESC,a.is_primary DESC,a.label"#)
            .bind(entity).fetch_all(pool).await.map_err(ServerFnError::new)?;
        use sqlx::Row;
        Ok(rows.into_iter().map(|r| BankAccountManagementItem {
            id:r.get("id"),label:r.get("label"),iban:r.get("iban"),bic:r.get("bic"),is_primary:r.get("is_primary"),active:r.get("active"),
            account_type:r.get("account_type"),currency_code:r.get("currency_code"),opening_balance_cents:r.get("opening_balance_cents"),
            opening_balance_date:r.get("opening_balance_date"),bank_name:r.get("bank_name")
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_bank_accounts_management est exécutée côté serveur"))
}

#[server]
pub async fn save_bank_account_management(id: Option<Uuid>, label: String, iban: String, bic: String, account_type: String, currency_code: String, opening_balance_cents: i64, opening_balance_date: Option<NaiveDate>, bank_name: String, primary: bool) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        if label.trim().is_empty() || iban.trim().is_empty() { return Err(ServerFnError::new("Libellé et IBAN requis")); }
        let mut tx = pool.begin().await.map_err(ServerFnError::new)?;
        if primary { sqlx::query("UPDATE legal_entity_bank_accounts SET is_primary=false,updated_at=now() WHERE legal_entity_id=$1").bind(entity).execute(&mut *tx).await.map_err(ServerFnError::new)?; }
        let account_id = if let Some(existing) = id {
            sqlx::query("UPDATE legal_entity_bank_accounts SET label=$3,iban=$4,bic=$5,is_primary=$6,active=true,updated_at=now() WHERE id=$2 AND legal_entity_id=$1").bind(entity).bind(existing).bind(label.trim()).bind(iban.trim()).bind(bic.trim()).bind(primary).execute(&mut *tx).await.map_err(ServerFnError::new)?;
            if sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM legal_entity_bank_accounts WHERE id=$1 AND legal_entity_id=$2)").bind(existing).bind(entity).fetch_one(&mut *tx).await.map_err(ServerFnError::new)? { existing } else { return Err(ServerFnError::new("Compte bancaire introuvable")); }
        } else {
            sqlx::query_scalar::<_,Uuid>("INSERT INTO legal_entity_bank_accounts(legal_entity_id,label,iban,bic,is_primary,active) VALUES($1,$2,$3,$4,$5,true) RETURNING id").bind(entity).bind(label.trim()).bind(iban.trim()).bind(bic.trim()).bind(primary).fetch_one(&mut *tx).await.map_err(ServerFnError::new)?
        };
        sqlx::query("INSERT INTO bank_account_profiles(id,legal_entity_id,account_type,currency_code,opening_balance_cents,opening_balance_date,bank_name,active,updated_at) VALUES($1,$2,$3,$4,$5,$6,$7,true,now()) ON CONFLICT(id) DO UPDATE SET account_type=EXCLUDED.account_type,currency_code=EXCLUDED.currency_code,opening_balance_cents=EXCLUDED.opening_balance_cents,opening_balance_date=EXCLUDED.opening_balance_date,bank_name=EXCLUDED.bank_name,active=true,updated_at=now()").bind(account_id).bind(entity).bind(account_type.trim().to_uppercase()).bind(currency_code.trim().to_uppercase()).bind(opening_balance_cents).bind(opening_balance_date).bind(bank_name.trim()).execute(&mut *tx).await.map_err(ServerFnError::new)?;
        tx.commit().await.map_err(ServerFnError::new)?;
        Ok(account_id)
    }
    #[cfg(not(feature = "server"))]
    { let _=(id,label,iban,bic,account_type,currency_code,opening_balance_cents,opening_balance_date,bank_name,primary); Err(ServerFnError::new("save_bank_account_management est exécutée côté serveur")) }
}

#[server]
pub async fn deactivate_bank_account_management(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id();
        sqlx::query("UPDATE legal_entity_bank_accounts SET active=false,is_primary=false,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(entity).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("UPDATE bank_account_profiles SET active=false,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(entity).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    { let _=id; Err(ServerFnError::new("deactivate_bank_account_management est exécutée côté serveur")) }
}

#[cfg(feature = "server")]
async fn import_records(account_id: Option<Uuid>, source_type: &str, source_name: &str, records: Vec<ParsedBankRecord>, rows_seen: usize) -> Result<BankImportResult, ServerFnError> {
    let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id();
    let batch:Uuid=sqlx::query_scalar("INSERT INTO bank_import_batches(legal_entity_id,bank_account_id,source_type,source_name,rows_seen,status) VALUES($1,$2,$3,$4,$5,'RUNNING') RETURNING id").bind(entity).bind(account_id).bind(source_type).bind(source_name.trim()).bind(rows_seen as i32).fetch_one(pool).await.map_err(ServerFnError::new)?;
    let mut imported=0i32; let mut skipped=0i32; let mut messages=Vec::new();
    for rec in records {
        let tx_hash=sha256_hex(format!("{}|{}|{}|{}|{}|{}",account_id.map(|v|v.to_string()).unwrap_or_default(),rec.booked_date,rec.value_date.map(|v|v.to_string()).unwrap_or_default(),rec.amount_cents,rec.label.trim().to_uppercase(),rec.external_id.trim()).as_bytes());
        let existing_hash:Option<Uuid>=sqlx::query_scalar("SELECT id FROM bank_transactions WHERE legal_entity_id=$1 AND transaction_hash=$2 AND bank_account_id IS NOT DISTINCT FROM $3 LIMIT 1").bind(entity).bind(&tx_hash).bind(account_id).fetch_optional(pool).await.map_err(ServerFnError::new)?;
        if existing_hash.is_some() && rec.external_id.starts_with("PDFOCR:") { skipped+=1; continue; }
        let ext_source=rec.external_id.trim().to_owned();
        let scoped_external_id=if ext_source.is_empty(){None}else{Some(match account_id{Some(a)=>format!("{}::{}",a,ext_source),None=>ext_source.clone()})};
        let id:Uuid=sqlx::query_scalar(r#"INSERT INTO bank_transactions(id,legal_entity_id,bank_account_id,booked_at,transaction_date,value_date,amount_cents,label,counterparty,external_id,reference,balance_cents,source,transaction_hash,debit_credit,import_batch_id,raw_payload,reconciliation_status,reconciliation_level)
            VALUES(gen_random_uuid(),$1,$2,$3,$3::date,$4,$5,$6,NULLIF($7,''),$8,$9,$10,$11,$12,CASE WHEN $5<0 THEN 'DEBIT' ELSE 'CREDIT' END,$13,$14,'UNMATCHED','UNMATCHED')
            ON CONFLICT(legal_entity_id,external_id) WHERE external_id IS NOT NULL AND btrim(external_id)<>''
            DO UPDATE SET bank_account_id=EXCLUDED.bank_account_id,booked_at=EXCLUDED.booked_at,transaction_date=EXCLUDED.transaction_date,value_date=EXCLUDED.value_date,amount_cents=EXCLUDED.amount_cents,label=EXCLUDED.label,counterparty=EXCLUDED.counterparty,reference=EXCLUDED.reference,balance_cents=EXCLUDED.balance_cents,source=EXCLUDED.source,transaction_hash=EXCLUDED.transaction_hash,import_batch_id=EXCLUDED.import_batch_id,raw_payload=EXCLUDED.raw_payload,updated_at=now()
            RETURNING id"#)
            .bind(entity).bind(account_id).bind(rec.booked_date.and_hms_opt(12,0,0).unwrap().and_utc()).bind(rec.value_date).bind(rec.amount_cents).bind(rec.label.trim()).bind(rec.counterparty.trim()).bind(scoped_external_id).bind(rec.reference.trim()).bind(rec.balance_cents).bind(source_type).bind(&tx_hash).bind(batch).bind(rec.raw_payload).fetch_one(pool).await.map_err(ServerFnError::new)?;
        crate::services::record_financial_transaction(pool,entity,rec.booked_date.and_hms_opt(12,0,0).unwrap().and_utc(),rec.amount_cents.unsigned_abs().min(i64::MAX as u64) as i64,if rec.amount_cents<0{"OUT"}else{"IN"},"BANK_TRANSACTION",None,"RECORDED",Some(&tx_hash),rec.counterparty.trim(),rec.label.trim(),serde_json::json!({"bank_transaction_id":id,"source":source_type})).await.map_err(ServerFnError::new)?;
        imported+=1;
    }
    let status=if skipped>0 && imported>0{"PARTIAL"}else{"COMPLETED"};
    sqlx::query("UPDATE bank_import_batches SET rows_imported=$2,rows_skipped=$3,status=$4 WHERE id=$1 AND legal_entity_id=$5").bind(batch).bind(imported).bind(skipped).bind(status).bind(entity).execute(pool).await.map_err(ServerFnError::new)?;
    if skipped>0 {messages.push(format!("{} doublon(s) ignoré(s)",skipped));}
    Ok(BankImportResult{batch_id:batch,source_type:source_type.to_owned(),rows_seen:rows_seen as i32,rows_imported:imported,rows_skipped:skipped,messages})
}

#[server]
pub async fn import_bank_csv_management(account_id: Option<Uuid>, csv: String, source_name: String) -> Result<BankImportResult, ServerFnError> {
    #[cfg(feature = "server")]
    { let (records,seen)=parse_csv_records(&csv).map_err(ServerFnError::new)?; import_records(account_id,"CSV",&source_name,records,seen).await }
    #[cfg(not(feature = "server"))]
    { let _=(account_id,csv,source_name); Err(ServerFnError::new("import_bank_csv_management est exécutée côté serveur")) }
}

#[server]
pub async fn import_bank_ofx_management(account_id: Option<Uuid>, ofx: String, source_name: String) -> Result<BankImportResult, ServerFnError> {
    #[cfg(feature = "server")]
    { let (records,seen)=parse_ofx_records(&ofx).map_err(ServerFnError::new)?; import_records(account_id,"OFX",&source_name,records,seen).await }
    #[cfg(not(feature = "server"))]
    { let _=(account_id,ofx,source_name); Err(ServerFnError::new("import_bank_ofx_management est exécutée côté serveur")) }
}

#[server]
pub async fn import_bank_ocr_text_management(account_id: Option<Uuid>, text: String, source_name: String) -> Result<BankImportResult, ServerFnError> {
    #[cfg(feature = "server")]
    { let (records,seen)=parse_ocr_records(&text).map_err(ServerFnError::new)?; import_records(account_id,"PDF_OCR",&source_name,records,seen).await }
    #[cfg(not(feature = "server"))]
    { let _=(account_id,text,source_name); Err(ServerFnError::new("import_bank_ocr_text_management est exécutée côté serveur")) }
}

#[server]
pub async fn import_bank_pdf_ocr_management(account_id: Option<Uuid>, path: String) -> Result<BankImportResult, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let command=std::env::var("SCI_OCR_COMMAND").ok().filter(|v|!v.trim().is_empty()).ok_or_else(||ServerFnError::new("SCI_OCR_COMMAND absent : configurez l'adaptateur OCR local"))?;
        let safe=std::path::PathBuf::from(path.trim()); if safe.as_os_str().is_empty(){return Err(ServerFnError::new("Chemin PDF requis"));}
        let output=tokio::process::Command::new(command.trim()).arg(&safe).arg("stdout").output().await.map_err(ServerFnError::new)?;
        if !output.status.success(){return Err(ServerFnError::new(format!("OCR PDF échoué: {}",String::from_utf8_lossy(&output.stderr))));}
        let text=String::from_utf8_lossy(&output.stdout).to_string(); let source=safe.file_name().and_then(|v|v.to_str()).unwrap_or("releve.pdf").to_owned();
        let (records,seen)=parse_ocr_records(&text).map_err(ServerFnError::new)?; import_records(account_id,"PDF_OCR",&source,records,seen).await
    }
    #[cfg(not(feature = "server"))]
    { let _=(account_id,path); Err(ServerFnError::new("import_bank_pdf_ocr_management est exécutée côté serveur")) }
}

fn reconciliation_level_fr(level:&str)->&'static str{match level{"MATCHED"=>"Rapproché","PROBABLE"=>"Probable","TO_VALIDATE"=>"À valider","UNMATCHED"=>"Non rapproché","ANOMALY"=>"Anomalie",_=>"—"}}

fn classify_bank_label(label:&str, amount:i64)->(&'static str,i32,&'static str){
    let l=label.to_lowercase();
    if ["frais","commission","agios","tenue de compte"].iter().any(|k|l.contains(k)){return ("FEE",9600,"Frais bancaire détecté");}
    if ["virement interne","transfert interne","compte à compte"].iter().any(|k|l.contains(k)){return ("INTERNAL_TRANSFER",9700,"Virement interne détecté");}
    if ["dépôt de garantie","depot de garantie","caution"].iter().any(|k|l.contains(k)){return ("DEPOSIT",9300,"Dépôt/caution détecté");}
    if ["remboursement","remb.","refund"].iter().any(|k|l.contains(k)){return ("REFUND",9300,"Remboursement détecté");}
    if ["impayé","impaye","rejet","rejet prélèvement","rejet prelevement"].iter().any(|k|l.contains(k)){return ("IMPAYE",9800,"Impayé/rejet détecté");}
    if amount==0{return ("UNKNOWN",0,"Montant nul : anomalie potentielle");}
    ("UNKNOWN",0,"Aucune nature certaine détectée")
}

#[server]
pub async fn auto_reconcile_bank_management() -> Result<BankReconciliationSummary, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id();
        let txs=sqlx::query("SELECT id,booked_at,amount_cents,label,transaction_hash,reconciliation_level FROM bank_transactions WHERE legal_entity_id=$1 AND reconciliation_level IN ('UNMATCHED','TO_VALIDATE','PROBABLE') ORDER BY booked_at,id LIMIT 1000").bind(entity).fetch_all(pool).await.map_err(ServerFnError::new)?;
        let mut out=BankReconciliationSummary{scanned:0,matched:0,probable:0,to_validate:0,anomalous:0,unmatched:0,messages:Vec::new()};
        for tx in txs {
            out.scanned+=1; let id:Uuid=tx.get("id"); let amount:i64=tx.get("amount_cents"); let label:String=tx.get("label"); let (kind,rule_conf,reason)=classify_bank_label(&label,amount);
            if kind!="UNKNOWN" {
                let level=if rule_conf>=9500{"MATCHED"}else{"PROBABLE"};
                sqlx::query("UPDATE bank_transactions SET reconciliation_status='MATCHED',reconciliation_level=$3,reconciliation_type=$4,reconciliation_confidence_bp=$5,reconciled_at=now(),updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(entity).bind(level).bind(kind).bind(rule_conf).execute(pool).await.map_err(ServerFnError::new)?;
                sqlx::query("INSERT INTO bank_reconciliation_matches(legal_entity_id,bank_transaction_id,target_type,matched_amount_cents,match_type,confidence_bp,status,reason) VALUES($1,$2,$3,$4,$5,$6,'CONFIRMED',$7)").bind(entity).bind(id).bind(kind).bind(amount.abs()).bind(kind).bind(rule_conf).bind(reason).execute(pool).await.map_err(ServerFnError::new)?;
                sqlx::query("INSERT INTO bank_reconciliation_events(legal_entity_id,bank_transaction_id,from_level,to_level,reason,actor) VALUES($1,$2,$3,$4,$5,'SYSTEM')").bind(entity).bind(id).bind(tx.get::<String,_>("reconciliation_level")).bind(level).bind(reason).execute(pool).await.map_err(ServerFnError::new)?;
                if level=="MATCHED"{out.matched+=1}else{out.probable+=1}
                continue;
            }
            let candidates=sqlx::query(r#"SELECT i.id,i.gross_cents,COALESCE((SELECT SUM(p.amount_cents) FROM payments p WHERE p.invoice_id=i.id AND p.legal_entity_id=i.legal_entity_id),0)::bigint paid,i.invoice_number,COALESCE(t.legal_name,'') tenant_name
                FROM invoices i LEFT JOIN leases l ON l.id=i.lease_id LEFT JOIN tenants t ON t.id=l.tenant_id
                WHERE i.legal_entity_id=$1 AND i.status IN ('VALIDATED','ISSUED','PAID_PARTIAL','OVERDUE')
                  AND i.gross_cents > COALESCE((SELECT SUM(p.amount_cents) FROM payments p WHERE p.invoice_id=i.id AND p.legal_entity_id=i.legal_entity_id),0)
                ORDER BY ABS((i.gross_cents-COALESCE((SELECT SUM(p.amount_cents) FROM payments p WHERE p.invoice_id=i.id AND p.legal_entity_id=i.legal_entity_id),0))-$2),i.due_date LIMIT 5"#)
                .bind(entity).bind(amount.abs()).fetch_all(pool).await.map_err(ServerFnError::new)?;
            let mut best:Option<(Uuid,i64,i32,String,String)>=None;
            for c in candidates {
                let iid:Uuid=c.get("id"); let gross:i64=c.get("gross_cents"); let paid:i64=c.get("paid"); let remaining=gross.saturating_sub(paid); let invn:String=c.get("invoice_number"); let tenant:String=c.get("tenant_name");
                if remaining<=0{continue;}
                let exact=remaining==amount.abs(); let label_match=label.to_lowercase().contains(&invn.to_lowercase()) || (!tenant.is_empty() && label.to_lowercase().contains(&tenant.to_lowercase()));
                let conf=if exact && label_match{9950}else if exact{9000}else if amount>0 && amount.abs()<remaining && label_match{8600}else{0};
                if conf>best.as_ref().map(|x|x.2).unwrap_or(0){best=Some((iid,remaining,conf,invn,tenant));}
            }
            if let Some((invoice_id,remaining,conf,invn,tenant))=best {
                let match_type=if amount.abs()==remaining{"RENT_OR_INVOICE"}else{"PAYMENT_PARTIAL"};
                let level=if conf>=9900{"MATCHED"}else{"TO_VALIDATE"};
                let match_status=if level=="MATCHED"{"CONFIRMED"}else{"PROPOSED"};
                let amount_for_payment=amount.abs().min(remaining);
                let payment_key=format!("BANK_AUTO:{}:{}",id,invoice_id);
                if level=="MATCHED" {
                    sqlx::query("INSERT INTO payments(legal_entity_id,invoice_id,received_at,amount_cents,reference,source,idempotency_key) VALUES($1,$2,$3,$4,$5,'BANK_AUTO_RECONCILIATION',$6) ON CONFLICT(legal_entity_id,idempotency_key) DO NOTHING")
                        .bind(entity).bind(invoice_id).bind(tx.get::<DateTime<Utc>,_>("booked_at")).bind(amount_for_payment).bind(format!("Rapprochement automatique {}",id)).bind(&payment_key).execute(pool).await.map_err(ServerFnError::new)?;
                    sqlx::query("UPDATE invoices SET updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(invoice_id).bind(entity).execute(pool).await.map_err(ServerFnError::new)?;
                }
                sqlx::query("INSERT INTO bank_reconciliation_matches(legal_entity_id,bank_transaction_id,target_type,target_id,invoice_id,matched_amount_cents,match_type,confidence_bp,status,reason) VALUES($1,$2,'INVOICE',$3,$3,$4,$5,$6,$7,$8)")
                    .bind(entity).bind(id).bind(invoice_id).bind(amount_for_payment).bind(match_type).bind(conf).bind(match_status).bind(format!("{} {}",if tenant.is_empty(){"Facture"}else{"Locataire"},invn)).execute(pool).await.map_err(ServerFnError::new)?;
                sqlx::query("UPDATE bank_transactions SET reconciliation_status=CASE WHEN $3='MATCHED' THEN 'MATCHED' ELSE 'PROBABLE' END,reconciliation_level=$3,reconciliation_type=$4,reconciliation_confidence_bp=$5,reconciled_at=CASE WHEN $3='MATCHED' THEN now() ELSE reconciled_at END,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(entity).bind(level).bind(match_type).bind(conf).execute(pool).await.map_err(ServerFnError::new)?;
                if level=="MATCHED"{out.matched+=1}else{out.to_validate+=1;}
                continue;
            }
            let level=if label.trim().is_empty(){"ANOMALY"}else{"UNMATCHED"};
            sqlx::query("UPDATE bank_transactions SET reconciliation_status=CASE WHEN $3='ANOMALY' THEN 'ANOMALY' ELSE 'UNMATCHED' END,reconciliation_level=$3,reconciliation_type='UNKNOWN',reconciliation_confidence_bp=0,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(entity).bind(level).execute(pool).await.map_err(ServerFnError::new)?;
            if level=="ANOMALY"{out.anomalous+=1}else{out.unmatched+=1;}
        }
        let _=sqlx::query("UPDATE bank_reconciliation_matches m SET match_type='PAYMENT_MULTIPLE' WHERE m.legal_entity_id=$1 AND m.status='CONFIRMED' AND m.invoice_id IS NOT NULL AND EXISTS(SELECT 1 FROM bank_reconciliation_matches m2 WHERE m2.legal_entity_id=m.legal_entity_id AND m2.invoice_id=m.invoice_id AND m2.status='CONFIRMED' AND m2.id<>m.id)").bind(entity).execute(pool).await;
        Ok(out)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("auto_reconcile_bank_management est exécutée côté serveur"))
}

#[server]
pub async fn confirm_bank_reconciliation(transaction_id: Uuid, match_id: Option<Uuid>) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id();
        if let Some(mid)=match_id { sqlx::query("UPDATE bank_reconciliation_matches SET status='CONFIRMED',confirmed_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(mid).bind(entity).execute(pool).await.map_err(ServerFnError::new)?; }
        sqlx::query("UPDATE bank_transactions SET reconciliation_level='MATCHED',reconciliation_status='MATCHED',reconciled_at=now(),updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(transaction_id).bind(entity).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    { let _=(transaction_id,match_id); Err(ServerFnError::new("confirm_bank_reconciliation est exécutée côté serveur")) }
}

#[server]
pub async fn bank_cash_positions(as_of: Option<NaiveDate>) -> Result<CashPositionsView, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id(); let date=as_of.unwrap_or_else(||Utc::now().date_naive());
        let imported_opt:Option<i64>=sqlx::query_scalar("SELECT SUM(x.balance_cents)::bigint FROM (SELECT DISTINCT ON (COALESCE(bank_account_id,'00000000-0000-0000-0000-000000000000'::uuid)) balance_cents FROM bank_transactions WHERE legal_entity_id=$1 AND transaction_date <= $2 AND balance_cents IS NOT NULL ORDER BY COALESCE(bank_account_id,'00000000-0000-0000-0000-000000000000'::uuid),transaction_date DESC,booked_at DESC) x").bind(entity).bind(date).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let fallback_opening:i64=sqlx::query_scalar("SELECT COALESCE(SUM(p.opening_balance_cents),0)::bigint FROM bank_account_profiles p WHERE p.legal_entity_id=$1 AND p.active").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let tx_sum:i64=sqlx::query_scalar("SELECT COALESCE(SUM(amount_cents),0)::bigint FROM bank_transactions WHERE legal_entity_id=$1 AND transaction_date <= $2").bind(entity).bind(date).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let imported_bank=imported_opt.unwrap_or(fallback_opening+tx_sum);
        let reconciled_moves:i64=sqlx::query_scalar("SELECT COALESCE(SUM(amount_cents),0)::bigint FROM bank_transactions WHERE legal_entity_id=$1 AND transaction_date <= $2 AND reconciliation_level='MATCHED'").bind(entity).bind(date).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let reconciled=fallback_opening+reconciled_moves;
        let theoretical=fallback_opening+tx_sum;
        let forecast:i64=sqlx::query_scalar("SELECT COALESCE(amount_cents,$3) FROM cash_position_snapshots WHERE legal_entity_id=$1 AND as_of_date=$2 AND position_type='FORECAST' ORDER BY generated_at DESC LIMIT 1").bind(entity).bind(date).bind(theoretical).fetch_optional(pool).await.map_err(ServerFnError::new)?.unwrap_or(theoretical);
        let available=if imported_opt.is_some(){imported_bank}else{reconciled};
        Ok(CashPositionsView{imported_bank_cents:imported_bank,reconciled_cents:reconciled,theoretical_cents:theoretical,forecast_cents:forecast,available_cents:available})
    }
    #[cfg(not(feature = "server"))]
    { let _=as_of; Err(ServerFnError::new("bank_cash_positions est exécutée côté serveur")) }
}

#[component]
pub fn BankManagementPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let accounts = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_bank_accounts_management().await.unwrap_or_default() } });
    let txs = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_bank_transactions_management().await.unwrap_or_default() } });
    let cash = use_resource(move || { let _ = refresh(); let _ = bump(); async move { bank_cash_positions(None).await } });
    let mut label = use_signal(String::new);
    let mut iban = use_signal(String::new);
    let mut bic = use_signal(String::new);
    let mut bank_name = use_signal(String::new);
    let mut opening = use_signal(String::new);
    let mut ext = use_signal(String::new);
    let mut selected_account = use_signal(String::new);
    let mut csv = use_signal(String::new);
    let mut ofx = use_signal(String::new);
    let mut ocr = use_signal(String::new);
    let mut pdf_path = use_signal(String::new);
    let mut msg = use_signal(String::new);
    let mut filter = use_signal(String::new);
    let cash_value = cash.read().clone();

    rsx! {
        ModuleHeader { title: "Banque", kicker: "COMPTES • CSV • OFX • PDF/OCR • RAPPROCHEMENT", detail: "Le mouvement importé, le rapprochement et le solde bancaire restent distincts. Les imports sont idempotents." }
        if let Some(Ok(c)) = cash_value.as_ref() {
            section {
                class: "facts-row",
                InfoTileOwned { label: "Bancaire importé", value: euro(c.imported_bank_cents) }
                InfoTileOwned { label: "Réellement rapproché", value: euro(c.reconciled_cents) }
                InfoTileOwned { label: "Théorique", value: euro(c.theoretical_cents) }
                InfoTileOwned { label: "Prévisionnel", value: euro(c.forecast_cents) }
                InfoTileOwned { label: "Disponible", value: euro(c.available_cents) }
            }
        }
        section {
            class: "two-col",
            div {
                class: "panel",
                h3 { "Comptes bancaires" }
                div {
                    class: "form-grid",
                    FormField { label: "Libellé", value: label(), oninput: move |e: FormEvent| label.set(e.value()) }
                    FormField { label: "IBAN", value: iban(), oninput: move |e: FormEvent| iban.set(e.value()) }
                    FormField { label: "BIC", value: bic(), oninput: move |e: FormEvent| bic.set(e.value()) }
                    FormField { label: "Banque", value: bank_name(), oninput: move |e: FormEvent| bank_name.set(e.value()) }
                    FormField { label: "Solde initial €", value: opening(), oninput: move |e: FormEvent| opening.set(e.value()) }
                }
                button {
                    class: "primary",
                    onclick: move |_| async move {
                        match save_bank_account_management(None, label(), iban(), bic(), "CURRENT".into(), "EUR".into(), crate::ui::euros_to_cents(&opening()), None, bank_name(), accounts.read().as_deref().unwrap_or(&[]).is_empty()).await {
                            Ok(_) => { msg.set("Compte bancaire enregistré".into()); bump += 1; }
                            Err(e) => msg.set(e.to_string()),
                        }
                    },
                    "Enregistrer"
                }
            }
            div {
                class: "panel",
                h3 { "Comptes enregistrés" }
                for a in accounts.read().as_deref().unwrap_or(&[]).iter() {
                    div {
                        class: "data-row",
                        div {
                            div { class: "data-title", "{a.label}" }
                            div { class: "small", "{a.iban} • {a.bank_name} • {a.account_type}" }
                        }
                        div {
                            class: "row-actions",
                            if a.active {
                                button { class: "secondary", onclick: { let id = a.id; move |_| async move { let _ = deactivate_bank_account_management(id).await; bump += 1 } }, "Désactiver" }
                            }
                        }
                    }
                }
            }
        }
        section {
            class: "panel",
            h3 { "Saisie d’un mouvement bancaire" }
            div {
                class: "form-grid",
                label {
                    class: "field",
                    span { "Compte" }
                    select {
                        value: selected_account(),
                        onchange: move |e: FormEvent| selected_account.set(e.value()),
                        option { value: "", "Aucun compte" }
                        for a in accounts.read().as_deref().unwrap_or(&[]).iter().filter(|a| a.active) {
                            option { value: a.id.to_string(), "{a.label}" }
                        }
                    }
                }
                div { class: "small", "Date : aujourd’hui" }
                FormField { label: "Montant €", value: opening(), oninput: move |e: FormEvent| opening.set(e.value()) }
                FormField { label: "Libellé", value: label(), oninput: move |e: FormEvent| label.set(e.value()) }
                FormField { label: "Référence", value: ext(), oninput: move |e: FormEvent| ext.set(e.value()) }
            }
            button {
                class: "primary",
                onclick: move |_| async move {
                    let account = Uuid::parse_str(&selected_account()).ok();
                    match record_bank_transaction_management(account, Utc::now().date_naive(), Some(Utc::now().date_naive()), crate::ui::euros_to_cents(&opening()), label(), String::new(), ext(), None, String::new()).await {
                        Ok(r) => { msg.set(format!("Mouvement enregistré ({})", r.rows_imported)); bump += 1; }
                        Err(e) => msg.set(e.to_string()),
                    }
                },
                "Enregistrer"
            }
        }
        section {
            class: "panel",
            h3 { "Importer un relevé CSV" }
            label {
                class: "field",
                span { "Compte" }
                select {
                    value: selected_account(),
                    onchange: move |e: FormEvent| selected_account.set(e.value()),
                    option { value: "", "Aucun compte sélectionné" }
                    for a in accounts.read().as_deref().unwrap_or(&[]).iter().filter(|a| a.active) {
                        option { value: a.id.to_string(), "{a.label}" }
                    }
                }
            }
            textarea { value: csv(), oninput: move |e: FormEvent| csv.set(e.value()), placeholder: "date;date valeur;libellé;montant;solde;référence" }
            button {
                class: "primary",
                onclick: move |_| async move {
                    let account = Uuid::parse_str(&selected_account()).ok();
                    match import_bank_csv_management(account, csv(), "import.csv".into()).await {
                        Ok(r) => { msg.set(format!("CSV : {} importé(s), {} ignoré(s)", r.rows_imported, r.rows_skipped)); bump += 1; }
                        Err(e) => msg.set(e.to_string()),
                    }
                },
                "Importer CSV"
            }
        }
        section {
            class: "panel",
            h3 { "Importer OFX" }
            textarea { value: ofx(), oninput: move |e: FormEvent| ofx.set(e.value()), placeholder: "<STMTTRN>…" }
            button {
                class: "primary",
                onclick: move |_| async move {
                    let account = Uuid::parse_str(&selected_account()).ok();
                    match import_bank_ofx_management(account, ofx(), "import.ofx".into()).await {
                        Ok(r) => { msg.set(format!("OFX : {} importé(s)", r.rows_imported)); bump += 1; }
                        Err(e) => msg.set(e.to_string()),
                    }
                },
                "Importer OFX"
            }
        }
        section {
            class: "two-col",
            div {
                class: "panel",
                h3 { "Relevé PDF / OCR" }
                FormField { label: "Chemin PDF local", value: pdf_path(), oninput: move |e: FormEvent| pdf_path.set(e.value()) }
                button {
                    class: "primary",
                    onclick: move |_| async move {
                        let account = Uuid::parse_str(&selected_account()).ok();
                        match import_bank_pdf_ocr_management(account, pdf_path()).await {
                            Ok(r) => { msg.set(format!("PDF/OCR : {} importé(s)", r.rows_imported)); bump += 1; }
                            Err(e) => msg.set(e.to_string()),
                        }
                    },
                    "Importer + OCR"
                }
            }
            div {
                class: "panel",
                h3 { "Texte OCR déjà extrait" }
                textarea { value: ocr(), oninput: move |e: FormEvent| ocr.set(e.value()) }
                button {
                    class: "secondary",
                    onclick: move |_| async move {
                        let account = Uuid::parse_str(&selected_account()).ok();
                        match import_bank_ocr_text_management(account, ocr(), "ocr.txt".into()).await {
                            Ok(r) => { msg.set(format!("OCR : {} importé(s)", r.rows_imported)); bump += 1; }
                            Err(e) => msg.set(e.to_string()),
                        }
                    },
                    "Importer le texte OCR"
                }
            }
        }
        section {
            class: "panel",
            div {
                class: "panel-head",
                h3 { "Rapprochement automatique" }
                button {
                    class: "primary",
                    onclick: move |_| async move {
                        match auto_reconcile_bank_management().await {
                            Ok(r) => { msg.set(format!("{} mouvement(s) analysé(s) • {} rapproché(s) • {} probable(s) • {} à valider • {} anomalie(s)", r.scanned, r.matched, r.probable, r.to_validate, r.anomalous)); bump += 1; }
                            Err(e) => msg.set(e.to_string()),
                        }
                    },
                    "Analyser / rapprocher"
                }
            }
            span { class: "small", "{msg}" }
            label {
                class: "field",
                span { "Niveau" }
                select {
                    value: filter(),
                    onchange: move |e: FormEvent| filter.set(e.value()),
                    option { value: "", "Tous" }
                    option { value: "MATCHED", "Rapproché" }
                    option { value: "PROBABLE", "Probable" }
                    option { value: "TO_VALIDATE", "À valider" }
                    option { value: "UNMATCHED", "Non rapproché" }
                    option { value: "ANOMALY", "Anomalie" }
                }
            }
            for t in txs.read().as_deref().unwrap_or(&[]).iter().filter(|t| filter().is_empty() || t.reconciliation_level == filter()) {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{t.transaction_date} • {t.label}" }
                        div { class: "small", "{euro(t.amount_cents)} • {t.source} • {reconciliation_level_fr(&t.reconciliation_level)} • {t.counterparty} • {t.reference}" }
                    }
                }
            }
        }
    }
}

fn euro(cents: i64) -> String { format!("{}.{:02} €", cents / 100, cents.abs() % 100) }


#[cfg(test)]
mod tests {
    #[test]
    fn csv_parser_handles_french_separator_and_quotes() {
        let csv = "Date;Libellé;Débit;Crédit;Solde\n25/09/2026;\"LOYER; TEST\";100,50;;900,00\n";
        let (rows, seen)=parse_csv_records(csv).expect("csv");
        assert_eq!(seen, 2);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].amount_cents, -10050);
        assert_eq!(rows[0].label, "LOYER; TEST");
    }

    #[test]
    fn ofx_parser_reads_fitid_and_amount() {
        let ofx = "<OFX><BANKTRANLIST><STMTTRN><DTPOSTED>20260925</DTPOSTED><TRNAMT>1250.50</TRNAMT><FITID>ABC-1</FITID><NAME>LOYER</NAME><MEMO>SEPTEMBRE</MEMO></STMTTRN></BANKTRANLIST></OFX>";
        let (rows, _)=parse_ofx_records(ofx).expect("ofx");
        assert_eq!(rows.len(),1);
        assert_eq!(rows[0].external_id,"ABC-1");
        assert_eq!(rows[0].amount_cents,125050);
    }

    use super::*;

    #[test]
    fn parse_money_french_and_negative() {
        assert_eq!(parse_money("1 234,56"), Some(123456));
        assert_eq!(parse_money("(42,10)"), Some(-4210));
        assert_eq!(parse_money("99.95"), Some(9995));
    }

    #[test]
    fn parse_csv_with_header() {
        let csv = "Date;Date valeur;Libellé;Montant;Solde;Référence\n25/09/2026;26/09/2026;LOYER TEST;1250,00;5000,00;REF-1\n";
        let (rows, seen) = parse_csv_records(csv).expect("CSV");
        assert_eq!(seen, 2);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].booked_date, NaiveDate::from_ymd_opt(2026, 9, 25).unwrap());
        assert_eq!(rows[0].value_date, NaiveDate::from_ymd_opt(2026, 9, 26));
        assert_eq!(rows[0].amount_cents, 125000);
        assert_eq!(rows[0].balance_cents, Some(500000));
        assert_eq!(rows[0].reference, "REF-1");
    }

    #[test]
    fn parse_ofx_transaction() {
        let ofx = "<STMTTRN><TRNTYPE>DEBIT<DTPOSTED>20260925<TRNAMT>-123.45<FITID>F-1<NAME>ASSURANCE<MEMO>Contrat</STMTTRN>";
        let (rows, _) = parse_ofx_records(ofx).expect("OFX");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].amount_cents, -12345);
        assert_eq!(rows[0].external_id, "F-1");
        assert!(rows[0].label.contains("ASSURANCE"));
    }

    #[test]
    fn reconciliation_levels_are_human_readable() {
        assert_eq!(reconciliation_level_fr("MATCHED"), "Rapproché");
        assert_eq!(reconciliation_level_fr("ANOMALY"), "Anomalie");
    }
}
