pub mod actions;
pub mod approval;
pub mod control;
pub mod llm;
pub mod tools;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssistantIntent {
    Information,
    PrepareTask,
    ExecuteTask,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssistantAction {
    Read,
    Propose,
    Execute,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantRequest {
    pub message: String,
    pub intent: AssistantIntent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantResponse {
    pub message: String,
    pub action: AssistantAction,
    pub requires_user_approval: bool,
    pub provider_id: Option<String>,
    pub proposal_id: Option<Uuid>,
    pub uncertainty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantContext {
    pub workspace_id: Uuid,
    pub legal_entity_id: Uuid,
}

impl AssistantResponse {
    pub fn information(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            action: AssistantAction::Read,
            requires_user_approval: false,
            provider_id: None,
            proposal_id: None,
            uncertainty: String::new(),
        }
    }

    pub fn proposal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            action: AssistantAction::Propose,
            requires_user_approval: true,
            provider_id: None,
            proposal_id: None,
            uncertainty: String::new(),
        }
    }

    pub fn execution(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            action: AssistantAction::Execute,
            requires_user_approval: false,
            provider_id: None,
            proposal_id: None,
            uncertainty: String::new(),
        }
    }
}

pub fn handle_request(
    request: &AssistantRequest,
    _context: &AssistantContext,
) -> AssistantResponse {
    let message = request.message.trim();

    if message.is_empty() {
        return AssistantResponse::information("Décris-moi ce que tu veux vérifier ou préparer.");
    }

    let normalized = message.to_lowercase();

    if normalized.contains("impay")
        || normalized.contains("impayé")
        || normalized.contains("impayees")
        || normalized.contains("impayées")
    {
        return AssistantResponse::information(
            "Je peux rechercher les factures impayées et identifier les échéances dépassées.",
        );
    }

    if normalized.contains("loyer")
        || normalized.contains("factur")
        || normalized.contains("facture")
    {
        if normalized.contains("cré")
            || normalized.contains("génér")
            || normalized.contains("prépar")
            || normalized.contains("emet")
            || normalized.contains("émet")
        {
            return AssistantResponse::proposal(
                "Je peux préparer la génération des factures de loyers à partir des baux actifs. Aucune écriture bancaire ne sera créée.",
            );
        }

        return AssistantResponse::information(
            "Je peux contrôler les factures de loyers, leurs échéances et les créances correspondantes.",
        );
    }

    if normalized.contains("banque")
        || normalized.contains("virement")
        || normalized.contains("rapproch")
    {
        return AssistantResponse::information(
            "Je peux analyser les mouvements bancaires non rapprochés et identifier ceux qui peuvent correspondre à une facture ou à un paiement.",
        );
    }

    if normalized.contains("tva") {
        return AssistantResponse::information(
            "Je peux consulter la situation TVA et les montants à préparer à partir des opérations enregistrées.",
        );
    }

    if normalized.contains("document")
        || normalized.contains("pièce")
        || normalized.contains("piece")
        || normalized.contains("pdf")
    {
        return AssistantResponse::information(
            "Je peux analyser les documents enregistrés, proposer leur classement et extraire les informations métier détectées.",
        );
    }

    if normalized.contains("échéance")
        || normalized.contains("echeance")
        || normalized.contains("calendrier")
        || normalized.contains("deadline")
    {
        return AssistantResponse::information(
            "Je peux consulter les échéances fiscales et administratives à venir.",
        );
    }

    if normalized.contains("tâche")
        || normalized.contains("tache")
        || normalized.contains("priorit")
        || normalized.contains("à faire")
        || normalized.contains("a faire")
    {
        return AssistantResponse::information(
            "Je peux prioriser les tâches selon leur état, leur urgence et leur caractère bloquant.",
        );
    }

    match request.intent {
        AssistantIntent::Information => AssistantResponse::information(
            "Je peux consulter les informations de la SCI concernant les baux, loyers, factures, paiements, banque, TVA, échéances, documents et tâches.",
        ),
        AssistantIntent::PrepareTask => AssistantResponse::proposal(
            "Je peux préparer une action administrative à partir de ta demande. Les actions ayant un impact métier nécessitent ta validation.",
        ),
        AssistantIntent::ExecuteTask => AssistantResponse::proposal(
            "Cette demande peut entraîner une modification des données métier. Je peux d'abord préparer l'action et te demander une validation explicite avant exécution.",
        ),
    }
}
