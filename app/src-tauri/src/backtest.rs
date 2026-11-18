use std::{
    collections::{BTreeSet, HashMap},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::{bail, Result};
use chrono::NaiveDate;
use rayon::prelude::*;

use crate::{
    engine::{build_pivots, build_strokes, find_third_buy, merge_bars},
    model::{BacktestOptions, BacktestStats, Bar, EquityPoint, Trade},
    screener::py_round,
};

const MIN_BARS: usize = 130;
const LOT: u64 = 100;
const MIN_COMMISSION: f64 = 5.0;

pub struct BacktestOutcome {
    pub stats: BacktestStats,
    pub equity_curve: Vec<EquityPoint>,
    pub trades: Vec<Trade>,
    pub signal_count: usize,
}

pub(crate) struct EntrySignal {
    idx: usize,
    zg: f64,
    amount: f64,
}

/// 逐日滚动判定三买信号，仅使用截止日及之前的行情，禁止未来函数。
fn rolling_signals(
    rows: &[Bar],
    start: NaiveDate,
    end: NaiveDate,
    min_amount: f64,
    max_pivot_age: usize,
) -> Vec<EntrySignal> {
    let mut out = Vec::new();
    let first = rows.partition_point(|bar| bar.dt < start);
    for t in first..rows.len() {
        if rows[t].dt > end {
            break;
        }
        if t + 1 < MIN_BARS {
            continue;
        }
        let merged = merge_bars(&rows[..=t]);
        let strokes = build_strokes(&merged);
        if strokes.len() < 8 {
            continue;
        }
        let pivots = build_pivots(&strokes);
        let Some(third_buy) = find_third_buy(&strokes, &pivots, true) else {
            continue;
        };
        if third_buy.pullback_idx != strokes.len() - 1
            || third_buy.leave_high <= third_buy.pivot.zg
            || third_buy.pullback.low <= third_buy.pivot.zg
            || t != third_buy.pullback.confirm_idx
            || t - third_buy.pivot.start_idx > max_pivot_age
        {
            continue;
        }
        let amount = rows[t].amt / 1e8;
        if amount < min_amount
            || rows[t].c <= third_buy.pivot.zg
            || rows[t].l < third_buy.pullback.low
        {
            continue;
        }
        out.push(EntrySignal {
            idx: t,
            zg: third_buy.pivot.zg,
            amount,
        });
    }
    out
}

struct Position {
    code: String,
    shares: u64,
    entry_idx: usize,
    entry_date: NaiveDate,
    entry_price: f64,
    entry_cost: f64,
    zg: f64,
    last_price: f64,
    stop_pending: bool,
}

fn sell_fees(proceeds: f64, options: &BacktestOptions) -> f64 {
    (proceeds * options.commission_rate).max(MIN_COMMISSION) + proceeds * options.stamp_duty_rate
}

/// 事件驱动撮合：信号次日开盘买入（T+1），收盘跌破 ZG 次日开盘止损，
/// 涨停无法买入、跌停无法卖出（顺延），期末未平仓按最后收盘结算。
pub(crate) fn simulate(
    bars: &HashMap<String, Vec<Bar>>,
    names: &HashMap<String, String>,
    signals: &HashMap<String, Vec<EntrySignal>>,
    start: NaiveDate,
    end: NaiveDate,
    options: &BacktestOptions,
    cancel: &Arc<AtomicBool>,
) -> Result<BacktestOutcome> {
    let index_maps: HashMap<String, HashMap<NaiveDate, usize>> = bars
        .iter()
        .map(|(code, rows)| {
            (
                code.clone(),
                rows.iter()
                    .enumerate()
                    .map(|(i, bar)| (bar.dt, i))
                    .collect(),
            )
        })
        .collect();
    let calendar: BTreeSet<NaiveDate> = bars
        .values()
        .flat_map(|rows| rows.iter().map(|bar| bar.dt))
        .filter(|dt| *dt >= start && *dt <= end)
        .collect();

    let mut candidates: HashMap<NaiveDate, Vec<(String, f64, f64)>> = HashMap::new();
    let mut signal_count = 0;
    for (code, list) in signals {
        signal_count += list.len();
        let rows = &bars[code];
        for signal in list {
            if signal.idx + 1 >= rows.len() {
                continue;
            }
            let entry_date = rows[signal.idx + 1].dt;
            if entry_date > end {
                continue;
            }
            candidates.entry(entry_date).or_default().push((
                code.clone(),
                signal.zg,
                signal.amount,
            ));
        }
    }
    for list in candidates.values_mut() {
        list.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
    }

    let mut cash = options.initial_capital;
    let mut positions: Vec<Position> = Vec::new();
    let mut trades: Vec<Trade> = Vec::new();
    let mut equity_curve = Vec::with_capacity(calendar.len());
    let mut peak = options.initial_capital;
    let mut max_drawdown = 0.0_f64;

    for &date in &calendar {
        if cancel.load(Ordering::Relaxed) {
            bail!("回测已取消");
        }
        // 1) 开盘止损卖出（T+1：买入当日不可卖）
        let mut i = 0;
        while i < positions.len() {
            let position = &positions[i];
            let mut sold = false;
            if position.stop_pending && position.entry_date < date {
                if let Some(&idx) = index_maps[&position.code].get(&date) {
                    let rows = &bars[&position.code];
                    let open = rows[idx].o;
                    let limit_down = py_round(rows[idx - 1].c * 0.9, 2);
                    if open > limit_down {
                        let exec = open * (1.0 - options.slippage);
                        let proceeds = position.shares as f64 * exec;
                        let net = proceeds - sell_fees(proceeds, options);
                        cash += net;
                        let pnl = net - position.entry_cost;
                        trades.push(Trade {
                            code: position.code.clone(),
                            name: names.get(&position.code).cloned().unwrap_or_default(),
                            entry_date: position.entry_date.format("%Y-%m-%d").to_string(),
                            entry_price: py_round(position.entry_price, 3),
                            exit_date: date.format("%Y-%m-%d").to_string(),
                            exit_price: py_round(exec, 3),
                            shares: position.shares,
                            zg: py_round(position.zg, 3),
                            pnl: py_round(pnl, 2),
                            pnl_pct: py_round(pnl / position.entry_cost * 100.0, 2),
                            hold_days: idx - position.entry_idx,
                            exit_reason: "结构止损".to_string(),
                        });
                        positions.swap_remove(i);
                        sold = true;
                    }
                }
            }
            if !sold {
                i += 1;
            }
        }
        // 2) 开盘买入（前一日确认的信号，涨停不成交）
        if let Some(list) = candidates.get(&date) {
            for (code, zg, _) in list {
                if positions.len() >= options.max_positions {
                    break;
                }
                if positions.iter().any(|p| &p.code == code) {
                    continue;
                }
                let rows = &bars[code];
                let idx = index_maps[code][&date];
                let open = rows[idx].o;
                if open >= py_round(rows[idx - 1].c * 1.1, 2) {
                    continue;
                }
                let exec = open * (1.0 + options.slippage);
                let budget = cash.min(options.initial_capital / options.max_positions as f64);
                let shares =
                    ((budget / (exec * (1.0 + options.commission_rate))) as u64 / LOT) * LOT;
                if shares == 0 {
                    continue;
                }
                let cost = shares as f64 * exec;
                let total = cost + (cost * options.commission_rate).max(MIN_COMMISSION);
                if total > cash {
                    continue;
                }
                cash -= total;
                positions.push(Position {
                    code: code.clone(),
                    shares,
                    entry_idx: idx,
                    entry_date: date,
                    entry_price: exec,
                    entry_cost: total,
                    zg: *zg,
                    last_price: rows[idx].c,
                    stop_pending: false,
                });
            }
        }
        // 3) 收盘估值与止损判定
        let mut equity = cash;
        for position in &mut positions {
            if let Some(&idx) = index_maps[&position.code].get(&date) {
                let close = bars[&position.code][idx].c;
                position.last_price = close;
                if close < position.zg {
                    position.stop_pending = true;
                }
            }
            equity += position.shares as f64 * position.last_price;
        }
        peak = peak.max(equity);
        let drawdown = (peak - equity) / peak * 100.0;
        max_drawdown = max_drawdown.max(drawdown);
        equity_curve.push(EquityPoint {
            date: date.format("%Y-%m-%d").to_string(),
            equity: py_round(equity, 2),
            drawdown: py_round(drawdown, 2),
        });
    }

    // 期末未平仓按最后收盘价结算
    for position in &positions {
        let rows = &bars[&position.code];
        let last_idx = rows.len() - 1;
        let exec = rows[last_idx].c * (1.0 - options.slippage);
        let proceeds = position.shares as f64 * exec;
        let net = proceeds - sell_fees(proceeds, options);
        cash += net;
        let pnl = net - position.entry_cost;
        trades.push(Trade {
            code: position.code.clone(),
            name: names.get(&position.code).cloned().unwrap_or_default(),
            entry_date: position.entry_date.format("%Y-%m-%d").to_string(),
            entry_price: py_round(position.entry_price, 3),
            exit_date: rows[last_idx].dt.format("%Y-%m-%d").to_string(),
            exit_price: py_round(exec, 3),
            shares: position.shares,
            zg: py_round(position.zg, 3),
            pnl: py_round(pnl, 2),
            pnl_pct: py_round(pnl / position.entry_cost * 100.0, 2),
            hold_days: last_idx - position.entry_idx,
            exit_reason: "期末结算".to_string(),
        });
    }
    trades.sort_by(|a, b| a.entry_date.cmp(&b.entry_date).then(a.code.cmp(&b.code)));

    let final_equity = cash;
    let wins = trades.iter().filter(|t| t.pnl > 0.0).count();
    let gross_profit: f64 = trades.iter().filter(|t| t.pnl > 0.0).map(|t| t.pnl).sum();
    let gross_loss: f64 = trades.iter().filter(|t| t.pnl < 0.0).map(|t| -t.pnl).sum();
    let days = equity_curve.len();
    let annualized = if days > 1 && final_equity > 0.0 {
        ((final_equity / options.initial_capital).powf(252.0 / days as f64) - 1.0) * 100.0
    } else {
        0.0
    };
    let stats = BacktestStats {
        total_return_pct: py_round((final_equity / options.initial_capital - 1.0) * 100.0, 2),
        annualized_pct: py_round(annualized, 2),
        max_drawdown_pct: py_round(max_drawdown, 2),
        win_rate_pct: py_round(
            if trades.is_empty() {
                0.0
            } else {
                wins as f64 / trades.len() as f64 * 100.0
            },
            2,
        ),
        profit_factor: if gross_loss > 0.0 {
            Some(py_round(gross_profit / gross_loss, 2))
        } else {
            None
        },
        trade_count: trades.len(),
        avg_hold_days: py_round(
            if trades.is_empty() {
                0.0
            } else {
                trades.iter().map(|t| t.hold_days as f64).sum::<f64>() / trades.len() as f64
            },
            1,
        ),
        final_equity: py_round(final_equity, 2),
    };
    Ok(BacktestOutcome {
        stats,
        equity_curve,
        trades,
        signal_count,
    })
}

pub fn run(
    bars: &HashMap<String, Vec<Bar>>,
    names: &HashMap<String, String>,
    start: NaiveDate,
    end: NaiveDate,
    options: &BacktestOptions,
    cancel: &Arc<AtomicBool>,
) -> Result<BacktestOutcome> {
    let signals: HashMap<String, Vec<EntrySignal>> = bars
        .par_iter()
        .map(|(code, rows)| {
            if cancel.load(Ordering::Relaxed) {
                return (code.clone(), Vec::new());
            }
            (
                code.clone(),
                rolling_signals(rows, start, end, options.min_amount, options.max_pivot_age),
            )
        })
        .collect();
    if cancel.load(Ordering::Relaxed) {
        bail!("回测已取消");
    }
    simulate(bars, names, &signals, start, end, options, cancel)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(dt: NaiveDate, o: f64, c: f64) -> Bar {
        Bar {
            dt,
            o,
            c,
            h: o.max(c),
            l: o.min(c),
            v: 1e6,
            amt: 1e8,
            turn: 1.0,
        }
    }

    fn options() -> BacktestOptions {
        BacktestOptions {
            start: "2026-01-01".into(),
            end: "2026-01-10".into(),
            initial_capital: 100_000.0,
            max_positions: 1,
            min_amount: 0.5,
            max_pivot_age: 90,
            commission_rate: 0.00025,
            stamp_duty_rate: 0.0005,
            slippage: 0.001,
            workers: 1,
            source: crate::model::DataSource::Sina,
        }
    }

    fn setup(
        rows: Vec<Bar>,
        signal_idx: usize,
        zg: f64,
    ) -> (HashMap<String, Vec<Bar>>, HashMap<String, Vec<EntrySignal>>) {
        let mut bars = HashMap::new();
        bars.insert("600000".to_string(), rows);
        let mut signals = HashMap::new();
        signals.insert(
            "600000".to_string(),
            vec![EntrySignal {
                idx: signal_idx,
                zg,
                amount: 1.0,
            }],
        );
        (bars, signals)
    }

    #[test]
    fn stop_exits_next_open_under_t1() {
        let d = |i: i64| NaiveDate::from_ymd_opt(2026, 1, 1).unwrap() + chrono::Duration::days(i);
        // day0 确认信号，day1 开盘买入，day2 收盘跌破 ZG，day3 开盘止损
        let rows = vec![
            bar(d(0), 10.0, 10.0),
            bar(d(1), 10.0, 10.0),
            bar(d(2), 9.0, 8.5),
            bar(d(3), 8.6, 8.8),
        ];
        let (bars, signals) = setup(rows, 0, 9.0);
        let outcome = simulate(
            &bars,
            &HashMap::new(),
            &signals,
            d(0),
            d(3),
            &options(),
            &Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        assert_eq!(outcome.trades.len(), 1);
        let trade = &outcome.trades[0];
        assert_eq!(trade.entry_date, "2026-01-02");
        assert_eq!(trade.exit_date, "2026-01-04");
        assert_eq!(trade.exit_reason, "结构止损");
        assert_eq!(trade.hold_days, 2);
        assert!(trade.pnl < 0.0);
        assert_eq!(trade.shares % 100, 0);
    }

    #[test]
    fn limit_up_open_blocks_entry() {
        let d = |i: i64| NaiveDate::from_ymd_opt(2026, 1, 1).unwrap() + chrono::Duration::days(i);
        let rows = vec![
            bar(d(0), 10.0, 10.0),
            bar(d(1), 11.0, 11.0), // 涨停开盘，无法买入
            bar(d(2), 11.0, 11.0),
        ];
        let (bars, signals) = setup(rows, 0, 9.0);
        let outcome = simulate(
            &bars,
            &HashMap::new(),
            &signals,
            d(0),
            d(2),
            &options(),
            &Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        assert!(outcome.trades.is_empty());
        assert_eq!(outcome.stats.final_equity, 100_000.0);
    }

    #[test]
    fn limit_down_open_defers_stop_sell() {
        let d = |i: i64| NaiveDate::from_ymd_opt(2026, 1, 1).unwrap() + chrono::Duration::days(i);
        let rows = vec![
            bar(d(0), 10.0, 10.0),
            bar(d(1), 10.0, 10.0),
            bar(d(2), 9.0, 8.5),   // 收盘破 ZG
            bar(d(3), 7.65, 7.65), // 跌停开盘，卖不出
            bar(d(4), 7.8, 8.0),   // 次日开盘成交
        ];
        let (bars, signals) = setup(rows, 0, 9.0);
        let outcome = simulate(
            &bars,
            &HashMap::new(),
            &signals,
            d(0),
            d(4),
            &options(),
            &Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        assert_eq!(outcome.trades.len(), 1);
        assert_eq!(outcome.trades[0].exit_date, "2026-01-05");
    }
}
