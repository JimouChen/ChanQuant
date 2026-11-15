use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

use anyhow::{Context, Result};
use chanquant_lib::model::ChartData;
use chanquant_lib::{execute_scan, DataSource, ScanOptions};
use clap::Parser;

#[derive(Parser)]
#[command(name = "chanquant-scan", about = "缠论日线三买扫描（沪深主板）")]
struct Args {
    #[arg(long, default_value_t = chrono::Local::now().format("%Y-%m-%d").to_string())]
    date: String,
    #[arg(long)]
    start: Option<String>,
    #[arg(long, default_value_t = 0.5)]
    min_amount: f64,
    #[arg(long, default_value_t = 90)]
    max_pivot_age: usize,
    #[arg(long, default_value_t = 16)]
    workers: usize,
    #[arg(long, default_value = "sina")]
    source: DataSource,
    #[arg(long)]
    plot: bool,
    #[arg(long, default_value = "result")]
    result_dir: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let result = execute_scan(
        ScanOptions {
            date: args.date,
            start: args.start,
            min_amount: args.min_amount,
            max_pivot_age: args.max_pivot_age,
            workers: args.workers,
            source: args.source,
        },
        &Arc::new(AtomicBool::new(false)),
    )?;
    if args.plot {
        let directory = Path::new(&args.result_dir).join(&result.trade_date);
        fs::create_dir_all(&directory).context("创建结构图目录失败")?;
        for signal in &result.signals {
            if let Some(chart) = &signal.chart {
                let path = directory.join(format!("{}.svg", signal.code));
                fs::write(&path, structure_svg(&signal.name, chart))
                    .with_context(|| format!("导出结构图失败：{}", path.display()))?;
                eprintln!("结构图：{}", path.display());
            }
        }
    }
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

fn structure_svg(name: &str, chart: &ChartData) -> String {
    const WIDTH: f64 = 1200.0;
    const LEFT: f64 = 70.0;
    const RIGHT: f64 = 30.0;
    const TOP: f64 = 70.0;
    const PRICE_BOTTOM: f64 = 480.0;
    const VOLUME_TOP: f64 = 530.0;
    const VOLUME_BOTTOM: f64 = 660.0;
    let count = chart.dates.len().max(1);
    let step = (WIDTH - LEFT - RIGHT) / count as f64;
    let low = chart
        .candles
        .iter()
        .map(|c| c[2])
        .fold(f64::INFINITY, f64::min)
        .min(chart.pivot.zd);
    let high = chart
        .candles
        .iter()
        .map(|c| c[3])
        .fold(f64::NEG_INFINITY, f64::max)
        .max(chart.pivot.zg);
    let range = (high - low).max(0.01);
    let x = |date: &str, positions: &HashMap<&str, usize>| {
        LEFT + positions.get(date).copied().unwrap_or(0) as f64 * step + step / 2.0
    };
    let y = |price: f64| TOP + (high - price) / range * (PRICE_BOTTOM - TOP);
    let positions = chart
        .dates
        .iter()
        .enumerate()
        .map(|(i, date)| (date.as_str(), i))
        .collect::<HashMap<_, _>>();
    let max_volume = chart
        .volumes
        .iter()
        .copied()
        .fold(0.0_f64, f64::max)
        .max(0.0001);
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="720" viewBox="0 0 1200 720"><rect width="1200" height="720" fill="#fbfcfa"/><text x="70" y="38" font-family="sans-serif" font-size="20" font-weight="600" fill="#17231e">{} · 日线三买结构</text>"##,
        escape_xml(name)
    );
    let pivot_x = x(&chart.pivot.start_date, &positions) - step / 2.0;
    let pivot_end = x(&chart.pivot.end_date, &positions) + step / 2.0;
    svg.push_str(&format!(r##"<rect x="{pivot_x:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="#dcae48" fill-opacity=".14" stroke="#c4952d" stroke-dasharray="5 4"/>"##, y(chart.pivot.zg), (pivot_end - pivot_x).max(0.0), (y(chart.pivot.zd) - y(chart.pivot.zg)).abs()));
    for (index, candle) in chart.candles.iter().enumerate() {
        let date = &chart.dates[index];
        let cx = x(date, &positions);
        let rising = candle[1] >= candle[0];
        let color = if rising { "#d9554d" } else { "#1c9873" };
        let body_top = y(candle[0].max(candle[1]));
        let body_height = (y(candle[0]) - y(candle[1])).abs().max(1.0);
        svg.push_str(&format!(r##"<g stroke="{color}" fill="{color}"><line x1="{cx:.2}" y1="{:.2}" x2="{cx:.2}" y2="{:.2}"/><rect x="{:.2}" y="{body_top:.2}" width="{:.2}" height="{body_height:.2}"/></g>"##, y(candle[3]), y(candle[2]), cx - (step * 0.55).max(1.0) / 2.0, (step * 0.55).max(1.0)));
        let volume_height = chart.volumes[index] / max_volume * (VOLUME_BOTTOM - VOLUME_TOP);
        svg.push_str(&format!(r##"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{volume_height:.2}" fill="{color}" opacity=".6"/>"##, cx - (step * 0.55).max(1.0) / 2.0, VOLUME_BOTTOM - volume_height, (step * 0.55).max(1.0)));
    }
    let points = chart
        .stroke_points
        .iter()
        .map(|(date, price)| format!("{:.2},{:.2}", x(date, &positions), y(*price)))
        .collect::<Vec<_>>()
        .join(" ");
    svg.push_str(&format!(
        r##"<polyline points="{points}" fill="none" stroke="#537aa6" stroke-width="2"/>"##
    ));
    let buy_x = x(&chart.buy_point.date, &positions);
    let buy_y = y(chart.buy_point.price);
    svg.push_str(&format!(r##"<path d="M {buy_x:.2} {buy_y:.2} l -8 14 h 16 z" fill="#d9554d"/><text x="{:.2}" y="{:.2}" font-family="sans-serif" font-size="13" fill="#d9554d">三买 {:.2}</text>"##, buy_x + 10.0, buy_y + 14.0, chart.buy_point.price));
    svg.push_str(r##"<text x="70" y="700" font-family="sans-serif" font-size="12" fill="#66716b">蓝线为笔 · 黄色区间为中枢 · 红色标记为三买回抽低点</text></svg>"##);
    svg
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
