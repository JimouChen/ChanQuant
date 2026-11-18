use std::sync::{atomic::AtomicBool, Arc};

use anyhow::Result;
use chanquant_lib::{execute_backtest, BacktestOptions, DataSource};
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "chanquant-backtest",
    about = "缠论日线三买历史回测（结构止损）"
)]
struct Args {
    #[arg(long, default_value_t = (chrono::Local::now() - chrono::Duration::days(365)).format("%Y-%m-%d").to_string())]
    start: String,
    #[arg(long, default_value_t = chrono::Local::now().format("%Y-%m-%d").to_string())]
    end: String,
    #[arg(long, default_value_t = 1_000_000.0)]
    capital: f64,
    #[arg(long, default_value_t = 5)]
    max_positions: usize,
    #[arg(long, default_value_t = 0.5)]
    min_amount: f64,
    #[arg(long, default_value_t = 90)]
    max_pivot_age: usize,
    #[arg(long, default_value_t = 0.00025)]
    commission: f64,
    #[arg(long, default_value_t = 0.0005)]
    stamp_duty: f64,
    #[arg(long, default_value_t = 0.001)]
    slippage: f64,
    #[arg(long, default_value_t = 16)]
    workers: usize,
    #[arg(long, default_value = "sina")]
    source: DataSource,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let result = execute_backtest(
        BacktestOptions {
            start: args.start,
            end: args.end,
            initial_capital: args.capital,
            max_positions: args.max_positions,
            min_amount: args.min_amount,
            max_pivot_age: args.max_pivot_age,
            commission_rate: args.commission,
            stamp_duty_rate: args.stamp_duty,
            slippage: args.slippage,
            workers: args.workers,
            source: args.source,
        },
        &Arc::new(AtomicBool::new(false)),
    )?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
