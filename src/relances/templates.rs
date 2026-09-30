use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelanceContext {
    pub sci_name: String,
    pub sci_address: String,
    pub sci_siren: String,
    pub sci_iban: String,
    pub sci_bic: String,
    pub client_name: String,
    pub invoice_ref: String,
    pub invoice_due_date: String,
    pub invoice_amount: String,
    pub outstanding_amount: String,
    pub days_overdue: i32,
    pub penalties: String,
    pub forfait: String,
    pub total_due: String,
    pub tribunal: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RelanceLevel {
    Amiable,
    Ferme,
    MiseEnDemeure,
}

impl RelanceLevel {
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            1 => Some(RelanceLevel::Amiable),
            2 => Some(RelanceLevel::Ferme),
            3 => Some(RelanceLevel::MiseEnDemeure),
            _ => None,
        }
    }

    pub fn code(self) -> i32 {
        match self {
            RelanceLevel::Amiable => 1,
            RelanceLevel::Ferme => 2,
            RelanceLevel::MiseEnDemeure => 3,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RelanceLevel::Amiable => "Relance amiable",
            RelanceLevel::Ferme => "Relance ferme",
            RelanceLevel::MiseEnDemeure => "Mise en demeure",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            RelanceLevel::Amiable => "#38bdf8",
            RelanceLevel::Ferme => "#fbbf24",
            RelanceLevel::MiseEnDemeure => "#f87171",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            RelanceLevel::Amiable => "Ton courtois, suppose un oubli. Recommandé à partir de J+7.",
            RelanceLevel::Ferme => "Ton ferme, mentionne les pénalités légales et l'indemnité forfaitaire. Recommandé à partir de J+30.",
            RelanceLevel::MiseEnDemeure => "Courrier formel engageant les poursuites. Recommandé à partir de J+60.",
        }
    }
}

fn substitute(template: &str, ctx: &RelanceContext) -> String {
    let mut s = template.to_string();
    s = s.replace("{{sci_name}}", &ctx.sci_name);
    s = s.replace("{{sci_address}}", &ctx.sci_address);
    s = s.replace("{{sci_siren}}", &ctx.sci_siren);
    s = s.replace("{{sci_iban}}", &ctx.sci_iban);
    s = s.replace("{{sci_bic}}", &ctx.sci_bic);
    s = s.replace("{{client_name}}", &ctx.client_name);
    s = s.replace("{{invoice_ref}}", &ctx.invoice_ref);
    s = s.replace("{{invoice_due_date}}", &ctx.invoice_due_date);
    s = s.replace("{{invoice_amount}}", &ctx.invoice_amount);
    s = s.replace("{{outstanding_amount}}", &ctx.outstanding_amount);
    s = s.replace("{{days_overdue}}", &ctx.days_overdue.to_string());
    s = s.replace("{{penalties}}", &ctx.penalties);
    s = s.replace("{{forfait}}", &ctx.forfait);
    s = s.replace("{{total_due}}", &ctx.total_due);
    s = s.replace("{{tribunal}}", &ctx.tribunal);
    s
}

pub fn subject_for(level: RelanceLevel, ctx: &RelanceContext) -> String {
    let tpl = match level {
        RelanceLevel::Amiable => "Rappel amical - Facture {{invoice_ref}}",
        RelanceLevel::Ferme => "Relance - Facture {{invoice_ref}} impayée depuis {{days_overdue}} jours",
        RelanceLevel::MiseEnDemeure => "MISE EN DEMEURE - Facture {{invoice_ref}}",
    };
    substitute(tpl, ctx)
}

pub fn body_for(level: RelanceLevel, ctx: &RelanceContext) -> String {
    let tpl = match level {
        RelanceLevel::Amiable => AMIABLE_TPL,
        RelanceLevel::Ferme => FERME_TPL,
        RelanceLevel::MiseEnDemeure => MED_TPL,
    };
    substitute(tpl, ctx)
}

const AMIABLE_TPL: &str = r#"Bonjour,

Sauf erreur de notre part, nous n'avons pas reçu le règlement de la facture {{invoice_ref}} d'un montant de {{outstanding_amount}} EUR, échue le {{invoice_due_date}} (soit {{days_overdue}} jours de retard).

Il s'agit peut-être d'un simple oubli. Si le paiement a déjà été effectué, merci d'ignorer ce message.

Vous pouvez régler par virement bancaire :
- IBAN : {{sci_iban}}
- BIC : {{sci_bic}}
- Référence à indiquer : {{invoice_ref}}

Si vous rencontrez une difficulté particulière, n'hésitez pas à nous contacter.

Bien cordialement,

{{sci_name}}
{{sci_address}}
SIREN : {{sci_siren}}
"#;

const FERME_TPL: &str = r#"Madame, Monsieur,

Malgré notre précédent rappel, nous constatons que la facture {{invoice_ref}} d'un montant initial de {{invoice_amount}} EUR, échue le {{invoice_due_date}}, reste impayée à ce jour (soit {{days_overdue}} jours de retard).

Solde restant dû : {{outstanding_amount}} EUR

Conformément à l'article L441-10 du Code de commerce, des pénalités de retard de {{penalties}} EUR sont désormais dues, ainsi qu'une indemnité forfaitaire pour frais de recouvrement de {{forfait}} EUR.

Montant total réclamé : {{total_due}} EUR

Nous vous demandons de procéder au règlement sous 8 jours à compter de la réception de ce message.

Coordonnées bancaires :
- IBAN : {{sci_iban}}
- BIC : {{sci_bic}}
- Référence : {{invoice_ref}}

À défaut de paiement dans ce délai, nous serons contraints d'engager une procédure de recouvrement.

Cordialement,

{{sci_name}}
{{sci_address}}
SIREN : {{sci_siren}}
"#;

const MED_TPL: &str = r#"Madame, Monsieur,

Par la présente, nous vous mettons en demeure de nous régler la somme de {{total_due}} EUR, se décomposant comme suit :

- Facture {{invoice_ref}} (échue le {{invoice_due_date}}) : {{outstanding_amount}} EUR
- Pénalités de retard : {{penalties}} EUR
- Indemnité forfaitaire pour frais de recouvrement : {{forfait}} EUR

Conformément aux articles 1231-6 et 1344 du Code civil, cette mise en demeure fait courir les intérêts au taux légal à compter de la date d'envoi du présent message.

À défaut de règlement intégral dans un délai de 8 jours, nous serons contraints d'engager une procédure judiciaire de recouvrement devant le Tribunal judiciaire de {{tribunal}}, sans autre préavis, les frais de procédure étant à votre charge.

Coordonnées bancaires pour le règlement :
- IBAN : {{sci_iban}}
- BIC : {{sci_bic}}
- Référence à indiquer : {{invoice_ref}}

{{sci_name}}
{{sci_address}}
SIREN : {{sci_siren}}
"#;