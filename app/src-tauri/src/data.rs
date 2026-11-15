use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::NaiveDate;
use rayon::{prelude::*, ThreadPoolBuilder};
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::model::{Bar, DataSource, StockInfo};

const MAIN_BOARD_PREFIXES: [&str; 8] = ["600", "601", "603", "605", "000", "001", "002", "003"];

pub fn http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/131 Safari/537.36")
        .build()
        .context("创建行情客户端失败")
}

pub fn main_board_universe(client: &Client) -> Result<Vec<StockInfo>> {
    if let Ok(path) = env::var("CHANQUANT_UNIVERSE") {
        return read_universe(Path::new(&path));
    }

    #[derive(Deserialize)]
    struct Response {
        data: Option<Data>,
    }
    #[derive(Deserialize)]
    struct Data {
        total: usize,
        diff: Vec<Stock>,
    }
    #[derive(Deserialize)]
    struct Stock {
        f12: String,
        f14: String,
    }

    let mut page = 1;
    let mut all_stocks = Vec::new();
    loop {
        let mut attempt = 0;
        let response: Response = loop {
            let result = client
                .get("https://push2.eastmoney.com/api/qt/clist/get")
                .query(&[
                    ("pn", page.to_string()),
                    ("pz", "100".to_string()),
                    ("po", "1".to_string()),
                    ("np", "1".to_string()),
                    ("fltt", "2".to_string()),
                    ("invt", "2".to_string()),
                    ("fid", "f12".to_string()),
                    ("fs", "m:0+t:6,m:0+t:80,m:1+t:2,m:1+t:23".to_string()),
                    ("fields", "f12,f14".to_string()),
                ])
                .send()
                .and_then(|response| response.error_for_status())
                .map_err(anyhow::Error::from)
                .and_then(|response| response.json().map_err(anyhow::Error::from));
            match result {
                Ok(response) => break response,
                Err(_) if attempt < 2 => {
                    attempt += 1;
                    thread::sleep(Duration::from_millis(800));
                }
                Err(error) => return Err(error).context("获取沪深股票池失败"),
            }
        };
        let data = response.data.ok_or_else(|| anyhow!("股票池接口无数据"))?;
        let total = data.total;
        let count = data.diff.len();
        all_stocks.extend(data.diff);
        if count == 0 || all_stocks.len() >= total {
            break;
        }
        page += 1;
    }

    let stocks = all_stocks
        .into_iter()
        .filter(|stock| {
            MAIN_BOARD_PREFIXES
                .iter()
                .any(|prefix| stock.f12.starts_with(prefix))
                && !stock.f14.contains("ST")
                && !stock.f14.contains('退')
        })
        .map(|stock| StockInfo {
            code: stock.f12,
            name: stock.f14,
        })
        .collect::<Vec<_>>();

    if stocks.is_empty() {
        bail!("沪深主板股票池为空")
    }
    Ok(stocks)
}

fn read_universe(path: &Path) -> Result<Vec<StockInfo>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("读取股票池基准失败：{}", path.display()))?;
    serde_json::from_str(&text).context("解析股票池基准失败")
}

pub fn cache_dir() -> PathBuf {
    env::var_os("CHANQUANT_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".cache/chanquant/daily")
        })
}

/// 清空行情缓存目录，返回删除的文件数与释放的字节数。
pub fn clear_cache() -> Result<crate::model::CacheCleanup> {
    let dir = cache_dir();
    let mut file_count = 0usize;
    let mut bytes = 0u64;
    if !dir.exists() {
        return Ok(crate::model::CacheCleanup { file_count, bytes });
    }
    for entry in
        fs::read_dir(&dir).with_context(|| format!("读取缓存目录失败：{}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("遍历缓存目录失败：{}", dir.display()))?;
        let metadata = entry
            .metadata()
            .with_context(|| format!("读取缓存文件信息失败：{}", entry.path().display()))?;
        if !metadata.is_file() {
            continue;
        }
        bytes += metadata.len();
        fs::remove_file(entry.path())
            .with_context(|| format!("删除缓存文件失败：{}", entry.path().display()))?;
        file_count += 1;
    }
    Ok(crate::model::CacheCleanup { file_count, bytes })
}

pub fn load_universe(
    client: &Client,
    stocks: &[StockInfo],
    start: NaiveDate,
    end: NaiveDate,
    workers: usize,
    source: DataSource,
    cancel: &Arc<AtomicBool>,
) -> Result<HashMap<String, Vec<Bar>>> {
    let pool = ThreadPoolBuilder::new()
        .num_threads(workers.max(1))
        .build()
        .context("创建行情线程池失败")?;
    let cache = cache_dir();
    let results = pool.install(|| {
        stocks
            .par_iter()
            .map(|stock| {
                if cancel.load(Ordering::Relaxed) {
                    return None;
                }
                fetch_daily(client, &stock.code, start, end, source, &cache, cancel)
                    .ok()
                    .flatten()
                    .filter(|bars| bars.len() >= 130)
                    .map(|bars| (stock.code.clone(), bars))
            })
            .collect::<Vec<_>>()
    });
    if cancel.load(Ordering::Relaxed) {
        bail!("扫描已取消")
    }
    Ok(results.into_iter().flatten().collect())
}

fn fetch_daily(
    client: &Client,
    code: &str,
    start: NaiveDate,
    end: NaiveDate,
    source: DataSource,
    cache_dir: &Path,
    cancel: &Arc<AtomicBool>,
) -> Result<Option<Vec<Bar>>> {
    if let Some(cached) = load_cached(code, start, end, source, cache_dir)? {
        return Ok(Some(cached));
    }

    let mut last_error = None;
    for attempt in 0..3 {
        if cancel.load(Ordering::Relaxed) {
            bail!("扫描已取消")
        }
        let fetched = match source {
            DataSource::Eastmoney => fetch_eastmoney(client, code, start, end, "1"),
            DataSource::Sina => fetch_sina(client, code, start, end),
        };
        match fetched {
            Ok(bars) if bars.is_empty() => return Ok(None),
            Ok(bars) => {
                save_cached(code, start, end, source, cache_dir, &bars)?;
                return Ok(Some(bars));
            }
            Err(error) => {
                last_error = Some(error);
                if attempt < 2 {
                    thread::sleep(Duration::from_millis(800));
                }
            }
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("获取 {code} 日线失败")))
}

fn source_prefix(source: DataSource) -> &'static str {
    match source {
        DataSource::Eastmoney => "",
        DataSource::Sina => "sina_",
    }
}

fn compact(date: NaiveDate) -> String {
    date.format("%Y%m%d").to_string()
}

fn cache_path(
    cache_dir: &Path,
    code: &str,
    start: NaiveDate,
    end: NaiveDate,
    source: DataSource,
) -> PathBuf {
    cache_dir.join(format!(
        "{code}_{}qfq_{}_{}.json",
        source_prefix(source),
        compact(start),
        compact(end)
    ))
}

fn load_cached(
    code: &str,
    start: NaiveDate,
    end: NaiveDate,
    source: DataSource,
    cache_dir: &Path,
) -> Result<Option<Vec<Bar>>> {
    let exact = cache_path(cache_dir, code, start, end, source);
    if exact.exists() {
        return read_bars(&exact).map(Some);
    }
    let prefix = format!("{code}_{}qfq_", source_prefix(source));
    let mut candidates = fs::read_dir(cache_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "json")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(&prefix))
        })
        .collect::<Vec<_>>();
    candidates.sort();
    for path in candidates {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let mut parts = stem.rsplitn(3, '_');
        let cached_end = parts
            .next()
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y%m%d").ok());
        let cached_start = parts
            .next()
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y%m%d").ok());
        if matches!((cached_start, cached_end), (Some(s), Some(e)) if s <= start && e >= end) {
            let bars = read_bars(&path)?
                .into_iter()
                .filter(|bar| bar.dt >= start && bar.dt <= end)
                .collect();
            return Ok(Some(bars));
        }
    }
    Ok(None)
}

fn read_bars(path: &Path) -> Result<Vec<Bar>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("读取行情缓存失败：{}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("解析行情缓存失败：{}", path.display()))
}

fn save_cached(
    code: &str,
    start: NaiveDate,
    end: NaiveDate,
    source: DataSource,
    cache_dir: &Path,
    bars: &[Bar],
) -> Result<()> {
    fs::create_dir_all(cache_dir).context("创建行情缓存目录失败")?;
    let path = cache_path(cache_dir, code, start, end, source);
    let tmp = path.with_extension("tmp");
    let data = serde_json::to_vec(bars).context("序列化行情缓存失败")?;
    fs::write(&tmp, data).context("写入行情缓存失败")?;
    fs::rename(&tmp, &path).context("提交行情缓存失败")?;
    Ok(())
}

#[derive(Deserialize)]
struct EastmoneyResponse {
    data: Option<EastmoneyData>,
}
#[derive(Deserialize)]
struct EastmoneyData {
    klines: Vec<String>,
}

fn fetch_eastmoney(
    client: &Client,
    code: &str,
    start: NaiveDate,
    end: NaiveDate,
    adjust: &str,
) -> Result<Vec<Bar>> {
    let market = if code.starts_with('6') { "1" } else { "0" };
    let response: EastmoneyResponse = client
        .get("https://push2his.eastmoney.com/api/qt/stock/kline/get")
        .query(&[
            ("fields1", "f1,f2,f3,f4,f5,f6"),
            (
                "fields2",
                "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f116",
            ),
            ("ut", "7eea3edcaed734bea9cbfc24409ed989"),
            ("klt", "101"),
            ("fqt", adjust),
            ("secid", &format!("{market}.{code}")),
            ("beg", &compact(start)),
            ("end", &compact(end)),
        ])
        .send()
        .with_context(|| format!("请求 {code} 东方财富日线失败"))?
        .error_for_status()?
        .json()
        .with_context(|| format!("解析 {code} 东方财富日线失败"))?;

    let Some(data) = response.data else {
        return Ok(Vec::new());
    };
    data.klines
        .iter()
        .map(|line| {
            let fields = line.split(',').collect::<Vec<_>>();
            if fields.len() < 11 {
                bail!("{code} 日线字段不足")
            }
            Ok(Bar {
                dt: NaiveDate::parse_from_str(fields[0], "%Y-%m-%d")?,
                o: fields[1].parse()?,
                c: fields[2].parse()?,
                h: fields[3].parse()?,
                l: fields[4].parse()?,
                v: fields[5].parse()?,
                amt: fields[6].parse()?,
                turn: fields[10].parse()?,
            })
        })
        .collect()
}

#[derive(Deserialize)]
struct SinaBar {
    day: String,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
}
#[derive(Deserialize)]
struct FactorFile {
    data: Vec<Factor>,
}
#[derive(Deserialize)]
struct Factor {
    d: String,
    f: String,
}

fn fetch_sina(client: &Client, code: &str, start: NaiveDate, end: NaiveDate) -> Result<Vec<Bar>> {
    let symbol = format!("{}{code}", if code.starts_with('6') { "sh" } else { "sz" });
    let raw = client
        .get("https://quotes.sina.cn/cn/api/json_v2.php/CN_MarketDataService.getKLineData")
        .query(&[
            ("symbol", symbol.as_str()),
            ("scale", "240"),
            ("ma", "no"),
            ("datalen", "1023"),
        ])
        .send()
        .with_context(|| format!("请求 {code} 新浪日线失败"))?
        .error_for_status()?
        .json::<Vec<SinaBar>>()
        .with_context(|| format!("解析 {code} 新浪日线失败"))?;
    if raw.is_empty() {
        return Ok(Vec::new());
    }

    let factor_text = client
        .get(format!(
            "https://finance.sina.com.cn/realstock/company/{symbol}/qfq.js"
        ))
        .send()
        .with_context(|| format!("请求 {code} 复权因子失败"))?
        .error_for_status()?
        .text()?;
    let json = factor_text
        .split_once('=')
        .map(|(_, value)| value)
        .and_then(|value| value.split("/*").next())
        .map(str::trim)
        .ok_or_else(|| anyhow!("{code} 复权因子格式异常"))?;
    let mut factors = serde_json::from_str::<FactorFile>(json)?
        .data
        .into_iter()
        .map(|factor| {
            Ok((
                NaiveDate::parse_from_str(&factor.d, "%Y-%m-%d")?,
                factor.f.parse::<f64>()?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    factors.sort_by_key(|factor| factor.0);

    // 新浪此接口不返回成交额；成交额取自同一交易日的东方财富原始日线，价格仍使用新浪前复权数据。
    let amounts = fetch_eastmoney(client, code, start, end, "0")?
        .into_iter()
        .map(|bar| (bar.dt, (bar.amt, bar.turn)))
        .collect::<HashMap<_, _>>();

    raw.into_iter()
        .filter_map(|row| {
            let dt = NaiveDate::parse_from_str(&row.day, "%Y-%m-%d").ok()?;
            if dt < start || dt > end {
                return None;
            }
            let factor = factors
                .partition_point(|(date, _)| *date <= dt)
                .checked_sub(1)
                .map(|index| factors[index].1)?;
            let (amt, turn) = amounts.get(&dt).copied().unwrap_or((0.0, 0.0));
            Some(Ok(Bar {
                dt,
                o: py_round(row.open.parse::<f64>().unwrap_or_default() / factor, 2),
                c: py_round(row.close.parse::<f64>().unwrap_or_default() / factor, 2),
                h: py_round(row.high.parse::<f64>().unwrap_or_default() / factor, 2),
                l: py_round(row.low.parse::<f64>().unwrap_or_default() / factor, 2),
                v: row.volume.parse().unwrap_or_default(),
                amt,
                turn,
            }))
        })
        .collect()
}

fn py_round(value: f64, digits: i32) -> f64 {
    let factor = 10_f64.powi(digits);
    (value * factor).round_ties_even() / factor
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// clear_cache 涉及进程级环境变量，串行化避免与其他用例竞争。
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn clear_cache_removes_files_and_reports_usage() {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous = env::var_os("CHANQUANT_CACHE");
        let staging = env::temp_dir().join(format!("chanquant-clear-test-{}", std::process::id()));
        fs::create_dir_all(&staging).unwrap();
        let first = staging.join("000001_sina_qfq_20240101_20261111.json");
        let second = staging.join("600863.pkl");
        fs::write(&first, "x".repeat(100)).unwrap();
        fs::write(&second, "y".repeat(50)).unwrap();
        env::set_var("CHANQUANT_CACHE", &staging);

        let cleanup = clear_cache().unwrap();

        assert_eq!(cleanup.file_count, 2);
        assert_eq!(cleanup.bytes, 150);
        assert!(!first.exists());
        assert!(!second.exists());
        assert!(staging.exists());

        let again = clear_cache().unwrap();
        assert_eq!(again.file_count, 0);
        assert_eq!(again.bytes, 0);

        fs::remove_dir_all(&staging).unwrap();
        match previous {
            Some(value) => env::set_var("CHANQUANT_CACHE", value),
            None => env::remove_var("CHANQUANT_CACHE"),
        }
    }
}
