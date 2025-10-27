use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::NaiveDate;
use serde_json::json;
use std::sync::Arc;
use std::collections::BTreeMap;
use rust_decimal::{Decimal};
use rust_decimal::prelude::{ToPrimitive, FromPrimitive};

use crate::{
    AppState,
    models::{
        performance::{Performance, PerformanceRequest, PerformanceResponse},
        strategy::{Status, Strategy, StrategyResponse},
        symbol::{Symbol, SymbolResponse},
        watermark::{WatermarkDataPoint, WatermarkRequest, WatermarkResponse},
        metrics::{MetricsRequest, MetricsResponseBody, DrawdownMetrics, SharpeMetrics, ProfitFactorMetrics, ExpectancyMetrics, RecoveryFactorMetrics, BASE_CAPITAL},
    },
};

enum AppError {
    DatabaseError(sqlx::Error),
    // Add other error types as needed
}

pub(crate) async fn metrics(
    Query(request): Query<MetricsRequest>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Fetch strategies within window, optionally by symbol, and closed only
    let status = Status::Closed;

    let result: Result<Vec<Strategy>, AppError> = if let Some(sym) = &request.symbol {
        let query = r#"
        SELECT
            *
        FROM
            strategy
        WHERE
            symbol = $1
        AND entry_time >= $2
        AND exit_time <= $3
        AND status = $4
        "#;

        sqlx::query_as::<_, Strategy>(query)
            .bind(sym)
            .bind(request.from)
            .bind(request.to)
            .bind(Into::<i32>::into(status))
            .fetch_all(&state.db.pool)
            .await
            .map_err(AppError::DatabaseError)
    } else {
        let query = r#"
        SELECT
            *
        FROM
            strategy
        WHERE
            entry_time >= $1
        AND exit_time <= $2
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
            // Derive per-trade net PnL (net of fees)
            let mut nets: Vec<Decimal> = Vec::with_capacity(rows.len());
            let mut wins: Vec<Decimal> = Vec::new();
            let mut losses: Vec<Decimal> = Vec::new();

            for s in &rows {
                let net = s.risk.stats.pnl - s.risk.stats.fee;
                if net > Decimal::ZERO {
                    wins.push(net);
                } else if net < Decimal::ZERO {
                    losses.push(net);
                }
                nets.push(net);
            }

            let trade_count = nets.len();

            // Profit factor components (used internally)
            let gross_profit: Decimal = wins.iter().cloned().sum();
            let gross_loss_abs: Decimal = losses.iter().cloned().map(|x| -x).sum();
            let profit_factor = if gross_loss_abs > Decimal::ZERO {
                Some((gross_profit.to_f64().unwrap_or(0.0)) / (gross_loss_abs.to_f64().unwrap_or(1.0)))
            } else if gross_profit > Decimal::ZERO && trade_count > 0 {
                None // treat as undefined/infinite, avoid special float
            } else {
                None
            };

            // Expectancy and related stats
            let expectancy_usd: Decimal = if trade_count > 0 {
                let sum_net: Decimal = nets.iter().cloned().sum();
                let denom = Decimal::from_i32(trade_count as i32).unwrap_or(Decimal::ZERO);
                if denom > Decimal::ZERO { sum_net / denom } else { Decimal::ZERO }
            } else { Decimal::ZERO };

            let median_usd: Decimal = if trade_count == 0 {
                Decimal::ZERO
            } else {
                let mut sorted = nets.clone();
                sorted.sort();
                let mid = trade_count / 2;
                if trade_count % 2 == 1 {
                    sorted[mid]
                } else {
                    let two = Decimal::from_i32(2).unwrap();
                    (sorted[mid - 1] + sorted[mid]) / two
                }
            };

            let wins_count = wins.len();
            let losses_count = losses.len();
            let win_rate = if trade_count > 0 {
                Some(wins_count as f64 / trade_count as f64)
            } else { None };

            let avg_win: Decimal = if wins_count > 0 {
                let denom = Decimal::from_i32(wins_count as i32).unwrap();
                wins.iter().cloned().sum::<Decimal>() / denom
            } else { Decimal::ZERO };
            let avg_loss: Decimal = if losses_count > 0 {
                let denom = Decimal::from_i32(losses_count as i32).unwrap();
                losses.iter().cloned().sum::<Decimal>() / denom
            } else { Decimal::ZERO };

            // Daily net PnL (fill zeros on non-trade days)
            let mut daily: BTreeMap<NaiveDate, Decimal> = BTreeMap::new();
            // initialize range with zeros
            let mut d = request.from;
            while d <= request.to {
                daily.insert(d, Decimal::ZERO);
                d = d.succ_opt().unwrap();
            }
            for s in &rows {
                let day = s.exit_time.date_naive();
                let net = s.risk.stats.pnl - s.risk.stats.fee;
                if let Some(v) = daily.get_mut(&day) {
                    *v += net;
                }
            }

            // Build equity curve (realized)
            let mut equity: Vec<(NaiveDate, Decimal)> = Vec::with_capacity(daily.len());
            let mut cum = Decimal::ZERO;
            for (day, val) in daily.iter() {
                cum += *val;
                equity.push((*day, cum));
            }

            // Max drawdown metrics
            let mut peak = Decimal::ZERO;
            let mut peak_date: Option<NaiveDate> = None;
            let mut max_dd = Decimal::ZERO; // absolute
            let mut max_dd_peak_date: Option<NaiveDate> = None;
            let mut max_dd_trough_date: Option<NaiveDate> = None;

            for (day, eq) in &equity {
                if *eq > peak {
                    peak = *eq;
                    peak_date = Some(*day);
                }
                let dd = peak - *eq; // absolute decline from peak
                if dd > max_dd {
                    max_dd = dd;
                    max_dd_peak_date = peak_date;
                    max_dd_trough_date = Some(*day);
                }
            }

            // Recovery days: from trough to date equity >= prior peak
            let recovery_days = if let (Some(p_d), Some(t_d)) = (max_dd_peak_date, max_dd_trough_date) {
                // Find index of trough
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

            // Sharpe: daily returns normalized by base capital, rf=0, annualized
            let rf_annual = 0.0_f64;
            let base = BASE_CAPITAL;
            let mut daily_returns: Vec<f64> = Vec::with_capacity(daily.len());
            for (_d, v) in daily.iter() {
                let r = v.to_f64().unwrap_or(0.0) / base;
                daily_returns.push(r);
            }
            let sample_days = daily_returns.len();
            let sharpe_tuple = if sample_days >= 2 {
                let mean = daily_returns.iter().sum::<f64>() / (sample_days as f64);
                let mut var = 0.0_f64;
                for r in &daily_returns {
                    var += (r - mean) * (r - mean);
                }
                var /= (sample_days as f64 - 1.0);
                let std = var.sqrt();
                if std > 0.0 {
                    let sharpe = (mean /* excess, rf_daily=0 */) / std * (252.0_f64).sqrt();
                    Some((sharpe, mean, std))
                } else { None }
            } else { None };

            let (sharpe_opt, mean_opt, vol_opt) = match sharpe_tuple {
                Some((s, m, v)) => (Some(s), Some(m), Some(v)),
                None => (None, None, None),
            };

            // Recovery factor
            let net_profit: Decimal = equity.last().map(|(_, eq)| *eq).unwrap_or(Decimal::ZERO);
            let recovery_factor = if max_dd > Decimal::ZERO {
                Some(net_profit.to_f64().unwrap_or(0.0) / max_dd.to_f64().unwrap_or(1.0))
            } else { None };

            let dd_pct_base: f64 = if BASE_CAPITAL > 0.0 {
                max_dd.to_f64().unwrap_or(0.0) / BASE_CAPITAL
            } else { 0.0 };

            let drawdown = DrawdownMetrics {
                max_dd_abs: max_dd,
                max_dd_pct_base: dd_pct_base,
                peak_date: max_dd_peak_date,
                trough_date: max_dd_trough_date,
                recovery_days,
            };

            let sharpe = SharpeMetrics {
                sharpe: sharpe_opt,
                mean_daily: mean_opt,
                vol_daily: vol_opt,
                rf_annual,
                sample_days,
            };

            let expectancy = ExpectancyMetrics {
                expectancy_usd,
                median_usd,
                win_rate,
                avg_win,
                avg_loss,
                trade_count,
            };

            let pf = ProfitFactorMetrics {
                profit_factor,
                gross_profit,
                gross_loss: gross_loss_abs,
                wins: wins_count,
                losses: losses_count,
                trade_count,
            };

            let recovery = RecoveryFactorMetrics {
                recovery_factor,
                net_profit,
                reference_max_dd: max_dd,
            };

            let body = MetricsResponseBody {
                symbol: request.symbol.clone(),
                from: request.from,
                to: request.to,
                drawdown,
                sharpe,
                expectancy,
                recovery,
                profit_factor: pf,
            };

            let response = Json(json!({
                "metrics": body
            }));

            (StatusCode::OK, response).into_response()
        }
    }
}

// Implement IntoResponse for your error type
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AppError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            ),
        };

        // Create a JSON response with the error details
        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

#[derive(serde::Deserialize, sqlx::Encode)]
pub struct SimpleRequest {
    from: NaiveDate,
    to: NaiveDate,
}
pub(crate) async fn health() -> StatusCode {
    StatusCode::OK
}

pub(crate) async fn symbols(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let query = r#"
    SELECT 
        Distinct(symbol)
    FROM 
        strategy
    "#;

    let result = sqlx::query_as::<_, Symbol>(query)
        .fetch_all(&state.db.pool)
        .await
        .map_err(AppError::DatabaseError);

    match result {
        Ok(rows) => SymbolResponse {
            response: rows.iter().map(Symbol::from).collect(),
        }
        .into_response(),
        Err(e) => e.into_response(),
    }
}

pub(crate) async fn strategy(
    Path(symbol): Path<String>,
    Query(request): Query<SimpleRequest>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let query = r#"
    SELECT
        *
    FROM
        strategy
    WHERE
        symbol = $1
    AND entry_time >= $2
    AND exit_time <= $3
    "#;

    let result = sqlx::query_as::<_, Strategy>(query)
        .bind(symbol)
        .bind(request.from)
        .bind(request.to)
        .fetch_all(&state.db.pool)
        .await
        .map_err(AppError::DatabaseError);

    match result {
        Ok(rows) => StrategyResponse { response: rows }.into_response(),
        Err(e) => e.into_response(),
    }
}

pub(crate) async fn universe(
    Query(request): Query<SimpleRequest>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let query = r#"
    SELECT
        *
    FROM
        strategy
    WHERE
        entry_time >= $1
    AND exit_time <= $2
    "#;

    let result = sqlx::query_as::<_, Strategy>(query)
        .bind(request.from)
        .bind(request.to)
        .fetch_all(&state.db.pool)
        .await
        .map_err(AppError::DatabaseError);

    match result {
        Ok(rows) => StrategyResponse { response: rows }.into_response(),
        Err(e) => e.into_response(),
    }
}

pub(crate) async fn performance(
    Query(request): Query<PerformanceRequest>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let query = r#"
    SELECT
        *
    FROM
        strategy
    WHERE
        entry_time >= $1
    AND exit_time <= $2
    AND status = $3
    "#;

    let status = match request.is_active {
        true => Status::Open,
        false => Status::Closed,
    };

    let result = sqlx::query_as::<_, Strategy>(query)
        .bind(request.from)
        .bind(request.to)
        .bind(Into::<i32>::into(status))
        .fetch_all(&state.db.pool)
        .await
        .map_err(AppError::DatabaseError);

    match result {
        Ok(rows) => PerformanceResponse {
            response: rows.iter().map(Performance::from).collect(),
        }
        .into_response(),
        Err(e) => e.into_response(),
    }
}

pub(crate) async fn watermarks(
    Query(request): Query<WatermarkRequest>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Always use full year range (365 days from request.to going backwards)
    let year_start = request.to - chrono::Duration::days(365);
    
    // Query to get watermarks from closed trades with positive PNL
    let query = r#"
    SELECT
        exit_time,
        (risk->>'stats')::jsonb->>'pnl' as pnl,
        (risk->>'loss')::jsonb->>'watermark' as watermark
    FROM
        strategy
    WHERE
        entry_time >= $1
    AND exit_time <= $2
    AND status = $3
    AND (risk->>'stats')::jsonb->>'pnl' IS NOT NULL
    AND ((risk->>'stats')::jsonb->>'pnl')::numeric > 0
    AND (risk->>'loss')::jsonb->>'watermark' IS NOT NULL
    "#;

    let status = Status::Closed;

    let result = sqlx::query(query)
        .bind(year_start)
        .bind(request.to)
        .bind(Into::<i32>::into(status))
        .fetch_all(&state.db.pool)
        .await
        .map_err(AppError::DatabaseError);

    match result {
        Ok(rows) => {
            use std::collections::HashMap;
            use rust_decimal::Decimal;
            use std::str::FromStr;
            use sqlx::Row;

            // First pass: collect all watermark values to find min/max
            let mut watermark_values: Vec<f64> = Vec::new();
            let mut row_data: Vec<(chrono::DateTime<chrono::Utc>, f64)> = Vec::new();

            for row in &rows {
                let exit_time: chrono::DateTime<chrono::Utc> = row.try_get("exit_time").unwrap_or_default();
                let watermark_str: String = row.try_get("watermark").unwrap_or_default();
                
                if let Ok(watermark) = Decimal::from_str(&watermark_str) {
                    let mut watermark_f64 = watermark.to_string().parse::<f64>().unwrap_or(0.0);
                    
                    // Watermark might be stored as decimal (0.2) instead of percentage (20)
                    if watermark_f64 > 0.0 && watermark_f64 < 1.0 {
                        watermark_f64 *= 100.0;
                    }
                    
                    watermark_values.push(watermark_f64);
                    row_data.push((exit_time, watermark_f64));
                }
            }

            // Fixed watermark range: 20-40
            let min_watermark = 20.0;
            let max_watermark = 40.0;

            // Create watermark ranges with scale of 1
            let mut watermark_ranges: Vec<(f64, f64, String)> = Vec::new();
            let mut current = min_watermark;
            while current < max_watermark {
                let next = current + 1.0;
                let label = format!("{}", current as i32);
                watermark_ranges.push((current, next, label));
                current = next;
            }

            let mut heatmap_data: HashMap<(String, String), i32> = HashMap::new();

            // Second pass: bucket the data by week
            for (exit_time, watermark_f64) in row_data {
                // Calculate week number from year_start
                let days_from_start = (exit_time.date_naive() - year_start).num_days();
                let week_number = (days_from_start / 7).max(0).min(51); // 0-51 for 52 weeks
                
                // Calculate the start date of this week
                let week_start = year_start + chrono::Duration::days(week_number * 7);
                let time_label = format!("W{:02}-{}", week_number + 1, week_start.format("%m/%d"));
                
                // Determine watermark range
                for (min, max, label) in &watermark_ranges {
                    if watermark_f64 >= *min && watermark_f64 < *max {
                        let key = (time_label.clone(), label.to_string());
                        *heatmap_data.entry(key).or_insert(0) += 1;
                        break;
                    }
                }
            }

            // Convert to response format
            let data: Vec<WatermarkDataPoint> = heatmap_data
                .into_iter()
                .map(|((x, y), value)| WatermarkDataPoint { x, y, value })
                .collect();

            WatermarkResponse { 
                data,
                min_watermark,
                max_watermark
            }.into_response()
        }
        Err(e) => e.into_response(),
    }
}
