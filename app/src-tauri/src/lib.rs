pub mod data;
pub mod engine;
pub mod model;
pub mod screener;

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};

use anyhow::{bail, Context, Result};
use chrono::{Duration, Local, NaiveDate};
pub use model::{DataSource, ScanOptions, ScanResult};
use tauri::State;

#[derive(Default)]
struct ActiveScan {
    next_id: u64,
    current: Option<(u64, Arc<AtomicBool>)>,
}

#[derive(Default)]
struct ScanState(Mutex<ActiveScan>);

fn parse_date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(value, "%Y%m%d"))
        .with_context(|| format!("日期格式无效：{value}"))
}

pub fn execute_scan(options: ScanOptions, cancel: &Arc<AtomicBool>) -> Result<ScanResult> {
    let started = Instant::now();
    if !(0.0..=100.0).contains(&options.min_amount) {
        bail!("最低成交额必须在 0 到 100 亿元之间")
    }
    if !(10..=250).contains(&options.max_pivot_age) {
        bail!("中枢最大年龄必须在 10 到 250 根 K 线之间")
    }
    let end = parse_date(&options.date)?;
    let start = options
        .start
        .as_deref()
        .map(parse_date)
        .transpose()?
        .unwrap_or(end - Duration::days(700));
    if start > end {
        bail!("取数起始日不能晚于截止日")
    }

    let client = data::http_client()?;
    let universe = data::main_board_universe(&client)?;
    let names = universe
        .iter()
        .map(|stock| (stock.code.clone(), stock.name.clone()))
        .collect::<HashMap<_, _>>();
    let bars = data::load_universe(
        &client,
        &universe,
        start,
        end,
        options.workers.clamp(1, 64),
        options.source,
        cancel,
    )?;
    let mut outcome = screener::scan(
        &bars,
        &names,
        options.min_amount,
        options.max_pivot_age,
        130,
    );
    for signal in &mut outcome.signals {
        signal.chart = bars
            .get(&signal.code)
            .and_then(|rows| screener::chart_data(rows, 160));
    }
    let trade_date = outcome.last_date.context("无有效行情数据")?;
    Ok(ScanResult {
        trade_date: trade_date.format("%Y-%m-%d").to_string(),
        source: options.source,
        universe_size: universe.len(),
        loaded_size: bars.len(),
        funnel: outcome.funnel,
        signals: outcome.signals,
        generated_at: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
async fn scan_market(
    options: ScanOptions,
    state: State<'_, ScanState>,
) -> Result<ScanResult, String> {
    let (id, cancel) = {
        let mut active = state.0.lock().map_err(|_| "扫描状态不可用".to_string())?;
        if active.current.is_some() {
            return Err("已有扫描任务正在运行".to_string());
        }
        active.next_id += 1;
        let id = active.next_id;
        let cancel = Arc::new(AtomicBool::new(false));
        active.current = Some((id, cancel.clone()));
        (id, cancel)
    };

    let task_cancel = cancel.clone();
    let result = tauri::async_runtime::spawn_blocking(move || execute_scan(options, &task_cancel))
        .await
        .map_err(|error| format!("扫描任务异常：{error}"))?
        .map_err(|error| error.to_string());
    if let Ok(mut active) = state.0.lock() {
        if active
            .current
            .as_ref()
            .is_some_and(|(current_id, _)| *current_id == id)
        {
            active.current = None;
        }
    }
    result
}

#[tauri::command]
fn cancel_scan(state: State<'_, ScanState>) -> bool {
    state
        .0
        .lock()
        .ok()
        .and_then(|active| {
            active
                .current
                .as_ref()
                .map(|(_, cancel)| cancel.store(true, Ordering::Relaxed))
        })
        .is_some()
}

#[tauri::command]
fn clear_cache() -> Result<model::CacheCleanup, String> {
    data::clear_cache().map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ScanState::default())
        .invoke_handler(tauri::generate_handler![
            scan_market,
            cancel_scan,
            clear_cache
        ])
        .run(tauri::generate_context!())
        .expect("运行 ChanQuant Tauri 应用失败");
}
