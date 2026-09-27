use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Payment {
    pub date: String,
    pub expected: f64,
    pub received: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tenant {
    pub id: usize,
    pub name: String,
    pub property: String,
    pub rent: f64,
    pub balance: f64,
    pub status: TenantStatus,
    pub payments: Vec<Payment>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TenantStatus { Paid, Late, Unpaid, Vacant }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: usize,
    pub label: String,
    pub due_in_days: i32,
    pub category: TaskCategory,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TaskCategory { Payment, Relance, Tax, Declaration }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Recurrence { Once, Monthly, Quarterly, Yearly }

impl Recurrence {
    pub fn label(&self) -> &'static str {
        match self {
            Recurrence::Once => "ponctuel",
            Recurrence::Monthly => "mois",
            Recurrence::Quarterly => "trimestre",
            Recurrence::Yearly => "an",
        }
    }
    /// Nombre de mois entre deux occurrences (0 pour Ponctuel — non utilisé).
    pub fn months(&self) -> i32 {
        match self {
            Recurrence::Once => 0,
            Recurrence::Monthly => 1,
            Recurrence::Quarterly => 3,
            Recurrence::Yearly => 12,
        }
    }
    pub fn is_recurring(&self) -> bool {
        !matches!(self, Recurrence::Once)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Flow {
    pub id: usize,
    pub label: String,
    pub amount: f64,
    pub color: String,
    pub active: bool,
    pub recurrence: Recurrence,
    pub start_year: i32,
    pub start_month: u32,
    /// Pour Ponctuel : jour unique. Pour les récurrents : jour du mois attendu.
    pub recurrence_day: u32,
    /// Jour effectif de paiement (uniquement pour les récurrents).
    pub payment_day: u32,
    pub matched_txs: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BankTx {
    pub id: usize,
    pub date: String,
    pub label: String,
    pub amount: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub struct WidgetState {
    pub id: &'static str,
    pub title: &'static str,
    pub col_span: u8,
    pub pinned: bool,
    pub order: usize,
}