# -*- coding: utf-8 -*-
"""三买信号结构图：K线 + 笔 + 中枢 + 回抽低点（止损位）。"""
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
from matplotlib.patches import Rectangle

from .engine import build_pivots, build_strokes, find_third_buy, merge_bars

plt.rcParams["font.sans-serif"] = ["Heiti TC", "Arial Unicode MS", "Songti SC"]
plt.rcParams["axes.unicode_minus"] = False

UP, DOWN = "#d64541", "#1e9e6a"      # A股习惯：红涨绿跌
STROKE_COLOR = "#2b6cb0"
PIVOT_COLOR = "#f0b429"


def plot_signal(code: str, name: str, df: pd.DataFrame, out_path: Path,
                lookback: int = 140) -> bool:
    """绘制单只标的结构图，无三买结构时返回 False。"""
    mk = merge_bars(df)
    strokes = build_strokes(mk)
    pivots = build_pivots(strokes)
    tb = find_third_buy(strokes, pivots, require_last=False)
    if tb is None:
        return False

    seg = df.iloc[-lookback:].reset_index(drop=True)
    offset = len(df) - len(seg)
    x = np.arange(len(seg))
    dates = seg["dt"].dt.strftime("%m-%d").tolist()

    fig, (ax, axv) = plt.subplots(
        2, 1, figsize=(15, 9), sharex=True,
        gridspec_kw={"height_ratios": [4, 1], "hspace": 0.04})

    for i in range(len(seg)):
        o, c, h, l = (float(seg["o"].iloc[i]), float(seg["c"].iloc[i]),
                      float(seg["h"].iloc[i]), float(seg["l"].iloc[i]))
        color = UP if c >= o else DOWN
        ax.vlines(x[i], l, h, color=color, lw=0.9, zorder=3)
        body = max(abs(c - o), (h - l) * 0.02)
        ax.add_patch(Rectangle((x[i] - 0.32, min(o, c)), 0.64, body,
                               facecolor=color, edgecolor=color, lw=0.6, zorder=4))
        axv.bar(x[i], float(seg["amt"].iloc[i]) / 1e8, width=0.64, color=color, alpha=0.75)

    # 笔
    for s in strokes:
        if s.end_idx < offset:
            continue
        ax.plot([s.start_idx - offset, s.end_idx - offset], [s.start_price, s.end_price],
                color=STROKE_COLOR, lw=1.6, zorder=5, solid_capstyle="round")

    # 中枢
    p = tb.pivot
    x0 = max(p.start_idx - offset, 0)
    x1 = len(seg) - 1
    ax.add_patch(Rectangle((x0 - 0.5, p.zd), x1 - x0 + 1, p.zg - p.zd,
                           facecolor=PIVOT_COLOR, alpha=0.18,
                           edgecolor=PIVOT_COLOR, lw=1.2, zorder=2))
    ax.hlines(p.zg, x0 - 0.5, x1 + 0.5, color=PIVOT_COLOR, ls="--", lw=1.0, zorder=6)
    ax.hlines(p.zd, x0 - 0.5, x1 + 0.5, color=PIVOT_COLOR, ls="--", lw=1.0, zorder=6)
    ax.text(x0, p.zg, f" ZG {p.zg:.2f}", va="bottom", ha="left",
            fontsize=9, color="#8a5a00")
    ax.text(x0, p.zd, f" ZD {p.zd:.2f}", va="top", ha="left",
            fontsize=9, color="#8a5a00")

    # 三买点与止损位
    pb = tb.pullback
    bx = pb.end_idx - offset
    ax.scatter([bx], [pb.low], marker="^", s=150, color=UP, zorder=9,
               edgecolor="white", linewidth=1.0)
    left_side = bx > len(seg) * 0.75
    ax.annotate(f"三买 {pb.low:.2f}", (bx, pb.low),
                xytext=(-18, -44) if left_side else (16, -40), textcoords="offset points",
                ha="right" if left_side else "left",
                fontsize=11, color=UP, fontweight="bold", zorder=10,
                arrowprops=dict(arrowstyle="->", color=UP, lw=1.2))
    ax.axhline(pb.low, color=UP, ls=":", lw=1.0, alpha=0.8, zorder=6)
    ax.text(0, pb.low, f"止损 {pb.low:.2f}", va="bottom", ha="left",
            fontsize=9, color=UP, zorder=10)

    last = seg.iloc[-1]
    info = (f"收盘 {float(last['c']):.2f}   涨幅 {(float(last['c'])/float(seg['c'].iloc[-2])-1)*100:+.2f}%\n"
            f"中枢 [{p.zd:.2f}, {p.zg:.2f}]   离开高点 {tb.leave_high:.2f}\n"
            f"回抽低点 {pb.low:.2f}（距ZG +{(pb.low/p.zg-1)*100:.1f}%）\n"
            f"三买确认日 {df['dt'].iloc[pb.confirm_idx].date()}")
    ax.text(0.012, 0.97, info, transform=ax.transAxes, va="top", ha="left",
            fontsize=10, linespacing=1.6, zorder=12,
            bbox=dict(boxstyle="round,pad=0.5", facecolor="white",
                      edgecolor="#cccccc", alpha=0.95))

    ax.set_title(f"{code} {name}   日线三买   数据截止 {df['dt'].iloc[-1].date()}",
                 fontsize=14, pad=12)
    ax.grid(alpha=0.25, ls="--", lw=0.6)
    ax.set_ylabel("价格")
    axv.set_ylabel("成交额(亿)")
    axv.grid(alpha=0.25, ls="--", lw=0.6)
    step = max(len(seg) // 12, 1)
    axv.set_xticks(x[::step])
    axv.set_xticklabels(dates[::step], rotation=0)
    axv.set_xlim(-1, len(seg))

    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out_path, dpi=160, bbox_inches="tight", facecolor="white")
    plt.close(fig)
    return True
