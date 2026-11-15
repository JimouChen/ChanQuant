# ChanQuant

Tauri + Vue + Rust 实现的 A 股缠论日线三买分析客户端。行情处理、包含关系、分型、笔、中枢、三买扫描和图表数据均由本地 Rust 引擎完成，运行时不需要 Python。

## 三买判定

- 沪深主板，剔除 ST / 退市标的
- 新浪或东方财富前复权日线，收盘结构确认，无未来函数
- 向上离开中枢，回抽一笔不进入 `ZG`
- 回抽端点底分型须在截止日确认
- 中枢起始日至截止日不超过 90 根 K 线
- 当日成交额不低于 0.5 亿元

## 桌面端

```bash
cd app
npm install
npm run tauri dev
```

生产构建：

```bash
cd app
npm run build
npm run tauri build
```

扫描结果展示信号、漏斗、结构图及本次扫描总耗时（行情加载 + 结构计算）。扫描期间可以取消，并保留上一次结果。

## Rust CLI

```bash
cargo run --manifest-path app/src-tauri/Cargo.toml --bin chanquant-scan -- \
  --date 2026-11-11 --source sina --min-amount 0.5 --workers 16
```

同时导出结构图 SVG：

```bash
cargo run --manifest-path app/src-tauri/Cargo.toml --bin chanquant-scan -- \
  --date 2026-11-11 --source sina --plot
```

参数：

| 参数 | 默认值 | 说明 |
|---|---|---|
| `--date` | 本机当前日期 | 截止日期，支持 `YYYY-MM-DD` / `YYYYMMDD` |
| `--start` | 截止日前 700 天 | 取数起始日 |
| `--min-amount` | `0.5` | 当日最低成交额（亿元） |
| `--max-pivot-age` | `90` | 中枢起始日至截止日的最大 K 线数 |
| `--workers` | `16` | 行情并发线程数 |
| `--source` | `sina` | `sina` 或 `eastmoney` |
| `--plot` | 关闭 | 导出信号结构图 SVG |
| `--result-dir` | `result` | 结构图输出目录 |

## 缓存

行情缓存位于 `~/.cache/chanquant/daily`，Rust 使用 JSON 文件，与同目录旧 Python `.pkl` 缓存隔离。可通过 `CHANQUANT_CACHE` 覆盖缓存目录；`CHANQUANT_UNIVERSE` 可指定离线股票池 JSON，用于固定基准验证。

## 结构

```
app/
├── src-tauri/
│   ├── src/data.rs       # 股票池、行情、复权、缓存、并发加载
│   ├── src/engine.rs     # 包含、分型、笔、中枢、三买
│   ├── src/screener.rs   # 扫描漏斗、信号与图表数据
│   ├── src/lib.rs        # Tauri 命令、取消与耗时统计
│   └── src/bin/          # Rust CLI 与 SVG 导出
└── src/renderer/
    └── src/              # Vue 组件与 ECharts 交互式结构图
```
