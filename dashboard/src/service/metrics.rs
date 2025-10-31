use axum::{
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::{ToPrimitive, FromPrimitive};
use serde_json::json;
use tracing::info;
 
use std::sync::Arc;

use crate::{
    AppState,
    models::{
        metrics::{
            DrawdownMetrics, ExpectancyMetrics, MetricsRequest, MetricsResponseBody, ProfitFactorMetrics,
            RecoveryFactorMetrics, SharpeMetrics, BASE_CAPITAL,
        },
        strategy::{Status, Strategy},
    },
};

use super::common::AppError;
// Inline helper functions and types for metric calculations
struct NetsSummary {
    nets: Vec<Decimal>,
    wins_sum: Decimal,
    losses_sum_abs: Decimal,
    wins_count: usize,
    losses_count: usize,
}

fn derive_nets(rows: &[Strategy]) -> NetsSummary {
    let mut nets: Vec<Decimal> = Vec::with_capacity(rows.len());
    let mut wins_sum = Decimal::ZERO;
    let mut losses_sum_abs = Decimal::ZERO;
    let mut wins_count = 0usize;
    let mut losses_count = 0usize;

    for s in rows {
        let net = s.risk.stats.pnl - s.risk.stats.fee;
        if net > Decimal::ZERO {
            wins_sum += net;
            wins_count += 1;
        } else if net < Decimal::ZERO {
            losses_sum_abs += -net;
            losses_count += 1;
        }
        nets.push(net);
    }

    NetsSummary { nets, wins_sum, losses_sum_abs, wins_count, losses_count }
}

fn daily_from_rows(from: chrono::NaiveDate, to: chrono::NaiveDate, rows: &[Strategy]) -> std::collections::BTreeMap<chrono::NaiveDate, Decimal> {
    let mut daily: std::collections::BTreeMap<chrono::NaiveDate, Decimal> = std::collections::BTreeMap::new();
    let mut d = from;
    while d <= to {
        daily.insert(d, Decimal::ZERO);
        d = d.succ_opt().unwrap();
    }
    for s in rows {
        let day = s.exit_time.date_naive();
        let net = s.risk.stats.pnl - s.risk.stats.fee;
        if let Some(v) = daily.get_mut(&day) {
            *v += net;
        }
    }
    daily
}

fn equity_from_daily(daily: &std::collections::BTreeMap<chrono::NaiveDate, Decimal>) -> Vec<(chrono::NaiveDate, Decimal)> {
    let mut equity: Vec<(chrono::NaiveDate, Decimal)> = Vec::with_capacity(daily.len());
    let mut cum = Decimal::ZERO;
    for (day, val) in daily.iter() {
        cum += *val;
        equity.push((*day, cum));
    }
    equity
}

struct DrawdownAux {
    metrics: DrawdownMetrics,
    max_dd_abs: Decimal,
}

fn compute_drawdown(equity: &[(chrono::NaiveDate, Decimal)]) -> DrawdownAux {
    let mut peak = Decimal::ZERO;
    let mut peak_date: Option<chrono::NaiveDate> = None;
    let mut max_dd = Decimal::ZERO;
    let mut max_dd_peak_date: Option<chrono::NaiveDate> = None;
    let mut max_dd_trough_date: Option<chrono::NaiveDate> = None;

    for (day, eq) in equity {
        if *eq > peak {
            peak = *eq;
            peak_date = Some(*day);
        }
        let dd = peak - *eq;
        if dd > max_dd {
            max_dd = dd;
            max_dd_peak_date = peak_date;
            max_dd_trough_date = Some(*day);
        }
    }

    let recovery_days = if let (Some(p_d), Some(t_d)) = (max_dd_peak_date, max_dd_trough_date) {
        let mut trough_idx: Option<usize> = None;
        let mut prior_peak_val = Decimal::ZERO;
        for (idx, (day, eq)) in equity.iter().enumerate() {
            if *day == p_d { prior_peak_val = *eq; }
            if *day == t_d { trough_idx = Some(idx); break; }
        }
        if let Some(ti) = trough_idx {
            let mut rec: Option<i64> = None;
            for (idx, (_day, eq)) in equity.iter().enumerate().skip(ti) {
                if *eq >= prior_peak_val {
                    rec = Some((idx as i64) - (ti as i64));
                    break;
                }
            }
            rec
        } else { None }
    } else { None };

    let dd_pct_base: f64 = if BASE_CAPITAL > 0.0 {
        max_dd.to_f64().unwrap_or(0.0) / BASE_CAPITAL
    } else { 0.0 };

    DrawdownAux {
        metrics: DrawdownMetrics {
            max_dd_abs: max_dd,
            max_dd_pct_base: dd_pct_base,
            peak_date: max_dd_peak_date,
            trough_date: max_dd_trough_date,
            recovery_days,
        },
        max_dd_abs: max_dd,
    }
}

fn compute_sharpe(daily: &std::collections::BTreeMap<chrono::NaiveDate, Decimal>, rf_annual: f64) -> SharpeMetrics {
    let mut daily_returns: Vec<f64> = Vec::with_capacity(daily.len());
    for (_d, v) in daily.iter() {
        let r = v.to_f64().unwrap_or(0.0) / BASE_CAPITAL;
        daily_returns.push(r);
    }
    let sample_days = daily_returns.len();
    let sharpe_tuple = if sample_days >= 2 {
        let rf_daily = rf_annual / 252.0_f64;
        let mut excess: Vec<f64> = Vec::with_capacity(sample_days);
        for r in &daily_returns { excess.push(*r - rf_daily); }
        let mean = excess.iter().sum::<f64>() / sample_days as f64;
        let mut var = 0.0_f64;
        for r in &excess { var += (r - mean) * (r - mean); }
        var /= sample_days as f64 - 1.0;
        let std = var.sqrt();
        if std > 0.0 { Some((mean / std * (252.0_f64).sqrt(), mean, std)) } else { None }
    } else { None };

    let (sharpe_opt, mean_opt, vol_opt) = match sharpe_tuple {
        Some((s, m, v)) => (Some(s), Some(m), Some(v)),
        None => (None, None, None),
    };

    SharpeMetrics { sharpe: sharpe_opt, mean_daily: mean_opt, vol_daily: vol_opt, rf_annual, sample_days }
}

fn compute_expectancy(nets: &[Decimal], wins_sum: Decimal, wins_count: usize, losses_sum_abs: Decimal, losses_count: usize) -> ExpectancyMetrics {
    let trade_count = nets.len();
    let expectancy_usd: Decimal = if trade_count > 0 {
        let sum_net: Decimal = nets.iter().cloned().sum();
        let denom = Decimal::from_i32(trade_count as i32).unwrap_or(Decimal::ZERO);
        if denom > Decimal::ZERO { sum_net / denom } else { Decimal::ZERO }
    } else { Decimal::ZERO };

    let median_usd: Decimal = if trade_count == 0 { Decimal::ZERO } else {
        let mut sorted = nets.to_vec();
        sorted.sort();
        let mid = trade_count / 2;
        if trade_count % 2 == 1 { sorted[mid] } else { let two = Decimal::from_i32(2).unwrap(); (sorted[mid - 1] + sorted[mid]) / two }
    };

    let win_rate = if trade_count > 0 { Some(wins_count as f64 / trade_count as f64) } else { None };
    let avg_win = if wins_count > 0 { let denom = Decimal::from_i32(wins_count as i32).unwrap(); wins_sum / denom } else { Decimal::ZERO };
    let avg_loss = if losses_count > 0 { let denom = Decimal::from_i32(losses_count as i32).unwrap(); -(losses_sum_abs) / denom } else { Decimal::ZERO };

    ExpectancyMetrics { expectancy_usd, median_usd, win_rate, avg_win, avg_loss, trade_count }
}

fn compute_profit_factor(wins_sum: Decimal, losses_sum_abs: Decimal, wins_count: usize, losses_count: usize, trade_count: usize) -> ProfitFactorMetrics {
    let profit_factor = if losses_sum_abs > Decimal::ZERO {
        Some((wins_sum.to_f64().unwrap_or(0.0)) / (losses_sum_abs.to_f64().unwrap_or(1.0)))
    } else if wins_sum > Decimal::ZERO && trade_count > 0 {
        None
    } else { None };

    ProfitFactorMetrics { profit_factor, gross_profit: wins_sum, gross_loss: losses_sum_abs, wins: wins_count, losses: losses_count, trade_count }
}

fn compute_recovery(net_profit: Decimal, max_dd: Decimal) -> RecoveryFactorMetrics {
    let recovery_factor = if max_dd > Decimal::ZERO { Some(net_profit.to_f64().unwrap_or(0.0) / max_dd.to_f64().unwrap_or(1.0)) } else { None };
    RecoveryFactorMetrics { recovery_factor, net_profit, reference_max_dd: max_dd }
}

pub(crate) async fn metrics(
    Query(request): Query<MetricsRequest>,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse {
    let status = Status::Closed;

    let result: Result<Vec<Strategy>, AppError> = {
        let query = r#"
        SELECT
            *
        FROM
            strategy
        WHERE
            exit_time::date >= $1
        AND exit_time::date <= $2
        AND status = $3
        "#;

        sqlx::query_as::<_, Strategy>(query)
            .bind(request.from)
            .bind(request.to)
            .bind(Into::<i32>::into(status))
            .fetch_all(&state.db.pool)
            .await
            .map_err(AppError::DatabaseError)
    };

    match result {
        Err(e) => e.into_response(),
        Ok(rows) => {
            // Build inputs
            let NetsSummary { nets, wins_sum, losses_sum_abs, wins_count, losses_count } = derive_nets(&rows);
            let daily = daily_from_rows(request.from, request.to, &rows);
            let equity = equity_from_daily(&daily);

            // Compute metrics
            let drawdown_aux = compute_drawdown(&equity);
            // Pull risk-free rate from the most recent available account snapshot in the set
            let rf_annual = rows
                .iter()
                .max_by_key(|s| s.exit_time)
                .map(|s| s.account.risk_free_annual)
                .unwrap_or(0.0_f64);
            let sharpe = compute_sharpe(&daily, rf_annual);
            let expectancy = compute_expectancy(&nets, wins_sum, wins_count, losses_sum_abs, losses_count);
            let pf = compute_profit_factor(wins_sum, losses_sum_abs, wins_count, losses_count, nets.len());

            let net_profit: Decimal = equity.last().map(|(_, eq)| *eq).unwrap_or(Decimal::ZERO);
            let recovery = compute_recovery(net_profit, drawdown_aux.max_dd_abs);

            let body = MetricsResponseBody {
                from: request.from,
                to: request.to,
                drawdown: drawdown_aux.metrics,
                sharpe,
                expectancy,
                recovery,
                profit_factor: pf,
            };

            let response = Json(json!({
                "metrics": body
            }));

            info!("Metrics: {}", json!(body));

            (StatusCode::OK, response).into_response()
        }
    }
}
