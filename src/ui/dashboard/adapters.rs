use crate::domain::{BankTransactionItem, TenantItem, TaskItem};
use super::models::{BankTx, Payment, Task, TaskCategory, Tenant, TenantStatus};

/// Convertit une transaction bancaire serveur en modèle UI.
pub fn bank_tx_from_item(item: BankTransactionItem) -> BankTx {
    BankTx {
        // Id UI = hash stable du UUID (suffisant pour matcher côté front)
        id: item.id.as_u128() as usize,
        date: item.booked_at.format("%Y-%m-%d").to_string(),
        label: item.label,
        amount: item.amount_cents as f64 / 100.0,
    }
}

/// Convertit un locataire serveur + son bail + ses paiements en modèle UI.
/// `payments` : liste des paiements rapprochés sur ses factures (déjà agrégés côté serveur, ou passés en params).
pub fn tenant_from_item(
    item: TenantItem,
    property_label: String,
    rent: f64,
    balance: f64,
    payments: Vec<Payment>,
) -> Tenant {
    let status = if !item.active {
        TenantStatus::Vacant
    } else if balance >= 0.0 {
        TenantStatus::Paid
    } else if balance > -300.0 {
        TenantStatus::Late
    } else {
        TenantStatus::Unpaid
    };
    Tenant {
        id: item.id.as_u128() as usize,
        name: item.legal_name,
        property: property_label,
        rent,
        balance,
        status,
        payments,
    }
}

/// Convertit une tâche serveur en modèle UI.
pub fn task_from_item(item: TaskItem) -> Task {
    let now = chrono::Utc::now();
    let due_in_days = (item.due_at - now).num_days() as i32;
    let category = match item.code.as_str() {
        c if c.contains("VAT") || c.contains("TVA") => TaskCategory::Tax,
        c if c.contains("DEADLINE") || c.contains("2072") => TaskCategory::Declaration,
        c if c.contains("RELANCE") || c.contains("RECOVERY") => TaskCategory::Relance,
        _ => TaskCategory::Payment,
    };
    Task {
        id: item.id.as_u128() as usize,
        label: item.title,
        due_in_days,
        category,
    }
}