# -*- coding: utf-8 -*-
"""桌面客户端 JSON 接口：复用同一套缠论引擎，输出信号及绘图数据。"""
import argparse
import json
from datetime import date, datetime

from .data import load_universe, main_board_universe
from .engine import build_pivots, build_strokes, find_third_buy, merge_bars
from .screener import _default_start, scan


def _chart_data(df, lookback: int):
    mk = merge_bars(df)
    strokes = build_strokes(mk)
    pivots = build_pivots(strokes)
    third_buy = find_third_buy(strokes, pivots, require_last=False)
    if third_buy is None:
        return None

    offset = max(0, len(df) - lookback)
    seg = df.iloc[offset:]
    dates = [dt.strftime("%Y-%m-%d") for dt in seg["dt"]]
    date_at = lambda idx: df["dt"].iloc[idx].strftime("%Y-%m-%d")

    stroke_points = []
    for stroke in strokes:
        if stroke.start_idx < offset:
            continue
        if not stroke_points:
            stroke_points.append([date_at(stroke.start_idx), stroke.start_price])
        stroke_points.append([date_at(stroke.end_idx), stroke.end_price])

    pivot = third_buy.pivot
    pullback = third_buy.pullback
    return {
        "dates": dates,
        "candles": [
            [round(float(row.o), 3), round(float(row.c), 3),
             round(float(row.l), 3), round(float(row.h), 3)]
            for row in seg.itertuples()
        ],
        "volumes": [round(float(value) / 1e8, 4) for value in seg["amt"]],
        "strokePoints": [[day, round(price, 3)] for day, price in stroke_points],
        "pivot": {
            "startDate": date_at(max(pivot.start_idx, offset)),
            "endDate": dates[-1],
            "zg": round(pivot.zg, 3),
            "zd": round(pivot.zd, 3),
        },
        "buyPoint": {
            "date": date_at(pullback.end_idx),
            "price": round(pullback.low, 3),
            "confirmDate": date_at(pullback.confirm_idx),
        },
    }


def run_scan(end: str, start: str, min_amount: float, max_pivot_age: int,
             workers: int, source: str, lookback: int = 160):
    universe = main_board_universe()
    names = dict(zip(universe["code"], universe["name"]))
    bars = load_universe(universe["code"], start, end, workers=workers, source=source)
    result = scan(bars, names, min_amount=min_amount, max_pivot_age=max_pivot_age)

    signals = []
    for _, row in result["signals"].iterrows():
        code = row["代码"]
        signals.append({
            "code": code,
            "name": row["名称"],
            "close": row["收盘"],
            "changePct": row["涨幅%"],
            "amount": row["成交额亿"],
            "zg": row["中枢ZG"],
            "zd": row["中枢ZD"],
            "leaveHigh": row["离开高点"],
            "pullbackLow": row["回抽低点"],
            "distanceZgPct": row["低点距ZG%"],
            "pullbackPct": row["回抽幅度%"],
            "pivotStart": row["中枢起点"],
            "confirmDate": row["三买确认日"],
            "chart": _chart_data(bars[code], lookback),
        })

    return {
        "tradeDate": result["last_date"].strftime("%Y-%m-%d"),
        "source": source,
        "universeSize": len(universe),
        "loadedSize": len(bars),
        "funnel": result["funnel"],
        "signals": signals,
        "generatedAt": datetime.now().isoformat(timespec="seconds"),
    }


def main():
    parser = argparse.ArgumentParser(description="ChanQuant 桌面端 JSON 扫描接口")
    parser.add_argument("--date", default=date.today().strftime("%Y%m%d"))
    parser.add_argument("--start", default=None)
    parser.add_argument("--min-amount", type=float, default=0.5)
    parser.add_argument("--max-pivot-age", type=int, default=90)
    parser.add_argument("--workers", type=int, default=16)
    parser.add_argument("--source", choices=("eastmoney", "sina"), default="sina")
    parser.add_argument("--lookback", type=int, default=160)
    args = parser.parse_args()
    payload = run_scan(args.date, args.start or _default_start(args.date),
                       args.min_amount, args.max_pivot_age, args.workers,
                       args.source, args.lookback)
    print(json.dumps(payload, ensure_ascii=False, separators=(",", ":")))


if __name__ == "__main__":
    main()
