use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Executed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionApproval {
    pub action_id: uuid::Uuid,
    pub status: ApprovalStatus,
    pub user_validated: bool,
}

impl ActionApproval {
    pub fn new(action_id: uuid::Uuid) -> Self {
        Self {
            action_id,
            status: ApprovalStatus::Pending,
            user_validated: false,
        }
    }

    pub fn approve(&mut self) {
        self.status = ApprovalStatus::Approved;
        self.user_validated = true;
    }

    pub fn reject(&mut self) {
        self.status = ApprovalStatus::Rejected;
        self.user_validated = false;
    }

    pub fn can_execute(&self) -> bool {
        self.status == ApprovalStatus::Approved && self.user_validated
    }
}
