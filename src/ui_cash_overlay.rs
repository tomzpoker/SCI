use std::collections::BTreeMap;

#[server]
pub async fn dashboard_real_cash_history(months: i32) -> Result<Vec<(NaiveDate, i64)>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        let months = months.clamp(3, 24);
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = crate::entity_scope::current_legal_entity_id();
        let rows = sqlx::query(
            r#"
            WITH months AS (
                SELECT date_trunc(
                    'month', CURRENT_DATE + (g || ' month')::interval
                )::date AS month_start
                FROM generate_series(-($1::int - 1), 0) g
            )
            SELECT
                m.month_start,
                (
                    COALESCE((
                        SELECT SUM(opening_balance_cents)::bigint
                        FROM bank_account_profiles
                        WHERE legal_entity_id = $2 AND active
                    ), 0)
                    + COALESCE((
                        SELECT SUM(amount_cents)::bigint
                        FROM bank_transactions
                        WHERE legal_entity_id = $2
                          AND transaction_date < (m.month_start + interval '1 month')::date
                    ), 0)
                )::bigint AS balance_cents
            FROM months m
            ORDER BY m.month_start
            "#,
        )
        .bind(months)
        .bind(entity)
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| (r.get("month_start"), r.get("balance_cents")))
            .collect())
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = months;
        Err(ServerFnError::new(
            "dashboard_real_cash_history est exécutée côté serveur",
        ))
    }
}

#[derive(Clone)]
struct CurveScale {
    min: f64,
    max: f64,
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

fn curve_x(index: usize, count: usize, scale: &CurveScale) -> f64 {
    if count <= 1 {
        return scale.left;
    }
    scale.left + (index as f64 / (count - 1) as f64) * (scale.right - scale.left)
}

fn curve_y(value_cents: i64, scale: &CurveScale) -> f64 {
    let span = (scale.max - scale.min).max(1.0);
    scale.bottom - ((value_cents as f64 / 100.0 - scale.min) / span) * (scale.bottom - scale.top)
}

fn build_curve_path(
    dates: &[NaiveDate],
    values: &BTreeMap<NaiveDate, i64>,
    scale: &CurveScale,
) -> String {
    let mut path = String::new();
    let mut drawing = false;
    for (index, date) in dates.iter().enumerate() {
        let Some(value) = values.get(date) else {
            drawing = false;
            continue;
        };
        let x = curve_x(index, dates.len(), scale);
        let y = curve_y(*value, scale);
        if drawing {
            path.push_str(&format!(" L {:.1} {:.1}", x, y));
        } else {
            path.push_str(&format!("M {:.1} {:.1}", x, y));
            drawing = true;
        }
    }
    path
}

#[component]
fn TreasuryDualCurveChart(
    real_history: Vec<(NaiveDate, i64)>,
    forecast: Vec<crate::treasury::TreasuryForecastPointItem>,
) -> Element {
    let mut dates = Vec::<NaiveDate>::new();
    for (date, _) in &real_history {
        if !dates.contains(date) {
            dates.push(*date);
        }
    }
    for point in &forecast {
        if !dates.contains(&point.date) {
            dates.push(point.date);
        }
    }
    dates.sort_unstable();

    let mut real = BTreeMap::<NaiveDate, i64>::new();
    for (date, value) in real_history {
        real.insert(date, value);
    }
    let mut planned = BTreeMap::<NaiveDate, i64>::new();
    for point in forecast {
        planned.insert(point.date, point.balance_cents);
    }

    let mut all_values = Vec::<i64>::new();
    all_values.extend(real.values().copied());
    all_values.extend(planned.values().copied());
    let min_eur = all_values
        .iter()
        .map(|v| *v as f64 / 100.0)
        .fold(f64::INFINITY, f64::min);
    let max_eur = all_values
        .iter()
        .map(|v| *v as f64 / 100.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let pad = ((max_eur - min_eur).abs() * 0.12).max(300.0);
    let scale = CurveScale {
        min: min_eur - pad,
        max: max_eur + pad,
        left: 22.0,
        right: 738.0,
        top: 20.0,
        bottom: 235.0,
    };
    let real_path = build_curve_path(&dates, &real, &scale);
    let planned_path = build_curve_path(&dates, &planned, &scale);
    let y0 = curve_y(((scale.min + scale.max) / 2.0 * 100.0) as i64, &scale);

    rsx! {
        section { class: "panel dual-curve-panel",
            div { class: "panel-head dual-curve-head",
                div {
                    h3 { "💰 Trésorerie" }
                    div { class: "small", "Ce qui est arrivé • ce qui est prévu" }
                }
                div { class: "curve-legend",
                    span { class: "curve-legend-item real", span { class: "curve-dot" }, "Réel" }
                    span { class: "curve-legend-item planned", span { class: "curve-dot" }, "Prévu" }
                }
            }
            if dates.is_empty() {
                div { class: "empty-state", h3 { "Pas encore assez de données" }, p { "Le graphique apparaîtra dès que la banque et le prévisionnel contiendront des données." } }
            } else {
                div { class: "dual-curve-wrap",
                    svg { class: "dual-curve-svg", view_box: "0 0 760 280", preserve_aspect_ratio: "none",
                        line { x1: "22", y1: "45", x2: "738", y2: "45", class: "curve-grid-line" }
                        line { x1: "22", y1: "95", x2: "738", y2: "95", class: "curve-grid-line" }
                        line { x1: "22", y1: "145", x2: "738", y2: "145", class: "curve-grid-line" }
                        line { x1: "22", y1: "195", x2: "738", y2: "195", class: "curve-grid-line" }
                        line { x1: "22", y1: "{y0}", x2: "738", y2: "{y0}", class: "curve-zero-line" }
                        path { d: "{planned_path}", class: "curve-line planned" }
                        path { d: "{real_path}", class: "curve-line real" }
                        for (index, date) in dates.iter().enumerate().filter(|(i, _)| *i % 3 == 0 || *i + 1 == dates.len()) {
                            text { x: "{curve_x(index, dates.len(), &scale)}", y: "260", class: "curve-x-label", text_anchor: "middle", "{date.format(\"%b %Y\").to_string()}" }
                        }
                    }
                }
                div { class: "dual-curve-note",
                    span { "Réel" }
                    span { "→ mouvements bancaires constatés" }
                    span { "Prévu" }
                    span { "→ projection de trésorerie du moteur" }
                }
            }
        }
    }
}
