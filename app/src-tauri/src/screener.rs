use std::collections::HashMap;

use chrono::NaiveDate;
use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{Euclid, ToPrimitive};

use crate::{
    engine::{build_pivots, build_strokes, find_third_buy, merge_bars},
    model::{Bar, BuyPoint, ChartData, Funnel, PivotChart, Signal},
};

#[derive(Debug)]
pub struct ScanOutcome {
    pub signals: Vec<Signal>,
    pub funnel: Funnel,
    pub lags: Vec<usize>,
    pub last_date: Option<NaiveDate>,
}

pub fn py_round(value: f64, digits: i32) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let scale = BigInt::from(10u8).pow(digits.max(0) as u32);
    let exact = BigRational::from_float(value).expect("有限浮点数可转换为有理数");
    let scaled = exact * BigRational::from_integer(scale.clone());
    let floor = scaled.numer().div_euclid(scaled.denom());
    let remainder = scaled.numer().rem_euclid(scaled.denom());
    let midpoint = (&remainder * 2u8).cmp(scaled.denom());
    let rounded = if midpoint.is_gt() || (midpoint.is_eq() && floor.is_odd()) {
        floor + BigInt::from(1u8)
    } else {
        floor
    };
    BigRational::new(rounded, scale).to_f64().unwrap_or(value)
}

pub fn scan(
    bars: &HashMap<String, Vec<Bar>>,
    names: &HashMap<String, String>,
    min_amount: f64,
    max_pivot_age: usize,
    min_bars: usize,
) -> ScanOutcome {
    let Some(last_date) = bars
        .values()
        .filter_map(|rows| rows.last().map(|bar| bar.dt))
        .max()
    else {
        return ScanOutcome {
            signals: Vec::new(),
            funnel: Funnel::default(),
            lags: Vec::new(),
            last_date: None,
        };
    };
    let mut funnel = Funnel::default();
    let mut lags = Vec::new();
    let mut signals = Vec::new();

    for (code, rows) in bars {
        if rows.len() < min_bars || rows.last().map_or(true, |bar| bar.dt != last_date) {
            continue;
        }
        funnel.bar += 1;
        let last_raw = rows.len() - 1;
        let merged = merge_bars(rows);
        let strokes = build_strokes(&merged);
        let pivots = build_pivots(&strokes);
        if strokes.len() < 8 || pivots.is_empty() {
            continue;
        }
        funnel.pivot += 1;

        let Some(third_buy) = find_third_buy(&strokes, &pivots, true) else {
            continue;
        };
        if third_buy.pullback_idx != strokes.len() - 1 {
            continue;
        }
        funnel.last_stroke += 1;
        if third_buy.leave_high <= third_buy.pivot.zg {
            continue;
        }
        funnel.leave += 1;
        if third_buy.pullback.low <= third_buy.pivot.zg {
            continue;
        }
        funnel.pullback += 1;

        let lag = last_raw - third_buy.pullback.confirm_idx;
        lags.push(lag);
        if lag != 0 {
            continue;
        }
        funnel.confirm += 1;
        if last_raw - third_buy.pivot.start_idx > max_pivot_age {
            continue;
        }
        funnel.age += 1;

        let close = rows[last_raw].c;
        let previous_close = rows[last_raw - 1].c;
        let amount = rows[last_raw].amt / 1e8;
        if amount < min_amount
            || close <= third_buy.pivot.zg
            || rows[last_raw].l < third_buy.pullback.low
        {
            continue;
        }
        funnel.liquid += 1;
        signals.push(Signal {
            code: code.clone(),
            name: names.get(code).cloned().unwrap_or_default(),
            close,
            change_pct: py_round((close / previous_close - 1.0) * 100.0, 2),
            amount: py_round(amount, 2),
            zg: py_round(third_buy.pivot.zg, 2),
            zd: py_round(third_buy.pivot.zd, 2),
            leave_high: py_round(third_buy.leave_high, 2),
            pullback_low: py_round(third_buy.pullback.low, 2),
            distance_zg_pct: py_round(
                (third_buy.pullback.low / third_buy.pivot.zg - 1.0) * 100.0,
                2,
            ),
            pullback_pct: py_round(
                (third_buy.leave_high - third_buy.pullback.low) / third_buy.leave_high * 100.0,
                1,
            ),
            pivot_start: rows[third_buy.pivot.start_idx]
                .dt
                .format("%Y-%m-%d")
                .to_string(),
            confirm_date: rows[third_buy.pullback.confirm_idx]
                .dt
                .format("%Y-%m-%d")
                .to_string(),
            chart: None,
        });
    }
    signals.sort_by(|a, b| b.amount.total_cmp(&a.amount));
    ScanOutcome {
        signals,
        funnel,
        lags,
        last_date: Some(last_date),
    }
}

pub fn chart_data(rows: &[Bar], lookback: usize) -> Option<ChartData> {
    let merged = merge_bars(rows);
    let strokes = build_strokes(&merged);
    let pivots = build_pivots(&strokes);
    let third_buy = find_third_buy(&strokes, &pivots, false)?;
    let offset = rows.len().saturating_sub(lookback);
    let segment = &rows[offset..];
    let dates = segment
        .iter()
        .map(|bar| bar.dt.format("%Y-%m-%d").to_string())
        .collect::<Vec<_>>();
    let date_at = |index: usize| rows[index].dt.format("%Y-%m-%d").to_string();

    let mut stroke_points = Vec::new();
    for stroke in &strokes {
        if stroke.start_idx < offset {
            continue;
        }
        if stroke_points.is_empty() {
            stroke_points.push((date_at(stroke.start_idx), py_round(stroke.start_price, 3)));
        }
        stroke_points.push((date_at(stroke.end_idx), py_round(stroke.end_price, 3)));
    }
    let pivot = third_buy.pivot;
    let pullback = third_buy.pullback;
    Some(ChartData {
        dates,
        candles: segment
            .iter()
            .map(|bar| {
                [
                    py_round(bar.o, 3),
                    py_round(bar.c, 3),
                    py_round(bar.l, 3),
                    py_round(bar.h, 3),
                ]
            })
            .collect(),
        volumes: segment
            .iter()
            .map(|bar| py_round(bar.amt / 1e8, 4))
            .collect(),
        stroke_points,
        pivot: PivotChart {
            start_date: date_at(pivot.start_idx.max(offset)),
            end_date: rows.last()?.dt.format("%Y-%m-%d").to_string(),
            zg: py_round(pivot.zg, 3),
            zd: py_round(pivot.zd, 3),
        },
        buy_point: BuyPoint {
            date: date_at(pullback.end_idx),
            price: py_round(pullback.low, 3),
            confirm_date: date_at(pullback.confirm_idx),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_uses_ties_even() {
        assert_eq!(py_round(1.225, 2), 1.23);
        assert_eq!(py_round(1.235, 2), 1.24);
        assert_eq!(py_round(1.25, 1), 1.2);
        assert_eq!(py_round(2.675, 2), 2.67);
    }
}
