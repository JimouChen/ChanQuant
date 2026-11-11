# ChanQuant

A股缠论结构引擎与量化筛选。

## 结构

```
chanquant/
  engine.py     # 包含处理 → 分型 → 笔 → 中枢 → 三类买点（纯函数，无未来函数）
  data.py       # akshare 日线前复权取数 + 本地缓存
  screener.py   # 主板全市场三买扫描 CLI
```

## 使用

```bash
python -m venv .venv && . .venv/bin/activate
pip install -r requirements.txt

python -m chanquant.screener --date 20261110
python -m chanquant.screener --date 20261110 --min-amount 1 --max-pivot-age 60
python -m chanquant.screener --date 20261110 --plot     # 结构图输出到 result/<日期>/
```

行情缓存默认写入 `~/.cache/chanquant/daily`，可用 `CHANQUANT_CACHE` 覆盖。

## 三买判定（日线笔中枢）

1. 连续三笔重叠构成中枢，ZG = min(三笔高点)，ZD = max(三笔低点)，向后延伸至不再重叠。
2. 离开笔向上突破 ZG。
3. 回抽笔低点 > ZG（不回到中枢区间），且回抽笔底分型于当日收盘确认。
4. 当日收盘 > ZG，且当日最低价未跌破回抽低点。
5. 剔除 ST / 退市 / 停牌，成交额低于阈值的不参与。

信号失效：收盘跌破回抽笔低点。
