use chrono::{Duration, NaiveDate, Utc};
use dioxus::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use uuid::Uuid;
use crate::domain::*;
use crate::server::*;

const CSS: Asset = asset!("/assets/main.css");

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Dashboard, Setup, Associates, Tenants, Patrimony, Rentals, Billing, Bank,
    Vat, Calendar, Documents, Automations, Tasks, Audit,
}
impl Page {
    fn label(self) -> &'static str {
        match self {
            Page::Dashboard=>"Vue d’ensemble", Page::Setup=>"Configuration", Page::Associates=>"Associés",
            Page::Tenants=>"Locataires", Page::Patrimony=>"Patrimoine", Page::Rentals=>"Locations",
            Page::Billing=>"Facturation", Page::Bank=>"Banque", Page::Vat=>"TVA", Page::Calendar=>"Calendrier",
            Page::Documents=>"Documents", Page::Automations=>"Automatisations", Page::Tasks=>"Tâches", Page::Audit=>"Audit",
        }
    }
}

#[component]
pub fn App() -> Element {
    let mut page = use_signal(|| Page::Dashboard);
    let mut refresh = use_signal(|| 0u64);
    rsx! {
        document::Link { rel:"stylesheet", href:CSS }
        div { class:"app-shell",
            aside { class:"sidebar",
                div{class:"brand","SCI FAMILY"}
                div{class:"brand-sub","PILOTAGE ADMINISTRATIF AUTONOME"}
                nav {
                    NavItem { page, current:Page::Dashboard } NavItem { page, current:Page::Setup }
                    NavItem { page, current:Page::Associates } NavItem { page, current:Page::Tenants }
                    NavItem { page, current:Page::Patrimony } NavItem { page, current:Page::Rentals }
                    NavItem { page, current:Page::Billing } NavItem { page, current:Page::Bank }
                    NavItem { page, current:Page::Vat } NavItem { page, current:Page::Calendar }
                    NavItem { page, current:Page::Documents } NavItem { page, current:Page::Automations }
                    NavItem { page, current:Page::Tasks } NavItem { page, current:Page::Audit }
                }
                div{class:"sidebar-footer","Données locales • règles versionnées • audit"}
            }
            main { class:"main",
                header { class:"topbar",
                    div{div{class:"eyebrow","SCI FAMILY PILOT"},h1{{page().label()}}}
                    div{class:"top-actions",button{class:"secondary",onclick:move |_| page.set(Page::Setup),"Configuration"},button{class:"primary",onclick:move |_|async move{let _=run_anticipation_cycle().await;refresh+=1},"Lancer l’anticipation"}}
                }
                match page() {
                    Page::Dashboard=>rsx!{Dashboard{refresh,on_setup:move |_|page.set(Page::Setup)}},
                    Page::Setup=>rsx!{SetupPage{refresh}}, Page::Associates=>rsx!{AssociatesPage{refresh}},
                    Page::Tenants=>rsx!{TenantsPage{refresh}}, Page::Patrimony=>rsx!{PatrimonyPage{refresh}},
                    Page::Rentals=>rsx!{RentalsPage{refresh}}, Page::Billing=>rsx!{BillingPage{refresh}},
                    Page::Bank=>rsx!{BankPage{refresh}}, Page::Vat=>rsx!{VatPage{refresh}}, Page::Calendar=>rsx!{CalendarPage{refresh}},
                    Page::Documents=>rsx!{DocumentsPage{refresh}}, Page::Automations=>rsx!{AutomationsPage{refresh}},
                    Page::Tasks=>rsx!{TasksPage{refresh}}, Page::Audit=>rsx!{AuditPage{refresh}},
                }
            }
        }
    }
}

#[component]fn NavItem(mut page:Signal<Page>,current:Page)->Element{rsx!{button{class:if page()==current{"nav-item active"}else{"nav-item"},onclick:move |_|page.set(current),{current.label()}}}}

#[component]
fn Dashboard(refresh:Signal<u64>,on_setup:EventHandler<MouseEvent>)->Element{
    let data=use_resource(move||{let _=refresh();async move{dashboard_snapshot().await.ok()}});
    let counts=use_resource(move||{let _=refresh();async move{module_counts().await.ok()}});
    let status=use_resource(move||{let _=refresh();async move{onboarding_status().await.ok()}});

    match(&*data.read(),&*counts.read(),&*status.read()){
        (Some(Some(d)),Some(Some(c)),Some(Some(s)))=>rsx!{
            if !s.completed {
                section{class:"setup-banner",
                    div{
                        div{
                            class:"eyebrow",
                            "MISE EN ROUTE • {s.completion_pct}%"
                        }
                        h2{"Votre SCI passe du déclaratif au pilotage"}
                        p{"Configurez les référentiels une fois ; les baux, factures, encaissements, TVA, banque et échéances seront ensuite reliés."}
                    }
                    button{class:"primary",onclick:on_setup,"Continuer"}
                }
            }

            section{class:"welcome",
                div{
                    span{class:"pill","SCI À L’IR"}
                    span{class:"pill muted","TVA sur encaissements"}
                    h2{{d.sci_name.clone()}}
                    p{{d.registered_office.clone()}}
                }
                div{class:"risk-block",
                    div{class:"eyebrow","VIGILANCE"}
                    div{class:"risk",{d.risk_level.clone()}}
                    div{class:"small",{format!("{} tâche(s) en retard",d.overdue_tasks)}}
                }
            }

            section{class:"metric-row",
                Metric{label:"Trésorerie",value:euro(d.cash_cents),tone:"positive"}
                Metric{label:"Créances",value:euro(d.receivables_cents),tone:"neutral"}
                Metric{label:"TVA du mois",value:euro(d.vat_to_prepare_cents),tone:"warning"}
                Metric{label:"À traiter < 30 j",value:d.tasks_due_30d.to_string(),tone:"neutral"}
            }

            section{class:"module-grid",
                ModuleCard{title:"Associés",value:c.associates.to_string(),label:"actifs",detail:"Capital et comptes courants"}
                ModuleCard{title:"Patrimoine",value:c.properties.to_string(),label:"biens",detail:format!("{} lots",c.units)}
                ModuleCard{title:"Locataires",value:c.tenants.to_string(),label:"actifs",detail:"Tiers exploitables"}
                ModuleCard{title:"Locations",value:c.leases.to_string(),label:"baux",detail:"Révision et facturation"}
                ModuleCard{title:"Facturation",value:c.invoices.to_string(),label:"factures",detail:format!("{} paiements",c.payments)}
                ModuleCard{title:"Banque",value:c.bank_transactions.to_string(),label:"mouvements",detail:format!("{} non rapprochés",c.unmatched_bank)}
                ModuleCard{title:"TVA",value:euro(c.vat_receipts_cents),label:"encaissé ce mois",detail:"Exigibilité calculée"}
                ModuleCard{title:"Documents",value:c.documents.to_string(),label:"pièces",detail:"Référentiel documentaire"}
                ModuleCard{title:"Tâches",value:c.open_tasks.to_string(),label:"ouvertes",detail:"Workflow administratif"}
            }

            section{class:"two-col",
                div{class:"panel",
                    div{class:"panel-head",
                        h3{"Prochaines actions"}
                        span{class:"small","Priorisées"}
                    }
                    if d.next_actions.is_empty() {
                        EmptyState{title:"Aucune action",text:"Lancez l’anticipation pour générer les premières tâches."}
                    }
                    for t in d.next_actions.iter() {
                        TaskRow{task:t.clone()}
                    }
                }
                div{class:"panel",
                    div{class:"panel-head",
                        h3{"Trésorerie prévisionnelle"}
                        span{class:"small","12 mois"}
                    }
                    div{class:"forecast-grid",
                        for f in d.forecast.iter().take(6) {
                            div{class:"forecast-card",
                                div{class:"small",{f.date.format("%b %Y").to_string()}}
                                div{class:"forecast-value",{euro(f.balance_cents)}}
                                div{class:"small",{format!("+{} / -{}",euro(f.expected_inflows_cents),euro(f.expected_outflows_cents))}}
                            }
                        }
                    }
                    div{class:"forecast-min",
                        "Point bas : "
                        {euro(d.forecast_min_cash_cents)}
                    }
                }
            }

            section{class:"panel",
                h3{"État de préparation"}
                div{class:"check-grid",
                    Check{ok:s.profile_ready,title:"SCI",text:"Identité et siège"}
                    Check{ok:s.associates_ready,title:"Associés",text:"Capital"}
                    Check{ok:s.property_ready,title:"Patrimoine",text:"Bien + lot"}
                    Check{ok:s.tenant_ready,title:"Locataires",text:"Tiers"}
                    Check{ok:s.lease_ready,title:"Baux",text:"Occupation"}
                    Check{ok:s.finance_ready,title:"Flux",text:"Banque ou factures"}
                    Check{ok:s.automation_ready,title:"Moteur",text:"Règles actives"}
                }
            }
        },
        _=>rsx!{Loading{}}
    }
}

#[component]
fn SetupPage(refresh:Signal<u64>)->Element{
    let _=refresh(); let profile=use_resource(||async move{get_sci_profile().await.ok()}); let mut init=use_signal(||false);
    let mut name=use_signal(String::new);let mut siren=use_signal(String::new);let mut siret=use_signal(String::new);let mut office=use_signal(String::new);let mut iban=use_signal(String::new);let mut bic=use_signal(String::new);let mut msg=use_signal(String::new);
    if !init(){if let Some(Some(p))=&*profile.read(){name.set(p.legal_name.clone());siren.set(p.siren.clone());siret.set(p.siret.clone());office.set(p.registered_office.clone());iban.set(p.iban.clone());bic.set(p.bic.clone());init.set(true);}}
    rsx!{ModuleHeader{title:"Configuration de la SCI",kicker:"IDENTITÉ • FISCALITÉ • BANQUE",detail:"La configuration est versionnée et réutilisée par les autres modules."}
      section{class:"panel",div{class:"form-grid",FormField{label:"Dénomination sociale",value:name(),oninput:move|e:FormEvent|name.set(e.value())}FormField{label:"SIREN",value:siren(),oninput:move|e:FormEvent|siren.set(e.value())}FormField{label:"SIRET",value:siret(),oninput:move|e:FormEvent|siret.set(e.value())}FormField{label:"Siège social",value:office(),oninput:move|e:FormEvent|office.set(e.value())}FormField{label:"IBAN",value:iban(),oninput:move|e:FormEvent|iban.set(e.value())}FormField{label:"BIC",value:bic(),oninput:move|e:FormEvent|bic.set(e.value())}},div{class:"facts-row",InfoTile{label:"Régime",value:"IR"}InfoTile{label:"TVA",value:"Option locations"}InfoTile{label:"Exigibilité",value:"Encaissement"}InfoTile{label:"Devise",value:"EUR"}},div{class:"action-row",button{class:"primary",onclick:move |_|async move{let p=SciProfile{legal_name:name(),siren:siren(),siret:siret(),registered_office:office(),tax_regime:"IR".into(),vat_status:"OPTION_LOYERS".into(),vat_basis:"COLLECTION".into(),accounting_period_start:1,fiscal_year_end:12,iban:iban(),bic:bic()};match update_sci_profile(p).await{Ok(_)=>msg.set("Configuration enregistrée".into()),Err(e)=>msg.set(e.to_string())}},"Enregistrer"}span{class:"save-ok",{msg()}}}}
    }
}

#[component]
fn AssociatesPage(refresh:Signal<u64>)->Element{
    let mut bump=use_signal(||0u64);let items=use_resource(move||{let _=refresh();let _=bump();async move{list_associates().await.unwrap_or_default()}});
    let mut name=use_signal(String::new);let mut pct=use_signal(||"0".to_string());let mut cca=use_signal(||"0".to_string());let mut msg=use_signal(String::new);
    rsx!{ModuleHeader{title:"Associés",kicker:"CAPITAL • COMPTES COURANTS",detail:"Le total des quote-parts actives est borné à 100 %."}
      section{class:"panel",div{class:"form-grid",FormField{label:"Nom",value:name(),oninput:move|e:FormEvent|name.set(e.value())}FormField{label:"Quote-part %",value:pct(),oninput:move|e:FormEvent|pct.set(e.value())}FormField{label:"Compte courant initial €",value:cca(),oninput:move|e:FormEvent|cca.set(e.value())}},div{class:"action-row",button{class:"primary",onclick:move |_|async move{match pct().replace(",",".").parse::<Decimal>(){Ok(v)=>match create_associate(name(),v,euros_to_cents(&cca())).await{Ok(_)=>{msg.set("Associé ajouté".into());bump+=1},Err(e)=>msg.set(e.to_string())},Err(_)=>msg.set("Quote-part invalide".into())}},"Ajouter"}span{class:"save-ok",{msg()}}}}
      section{class:"panel",div{for a in items.read().as_deref().unwrap_or(&[]).iter(){AssociateRow{item:a.clone(),bump}}}}
    }
}

#[component]
fn TenantsPage(refresh:Signal<u64>)->Element{
    let mut bump=use_signal(||0u64);let items=use_resource(move||{let _=refresh();let _=bump();async move{list_tenants().await.unwrap_or_default()}});let mut name=use_signal(String::new);let mut siret=use_signal(String::new);let mut email=use_signal(String::new);let mut phone=use_signal(String::new);let mut msg=use_signal(String::new);
    rsx!{ModuleHeader{title:"Locataires",kicker:"TIERS • CONTACTS • DOSSIERS",detail:"Les locataires alimentent les baux et les factures. Les suppressions sont bloquées dès qu’un bail existe."}
      section{class:"panel",div{class:"form-grid",FormField{label:"Raison sociale / nom",value:name(),oninput:move|e:FormEvent|name.set(e.value())}FormField{label:"SIRET",value:siret(),oninput:move|e:FormEvent|siret.set(e.value())}FormField{label:"Email",value:email(),oninput:move|e:FormEvent|email.set(e.value())}FormField{label:"Téléphone",value:phone(),oninput:move|e:FormEvent|phone.set(e.value())}},div{class:"action-row",button{class:"primary",onclick:move |_|async move{match create_tenant(name(),siret(),email(),phone()).await{Ok(_)=>{msg.set("Locataire créé".into());bump+=1},Err(e)=>msg.set(e.to_string())}},"Créer"}span{class:"save-ok",{msg()}}}}
      section{class:"panel",div{for t in items.read().as_deref().unwrap_or(&[]).iter(){TenantRow{item:t.clone(),bump}}}}
    }
}

#[component]
fn PatrimonyPage(refresh:Signal<u64>)->Element{
    let mut bump=use_signal(||0u64);let props=use_resource(move||{let _=refresh();let _=bump();async move{list_properties().await.unwrap_or_default()}});let units=use_resource(move||{let _=refresh();let _=bump();async move{list_units().await.unwrap_or_default()}});
    let mut name=use_signal(String::new);let mut address=use_signal(String::new);let mut acq=use_signal(String::new);let mut pid=use_signal(String::new);let mut code=use_signal(String::new);let mut label=use_signal(String::new);let mut rent=use_signal(String::new);let mut rate=use_signal(||"20".to_string());let mut msg=use_signal(String::new);
    rsx!{ModuleHeader{title:"Patrimoine",kicker:"BIENS • LOTS • LOYERS",detail:"Le patrimoine est la source des baux, indexations et factures."}
      section{class:"two-col",div{class:"panel",h3{"Nouveau bien"},div{class:"form-grid",FormField{label:"Nom",value:name(),oninput:move|e:FormEvent|name.set(e.value())}FormField{label:"Adresse",value:address(),oninput:move|e:FormEvent|address.set(e.value())}FormField{label:"Acquisition €",value:acq(),oninput:move|e:FormEvent|acq.set(e.value())}},button{class:"primary",onclick:move |_|async move{match create_property(name(),address(),None,Some(euros_to_cents(&acq()))).await{Ok(_)=>{msg.set("Bien créé".into());bump+=1},Err(e)=>msg.set(e.to_string())}},"Créer le bien"}},div{class:"panel",h3{"Nouveau lot"},div{class:"form-grid",label{class:"field",span{"Bien"},select{value:pid(),onchange:move|e:FormEvent|pid.set(e.value()),option{value:"","Sélectionner"},for p in props.read().as_deref().unwrap_or(&[]).iter(){option{value:p.id.to_string(),{p.name.clone()}}}}}FormField{label:"Code",value:code(),oninput:move|e:FormEvent|code.set(e.value())}FormField{label:"Libellé",value:label(),oninput:move|e:FormEvent|label.set(e.value())}FormField{label:"Loyer €",value:rent(),oninput:move|e:FormEvent|rent.set(e.value())}FormField{label:"TVA %",value:rate(),oninput:move|e:FormEvent|rate.set(e.value())}},button{class:"primary",onclick:move |_|async move{match Uuid::parse_str(&pid()){Ok(id)=>{let bp=rate().replace(",",".").parse::<Decimal>().unwrap_or(Decimal::from(20u32));let bp=(bp*Decimal::from(100u32)).round_dp(0).to_i32().unwrap_or(2000);match create_unit(id,code(),label(),"COMMERCIAL".into(),None,euros_to_cents(&rent()),bp).await{Ok(_)=>{msg.set("Lot créé".into());bump+=1},Err(e)=>msg.set(e.to_string())}},Err(_)=>msg.set("Sélectionnez un bien".into())}},"Créer le lot"}}}
      section{class:"panel",div{class:"panel-head",h3{"Biens"},span{class:"small",{msg()}}},div{for p in props.read().as_deref().unwrap_or(&[]).iter(){PropertyRow{item:p.clone(),bump}}}}
      section{class:"panel",h3{"Lots"},div{for u in units.read().as_deref().unwrap_or(&[]).iter(){UnitRow{item:u.clone(),bump}}}}
    }
}

#[component]
fn RentalsPage(refresh:Signal<u64>)->Element{
    let mut bump=use_signal(||0u64);let leases=use_resource(move||{let _=refresh();let _=bump();async move{list_leases().await.unwrap_or_default()}});let units=use_resource(move||{let _=refresh();async move{list_units().await.unwrap_or_default()}});let tenants=use_resource(move||{let _=refresh();async move{list_tenants().await.unwrap_or_default()}});
    let mut unit=use_signal(String::new);let mut tenant=use_signal(String::new);let mut refx=use_signal(String::new);let mut start=use_signal(||Utc::now().date_naive().to_string());let mut end=use_signal(String::new);let mut day=use_signal(||"5".to_string());let mut msg=use_signal(String::new);
    rsx!{ModuleHeader{title:"Locations",kicker:"BAUX • PRÉAVIS • RÉVISIONS",detail:"Les baux actifs pilotent la préparation des loyers et les échéances administratives."}
      section{class:"panel",div{class:"form-grid",label{class:"field",span{"Lot"},select{value:unit(),onchange:move|e:FormEvent|unit.set(e.value()),option{value:"","Sélectionner"},for u in units.read().as_deref().unwrap_or(&[]).iter(){option{value:u.id.to_string(),{format!("{} • {}",u.property_name,u.label)}}}}}label{class:"field",span{"Locataire"},select{value:tenant(),onchange:move|e:FormEvent|tenant.set(e.value()),option{value:"","Sélectionner"},for t in tenants.read().as_deref().unwrap_or(&[]).iter(){option{value:t.id.to_string(),{t.legal_name.clone()}}}}}FormField{label:"Référence",value:refx(),oninput:move|e:FormEvent|refx.set(e.value())}FormField{label:"Début AAAA-MM-JJ",value:start(),oninput:move|e:FormEvent|start.set(e.value())}FormField{label:"Fin AAAA-MM-JJ",value:end(),oninput:move|e:FormEvent|end.set(e.value())}FormField{label:"Jour de paiement",value:day(),oninput:move|e:FormEvent|day.set(e.value())}},div{class:"action-row",button{class:"primary",onclick:move |_|async move{match(Uuid::parse_str(&unit()),Uuid::parse_str(&tenant()),NaiveDate::parse_from_str(&start(),"%Y-%m-%d")){(Ok(u),Ok(t),Ok(s))=>{let e=if end().trim().is_empty(){None}else{NaiveDate::parse_from_str(&end(),"%Y-%m-%d").ok()};match create_lease(u,t,refx(),s,e,3,day().parse().unwrap_or(5),None).await{Ok(_)=>{msg.set("Bail créé".into());bump+=1},Err(e)=>msg.set(e.to_string())}},_=>(msg.set("Lot, locataire ou date invalide".into()))}},"Créer le bail"}span{class:"save-ok",{msg()}}}}
      section{class:"panel",div{for l in leases.read().as_deref().unwrap_or(&[]).iter(){LeaseRow{item:l.clone(),bump}}}}
    }
}

#[component]
fn BillingPage(refresh:Signal<u64>)->Element{
    let mut bump=use_signal(||0u64);
    let inv=use_resource(move||{let _=refresh();let _=bump();async move{list_invoices().await.unwrap_or_default()}});
    let leases=use_resource(move||{let _=refresh();async move{list_leases().await.unwrap_or_default()}});
    let mut lease=use_signal(String::new);
    let mut issue=use_signal(||Utc::now().date_naive().to_string());
    let mut due=use_signal(||(Utc::now().date_naive()+Duration::days(30)).to_string());
    let mut pay_invoice=use_signal(String::new);
    let mut pay_amount=use_signal(String::new);
    let mut pay_ref=use_signal(String::new);
    let mut msg=use_signal(String::new);
    rsx!{
        ModuleHeader{title:"Facturation",kicker:"FACTURES • ÉMISSIONS • ENCAISSEMENTS",detail:"Création des brouillons depuis les baux, émission, suivi des paiements et calcul de TVA à l’encaissement."}
        section{class:"two-col",
            div{class:"panel",
                h3{"Préparer une facture"}
                div{class:"form-grid",
                    label{class:"field",
                        span{"Bail"}
                        select{value:lease(),onchange:move|e:FormEvent|lease.set(e.value()),
                            option{value:"","Sélectionner"}
                            for l in leases.read().as_deref().unwrap_or(&[]).iter(){
                                option{value:l.id.to_string(),{format!("{} • {}",l.reference,l.tenant_name)}}
                            }
                        }
                    }
                    FormField{label:"Émission",value:issue(),oninput:move|e:FormEvent|issue.set(e.value())}
                    FormField{label:"Échéance",value:due(),oninput:move|e:FormEvent|due.set(e.value())}
                }
                button{class:"primary",
                    onclick:move |_|async move{
                        match(Uuid::parse_str(&lease()),NaiveDate::parse_from_str(&issue(),"%Y-%m-%d"),NaiveDate::parse_from_str(&due(),"%Y-%m-%d")){
                            (Ok(l),Ok(i),Ok(d))=>match create_invoice_from_lease(l,i,d).await{
                                Ok(_)=>{msg.set("Brouillon créé".into());bump+=1},
                                Err(e)=>msg.set(e.to_string())
                            },
                            _=>msg.set("Données de facture invalides".into())
                        }
                    },
                    "Créer le brouillon"
                }
            }
            div{class:"panel",
                h3{"Enregistrer un encaissement"}
                div{class:"form-grid",
                    label{class:"field",
                        span{"Facture"}
                        select{value:pay_invoice(),onchange:move|e:FormEvent|pay_invoice.set(e.value()),
                            option{value:"","Sélectionner"}
                            for i in inv.read().as_deref().unwrap_or(&[]).iter().filter(|x|x.status!="BROUILLON"&&x.paid_cents<x.gross_cents){
                                option{value:i.id.to_string(),{format!("{} • {}",i.invoice_number,i.tenant_name)}}
                            }
                        }
                    }
                    FormField{label:"Montant €",value:pay_amount(),oninput:move|e:FormEvent|pay_amount.set(e.value())}
                    FormField{label:"Référence",value:pay_ref(),oninput:move|e:FormEvent|pay_ref.set(e.value())}
                }
                button{class:"primary",
                    onclick:move |_|async move{
                        let iid=Uuid::parse_str(&pay_invoice()).ok();
                        match create_payment(iid,Utc::now(),euros_to_cents(&pay_amount()),pay_ref(),"BANK".into()).await{
                            Ok(_)=>{msg.set("Encaissement enregistré".into());bump+=1},
                            Err(e)=>msg.set(e.to_string())
                        }
                    },
                    "Enregistrer"
                }
            }
        }
        span{class:"save-ok",{msg()}}
        section{class:"panel",
            div{
                for i in inv.read().as_deref().unwrap_or(&[]).iter(){
                    InvoiceRow{item:i.clone(),bump}
                }
            }
        }
        section{class:"panel",
            h3{"Historique des paiements"}
            PaymentList{refresh:bump}
        }
    }
}

#[component]
fn BankPage(refresh:Signal<u64>)->Element{
    let mut bump=use_signal(||0u64);let txs=use_resource(move||{let _=refresh();let _=bump();async move{list_bank_transactions().await.unwrap_or_default()}});let inv=use_resource(move||{let _=refresh();async move{list_invoices().await.unwrap_or_default()}});
    let mut amount=use_signal(String::new);let mut label=use_signal(String::new);let mut cp=use_signal(String::new);let mut ext=use_signal(String::new);let mut selected=use_signal(String::new);let mut selected_inv=use_signal(String::new);let mut csv=use_signal(String::new);let mut msg=use_signal(String::new);
    rsx!{ModuleHeader{title:"Banque",kicker:"IMPORT CSV • RAPPROCHEMENT • CONTRÔLE",detail:"Import idempotent par identifiant externe, suivi des mouvements et rapprochement vers les factures."}
      section{class:"two-col",div{class:"panel",h3{"Saisie d’un mouvement"},div{class:"form-grid",FormField{label:"Montant €",value:amount(),oninput:move|e:FormEvent|amount.set(e.value())}FormField{label:"Libellé",value:label(),oninput:move|e:FormEvent|label.set(e.value())}FormField{label:"Contrepartie",value:cp(),oninput:move|e:FormEvent|cp.set(e.value())}FormField{label:"Identifiant externe",value:ext(),oninput:move|e:FormEvent|ext.set(e.value())}},button{class:"primary",onclick:move |_|async move{match create_bank_transaction(Utc::now(),Some(Utc::now().date_naive()),euros_to_cents(&amount()),label(),cp(),ext()).await{Ok(_)=>{msg.set("Mouvement enregistré".into());bump+=1},Err(e)=>msg.set(e.to_string())}},"Enregistrer"}},div{class:"panel",h3{"Import bancaire"},p{class:"small","Format : AAAA-MM-JJ;montant;libellé;identifiant"},textarea{value:csv(),oninput:move|e:FormEvent|csv.set(e.value()),placeholder:"2026-09-01;1200,50;VIREMENT LOYER;CA-001"},button{class:"primary",onclick:move |_|async move{match import_bank_csv(csv()).await{Ok(n)=>{msg.set(format!("{} mouvement(s) importé(s)",n));bump+=1},Err(e)=>msg.set(e.to_string())}},"Importer le CSV"}}}
      section{class:"panel",h3{"Rapprochement"},div{class:"form-grid",label{class:"field",span{"Mouvement"},select{value:selected(),onchange:move|e:FormEvent|selected.set(e.value()),option{value:"","Sélectionner"},for t in txs.read().as_deref().unwrap_or(&[]).iter().filter(|x|x.reconciliation_status=="UNMATCHED"&&x.amount_cents>0){option{value:t.id.to_string(),{format!("{} • {}",euro(t.amount_cents),t.label)}}}}}label{class:"field",span{"Facture"},select{value:selected_inv(),onchange:move|e:FormEvent|selected_inv.set(e.value()),option{value:"","Sélectionner"},for i in inv.read().as_deref().unwrap_or(&[]).iter().filter(|x|x.status!="BROUILLON"&&x.paid_cents<x.gross_cents){option{value:i.id.to_string(),{format!("{} • {}",i.invoice_number,i.tenant_name)}}}}}},button{class:"primary",onclick:move |_|async move{match(Uuid::parse_str(&selected()),Uuid::parse_str(&selected_inv())){(Ok(b),Ok(i))=>match reconcile_bank_transaction(b,i).await{Ok(_)=>{msg.set("Rapprochement effectué".into());bump+=1},Err(e)=>msg.set(e.to_string())},_=>(msg.set("Sélection incomplète".into()))}},"Rapprocher"}span{class:"save-ok",{msg()}}}
      section{class:"panel",div{for t in txs.read().as_deref().unwrap_or(&[]).iter(){BankRow{item:t.clone(),bump}}}}
    }
}

#[component]
fn VatPage(refresh:Signal<u64>)->Element{
    let _=refresh();
    let mut period=use_signal(||Utc::now().date_naive().format("%Y-%m").to_string());
    let summary=use_resource(move||{
        let _=refresh();
        let p=period();
        async move{vat_summary(p).await.ok()}
    });
    rsx!{
        ModuleHeader{title:"TVA",kicker:"COLLECTE • ENCAISSEMENT • PRÉPARATION",detail:"La TVA présentée ici est dérivée des encaissements rapprochés avec les factures."}
        section{class:"panel",FormField{label:"Période AAAA-MM",value:period(),oninput:move|e:FormEvent|period.set(e.value())}}
        match &*summary.read(){
            Some(Some(v))=>rsx!{
                section{class:"facts-row",
                    InfoTileOwned{label:"Encaissements",value:euro(v.receipts_gross_cents)}
                    InfoTileOwned{label:"Base taxable",value:euro(v.taxable_net_cents)}
                    InfoTileOwned{label:"TVA exigible",value:euro(v.vat_due_cents)}
                    InfoTileOwned{label:"Paiements",value:v.payments_count.to_string()}
                }
            },
            _=>rsx!{Loading{}}
        }
    }
}

#[component]
fn CalendarPage(refresh:Signal<u64>)->Element{let mut bump=use_signal(||0u64);let items=use_resource(move||{let _=refresh();let _=bump();async move{list_deadlines().await.unwrap_or_default()}});let mut code=use_signal(String::new);let mut label=use_signal(String::new);let mut date=use_signal(||(Utc::now().date_naive()+Duration::days(30)).to_string());let mut period=use_signal(String::new);let mut msg=use_signal(String::new);rsx!{ModuleHeader{title:"Calendrier",kicker:"ÉCHÉANCES • PRÉPARATION • RELANCES",detail:"Les échéances saisies deviennent des tâches bloquantes avant la date limite."}section{class:"panel",div{class:"form-grid",FormField{label:"Code",value:code(),oninput:move|e:FormEvent|code.set(e.value())}FormField{label:"Libellé",value:label(),oninput:move|e:FormEvent|label.set(e.value())}FormField{label:"Date AAAA-MM-JJ",value:date(),oninput:move|e:FormEvent|date.set(e.value())}FormField{label:"Période",value:period(),oninput:move|e:FormEvent|period.set(e.value())}},button{class:"primary",onclick:move |_|async move{match NaiveDate::parse_from_str(&date(),"%Y-%m-%d"){Ok(d)=>match create_deadline(code(),label(),d,period()).await{Ok(_)=>{msg.set("Échéance enregistrée".into());bump+=1},Err(e)=>msg.set(e.to_string())},Err(_)=>msg.set("Date invalide".into())}},"Ajouter"}span{class:"save-ok",{msg()}}}section{class:"panel",div{for d in items.read().as_deref().unwrap_or(&[]).iter(){DeadlineRow{item:d.clone(),bump}}}}}}

#[component]
fn DocumentsPage(refresh:Signal<u64>)->Element{let mut bump=use_signal(||0u64);let items=use_resource(move||{let _=refresh();let _=bump();async move{list_documents().await.unwrap_or_default()}});let mut cat=use_signal(||"ADMINISTRATIF".to_string());let mut title=use_signal(String::new);let mut file=use_signal(String::new);let mut key=use_signal(String::new);let mut expiry=use_signal(String::new);let mut msg=use_signal(String::new);rsx!{ModuleHeader{title:"Documents",kicker:"RÉFÉRENTIEL • EXPIRATIONS • TRAÇABILITÉ",detail:"Les pièces sont indexées dans le référentiel métier et peuvent être reliées aux contrôles."}section{class:"panel",div{class:"form-grid",FormField{label:"Catégorie",value:cat(),oninput:move|e:FormEvent|cat.set(e.value())}FormField{label:"Titre",value:title(),oninput:move|e:FormEvent|title.set(e.value())}FormField{label:"Nom de fichier",value:file(),oninput:move|e:FormEvent|file.set(e.value())}FormField{label:"Clé de stockage",value:key(),oninput:move|e:FormEvent|key.set(e.value())}FormField{label:"Expiration AAAA-MM-JJ",value:expiry(),oninput:move|e:FormEvent|expiry.set(e.value())}},button{class:"primary",onclick:move |_|async move{let d=if expiry().trim().is_empty(){None}else{NaiveDate::parse_from_str(&expiry(),"%Y-%m-%d").ok()};match register_document(cat(),title(),file(),key(),None,d).await{Ok(_)=>{msg.set("Document indexé".into());bump+=1},Err(e)=>msg.set(e.to_string())}},"Indexer"}span{class:"save-ok",{msg()}}}section{class:"panel",div{for d in items.read().as_deref().unwrap_or(&[]).iter(){DocumentRow{item:d.clone(),bump}}}}}}

#[component]
fn AutomationsPage(refresh:Signal<u64>)->Element{let mut bump=use_signal(||0u64);let rules=use_resource(move||{let _=refresh();let _=bump();async move{list_automation_rules().await.unwrap_or_default()}});let mut result=use_signal(||None::<AutomationRunResult>);rsx!{ModuleHeader{title:"Automatisations",kicker:"RÈGLES • MOTEUR • IDÉMPOTENCE",detail:"Les règles sont exécutées automatiquement et chaque occurrence est protégée contre les doublons."}section{class:"panel",div{class:"panel-head",h3{"Cycle manuel"},button{class:"primary",onclick:move |_|async move{result.set(run_anticipation_cycle().await.ok());bump+=1}},"Exécuter maintenant"}}if let Some(r)=result(){section{class:"run-result",h3{"Cycle terminé"},p{{format!("{} règles évaluées • {} tâches créées • {}",r.evaluated_rules,r.created_tasks,r.ran_at.format("%d/%m/%Y %H:%M"))}}}}section{class:"automation-grid",div{for r in rules.read().as_deref().unwrap_or(&[]).iter(){AutomationRow{item:r.clone(),bump}}}}}}

#[component]
fn TasksPage(refresh:Signal<u64>)->Element{let mut bump=use_signal(||0u64);let tasks=use_resource(move||{let _=refresh();let _=bump();async move{list_tasks().await.unwrap_or_default()}});rsx!{ModuleHeader{title:"Tâches",kicker:"WORKFLOW • ÉTATS • BLOQUANTS",detail:"Toute action administrative peut être suivie jusqu’à sa clôture."}section{class:"panel",div{for t in tasks.read().as_deref().unwrap_or(&[]).iter(){TaskManagerRow{item:t.clone(),bump}}}}}}

#[component]
fn AuditPage(refresh:Signal<u64>)->Element{let items=use_resource(move||{let _=refresh();async move{list_audit().await.unwrap_or_default()}});rsx!{ModuleHeader{title:"Audit",kicker:"JOURNAL • JUSTIFICATION • HISTORIQUE",detail:"Les actions métiers sont conservées avec leur acteur et leur contexte."}section{class:"panel",div{for a in items.read().as_deref().unwrap_or(&[]).iter(){AuditRow{item:a.clone()}}}}}}

#[component]fn PaymentList(refresh:Signal<u64>)->Element{let items=use_resource(move||{let _=refresh();async move{list_payments().await.unwrap_or_default()}});rsx!{div{for p in items.read().as_deref().unwrap_or(&[]).iter(){PaymentRow{item:p.clone()}}}}}
#[component]fn AssociateRow(item:AssociateItem,bump:Signal<u64>)->Element{let id=item.id;rsx!{div{class:"data-row",div{div{class:"data-title",{item.display_name}},div{class:"small",{format!("Quote-part {} % • C/C {}",item.ownership_pct,euro(item.current_account_cents))}}},div{class:"row-actions",button{class:"secondary",onclick:move |_|async move{let _=delete_associate(id).await;bump+=1},"Supprimer"}}}}}
#[component]fn TenantRow(item:TenantItem,bump:Signal<u64>)->Element{let id=item.id;rsx!{div{class:"data-row",div{div{class:"data-title",{item.legal_name}},div{class:"small",{format!("{} • {}",item.contact_email,item.contact_phone)}}},div{class:"row-actions",button{class:"secondary",onclick:move |_|async move{let _=delete_tenant(id).await;bump+=1},"Supprimer"}}}}}
#[component]fn PropertyRow(item:PropertyItem,bump:Signal<u64>)->Element{let id=item.id;rsx!{div{class:"data-row",div{div{class:"data-title",{item.name}},div{class:"small",{item.address}}},div{class:"row-actions",span{class:"row-value",{format!("{} lots",item.units_count)}},button{class:"secondary",onclick:move |_|async move{let _=delete_property(id).await;bump+=1},"Supprimer"}}}}}
#[component]fn UnitRow(item:UnitItem,bump:Signal<u64>)->Element{let id=item.id;rsx!{div{class:"data-row",div{div{class:"data-title",{format!("{} • {}",item.property_name,item.label)}},div{class:"small",{format!("{} • TVA {} %",item.code,(item.vat_rate_bp as f64)/100.0)}}},div{class:"row-actions",span{class:"row-value",{euro(item.base_rent_cents)}},button{class:"secondary",onclick:move |_|async move{let _=delete_unit(id).await;bump+=1},"Supprimer"}}}}}
#[component]fn LeaseRow(item:LeaseItem,bump:Signal<u64>)->Element{let id=item.id;rsx!{div{class:"data-row",div{div{class:"data-title",{item.reference}},div{class:"small",{format!("{} • {} • {}",item.property_name,item.unit_label,item.tenant_name)}},div{class:"small",{format!("{} → {}",item.start_date,item.end_date.map(|x|x.to_string()).unwrap_or_else(||"ouvert".into()))}}},div{class:"row-actions",button{class:"secondary",onclick:move |_|async move{let _=delete_lease(id).await;bump+=1},"Supprimer"}}}}}
#[component]fn InvoiceRow(item:InvoiceItem,bump:Signal<u64>)->Element{let id=item.id;let draft=item.status=="BROUILLON";rsx!{div{class:"data-row",div{div{class:"data-title",{item.invoice_number}},div{class:"small",{format!("{} • {} • {}",item.tenant_name,item.issue_date,item.status)}},div{class:"small",{format!("{} payé sur {}",euro(item.paid_cents),euro(item.gross_cents))}}},div{class:"row-actions",if draft{button{class:"secondary",onclick:move |_|async move{let _=issue_invoice(id).await;bump+=1},"Émettre"}button{class:"secondary",onclick:move |_|async move{let _=delete_invoice(id).await;bump+=1},"Supprimer"}}}}}}
#[component]fn PaymentRow(item:PaymentItem)->Element{rsx!{div{class:"data-row",div{div{class:"data-title",{item.invoice_number}},div{class:"small",{item.received_at.format("%d/%m/%Y %H:%M").to_string()}},div{class:"small",{item.reference}}},div{class:"row-value",{euro(item.amount_cents)}}}}}
#[component]fn BankRow(item:BankTransactionItem,bump:Signal<u64>)->Element{let id=item.id;rsx!{div{class:"data-row",div{div{class:"data-title",{item.label}},div{class:"small",{item.counterparty}},div{class:"small",{item.reconciliation_status.clone()}}},div{class:"row-actions",span{class:"row-value",{euro(item.amount_cents)}},if item.reconciliation_status=="UNMATCHED"{button{class:"secondary",onclick:move |_|async move{let _=delete_bank_transaction(id).await;bump+=1},"Supprimer"}}}}}}
#[component]fn DeadlineRow(item:DeadlineItem,bump:Signal<u64>)->Element{let id=item.id;rsx!{div{class:"data-row",div{div{class:"data-title",{item.label}},div{class:"small",{format!("{} • {}",item.deadline_date,item.period_label)}}},div{class:"row-actions",span{class:"status",{item.status.clone()}},if item.status!="DONE"{button{class:"secondary",onclick:move |_|async move{let _=set_deadline_status(id,"DONE".into()).await;bump+=1},"Terminer"}button{class:"secondary",onclick:move |_|async move{let _=delete_deadline(id).await;bump+=1},"Supprimer"}}}}}}
#[component]fn DocumentRow(item:DocumentItem,bump:Signal<u64>)->Element{let id=item.id;rsx!{div{class:"data-row",div{div{class:"data-title",{item.title}},div{class:"small",{format!("{} • {}",item.category,item.file_name)}},div{class:"small",{item.expires_at.map(|d|format!("Expiration {}",d)).unwrap_or_default()}}},div{class:"row-actions",button{class:"secondary",onclick:move |_|async move{let _=delete_document(id).await;bump+=1},"Supprimer"}}}}}
#[component]fn AutomationRow(item:AutomationRuleItem,bump:Signal<u64>)->Element{let id=item.id;let enabled=item.enabled;rsx!{div{class:"automation-tile",div{class:"code",{item.code}},h3{{item.name}},p{{item.description}},div{class:"small",{format!("Horizon {} j • priorité {}",item.horizon_days,item.priority)}},button{class:"secondary",onclick:move |_|async move{let _=set_automation_enabled(id,!enabled).await;bump+=1},if enabled{"Désactiver"}else{"Activer"}}}}}
#[component]fn TaskManagerRow(item:TaskItem,bump:Signal<u64>)->Element{let id=item.id;let v=task_state_value(&item.state).to_string();rsx!{div{class:"data-row",div{div{class:"data-title",{item.title}},div{class:"small",{item.description}},div{class:"small",{item.due_at.format("%d/%m/%Y %H:%M").to_string()}}},select{value:v,onchange:move|e:FormEvent| async move { let _=set_task_state(id,e.value()).await; bump+=1 },option{value:"PLANNED","Planifiée"},option{value:"READY","Prête"},option{value:"RUNNING","En cours"},option{value:"BLOCKED","Bloquée"},option{value:"DONE","Terminée"},option{value:"SKIPPED","Ignorée"}}}}}
#[component]fn TaskRow(task:TaskItem)->Element{rsx!{div{class:"task-row",div{div{class:"task-title",{task.title}},div{class:"small",{format!("{} • {}",task.code,task.due_at.format("%d/%m/%Y %H:%M"))}}},span{class:"status",{task_state_label(&task.state)}}}}}
#[component]fn AuditRow(item:AuditItem)->Element{rsx!{div{class:"data-row",div{div{class:"data-title",{item.action}},div{class:"small",{format!("{} • {} • {}",item.occurred_at.format("%d/%m/%Y %H:%M"),item.actor,item.entity_type)}}},div{class:"small audit-payload",{item.payload}}}}}
#[component]fn ModuleHeader(title:&'static str,kicker:&'static str,detail:&'static str)->Element{rsx!{section{class:"page-intro",div{div{class:"eyebrow",{kicker}},h2{{title}},p{{detail}}}}}}
#[component]fn Metric(label:&'static str,value:String,tone:&'static str)->Element{rsx!{div{class:"metric-card {tone}",div{class:"metric-label",{label}},div{class:"metric-value",{value}}}}}
#[component]fn ModuleCard(title:&'static str,value:String,label:&'static str,detail:String)->Element{rsx!{div{class:"module-card",div{class:"eyebrow",{title}},div{class:"module-number",{value}},div{class:"small",{label}},p{{detail}}}}}
#[component]fn Check(ok:bool,title:&'static str,text:&'static str)->Element{rsx!{div{class:if ok{"check ok"}else{"check"},span{class:"check-icon",if ok{"✓"}else{"·"}},div{strong{{title}},div{class:"small",{text}}}}}}
#[component]fn EmptyState(title:&'static str,text:&'static str)->Element{rsx!{div{class:"empty-state",h3{{title}},p{{text}}}}}
#[component]fn FormField(label:&'static str,value:String,oninput:EventHandler<FormEvent>)->Element{rsx!{label{class:"field",span{{label}},input{value:value,oninput:oninput}}}}
#[component]fn InfoTile(label:&'static str,value:&'static str)->Element{rsx!{div{class:"info-tile",div{class:"small",{label}},strong{{value}}}}}
#[component]fn InfoTileOwned(label:&'static str,value:String)->Element{rsx!{div{class:"info-tile",div{class:"small",{label}},strong{{value}}}}}
#[component]fn Loading()->Element{rsx!{div{class:"loading-grid",div{class:"hero-card skeleton"},div{class:"metric-row",for _ in 0..4{div{class:"metric skeleton"}}}}}}

fn euro(cents:i64)->String{format!("{:.2} €",(cents as f64)/100.0)}
fn euros_to_cents(value:&str)->i64{let normalized=value.trim().replace(" ","").replace(",",".");normalized.parse::<f64>().unwrap_or(0.0).round() as i64*100}
fn task_state_label(v:&TaskState)->&'static str{match v{TaskState::Planned=>"Planifiée",TaskState::Ready=>"Prête",TaskState::Running=>"En cours",TaskState::Blocked=>"Bloquée",TaskState::Done=>"Terminée",TaskState::Skipped=>"Ignorée"}}
fn task_state_value(v:&TaskState)->&'static str{match v{TaskState::Planned=>"PLANNED",TaskState::Ready=>"READY",TaskState::Running=>"RUNNING",TaskState::Blocked=>"BLOCKED",TaskState::Done=>"DONE",TaskState::Skipped=>"SKIPPED"}}
