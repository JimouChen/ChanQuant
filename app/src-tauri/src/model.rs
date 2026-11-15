use std::{fmt, str::FromStr};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bar {
    pub dt: NaiveDate,
    pub o: f64,
    pub c: f64,
    pub h: f64,
    pub l: f64,
    pub v: f64,
    pub amt: f64,
    pub turn: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInfo {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataSource {
    Sina,
    Eastmoney,
}

impl fmt::Display for DataSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Sina => "sina",
            Self::Eastmoney => "eastmoney",
        })
    }
}

impl FromStr for DataSource {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "sina" => Ok(Self::Sina),
            "eastmoney" => Ok(Self::Eastmoney),
            _ => Err(format!("不支持的行情源：{value}")),
        }
    }
}

fn default_min_amount() -> f64 {
    0.5
}
fn default_max_pivot_age() -> usize {
    90
}
fn default_workers() -> usize {
    16
}
fn default_source() -> DataSource {
    DataSource::Sina
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOptions {
    pub date: String,
    pub start: Option<String>,
    #[serde(default = "default_min_amount")]
    pub min_amount: f64,
    #[serde(default = "default_max_pivot_age")]
    pub max_pivot_age: usize,
    #[serde(default = "default_workers")]
    pub workers: usize,
    #[serde(default = "default_source")]
    pub source: DataSource,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Funnel {
    pub bar: usize,
    pub pivot: usize,
    pub last_stroke: usize,
    pub leave: usize,
    pub pullback: usize,
    pub confirm: usize,
    pub age: usize,
    pub liquid: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PivotChart {
    pub start_date: String,
    pub end_date: String,
    pub zg: f64,
    pub zd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BuyPoint {
    pub date: String,
    pub price: f64,
    pub confirm_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChartData {
    pub dates: Vec<String>,
    pub candles: Vec<[f64; 4]>,
    pub volumes: Vec<f64>,
    pub stroke_points: Vec<(String, f64)>,
    pub pivot: PivotChart,
    pub buy_point: BuyPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Signal {
    pub code: String,
    pub name: String,
    pub close: f64,
    pub change_pct: f64,
    pub amount: f64,
    pub zg: f64,
    pub zd: f64,
    pub leave_high: f64,
    pub pullback_low: f64,
    pub distance_zg_pct: f64,
    pub pullback_pct: f64,
    pub pivot_start: String,
    pub confirm_date: String,
    pub chart: Option<ChartData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub trade_date: String,
    pub source: DataSource,
    pub universe_size: usize,
    pub loaded_size: usize,
    pub funnel: Funnel,
    pub signals: Vec<Signal>,
    pub generated_at: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CacheCleanup {
    pub file_count: usize,
    pub bytes: u64,
}
