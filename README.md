# ChanQuant

A 股缠论日线三买扫描客户端，Tauri 2 + Vue 3 + Rust 实现。

行情加载、K 线包含处理、分型、笔、中枢、三买判定、图表数据全部由本地 Rust 引擎完成，运行时无需 Python。桌面端安装包约 5 MB（上一代 Electron 版为 347 MB）。

[![Release](https://img.shields.io/badge/release-v0.3.0-blue)](https://github.com/JimouChen/ChanQuant/releases/tag/v0.3.0) ![Rust](https://img.shields.io/badge/Rust-1.77%2B-000000?logo=rust&logoColor=white) ![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white) ![Vue](https://img.shields.io/badge/Vue-3-4FC08D?logo=vuedotjs&logoColor=white) ![Vite](https://img.shields.io/badge/Vite-6-646CFF?logo=vite&logoColor=white) ![platform](https://img.shields.io/badge/platform-macOS%20Apple%20Silicon-lightgrey) ![license](https://img.shields.io/badge/license-MIT-green)

## 功能特性

- **全市场扫描**：沪深主板（剔除 ST / 退市），约 3000+ 标的，典型耗时 2 秒内（缓存命中）
- **严格缠论结构**：K 线包含处理 → 分型 → 笔 → 笔中枢 → 三买，全程无未来函数
- **双数据源**：新浪 / 东方财富前复权日线，可切换
- **交互式结构图**：K 线 + 笔 + 中枢区间 + 三买点 + 止损位（ECharts）
- **扫描管理**：信号搜索、扫描取消（保留上次结果）、扫描耗时统计
- **行情缓存**：本地 JSON 缓存加速二次扫描，一键清理
- **Python 基准一致**：与原 Python 引擎固定输入逐字段比对，信号、漏斗、图表数据 100% 一致

## 三买判定规则

只输出已确认结构，不预测、不抄底：

1. 向上离开最后一个中枢
2. 回抽一笔的低点不进入中枢上沿 `ZG`
3. 回抽端点底分型在截止日右侧 K 线确认
4. 中枢起始日至截止日不超过 90 根 K 线
5. 当日成交额不低于 0.5 亿元（可调）

扫描漏斗：`bar → pivot → last_stroke → leave → pullback → confirm → age → liquid`

## 安装

从 [Releases](https://github.com/JimouChen/ChanQuant/releases) 下载 `ChanQuant_0.3.0_aarch64.dmg`，拖入「应用程序」即可。

安装包未签名、未公证，首次打开如被 Gatekeeper 拦截：右键点击应用 → 「打开」，或在「系统设置 → 隐私与安全性」点击「仍要打开」。

## 桌面端开发

```bash
cd app
npm install
npm run tauri dev      # 开发模式
npm run tauri build    # 生产构建（app + dmg）
```

## Rust CLI

无界面扫描，结果输出 JSON 到 stdout：

```bash
cargo run --manifest-path app/src-tauri/Cargo.toml --bin chanquant-scan -- \
  --date 2026-11-11 --source sina --min-amount 0.5 --workers 16
```

导出结构图 SVG：

```bash
cargo run --manifest-path app/src-tauri/Cargo.toml --bin chanquant-scan -- \
  --date 2026-11-11 --source sina --plot --result-dir result
# 输出：result/2026-11-11/600863.svg
```

### 参数

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

行情缓存位于 `~/.cache/chanquant/daily`（Rust JSON 与旧 Python `.pkl` 隔离）。桌面端侧边栏「清理行情缓存」按钮可一键清空。

环境变量：

| 变量 | 说明 |
|---|---|
| `CHANQUANT_CACHE` | 覆盖缓存目录 |
| `CHANQUANT_UNIVERSE` | 指定离线股票池 JSON，用于固定基准验证 |

## 项目结构

```
app/
├── src-tauri/
│   ├── src/data.rs       # 股票池、行情加载、复权、缓存、并发
│   ├── src/engine.rs     # 包含处理、分型、笔、中枢、三买
│   ├── src/screener.rs   # 扫描漏斗、信号与图表数据
│   ├── src/lib.rs        # Tauri 命令、取消与耗时统计
│   └── src/bin/          # CLI 与 SVG 导出
└── src/renderer/
    └── src/              # Vue 组件与 ECharts 结构图
```

## License

MIT
