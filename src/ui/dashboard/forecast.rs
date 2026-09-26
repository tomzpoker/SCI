use dioxus::prelude::*;
use uuid::Uuid;

use crate::prevision::{
    create_prevision_flow, delete_prevision_flow, list_prevision_flows,
    set_prevision_flow_active, set_prevision_flow_matches, update_prevision_flow,
    PrevisionFlowDraft, PrevisionFlowItem,
};
use crate::server::list_bank_transactions;

use super::models::{BankTx, Flow, Recurrence};

const TREASURY_PLANNED_COLOR: &str = "#facc15";
const TREASURY_POS_COLOR: &str = "#22c55e";
const TREASURY_NEG_COLOR: &str = "#ef4444";
const TODAY_MARKER_COLOR: &str = "#a78bfa";
const OVERDUE_COLOR: &str = "#f97316";

const MONTHS_FR: &[&str] = &[
    "janv.", "févr.", "mars", "avr.", "mai", "juin",
    "juil.", "août", "sept.", "oct.", "nov.", "déc.",
];

const MONTHS_FR_FULL: &[&str] = &[
    "janvier", "février", "mars", "avril", "mai", "juin",
    "juillet", "août", "septembre", "octobre", "novembre", "décembre",
];

// ============================================================
//  Adaptateurs BDD → UI
// ============================================================
/// Horodatage en millisecondes, compatible wasm32 et natif.
fn now_millis() -> usize {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as usize
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as usize)
            .unwrap_or(1)
    }
}

fn flow_from_item(item: PrevisionFlowItem) -> Flow {
    let recurrence = match item.recurrence.as_str() {
        "ONCE" => Recurrence::Once,
        "MONTHLY" => Recurrence::Monthly,
        "QUARTERLY" => Recurrence::Quarterly,
        "YEARLY" => Recurrence::Yearly,
        _ => Recurrence::Monthly,
    };
    Flow {
        id: item.id.as_u128() as usize,
        label: item.label,
        amount: item.amount_cents as f64 / 100.0,
        color: item.color,
        active: item.active,
        recurrence,
        start_year: item.start_year,
        start_month: item.start_month as u32,
        recurrence_day: item.recurrence_day as u32,
        payment_day: item.payment_day as u32,
        matched_txs: item
            .matched_tx_ids
            .iter()
            .map(|u| u.as_u128() as usize)
            .collect(),
    }
}

fn recurrence_to_str(r: Recurrence) -> String {
    match r {
        Recurrence::Once => "ONCE".into(),
        Recurrence::Monthly => "MONTHLY".into(),
        Recurrence::Quarterly => "QUARTERLY".into(),
        Recurrence::Yearly => "YEARLY".into(),
    }
}

fn ui_id_to_uuid(items: &[PrevisionFlowItem], ui_id: usize) -> Option<Uuid> {
    items
        .iter()
        .find(|it| it.id.as_u128() as usize == ui_id)
        .map(|it| it.id)
}

// ============================================================
//  Date minimaliste
// ============================================================
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SimpleDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap(y) { 29 } else { 28 },
        _ => 30,
    }
}

impl SimpleDate {
    pub fn fallback() -> Self {
        Self { year: 2026, month: 9, day: 1 }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn today() -> Self {
        let d = js_sys::Date::new_0();
        Self {
            year: d.get_full_year() as i32,
            month: (d.get_month() + 1) as u32,
            day: d.get_date() as u32,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn today() -> Self { Self::fallback() }

    pub fn start_of_month(&self) -> Self {
        Self { year: self.year, month: self.month, day: 1 }
    }

    pub fn month_abs(&self) -> i32 {
        self.year * 12 + (self.month as i32 - 1)
    }

    pub fn from_month_abs(abs: i32) -> Self {
        let y = abs.div_euclid(12);
        let m = (abs.rem_euclid(12) + 1) as u32;
        Self { year: y, month: m, day: 1 }
    }

    pub fn shift_months(&self, delta: i32) -> Self {
        Self::from_month_abs(self.month_abs() + delta)
    }

    pub fn add_days(&self, n: i64) -> Self {
        let mut y = self.year;
        let mut m = self.month as i32;
        let mut day = self.day as i64 + n;
        while day > days_in_month(y, m as u32) as i64 {
            day -= days_in_month(y, m as u32) as i64;
            m += 1;
            if m > 12 { m = 1; y += 1; }
        }
        while day <= 0 {
            m -= 1;
            if m < 1 { m = 12; y -= 1; }
            day += days_in_month(y, m as u32) as i64;
        }
        Self { year: y, month: m as u32, day: day as u32 }
    }

    pub fn as_tuple(&self) -> (i32, u32, u32) {
        (self.year, self.month, self.day)
    }
}

fn date_label_long(d: SimpleDate) -> String {
    format!("{} {}", MONTHS_FR[(d.month - 1) as usize], d.year)
}

fn date_label_short(d: SimpleDate) -> String {
    format!("{} {}", MONTHS_FR[(d.month - 1) as usize], d.year % 100)
}

fn parse_iso_date(s: &str) -> Option<SimpleDate> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() < 3 { return None; }
    let year = parts[0].parse::<i32>().ok()?;
    let month = parts[1].parse::<u32>().ok()?;
    let day = parts[2].parse::<u32>().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) { return None; }
    Some(SimpleDate { year, month, day })
}

#[derive(Clone)]
enum EventDate {
    Recurring(u32),
    Once(i32, u32, u32),
}

#[derive(Clone)]
struct DayEvent {
    date: EventDate,
    color: String,
}

fn is_event_at(ev: &DayEvent, d: SimpleDate) -> bool {
    match ev.date {
        EventDate::Recurring(day) => d.day == day,
        EventDate::Once(y, m, dd) => d.year == y && d.month == m && d.day == dd,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum TimeRange { Month, Quarter, Half, Year, ThreeYears }

impl TimeRange {
    fn label(self) -> &'static str {
        match self {
            TimeRange::Month => "Mois",
            TimeRange::Quarter => "Trim.",
            TimeRange::Half => "Sem.",
            TimeRange::Year => "An",
            TimeRange::ThreeYears => "3 ans",
        }
    }
    fn tooltip(self) -> &'static str {
        match self {
            TimeRange::Month => "Mois calendaire complet",
            TimeRange::Quarter => "3 mois, jour par jour",
            TimeRange::Half => "6 mois (pas de 2 jours)",
            TimeRange::Year => "12 mois (pas de 5 jours)",
            TimeRange::ThreeYears => "36 mois (pas de 15 jours)",
        }
    }
    fn total_days(self) -> i64 {
        match self {
            TimeRange::Month => 30,
            TimeRange::Quarter => 90,
            TimeRange::Half => 180,
            TimeRange::Year => 365,
            TimeRange::ThreeYears => 1095,
        }
    }
    fn day_step(self) -> i64 {
        match self {
            TimeRange::Month => 1,
            TimeRange::Quarter => 1,
            TimeRange::Half => 2,
            TimeRange::Year => 5,
            TimeRange::ThreeYears => 15,
        }
    }
    fn cursor_step(self) -> i32 {
        match self {
            TimeRange::Month => 1,
            TimeRange::Quarter => 3,
            TimeRange::Half => 6,
            TimeRange::Year => 12,
            TimeRange::ThreeYears => 12,
        }
    }
    fn n_months(self) -> usize {
        match self {
            TimeRange::Month => 1,
            TimeRange::Quarter => 3,
            TimeRange::Half => 6,
            TimeRange::Year => 12,
            TimeRange::ThreeYears => 12,
        }
    }
    fn month_step(self) -> i32 {
        match self {
            TimeRange::ThreeYears => 3,
            _ => 1,
        }
    }
    fn all() -> [TimeRange; 5] {
        [TimeRange::Month, TimeRange::Quarter, TimeRange::Half, TimeRange::Year, TimeRange::ThreeYears]
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ViewMode { Chart, Table }

// ============================================================
//  Cycle de vie des flux
// ============================================================

fn flow_date(flow: &Flow) -> SimpleDate {
    SimpleDate {
        year: flow.start_year,
        month: flow.start_month,
        day: flow.recurrence_day.clamp(1, 31),
    }
}

fn flow_in_period(flow: &Flow, p_start: SimpleDate, p_end: SimpleDate) -> bool {
    match flow.recurrence {
        Recurrence::Once => {
            let d = flow_date(flow);
            d.as_tuple() >= p_start.as_tuple() && d.as_tuple() <= p_end.as_tuple()
        }
        _ => {
            let start_abs = flow.start_year * 12 + (flow.start_month as i32 - 1);
            start_abs <= p_end.month_abs()
        }
    }
}

fn matched_amount(flow: &Flow, bank_txs: &[BankTx]) -> f64 {
    bank_txs.iter()
        .filter(|tx| flow.matched_txs.contains(&tx.id))
        .map(|tx| tx.amount)
        .sum()
}

fn is_archived(flow: &Flow, bank_txs: &[BankTx]) -> bool {
    match flow.recurrence {
        Recurrence::Once => {
            let m = matched_amount(flow, bank_txs);
            (m - flow.amount).abs() < 0.01
        }
        _ => false,
    }
}

fn is_overdue(flow: &Flow, today: SimpleDate, bank_txs: &[BankTx]) -> bool {
    if !matches!(flow.recurrence, Recurrence::Once) { return false; }
    let d = flow_date(flow);
    d.as_tuple() < today.as_tuple() && !is_archived(flow, bank_txs)
}

fn flow_cumulative_at(flow: &Flow, target: SimpleDate) -> f64 {
    if matches!(flow.recurrence, Recurrence::Once) {
        let start = flow_date(flow);
        return if target.as_tuple() >= start.as_tuple() { flow.amount } else { 0.0 };
    }
    let start_abs = flow.start_year * 12 + (flow.start_month as i32 - 1);
    let target_abs = target.month_abs();
    if target_abs < start_abs { return 0.0; }
    let period = flow.recurrence.months().max(1);
    let diff = target_abs - start_abs;
    let k = diff / period;
    let mut occurrences = k + 1;
    let last_occ_month_abs = start_abs + k * period;
    if last_occ_month_abs == target_abs && target.day < flow.recurrence_day {
        occurrences -= 1;
    }
    (flow.amount * occurrences.max(0) as f64).max(f64::MIN)
}

fn build_dates(range: TimeRange, cursor: SimpleDate) -> Vec<SimpleDate> {
    if range == TimeRange::Month {
        let dim = days_in_month(cursor.year, cursor.month);
        (0..dim as i64).map(|i| cursor.add_days(i)).collect()
    } else {
        let total = range.total_days();
        let step = range.day_step();
        (0..total).step_by(step as usize).map(|i| cursor.add_days(i)).collect()
    }
}

fn treasury_real_at(
    d: SimpleDate,
    cursor: SimpleDate,
    today: SimpleDate,
    baseline: f64,
    parsed_bank: &[(SimpleDate, f64)],
    planned_at_d: f64,
) -> Option<f64> {
    let is_future = d.month_abs() > today.month_abs()
        || (d.month_abs() == today.month_abs() && d.day > today.day);
    if is_future { return None; }
    let cursor_key = cursor.as_tuple();
    let upper = (d.year, d.month, days_in_month(d.year, d.month));
    let delta: f64 = parsed_bank.iter()
        .filter(|(td, _)| {
            let tk = td.as_tuple();
            tk > cursor_key && tk <= upper
        })
        .map(|(_, amt)| *amt)
        .sum();
    Some((baseline + delta).min(planned_at_d))
}

// ============================================================
//  Composant principal
// ============================================================
#[component]
pub fn ForecastWidget() -> Element {
    let mut real_today = use_signal(|| SimpleDate::fallback());
    use_effect(move || { real_today.set(SimpleDate::today()); });

    let mut cursor = use_signal(|| SimpleDate::fallback());
    let mut cursor_init = use_signal(|| false);
    use_effect(move || {
        if !cursor_init() {
            cursor.set(real_today().start_of_month());
            cursor_init.set(true);
        }
    });

    let mut view_mode = use_signal(|| ViewMode::Chart);
    let mut range = use_signal(|| TimeRange::Month);
    let mut show_settings = use_signal(|| false);
    let mut focus_treasury = use_signal(|| true);
    let mut recon_flow_id = use_signal(|| None::<usize>);
    let mut maximized = use_signal(|| false);

    // === Chargement depuis la BDD ===
    let mut reload = use_signal(|| 0u64);
    let mut flows = use_signal(|| Vec::<Flow>::new());
    let mut bdd_items = use_signal(|| Vec::<PrevisionFlowItem>::new());

    // Flux prévisionnels
    let flows_resource = use_resource(move || {
        let _ = reload();
        async move {
            let items = list_prevision_flows().await.unwrap_or_default();
            let flows: Vec<Flow> = items.iter().cloned().map(flow_from_item).collect();
            (flows, items)
        }
    });

    use_effect(move || {
        if let Some((local_flows, items)) = flows_resource.read().as_ref().cloned() {
            flows.set(local_flows);
            bdd_items.set(items);
        }
    });

    // Transactions bancaires (rechargées en même temps)
    let bank_txs_resource = use_resource(move || {
        let _ = reload();
        async move {
            list_bank_transactions()
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|item| BankTx {
                    id: item.id.as_u128() as usize,
                    date: item.booked_at.format("%Y-%m-%d").to_string(),
                    label: item.label,
                    amount: item.amount_cents as f64 / 100.0,
                })
                .collect::<Vec<_>>()
        }
    });

    let bank_txs: Vec<BankTx> = (*bank_txs_resource.read()).clone().unwrap_or_default();

    let any_inactive = flows().iter().any(|f| !f.active);
    let cursor_is_today = cursor().month_abs() == real_today().month_abs();
    let year_min = real_today().year - 5;
    let year_max = real_today().year + 10;

    let is_max = maximized();

    let filter_dates = build_dates(range(), cursor());
    let filter_start = filter_dates.first().copied().unwrap_or(cursor());
    let filter_end = filter_dates.last().copied().unwrap_or(cursor());

    let container_style = if is_max {
        "position: fixed; inset: 16px; z-index: 9999; background: #1e293b; border-radius: 12px; padding: 16px; border: 1px solid #334155; box-shadow: 0 25px 50px -12px rgba(0,0,0,0.7); display: flex; flex-direction: column; gap: 10px; overflow: hidden;"
    } else {
        "display: flex; flex-direction: column; gap: 10px; min-width: 0; max-width: 100%;"
    };

    rsx! {
        div {
            style: "{container_style}",

            // Ligne 1
            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 8px; flex-wrap: wrap; flex-shrink: 0;",
                div { style: "display: flex; align-items: center; gap: 6px;",
                    button {
                        draggable: "false",
                        style: "background: transparent; border: 1px solid #334155; color: #94a3b8; border-radius: 6px; cursor: pointer; padding: 0; width: 30px; height: 28px; display: inline-flex; align-items: center; justify-content: center; font-size: 1.15rem; line-height: 1;",
                        title: "Configurer les flux",
                        onclick: move |_| show_settings.set(true),
                        "⚙"
                    }
                    button {
                        draggable: "false",
                        style: "background: transparent; border: 1px solid #334155; color: #94a3b8; border-radius: 6px; cursor: pointer; padding: 0; width: 30px; height: 28px; display: inline-flex; align-items: center; justify-content: center; font-size: 0.9rem; line-height: 1;",
                        title: if is_max { "Réduire" } else { "Plein écran" },
                        onmousedown: move |e| e.stop_propagation(),
                        onpointerdown: move |e| e.stop_propagation(),
                        onclick: move |e| {
                            e.stop_propagation();
                            let next = !maximized();
                            maximized.set(next);
                        },
                        if is_max { "⤡" } else { "⛶" }
                    }
                    div { style: "display: inline-flex; background: #0f172a; border-radius: 6px; padding: 2px; border: 1px solid #334155;",
                        button {
                            draggable: "false",
                            style: if view_mode() == ViewMode::Chart {
                                "background: #38bdf8; color: #0f172a; border: none; padding: 4px 12px; border-radius: 4px; font-size: 0.75rem; cursor: pointer; font-weight: 600;"
                            } else {
                                "background: transparent; color: #94a3b8; border: none; padding: 4px 12px; border-radius: 4px; font-size: 0.75rem; cursor: pointer;"
                            },
                            onclick: move |_| view_mode.set(ViewMode::Chart),
                            "Graph"
                        }
                        button {
                            draggable: "false",
                            style: if view_mode() == ViewMode::Table {
                                "background: #38bdf8; color: #0f172a; border: none; padding: 4px 12px; border-radius: 4px; font-size: 0.75rem; cursor: pointer; font-weight: 600;"
                            } else {
                                "background: transparent; color: #94a3b8; border: none; padding: 4px 12px; border-radius: 4px; font-size: 0.75rem; cursor: pointer;"
                            },
                            onclick: move |_| view_mode.set(ViewMode::Table),
                            "Tableau"
                        }
                    }
                }
                if view_mode() == ViewMode::Chart {
                    button {
                        draggable: "false",
                        style: if focus_treasury() {
                            "display: inline-flex; align-items: center; gap: 5px; background: transparent; border: 1px solid #334155; color: #94a3b8; padding: 4px 10px; border-radius: 6px; font-size: 0.72rem; cursor: pointer;"
                        } else {
                            "display: inline-flex; align-items: center; gap: 5px; background: #1e293b; border: 1px solid #facc15; color: #facc15; padding: 4px 10px; border-radius: 6px; font-size: 0.72rem; cursor: pointer; font-weight: 600;"
                        },
                        onclick: move |_| focus_treasury.set(!focus_treasury()),
                        span {
                            style: if focus_treasury() {
                                "width: 8px; height: 8px; border-radius: 2px; background: #475569;"
                            } else {
                                "width: 8px; height: 8px; border-radius: 2px; background: #facc15;"
                            }
                        }
                        if focus_treasury() { "Tous les flux" } else { "Trésorerie seule" }
                    }
                }
            }

            // Ligne 2
            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap; flex-shrink: 0;",
                div { style: "display: flex; align-items: center; gap: 4px;",
                    button {
                        draggable: "false",
                        style: "background: #0f172a; border: 1px solid #334155; color: #94a3b8; padding: 4px 10px; border-radius: 6px; cursor: pointer; font-size: 0.85rem; line-height: 1;",
                        onclick: move |_| {
                            let step = range().cursor_step();
                            cursor.set(cursor().shift_months(-step).start_of_month());
                        },
                        "◀"
                    }
                    select {
                        style: "background: #0f172a; border: 1px solid #334155; color: #f8fafc; padding: 5px 10px; border-radius: 6px; font-size: 0.82rem; font-family: inherit; cursor: pointer; min-width: 110px;",
                        value: "{cursor().month}",
                        onchange: move |e| {
                            if let Ok(m) = e.value().parse::<u32>() {
                                let mut c = cursor();
                                c.month = m;
                                c.day = 1;
                                cursor.set(c);
                            }
                        },
                        for m in 1..=12u32 {
                            option { value: "{m}", selected: cursor().month == m, "{MONTHS_FR_FULL[(m - 1) as usize]}" }
                        }
                    }
                    select {
                        style: "background: #0f172a; border: 1px solid #334155; color: #f8fafc; padding: 5px 10px; border-radius: 6px; font-size: 0.82rem; font-family: inherit; cursor: pointer; min-width: 72px;",
                        value: "{cursor().year}",
                        onchange: move |e| {
                            if let Ok(y) = e.value().parse::<i32>() {
                                let mut c = cursor();
                                c.year = y;
                                c.day = 1;
                                cursor.set(c);
                            }
                        },
                        for y in year_min..=year_max {
                            option { value: "{y}", selected: cursor().year == y, "{y}" }
                        }
                    }
                    button {
                        draggable: "false",
                        style: "background: #0f172a; border: 1px solid #334155; color: #94a3b8; padding: 4px 10px; border-radius: 6px; cursor: pointer; font-size: 0.85rem; line-height: 1;",
                        onclick: move |_| {
                            let step = range().cursor_step();
                            cursor.set(cursor().shift_months(step).start_of_month());
                        },
                        "▶"
                    }
                    if !cursor_is_today {
                        button {
                            draggable: "false",
                            style: "background: transparent; border: 1px solid #4c1d95; color: #a78bfa; padding: 4px 10px; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 500; margin-left: 4px;",
                            onclick: move |_| cursor.set(real_today().start_of_month()),
                            "Aujourd'hui"
                        }
                    }
                }
                div { style: "display: inline-flex; background: #0f172a; border-radius: 6px; padding: 2px; border: 1px solid #334155;",
                    for r in TimeRange::all() {
                        button {
                            draggable: "false",
                            style: if range() == r {
                                "background: #38bdf8; color: #0f172a; border: none; padding: 4px 14px; border-radius: 4px; font-size: 0.72rem; cursor: pointer; font-weight: 600;"
                            } else {
                                "background: transparent; color: #94a3b8; border: none; padding: 4px 14px; border-radius: 4px; font-size: 0.72rem; cursor: pointer;"
                            },
                            title: "{r.tooltip()}",
                            onclick: move |_| range.set(r),
                            "{r.label()}"
                        }
                    }
                }
            }

            // Ligne 3 : pills
            div { style: "display: flex; flex-wrap: wrap; gap: 6px; flex-shrink: 0;",
                for f in flows().iter().filter(|f| flow_in_period(f, filter_start, filter_end)).cloned().collect::<Vec<_>>() {
                    {
                        let fid = f.id;
                        let color = f.color.clone();
                        let label = f.label.clone();
                        let is_active = f.active;
                        let dim = focus_treasury();
                        let is_recurring = f.recurrence.is_recurring();
                        let hint = if is_recurring {
                            format!("Démarre {} {} • récur. {} • paie le {}",
                                MONTHS_FR[(f.start_month - 1) as usize], f.start_year,
                                f.recurrence.label(), f.payment_day)
                        } else {
                            format!("Ponctuel le {} {} {}",
                                f.recurrence_day,
                                MONTHS_FR[(f.start_month - 1) as usize],
                                f.start_year)
                        };
                        rsx! {
                            button {
                                draggable: "false",
                                style: format!(
                                    "display: inline-flex; align-items: center; gap: 5px; background: {}; border: 1px solid {}; color: {}; padding: 3px 10px; border-radius: 12px; font-size: 0.7rem; cursor: pointer; transition: all 0.15s; opacity: {};",
                                    if is_active { "#1e293b" } else { "#0f172a" },
                                    if is_active { &color } else { "#334155" },
                                    if is_active { "#f8fafc" } else { "#64748b" },
                                    if dim && is_active { "0.7" } else { "1" }
                                ),
                                title: "{hint}",
                                onclick: move |_| {
                                    let new_active = flows.with(|v| {
                                        !v.iter().find(|x| x.id == fid).map(|x| x.active).unwrap_or(true)
                                    });
                                    flows.with_mut(|v| {
                                        if let Some(fl) = v.iter_mut().find(|x| x.id == fid) {
                                            fl.active = new_active;
                                        }
                                    });
                                    if let Some(uuid) = ui_id_to_uuid(&bdd_items(), fid) {
                                        spawn(async move {
                                            let _ = set_prevision_flow_active(uuid, new_active).await;
                                            let r = reload();
                                            reload.set(r + 1);
                                        });
                                    }
                                },
                                span { style: format!("width: 8px; height: 8px; border-radius: 50%; background: {};", if is_active { &color } else { "#475569" }) }
                                "{label}"
                                if !is_recurring {
                                    span { style: "font-size: 0.6rem; color: #64748b;", "•" }
                                }
                            }
                        }
                    }
                }
                if any_inactive {
                    button {
                        draggable: "false",
                        style: "background: transparent; border: 1px dashed #334155; color: #94a3b8; padding: 3px 10px; border-radius: 12px; font-size: 0.7rem; cursor: pointer;",
                        onclick: move |_| {
                            let items_snapshot = bdd_items();
                            flows.with_mut(|v| for f in v.iter_mut() { f.active = true; });
                            spawn(async move {
                                for item in items_snapshot.iter() {
                                    let _ = set_prevision_flow_active(item.id, true).await;
                                }
                                let r = reload();
                                reload.set(r + 1);
                            });
                        },
                        "Tout activer"
                    }
                }
            }

            // Contenu
            if view_mode() == ViewMode::Chart {
                ForecastChart {
                    flows: flows(),
                    range: range(),
                    cursor: cursor(),
                    real_today: real_today(),
                    focus_treasury: focus_treasury(),
                    maximized: is_max,
                    bank_txs: bank_txs.clone(),
                }
            } else {
                ForecastTable {
                    flows: flows(),
                    range: range(),
                    cursor: cursor(),
                    real_today: real_today(),
                    bank_txs: bank_txs.clone(),
                    on_reconcile: move |id: usize| recon_flow_id.set(Some(id)),
                }
            }

            if show_settings() {
                ForecastSettingsModal {
                    flows,
                    bdd_items,
                    reload,
                    real_today: real_today(),
                    on_close: move |_| show_settings.set(false),
                }
            }

            if let Some(fid) = recon_flow_id() {
                ReconciliationModal {
                    flow_id: fid,
                    flows,
                    bdd_items,
                    reload,
                    bank_txs: bank_txs.clone(),
                    on_close: move |_| recon_flow_id.set(None),
                }
            }
        }
    }
}

// ============================================================
//  Graph
// ============================================================
#[component]
fn ForecastChart(
    flows: Vec<Flow>,
    range: TimeRange,
    cursor: SimpleDate,
    real_today: SimpleDate,
    focus_treasury: bool,
    maximized: bool,
    bank_txs: Vec<BankTx>,
) -> Element {
    let dates = build_dates(range, cursor);
    let p_start = dates.first().copied().unwrap_or(cursor);
    let p_end = dates.last().copied().unwrap_or(cursor);
    let n = dates.len();
    let today_abs = real_today.month_abs();
    let is_month_view = range == TimeRange::Month;

    let active_all: Vec<&Flow> = flows.iter().filter(|f| f.active).collect();
    let active: Vec<&Flow> = active_all.into_iter()
        .filter(|f| flow_in_period(f, p_start, p_end))
        .collect();

    let width = 1000.0_f64;
    let height = 340.0_f64;
    let padding_x = 52.0_f64;
    let padding_top = 46.0_f64;
    let padding_bottom = 80.0_f64;
    let plot_bottom = height - padding_bottom;

    let x_of = move |i: usize| {
        if n <= 1 { width / 2.0 }
        else { padding_x + (i as f64 / (n - 1) as f64) * (width - 2.0 * padding_x) }
    };

    let mut day_events: Vec<DayEvent> = Vec::new();
    for f in &active {
        let rec = f.recurrence_day.clamp(1, 31);
        let ev = if f.recurrence.is_recurring() {
            EventDate::Recurring(rec)
        } else {
            EventDate::Once(f.start_year, f.start_month, rec)
        };
        let already = day_events.iter().any(|e| match (&e.date, &ev) {
            (EventDate::Recurring(a), EventDate::Recurring(b)) => a == b,
            (EventDate::Once(ay, am, ad), EventDate::Once(by, bm, bd)) => {
                ay == by && am == bm && ad == bd
            }
            _ => false,
        });
        if !already {
            day_events.push(DayEvent { date: ev, color: f.color.clone() });
        }
        if f.recurrence.is_recurring() {
            let pay = f.payment_day.clamp(1, 31);
            if pay != rec
                && !day_events.iter().any(|e| matches!(e.date, EventDate::Recurring(d) if d == pay))
            {
                day_events.push(DayEvent { date: EventDate::Recurring(pay), color: f.color.clone() });
            }
        }
    }

    let flow_series: Vec<Vec<f64>> = active.iter().map(|f| {
        dates.iter().map(|d| flow_cumulative_at(f, *d)).collect()
    }).collect();

    let treasury_planned: Vec<f64> = (0..n).map(|i| {
        active.iter().map(|f| flow_cumulative_at(f, dates[i])).sum()
    }).collect();

    let parsed_bank: Vec<(SimpleDate, f64)> = bank_txs.iter()
        .filter_map(|tx| parse_iso_date(&tx.date).map(|d| (d, tx.amount)))
        .collect();
    let baseline_real = treasury_planned.first().copied().unwrap_or(0.0);

    let treasury_real: Vec<Option<f64>> = (0..n).map(|i| {
        treasury_real_at(dates[i], cursor, real_today, baseline_real, &parsed_bank, treasury_planned[i])
    }).collect();

    let mut all_values: Vec<f64> = Vec::new();
    if !focus_treasury {
        all_values.extend(flow_series.iter().flat_map(|v| v.iter().copied()));
    }
    all_values.extend(treasury_planned.iter().copied());
    all_values.extend(treasury_real.iter().filter_map(|v| *v));

    let min_val = all_values.iter().copied().fold(f64::INFINITY, f64::min).min(0.0);
    let max_val = all_values.iter().copied().fold(f64::NEG_INFINITY, f64::max).max(0.0);
    let range_v = (max_val - min_val).max(1.0);

    let y_of = move |v: f64| plot_bottom - ((v - min_val) / range_v) * (plot_bottom - padding_top);
    let y_zero = y_of(0.0);

    let real_pts: Vec<Option<(f64, f64)>> = treasury_real.iter().enumerate()
        .map(|(i, v)| v.map(|val| (x_of(i), y_of(val)))).collect();
    let mut real_segments: Vec<(f64, f64, f64, f64, &'static str)> = Vec::new();
    for i in 0..real_pts.len().saturating_sub(1) {
        if let (Some((x1, y1)), Some((x2, y2))) = (real_pts[i], real_pts[i + 1]) {
            let v1 = treasury_real[i].unwrap_or(0.0);
            let v2 = treasury_real[i + 1].unwrap_or(0.0);
            let neg1 = v1 < 0.0;
            let neg2 = v2 < 0.0;
            if neg1 == neg2 {
                real_segments.push((x1, y1, x2, y2, if neg1 { TREASURY_NEG_COLOR } else { TREASURY_POS_COLOR }));
            } else {
                let t = (0.0 - v1) / (v2 - v1);
                let xm = x1 + t * (x2 - x1);
                real_segments.push((x1, y1, xm, y_zero, if neg1 { TREASURY_NEG_COLOR } else { TREASURY_POS_COLOR }));
                real_segments.push((xm, y_zero, x2, y2, if neg2 { TREASURY_NEG_COLOR } else { TREASURY_POS_COLOR }));
            }
        }
    }

    let path_of = |series: &[f64]| -> String {
        series.iter().enumerate().map(|(i, v)| {
            let x = x_of(i); let y = y_of(*v);
            if i == 0 { format!("M {:.1} {:.1}", x, y) } else { format!(" L {:.1} {:.1}", x, y) }
        }).collect()
    };

    let treasury_planned_path = path_of(&treasury_planned);

    let today_idx: Option<usize> = dates.iter().position(|d| d.as_tuple() == real_today.as_tuple());
    let today_marker_y: Option<f64> = today_idx.map(|ti| {
        let v = treasury_real[ti].unwrap_or(treasury_planned[ti]);
        y_of(v)
    });

    let mut month_marks: Vec<(usize, String, bool)> = Vec::new();
    let mut last_month_seen: Option<(i32, u32)> = None;
    for (i, d) in dates.iter().enumerate() {
        let key = (d.year, d.month);
        if last_month_seen != Some(key) {
            let is_current = d.month_abs() == today_abs;
            let lbl = date_label_short(*d);
            month_marks.push((i, lbl, is_current));
            last_month_seen = Some(key);
        }
    }

    let cursor_date = dates[0];
    let cursor_planned = treasury_planned[0];
    let cursor_real = treasury_real[0];
    let cursor_flow_values: Vec<f64> = flow_series.iter().map(|s| s[0]).collect();

    let chart_box_style = if maximized {
        "border-radius: 8px; overflow: hidden; width: 100%; flex: 1; min-height: 0;"
    } else {
        "border-radius: 8px; overflow: hidden; width: 100%; aspect-ratio: 1000 / 340;"
    };

    let outer_style = if maximized {
        "display: flex; flex-direction: column; gap: 10px; width: 100%; flex: 1; min-height: 0;"
    } else {
        "display: flex; flex-direction: column; gap: 10px; width: 100%; min-width: 0;"
    };

    rsx! {
        div { style: "{outer_style}",
            div {
                class: "dash-chart-container",
                style: "{chart_box_style}",
                if active.is_empty() {
                    div { style: "display: flex; align-items: center; justify-content: center; height: 200px; color: #64748b; font-size: 0.85rem;",
                        "Aucun flux actif dans cette période."
                    }
                } else {
                    svg {
                        view_box: "0 0 {width} {height}",
                        preserve_aspect_ratio: "xMidYMid meet",
                        style: "width: 100%; height: 100%; display: block;",
                        for i in 0..5 {
                            {
                                let y = padding_top + (i as f64 / 4.0) * (plot_bottom - padding_top);
                                rsx! { line { x1: "{padding_x}", y1: "{y}", x2: "{width - padding_x}", y2: "{y}", stroke: "#334155", stroke_width: "0.5", stroke_dasharray: "2,3" } }
                            }
                        }
                        line { x1: "{padding_x}", y1: "{y_zero}", x2: "{width - padding_x}", y2: "{y_zero}", stroke: "#64748b", stroke_width: "1" }
                        line { x1: "{padding_x}", y1: "{padding_top}", x2: "{padding_x}", y2: "{plot_bottom}", stroke: "#334155", stroke_width: "1" }
                        line { x1: "{padding_x}", y1: "{plot_bottom}", x2: "{width - padding_x}", y2: "{plot_bottom}", stroke: "#334155", stroke_width: "1" }

                        if let Some(ti) = today_idx {
                            line {
                                x1: "{x_of(ti)}", y1: "{padding_top}",
                                x2: "{x_of(ti)}", y2: "{plot_bottom}",
                                stroke: "{TODAY_MARKER_COLOR}",
                                stroke_width: "1",
                                stroke_dasharray: "3,4",
                                opacity: "0.25"
                            }
                        }

                        if !focus_treasury {
                            for (fi, flow) in active.iter().enumerate() {
                                {
                                    let path_d = path_of(&flow_series[fi]);
                                    let color = flow.color.clone();
                                    let archived = is_archived(flow, &bank_txs);
                                    let opacity = if archived { "0.4" } else { "0.85" };
                                    let dash = if archived { "4,3" } else { "none" };
                                    rsx! {
                                        path {
                                            d: "{path_d}",
                                            fill: "none",
                                            stroke: "{color}",
                                            stroke_width: "1.5",
                                            opacity: "{opacity}",
                                            stroke_dasharray: "{dash}",
                                            stroke_linejoin: "round"
                                        }
                                    }
                                }
                            }
                        }

                        path {
                            d: "{treasury_planned_path}",
                            fill: "none",
                            stroke: "{TREASURY_PLANNED_COLOR}",
                            stroke_width: "3",
                            stroke_linejoin: "round",
                            stroke_linecap: "round"
                        }

                        for (x1, y1, x2, y2, color) in real_segments.iter() {
                            line {
                                x1: "{x1}", y1: "{y1}", x2: "{x2}", y2: "{y2}",
                                stroke: "{color}", stroke_width: "2.5", stroke_linecap: "round"
                            }
                        }

                        if let (Some(ti), Some(y)) = (today_idx, today_marker_y) {
                            circle {
                                cx: "{x_of(ti)}", cy: "{y}",
                                r: "4.5", fill: "{TODAY_MARKER_COLOR}", opacity: "0.25"
                            }
                            circle {
                                cx: "{x_of(ti)}", cy: "{y}",
                                r: "2.5", fill: "{TODAY_MARKER_COLOR}",
                                stroke: "#ffffff", stroke_width: "1"
                            }
                        }

                        if is_month_view {
                            {
                                let ticks: Vec<_> = dates.iter().enumerate().map(|(i, d)| {
                                    let matched = day_events.iter().find(|ev| is_event_at(ev, *d));
                                    (i, d.day, matched.map(|m| m.color.clone()))
                                }).collect();
                                rsx! {
                                    for (i, day, evt_color) in ticks {
                                        {
                                            let x = x_of(i);
                                            let is_evt = evt_color.is_some();
                                            let tick_len = if is_evt { 6.0 } else { 3.0 };
                                            let opacity = if is_evt { "1" } else { "0.5" };
                                            let col = evt_color.unwrap_or_else(|| "#334155".to_string());
                                            rsx! {
                                                line {
                                                    x1: "{x}", y1: "{plot_bottom + 1.0}",
                                                    x2: "{x}", y2: "{plot_bottom + 1.0 + tick_len}",
                                                    stroke: "{col}",
                                                    stroke_width: "1.2",
                                                    opacity: "{opacity}"
                                                }
                                                if is_evt {
                                                    text {
                                                        x: "{x}",
                                                        y: "{plot_bottom + 18.0}",
                                                        fill: "{col}",
                                                        font_size: "8",
                                                        font_weight: "600",
                                                        text_anchor: "middle",
                                                        "{day}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        for (idx, label, is_current) in month_marks.iter() {
                            {
                                let x = x_of(*idx);
                                let fill = if *is_current { TODAY_MARKER_COLOR } else { "#cbd5e1" };
                                let weight = if *is_current { "700" } else { "600" };
                                let anchor = if *idx == 0 { "start" } else { "middle" };
                                rsx! {
                                    text {
                                        x: "{x}",
                                        y: "{height - 8.0}",
                                        fill: "{fill}",
                                        font_size: "11",
                                        font_weight: "{weight}",
                                        text_anchor: "{anchor}",
                                        "{label}"
                                    }
                                }
                            }
                        }

                        text { x: "{padding_x - 6.0}", y: "{y_of(min_val) + 4.0}", fill: "#64748b", font_size: "10", text_anchor: "end", {format!("{:.0}", min_val)} }
                        text { x: "{padding_x - 6.0}", y: "{y_zero + 4.0}", fill: "#64748b", font_size: "10", text_anchor: "end", "0" }
                        text { x: "{padding_x - 6.0}", y: "{y_of(max_val) + 4.0}", fill: "#64748b", font_size: "10", text_anchor: "end", {format!("{:.0}", max_val)} }
                    }
                }
            }

            if !active.is_empty() {
                div {
                    style: "display: flex; flex-direction: column; gap: 8px; padding: 10px 12px; background: #0f172a; border-radius: 6px; border: 1px solid #1e293b; flex-shrink: 0;",
                    div { style: "display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; padding-bottom: 8px; border-bottom: 1px solid #1e293b;",
                        div {
                            div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.06em;", "Position du curseur" }
                            div { style: "font-size: 0.95rem; font-weight: 600; color: #f8fafc;",
                                {date_label_long(cursor_date)}
                            }
                        }
                        div { style: "display: flex; gap: 20px; align-items: center;",
                            div { style: "text-align: right;",
                                div { style: "font-size: 0.65rem; color: #64748b;", "Trésorerie prévue" }
                                div { style: "font-size: 1rem; font-weight: 700; color: {TREASURY_PLANNED_COLOR}; font-variant-numeric: tabular-nums;",
                                    {format!("{:.0} EUR", cursor_planned)}
                                }
                            }
                            if let Some(rv) = cursor_real {
                                div { style: "text-align: right;",
                                    div { style: "font-size: 0.65rem; color: #64748b;", "Trésorerie réelle" }
                                    {
                                        let col = if rv >= 0.0 { TREASURY_POS_COLOR } else { TREASURY_NEG_COLOR };
                                        rsx! {
                                            div { style: "font-size: 1rem; font-weight: 700; color: {col}; font-variant-numeric: tabular-nums;",
                                                {format!("{:.0} EUR", rv)}
                                            }
                                        }
                                    }
                                }
                            } else {
                                div { style: "text-align: right;",
                                    div { style: "font-size: 0.65rem; color: #64748b;", "Trésorerie réelle" }
                                    div { style: "font-size: 0.9rem; color: #475569; font-style: italic;", "à venir" }
                                }
                            }
                        }
                    }
                    div { style: "display: flex; flex-wrap: wrap; gap: 6px;",
                        for (fi, f) in active.iter().enumerate() {
                            {
                                let v = cursor_flow_values[fi];
                                let col = f.color.clone();
                                let archived = is_archived(f, &bank_txs);
                                let overdue = is_overdue(f, real_today, &bank_txs);
                                let (bg, border, opacity, icon) = if archived {
                                    ("#0f172a", "#334155", "0.6", "📌")
                                } else if overdue {
                                    ("#1e293b", OVERDUE_COLOR, "1", "⚠️")
                                } else {
                                    ("#1e293b", "#1e293b", "1", "")
                                };
                                rsx! {
                                    div {
                                        style: format!("display: inline-flex; align-items: center; gap: 6px; padding: 3px 10px; background: {}; border-radius: 10px; border: 1px solid {}; font-size: 0.72rem; opacity: {};",
                                            bg, border, opacity),
                                        span { style: "width: 8px; height: 8px; border-radius: 50%; background: {col}; flex-shrink: 0;" }
                                        span { style: "color: #94a3b8;", "{f.label}" }
                                        if !icon.is_empty() {
                                            span { style: "font-size: 0.7rem;", "{icon}" }
                                        }
                                        span { style: "color: #f8fafc; font-weight: 600; font-variant-numeric: tabular-nums;",
                                            {format!("{:.0}", v)}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
//  Tableau
// ============================================================
#[component]
fn ForecastTable(
    flows: Vec<Flow>,
    range: TimeRange,
    cursor: SimpleDate,
    real_today: SimpleDate,
    bank_txs: Vec<BankTx>,
    on_reconcile: EventHandler<usize>,
) -> Element {
    let n = range.n_months();
    let step = range.month_step();
    let timeline: Vec<i32> = (0..n as i32).map(|k| k * step).collect();
    let dates: Vec<SimpleDate> = timeline.iter().map(|m| cursor.shift_months(*m)).collect();
    let p_start = dates.first().copied().unwrap_or(cursor);
    let p_end = dates.last().copied().unwrap_or(cursor);
    let today_abs = real_today.month_abs();

    let active_all: Vec<&Flow> = flows.iter().filter(|f| f.active).collect();
    let active: Vec<&Flow> = active_all.into_iter()
        .filter(|f| flow_in_period(f, p_start, p_end))
        .collect();

    let mut sorted: Vec<&Flow> = active.clone();
    sorted.sort_by_key(|f| flow_date(f).as_tuple());

    let parsed_bank: Vec<(SimpleDate, f64)> = bank_txs.iter()
        .filter_map(|tx| parse_iso_date(&tx.date).map(|d| (d, tx.amount)))
        .collect();

    let baseline_real: f64 = dates.first().map(|d0| {
        sorted.iter().map(|f| flow_cumulative_at(f, *d0)).sum::<f64>()
    }).unwrap_or(0.0);

    rsx! {
        div { style: "overflow-x: auto; width: 100%; max-width: 100%;",
            table {
                style: "width: 100%; border-collapse: collapse; font-size: 0.75rem;",
                thead {
                    tr {
                        th { style: "text-align: left; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500; white-space: nowrap; position: sticky; left: 0; background: #1e293b; z-index: 1;", "Flux" }
                        for d in dates.iter() {
                            {
                                let lbl = date_label_long(*d);
                                let is_today = d.month_abs() == today_abs;
                                let is_past = d.month_abs() < today_abs;
                                let color = if is_today { TODAY_MARKER_COLOR } else if is_past { "#64748b" } else { "#94a3b8" };
                                let delta = d.month_abs() - today_abs;
                                let hint = if is_today { "aujourd'hui".to_string() }
                                           else if delta > 0 { format!("+{}m", delta) }
                                           else { format!("{}m", delta) };
                                rsx! {
                                    th {
                                        style: "text-align: right; padding: 6px 8px; border-bottom: 1px solid #334155; color: {color}; font-weight: 500; white-space: nowrap;",
                                        div { style: "font-size: 0.65rem; color: #475569; font-weight: 400;", "{hint}" }
                                        "{lbl}"
                                    }
                                }
                            }
                        }
                        th { style: "text-align: right; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500; white-space: nowrap;", "Banque" }
                    }
                }
                tbody {
                    for f in sorted.iter() {
                        {
                            let color = f.color.clone();
                            let fid = f.id;
                            let matched = f.matched_txs.len();
                            let total_tx = bank_txs.iter().filter(|tx| (tx.amount >= 0.0) == (f.amount >= 0.0)).count();
                            let ratio_color = if matched == 0 { "#64748b" }
                                              else if matched >= total_tx / 2 { "#22c55e" }
                                              else { "#f59e0b" };
                            let f_ref = (*f).clone();
                            let archived = is_archived(f, &bank_txs);
                            let overdue = is_overdue(f, real_today, &bank_txs);
                            let row_opacity = if archived { "0.55" } else { "1" };
                            let row_style = if archived { "italic" } else { "normal" };
                            let subtitle = if f.recurrence.is_recurring() {
                                format!("récur. {} • paie {}", f.recurrence_day, f.payment_day)
                            } else if archived {
                                format!("📌 archivé le {} {}", f.recurrence_day, MONTHS_FR[(f.start_month - 1) as usize])
                            } else if overdue {
                                format!("⚠️ en attente depuis le {} {}", f.recurrence_day, MONTHS_FR[(f.start_month - 1) as usize])
                            } else {
                                format!("ponctuel le {}", f.recurrence_day)
                            };
                            rsx! {
                                tr {
                                    style: "opacity: {row_opacity}; font-style: {row_style};",
                                    td { style: "padding: 6px 8px; white-space: nowrap; position: sticky; left: 0; background: #1e293b; z-index: 1;",
                                        div { style: "display: flex; align-items: center; gap: 6px;",
                                            span { style: "width: 8px; height: 8px; border-radius: 50%; background: {color}; flex-shrink: 0;" }
                                            div {
                                                div { "{f.label}" }
                                                div { style: "font-size: 0.62rem; color: #64748b;", "{subtitle}" }
                                            }
                                        }
                                    }
                                    for d in dates.iter() {
                                        td {
                                            style: "padding: 6px 8px; text-align: right; font-variant-numeric: tabular-nums; white-space: nowrap;",
                                            {format!("{:.0}", flow_cumulative_at(&f_ref, *d))}
                                        }
                                    }
                                    td { style: "padding: 6px 8px; text-align: right;",
                                        if archived {
                                            span {
                                                style: "display: inline-flex; align-items: center; gap: 4px; color: #64748b; font-size: 0.7rem;",
                                                "📌"
                                                span { "archivé" }
                                            }
                                        } else if overdue {
                                            button {
                                                draggable: "false",
                                                style: format!(
                                                    "background: transparent; border: 1px solid {}; color: {}; padding: 2px 8px; border-radius: 10px; font-size: 0.7rem; cursor: pointer; font-weight: 500;",
                                                    OVERDUE_COLOR, OVERDUE_COLOR
                                                ),
                                                onclick: move |_| on_reconcile.call(fid),
                                                "⚠️ à rapprocher"
                                            }
                                        } else {
                                            button {
                                                draggable: "false",
                                                style: format!(
                                                    "background: transparent; border: 1px solid {}; color: {}; padding: 2px 8px; border-radius: 10px; font-size: 0.7rem; cursor: pointer; font-weight: 500;",
                                                    ratio_color, ratio_color
                                                ),
                                                onclick: move |_| on_reconcile.call(fid),
                                                "🔗 {matched}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    tr {
                        style: "border-top: 2px solid #334155;",
                        td {
                            style: "padding: 8px; font-weight: 600; color: #facc15; white-space: nowrap; position: sticky; left: 0; background: #1e293b; z-index: 1;",
                            span { style: "display: inline-flex; align-items: center; gap: 6px;",
                                span { style: "width: 8px; height: 8px; border-radius: 50%; background: #facc15; flex-shrink: 0;" }
                                "Trésorerie prévue"
                            }
                        }
                        for d in dates.iter() {
                            {
                                let total: f64 = sorted.iter().map(|f| flow_cumulative_at(f, *d)).sum();
                                let is_today = d.month_abs() == today_abs;
                                let color = if is_today { TODAY_MARKER_COLOR }
                                            else if total >= 0.0 { "#22c55e" }
                                            else { "#ef4444" };
                                rsx! {
                                    td {
                                        style: "padding: 8px; text-align: right; font-weight: 600; color: {color}; font-variant-numeric: tabular-nums; white-space: nowrap;",
                                        {format!("{:.0}", total)}
                                    }
                                }
                            }
                        }
                        td { style: "padding: 8px; text-align: right; color: #64748b; font-size: 0.7rem;", "—" }
                    }
                    tr {
                        style: "border-top: 1px solid #334155;",
                        td {
                            style: "padding: 8px; font-weight: 600; color: #22c55e; white-space: nowrap; position: sticky; left: 0; background: #1e293b; z-index: 1;",
                            span { style: "display: inline-flex; align-items: center; gap: 6px;",
                                span { style: "width: 8px; height: 8px; border-radius: 50%; background: #22c55e; flex-shrink: 0;" }
                                "Trésorerie réelle"
                            }
                        }
                        for d in dates.iter() {
                            {
                                let planned_at_d: f64 = sorted.iter().map(|f| flow_cumulative_at(f, *d)).sum();
                                let real = treasury_real_at(*d, dates[0], real_today, baseline_real, &parsed_bank, planned_at_d);
                                match real {
                                    Some(v) => {
                                        let color = if v >= 0.0 { "#22c55e" } else { "#ef4444" };
                                        let is_today = d.month_abs() == today_abs;
                                        let weight = if is_today { "700" } else { "500" };
                                        rsx! {
                                            td {
                                                style: "padding: 8px; text-align: right; font-weight: {weight}; color: {color}; font-variant-numeric: tabular-nums; white-space: nowrap;",
                                                {format!("{:.0}", v)}
                                            }
                                        }
                                    }
                                    None => rsx! {
                                        td { style: "padding: 8px; text-align: right; color: #475569; font-size: 0.7rem; white-space: nowrap;", "—" }
                                    }
                                }
                            }
                        }
                        td { style: "padding: 8px; text-align: right; color: #64748b; font-size: 0.7rem;", "—" }
                    }
                }
            }
        }
    }
}

// ============================================================
//  Modale config
// ============================================================
#[component]
fn ForecastSettingsModal(
    flows: Signal<Vec<Flow>>,
    bdd_items: Signal<Vec<PrevisionFlowItem>>,
    reload: Signal<u64>,
    real_today: SimpleDate,
    on_close: EventHandler<()>,
) -> Element {
    let mut editing_id = use_signal(|| None::<usize>);
    let mut label = use_signal(String::new);
    let mut amount = use_signal(String::new);
    let mut color = use_signal(|| "#38bdf8".to_string());
    let mut recurrence = use_signal(|| Recurrence::Monthly);
    let mut start = use_signal(|| real_today.start_of_month());
    let mut rec_day = use_signal(|| "1".to_string());
    let mut pay_day = use_signal(|| "5".to_string());
    let mut error_msg = use_signal(String::new);

    let mut reset = move || {
        editing_id.set(None);
        label.set(String::new());
        amount.set(String::new());
        color.set("#38bdf8".to_string());
        recurrence.set(Recurrence::Monthly);
        start.set(real_today.start_of_month());
        rec_day.set("1".to_string());
        pay_day.set("5".to_string());
        error_msg.set(String::new());
    };

    let is_once = recurrence() == Recurrence::Once;

    rsx! {
        div {
            class: "dash-modal-overlay",
            onclick: move |_| on_close.call(()),
            div {
                class: "dash-modal",
                onclick: move |e| e.stop_propagation(),
                style: "max-width: 680px; max-height: 85vh; overflow-y: auto;",
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                    h2 { style: "margin: 0;", "Configuration du prévisionnel" }
                    button {
                        style: "background: transparent; border: none; color: #94a3b8; font-size: 1.5rem; cursor: pointer; line-height: 1;",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }
                if !error_msg().is_empty() {
                    div { style: "background: #450a0a; border: 1px solid #7f1d1d; color: #fecaca; padding: 8px 12px; border-radius: 4px; font-size: 0.8rem; margin-bottom: 12px;",
                        "{error_msg()}"
                    }
                }
                h4 { style: "color: #38bdf8; margin: 8px 0; font-size: 0.9rem;", "Flux" }
                div { style: "display: flex; flex-direction: column; gap: 6px; margin-bottom: 16px;",
                    for f in flows().iter() {
                        {
                            let fid = f.id;
                            let is_editing = editing_id() == Some(fid);
                            let f_color = f.color.clone();
                            let f_label = f.label.clone();
                            let f_amount = f.amount;
                            let f_recurrence = f.recurrence;
                            let f_start = SimpleDate { year: f.start_year, month: f.start_month, day: 1 };
                            let f_rec = f.recurrence_day;
                            let f_pay = f.payment_day;
                            let summary = if f_recurrence.is_recurring() {
                                format!("{} EUR / {} • récur. {} • paie {}", f_amount, f_recurrence.label(), f_rec, f_pay)
                            } else {
                                format!("{} EUR ponctuel le {} {}", f_amount, f_rec, MONTHS_FR[(f_start.month - 1) as usize])
                            };
                            rsx! {
                                div {
                                    style: format!(
                                        "display: flex; align-items: center; gap: 8px; padding: 8px; background: {}; border-radius: 6px; border: 1px solid {};",
                                        if is_editing { "#0c4a6e" } else { "#0f172a" },
                                        if is_editing { "#38bdf8" } else { "#334155" }
                                    ),
                                    span { style: "width: 12px; height: 12px; border-radius: 50%; background: {f_color}; flex-shrink: 0;" }
                                    div { style: "flex: 1; min-width: 0;",
                                        div { style: "font-size: 0.85rem; font-weight: 500;", "{f_label}" }
                                        div { style: "font-size: 0.7rem; color: #94a3b8;", "{summary}" }
                                    }
                                    button {
                                        style: "background: transparent; border: 1px solid #334155; color: #94a3b8; padding: 3px 8px; border-radius: 4px; cursor: pointer; font-size: 0.7rem;",
                                        onclick: move |_| {
                                            editing_id.set(Some(fid));
                                            label.set(f_label.clone());
                                            amount.set(format!("{}", f_amount));
                                            color.set(f_color.clone());
                                            recurrence.set(f_recurrence);
                                            start.set(f_start);
                                            rec_day.set(f_rec.to_string());
                                            pay_day.set(f_pay.to_string());
                                        },
                                        "Modifier"
                                    }
                                    button {
                                        style: "background: transparent; border: 1px solid #7f1d1d; color: #ef4444; padding: 3px 8px; border-radius: 4px; cursor: pointer; font-size: 0.7rem;",
                                        onclick: move |_| {
                                            flows.with_mut(|v| v.retain(|x| x.id != fid));
                                            if editing_id() == Some(fid) { reset(); }
                                            if let Some(uuid) = ui_id_to_uuid(&bdd_items(), fid) {
                                                spawn(async move {
                                                    match delete_prevision_flow(uuid).await {
                                                        Ok(_) => {
                                                            let r = reload();
                                                            reload.set(r + 1);
                                                        }
                                                        Err(e) => error_msg.set(e.to_string()),
                                                    }
                                                });
                                            }
                                        },
                                        "Suppr"
                                    }
                                }
                            }
                        }
                    }
                    if flows().is_empty() {
                        div { style: "color: #64748b; font-size: 0.8rem; padding: 8px; text-align: center;",
                            "Aucun flux. Ajoutez-en un ci-dessous."
                        }
                    }
                }
                h4 { style: "color: #38bdf8; margin: 8px 0; font-size: 0.9rem;",
                    if editing_id().is_some() { "Modifier le flux" } else { "Ajouter un flux" }
                }
                div { style: "display: flex; flex-direction: column; gap: 8px;",
                    div { style: "display: flex; gap: 8px;",
                        div { style: "flex: 2;",
                            label { style: "display: block; font-size: 0.7rem; color: #94a3b8; margin-bottom: 2px;", "Libellé" }
                            input {
                                style: "width: 100%; background: #0f172a; border: 1px solid #334155; color: #f8fafc; padding: 6px 8px; border-radius: 4px; font-size: 0.85rem; box-sizing: border-box;",
                                value: "{label}",
                                oninput: move |e| label.set(e.value()),
                                placeholder: "Loyers, Crédit, Notaire..."
                            }
                        }
                        div { style: "flex: 1;",
                            label { style: "display: block; font-size: 0.7rem; color: #94a3b8; margin-bottom: 2px;", "Montant" }
                            input {
                                style: "width: 100%; background: #0f172a; border: 1px solid #334155; color: #f8fafc; padding: 6px 8px; border-radius: 4px; font-size: 0.85rem; box-sizing: border-box;",
                                r#type: "number",
                                value: "{amount}",
                                oninput: move |e| amount.set(e.value()),
                                placeholder: "8500"
                            }
                        }
                    }
                    div { style: "display: flex; gap: 8px;",
                        div { style: "flex: 1;",
                            label { style: "display: block; font-size: 0.7rem; color: #94a3b8; margin-bottom: 2px;", "Démarre / date" }
                            div { style: "display: flex; gap: 4px;",
                                select {
                                    style: "flex: 1; background: #0f172a; border: 1px solid #334155; color: #f8fafc; padding: 6px 8px; border-radius: 4px; font-size: 0.85rem; cursor: pointer;",
                                    value: "{start().month}",
                                    onchange: move |e| {
                                        if let Ok(m) = e.value().parse::<u32>() {
                                            let mut s = start();
                                            s.month = m;
                                            s.day = 1;
                                            start.set(s);
                                        }
                                    },
                                    for m in 1..=12u32 {
                                        option { value: "{m}", selected: start().month == m, "{MONTHS_FR_FULL[(m - 1) as usize]}" }
                                    }
                                }
                                select {
                                    style: "background: #0f172a; border: 1px solid #334155; color: #f8fafc; padding: 6px 8px; border-radius: 4px; font-size: 0.85rem; cursor: pointer;",
                                    value: "{start().year}",
                                    onchange: move |e| {
                                        if let Ok(y) = e.value().parse::<i32>() {
                                            let mut s = start();
                                            s.year = y;
                                            s.day = 1;
                                            start.set(s);
                                        }
                                    },
                                    for y in (real_today.year - 5)..=(real_today.year + 10) {
                                        option { value: "{y}", selected: start().year == y, "{y}" }
                                    }
                                }
                            }
                        }
                        div { style: "flex: 1;",
                            label { style: "display: block; font-size: 0.7rem; color: #94a3b8; margin-bottom: 2px;", "Récurrence" }
                            select {
                                style: "width: 100%; background: #0f172a; border: 1px solid #334155; color: #f8fafc; padding: 6px 8px; border-radius: 4px; font-size: 0.85rem;",
                                value: match recurrence() {
                                    Recurrence::Once => "once",
                                    Recurrence::Monthly => "monthly",
                                    Recurrence::Quarterly => "quarterly",
                                    Recurrence::Yearly => "yearly",
                                },
                                onchange: move |e| {
                                    recurrence.set(match e.value().as_str() {
                                        "once" => Recurrence::Once,
                                        "quarterly" => Recurrence::Quarterly,
                                        "yearly" => Recurrence::Yearly,
                                        _ => Recurrence::Monthly,
                                    });
                                },
                                option { value: "once", "Ponctuel" }
                                option { value: "monthly", "Mensuel" }
                                option { value: "quarterly", "Trimestriel" }
                                option { value: "yearly", "Annuel" }
                            }
                        }
                    }
                    div { style: "display: flex; gap: 8px;",
                        div { style: "flex: 1;",
                            label { style: "display: block; font-size: 0.7rem; color: #94a3b8; margin-bottom: 2px;",
                                if is_once { "Jour" } else { "Jour de récurrence (1-31)" }
                            }
                            input {
                                r#type: "number", min: "1", max: "31",
                                style: "width: 100%; background: #0f172a; border: 1px solid #334155; color: #f8fafc; padding: 6px 8px; border-radius: 4px; font-size: 0.85rem; box-sizing: border-box;",
                                value: "{rec_day}",
                                oninput: move |e| rec_day.set(e.value())
                            }
                        }
                        div { style: "flex: 1;",
                            if !is_once {
                                label { style: "display: block; font-size: 0.7rem; color: #94a3b8; margin-bottom: 2px;", "Jour de paiement (1-31)" }
                                input {
                                    r#type: "number", min: "1", max: "31",
                                    style: "width: 100%; background: #0f172a; border: 1px solid #334155; color: #f8fafc; padding: 6px 8px; border-radius: 4px; font-size: 0.85rem; box-sizing: border-box;",
                                    value: "{pay_day}",
                                    oninput: move |e| pay_day.set(e.value())
                                }
                            }
                        }
                    }
                    div { style: "display: flex; gap: 8px;",
                        div { style: "flex: 1;",
                            label { style: "display: block; font-size: 0.7rem; color: #94a3b8; margin-bottom: 2px;", "Couleur" }
                            div { style: "display: flex; align-items: center; gap: 8px;",
                                input {
                                    r#type: "color",
                                    style: "width: 44px; height: 34px; border: 1px solid #334155; border-radius: 4px; background: #0f172a; cursor: pointer; padding: 2px;",
                                    value: "{color}",
                                    oninput: move |e| color.set(e.value())
                                }
                                span { style: "font-size: 0.75rem; color: #94a3b8; font-family: monospace;", "{color}" }
                            }
                        }
                        div { style: "flex: 1;" }
                    }
                    div { style: "display: flex; gap: 8px; margin-top: 4px;",
                        button {
                            style: "flex: 1; background: #38bdf8; color: #0f172a; border: none; padding: 8px; border-radius: 6px; cursor: pointer; font-weight: 600; font-size: 0.85rem;",
                            onclick: move |_| {
                                let l = label().trim().to_string();
                                if l.is_empty() { error_msg.set("Libellé requis".into()); return; }
                                let a: f64 = amount().parse().unwrap_or(0.0);
                                let c = color().clone();
                                let r = recurrence();
                                let s = start();
                                let rd = rec_day().parse::<u32>().unwrap_or(1).clamp(1, 31);
                                let pd = if r.is_recurring() {
                                    pay_day().parse::<u32>().unwrap_or(5).clamp(1, 31)
                                } else {
                                    rd
                                };

                                let draft = PrevisionFlowDraft {
                                    label: l.clone(),
                                    amount_cents: (a * 100.0).round() as i64,
                                    color: c.clone(),
                                    active: true,
                                    recurrence: recurrence_to_str(r),
                                    start_year: s.year,
                                    start_month: s.month as i32,
                                    recurrence_day: rd as i32,
                                    payment_day: pd as i32,
                                };

                                if let Some(eid) = editing_id() {
                                    flows.with_mut(|v| {
                                        if let Some(f) = v.iter_mut().find(|x| x.id == eid) {
                                            f.label = l; f.amount = a; f.color = c;
                                            f.recurrence = r; f.start_year = s.year; f.start_month = s.month;
                                            f.recurrence_day = rd; f.payment_day = pd;
                                        }
                                    });
                                    if let Some(uuid) = ui_id_to_uuid(&bdd_items(), eid) {
                                        spawn(async move {
                                            match update_prevision_flow(uuid, draft).await {
                                                Ok(_) => {
                                                    let n = reload();
                                                    reload.set(n + 1);
                                                }
                                                Err(e) => error_msg.set(e.to_string()),
                                            }
                                        });
                                    }
                                } else {
                                    let temp_id = now_millis() | 1;
                                    flows.with_mut(|v| v.push(Flow {
                                        id: temp_id, label: l, amount: a, color: c,
                                        active: true, recurrence: r,
                                        start_year: s.year, start_month: s.month,
                                        recurrence_day: rd, payment_day: pd,
                                        matched_txs: vec![],
                                    }));
                                    spawn(async move {
                                        match create_prevision_flow(draft).await {
                                            Ok(_) => {
                                                let n = reload();
                                                reload.set(n + 1);
                                            }
                                            Err(e) => error_msg.set(e.to_string()),
                                        }
                                    });
                                }
                                reset();
                            },
                            if editing_id().is_some() { "Enregistrer" } else { "Ajouter" }
                        }
                        if editing_id().is_some() {
                            button {
                                style: "background: transparent; color: #94a3b8; border: 1px solid #334155; padding: 8px 12px; border-radius: 6px; cursor: pointer; font-size: 0.85rem;",
                                onclick: move |_| reset(),
                                "Annuler"
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
//  Modale rapprochement bancaire
// ============================================================
#[component]
fn ReconciliationModal(
    flow_id: usize,
    flows: Signal<Vec<Flow>>,
    bdd_items: Signal<Vec<PrevisionFlowItem>>,
    reload: Signal<u64>,
    bank_txs: Vec<BankTx>,
    on_close: EventHandler<()>,
) -> Element {
    let current = flows().into_iter().find(|f| f.id == flow_id);
    let Some(flow) = current else { return rsx! { div {} }; };

    let flow_amount = flow.amount;
    let flow_label = flow.label.clone();
    let flow_color = flow.color.clone();
    let initial_matches = flow.matched_txs.clone();
    let mut selected = use_signal(move || initial_matches.clone());

    let compatible: Vec<BankTx> = bank_txs.iter()
        .filter(|tx| (tx.amount >= 0.0) == (flow_amount >= 0.0))
        .cloned()
        .collect();

    let matched_sum: f64 = compatible.iter()
        .filter(|tx| selected().contains(&tx.id))
        .map(|tx| tx.amount).sum();

    rsx! {
        div {
            class: "dash-modal-overlay",
            onclick: move |_| on_close.call(()),
            div {
                class: "dash-modal",
                onclick: move |e| e.stop_propagation(),
                style: "max-width: 640px; max-height: 85vh; overflow-y: auto;",
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                    div {
                        div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em;", "Rapprochement bancaire" }
                        h2 { style: "margin: 4px 0 0 0; display: flex; align-items: center; gap: 8px;",
                            span { style: "width: 10px; height: 10px; border-radius: 50%; background: {flow_color};" }
                            "{flow_label}"
                        }
                    }
                    button {
                        style: "background: transparent; border: none; color: #94a3b8; font-size: 1.5rem; cursor: pointer; line-height: 1;",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }
                div { style: "display: flex; gap: 12px; padding: 10px 12px; background: #0f172a; border-radius: 6px; margin-bottom: 12px; font-size: 0.8rem;",
                    div { style: "flex: 1;",
                        div { style: "color: #94a3b8; font-size: 0.7rem;", "Cible" }
                        div { style: "font-weight: 600;", {format!("{:.2} EUR / {}", flow_amount, flow.recurrence.label())} }
                    }
                    div { style: "flex: 1;",
                        div { style: "color: #94a3b8; font-size: 0.7rem;", "Rapproché" }
                        {
                            let color = if matched_sum.abs() > 0.0 && (matched_sum - flow_amount).abs() < flow_amount.abs() * 0.1 {
                                "#22c55e"
                            } else if matched_sum == 0.0 { "#64748b" }
                            else { "#f59e0b" };
                            rsx! { div { style: "font-weight: 600; color: {color};", {format!("{:.2} EUR", matched_sum)} } }
                        }
                    }
                    div { style: "flex: 1;",
                        div { style: "color: #94a3b8; font-size: 0.7rem;", "Sélection" }
                        div { style: "font-weight: 600;", "{selected().len()} / {compatible.len()}" }
                    }
                }
                if compatible.is_empty() {
                    div { style: "text-align: center; padding: 24px; color: #64748b; font-size: 0.85rem;", "Aucune transaction compatible." }
                } else {
                    div { style: "display: flex; flex-direction: column; gap: 4px;",
                        for tx in compatible.iter() {
                            {
                                let tid = tx.id;
                                let is_sel = selected().contains(&tid);
                                let tx_color = if tx.amount >= 0.0 { "#22c55e" } else { "#ef4444" };
                                rsx! {
                                    label {
                                        style: format!(
                                            "display: flex; align-items: center; gap: 10px; padding: 8px 10px; background: {}; border-radius: 6px; border: 1px solid {}; cursor: pointer; font-size: 0.8rem;",
                                            if is_sel { "#1e293b" } else { "#0f172a" },
                                            if is_sel { "#38bdf8" } else { "#334155" }
                                        ),
                                        input {
                                            r#type: "checkbox", checked: is_sel,
                                            onchange: move |_| {
                                                selected.with_mut(|v| {
                                                    if v.contains(&tid) { v.retain(|x| *x != tid); }
                                                    else { v.push(tid); }
                                                });
                                            }
                                        }
                                        div { style: "flex: 1; min-width: 0;",
                                            div { style: "font-weight: 500;", "{tx.label}" }
                                            div { style: "color: #94a3b8; font-size: 0.7rem;", "{tx.date}" }
                                        }
                                        div { style: "font-weight: 600; color: {tx_color}; font-variant-numeric: tabular-nums;", {format!("{:.2} EUR", tx.amount)} }
                                    }
                                }
                            }
                        }
                    }
                }
                div { style: "display: flex; gap: 8px; margin-top: 16px;",
                    button {
                        style: "flex: 1; background: transparent; color: #94a3b8; border: 1px solid #334155; padding: 8px; border-radius: 6px; cursor: pointer; font-size: 0.85rem;",
                        onclick: move |_| selected.set(Vec::new()),
                        "Tout décocher"
                    }
                    button {
                        style: "flex: 2; background: #38bdf8; color: #0f172a; border: none; padding: 8px; border-radius: 6px; cursor: pointer; font-weight: 600; font-size: 0.85rem;",
                        onclick: move |_| {
                            let sel = selected();
                            flows.with_mut(|v| {
                                if let Some(f) = v.iter_mut().find(|x| x.id == flow_id) {
                                    f.matched_txs = sel.clone();
                                }
                            });
                            if let Some(uuid) = ui_id_to_uuid(&bdd_items(), flow_id) {
                                let tx_uuids: Vec<Uuid> = sel.iter()
                                    .map(|ui_id| Uuid::from_u128(*ui_id as u128))
                                    .collect();
                                spawn(async move {
                                    let _ = set_prevision_flow_matches(uuid, tx_uuids).await;
                                    let r = reload();
                                    reload.set(r + 1);
                                });
                            }
                            on_close.call(());
                        },
                        "Valider le rapprochement"
                    }
                }
            }
        }
    }
}