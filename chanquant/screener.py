# -*- coding: utf-8 -*-
"""日线三类买点扫描：主板全市场 + 严格结构过滤。

用法:
    python -m chanquant.screener --date 2026-11-10
    python -m chanquant.screener --min-amount 1 --max-pivot-age 60
"""
import argparse
from datetime import date, datetime, timedelta
from pathlib import Path

import pandas as pd

from .data import load_universe, main_board_universe
from .engine import build_pivots, build_strokes, find_third_buy, merge_bars

STAGES = ("bar", "pivot", "last_stroke", "leave", "pullback", "confirm", "age", "liquid")


def _default_start(end: str) -> str:
    return (datetime.strptime(end, "%Y%m%d") - timedelta(days=700)).strftime("%Y%m%d")


def scan(bars: dict, names: dict, min_amount: float = 0.5,
         max_pivot_age: int = 90, min_bars: int = 130) -> dict:
    """对给定日线集合执行三买扫描，返回信号表、漏斗统计与确认滞后分布。"""
    if not bars:
        return {"signals": pd.DataFrame(), "funnel": dict.fromkeys(STAGES, 0), "lags": {}, "last_date": None}

    last_date = max(df["dt"].iloc[-1] for df in bars.values())
    funnel = dict.fromkeys(STAGES, 0)
    lags, rows = [], []

    for code, df in bars.items():
        if len(df) < min_bars:
            continue
        if df["dt"].iloc[-1] != last_date:          # 停牌或数据滞后
            continue
        funnel["bar"] += 1

        last_raw = len(df) - 1
        mk = merge_bars(df)
        strokes = build_strokes(mk)
        pivots = build_pivots(strokes)
        if len(strokes) < 8 or not pivots:
            continue
        funnel["pivot"] += 1

        tb = find_third_buy(strokes, pivots)
        if tb is None:
            continue
        if tb.pullback is not strokes[-1]:
            continue
        funnel["last_stroke"] += 1
        if tb.leave_high <= tb.pivot.zg:
            continue
        funnel["leave"] += 1
        if tb.pullback.low <= tb.pivot.zg:
            continue
        funnel["pullback"] += 1

        lag = last_raw - tb.pullback.confirm_idx
        lags.append(lag)
        if lag != 0:                                # 回抽笔底分型须在当日收盘确认
            continue
        funnel["confirm"] += 1
        if last_raw - tb.pivot.start_idx > max_pivot_age:
            continue
        funnel["age"] += 1

        close = float(df["c"].iloc[-1])
        prev_close = float(df["c"].iloc[-2])
        amount = float(df["amt"].iloc[-1]) / 1e8
        if amount < min_amount:
            continue
        if close <= tb.pivot.zg or float(df["l"].iloc[-1]) < tb.pullback.low:
            continue                                # 收盘失守中枢上沿或跌破回抽低点
        funnel["liquid"] += 1

        rows.append({
            "代码": code,
            "名称": names.get(code, ""),
            "收盘": close,
            "涨幅%": round((close / prev_close - 1) * 100, 2),
            "成交额亿": round(amount, 2),
            "中枢ZG": round(tb.pivot.zg, 2),
            "中枢ZD": round(tb.pivot.zd, 2),
            "离开高点": round(tb.leave_high, 2),
            "回抽低点": round(tb.pullback.low, 2),
            "低点距ZG%": round((tb.pullback.low / tb.pivot.zg - 1) * 100, 2),
            "回抽幅度%": round((tb.leave_high - tb.pullback.low) / tb.leave_high * 100, 1),
            "中枢起点": str(df["dt"].iloc[tb.pivot.start_idx].date()),
            "三买确认日": str(df["dt"].iloc[tb.pullback.confirm_idx].date()),
        })

    signals = pd.DataFrame(rows)
    if len(signals):
        signals = signals.sort_values("成交额亿", ascending=False).reset_index(drop=True)
    return {"signals": signals, "funnel": funnel, "lags": lags, "last_date": last_date}


def main():
    parser = argparse.ArgumentParser(description="缠论日线三买扫描（沪深主板）")
    parser.add_argument("--date", default=date.today().strftime("%Y%m%d"), help="数据截止日 YYYYMMDD")
    parser.add_argument("--start", default=None, help="取数起始日 YYYYMMDD，默认截止日前700天")
    parser.add_argument("--min-amount", type=float, default=0.5, help="当日成交额下限（亿元）")
    parser.add_argument("--max-pivot-age", type=int, default=90, help="中枢起点距今最大K线数")
    parser.add_argument("--workers", type=int, default=16)
    parser.add_argument("--source", choices=("eastmoney", "sina"), default="eastmoney", help="行情数据源")
    parser.add_argument("--no-cache", action="store_true", help="忽略本地行情缓存")
    parser.add_argument("--plot", action="store_true", help="导出信号结构图")
    parser.add_argument("--result-dir", default=None, help="结构图输出目录，默认 result/<日期>")
    args = parser.parse_args()

    end = args.date
    start = args.start or _default_start(end)

    universe = main_board_universe()
    names = dict(zip(universe["code"], universe["name"]))
    print(f"股票池 {len(universe)} 只，取数区间 {start} ~ {end}")
    bars = load_universe(universe["code"], start, end, workers=args.workers,
                         use_cache=not args.no_cache, source=args.source)
    print(f"有效日线 {len(bars)} 只，开始结构扫描")

    result = scan(bars, names, min_amount=args.min_amount, max_pivot_age=args.max_pivot_age)
    print(f"数据截止 {result['last_date'].date()} | 漏斗 {result['funnel']}")
    print(f"三买信号 {len(result['signals'])} 只")
    if len(result["signals"]):
        print(result["signals"].to_string(index=False))

    if args.plot and len(result["signals"]):
        from .plot import plot_signal
        out_dir = Path(args.result_dir) if args.result_dir else \
            Path("result") / result["last_date"].strftime("%Y%m%d")
        for _, row in result["signals"].iterrows():
            code = row["代码"]
            saved = plot_signal(code, row["名称"], bars[code], out_dir / f"{code}.png")
            if saved:
                print(f"结构图 {out_dir / f'{code}.png'}")


if __name__ == "__main__":
    main()
