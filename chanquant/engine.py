# -*- coding: utf-8 -*-
"""缠论结构引擎：包含关系处理 → 分型 → 笔 → 中枢 → 三类买点。

所有函数均为纯函数，输入只使用当日收盘及之前的数据，
分型/笔的确认位置通过 confirm 索引显式给出，不引入未来函数。
"""
from dataclasses import dataclass
from typing import List, Optional

import pandas as pd

MIN_STROKE_GAP = 4      # 顶底分型间至少间隔的合并K线数（含端点），即至少1根独立K线


@dataclass
class MergedBar:
    high: float
    low: float
    start: int          # 首根原始K线下标
    end: int            # 末根原始K线下标


@dataclass
class Fractal:
    idx: int            # 所属合并K线下标
    kind: str           # "top" / "bot"
    price: float
    confirm: int        # 分型成立时可获得的最新原始K线下标


@dataclass
class Stroke:
    direction: str      # "up" / "down"
    start_price: float
    end_price: float
    high: float
    low: float
    start_idx: int      # 起点分型所在合并K线的末根原始K线下标
    end_idx: int        # 终点分型所在合并K线的末根原始K线下标
    confirm_idx: int    # 该笔终点被确认的原始K线下标


@dataclass
class Pivot:
    i0: int             # 中枢首笔在笔序列中的下标
    i1: int             # 中枢末笔下标（含延伸）
    zg: float           # 中枢上沿
    zd: float           # 中枢下沿
    start_idx: int      # 中枢起点原始K线下标


@dataclass
class ThirdBuy:
    pivot: Pivot
    pullback: Stroke    # 回抽笔（不回中枢）
    leave_high: float   # 离开段高点（回抽笔起点）


def merge_bars(df: pd.DataFrame) -> List[MergedBar]:
    """K线包含关系处理，返回合并后的K线序列。"""
    out: List[MergedBar] = []
    highs = df["h"].to_numpy()
    lows = df["l"].to_numpy()
    for i in range(len(df)):
        h, l = float(highs[i]), float(lows[i])
        if not out:
            out.append(MergedBar(h, l, i, i))
            continue
        last = out[-1]
        contained = (h <= last.high and l >= last.low) or (h >= last.high and l <= last.low)
        if not contained:
            out.append(MergedBar(h, l, i, i))
            continue
        going_up = True if len(out) < 2 else (out[-1].high > out[-2].high)
        if going_up:
            last.high = max(last.high, h)
            last.low = max(last.low, l)
        else:
            last.high = min(last.high, h)
            last.low = min(last.low, l)
        last.end = i
    return out


def find_fractals(mk: List[MergedBar]) -> List[Fractal]:
    """在合并K线上识别顶/底分型，confirm 为右侧合并K线首根原始K线下标。"""
    fx: List[Fractal] = []
    for i in range(1, len(mk) - 1):
        a, b, c = mk[i - 1], mk[i], mk[i + 1]
        if b.high > a.high and b.high > c.high and b.low > a.low and b.low > c.low:
            fx.append(Fractal(i, "top", b.high, c.start))
        elif b.low < a.low and b.low < c.low and b.high < a.high and b.high < c.high:
            fx.append(Fractal(i, "bot", b.low, c.start))
    return fx


def build_strokes(mk: List[MergedBar]) -> List[Stroke]:
    """由分型序列构造笔：顶底交替、同类取极值、保证至少一根独立K线。"""
    chosen: List[Fractal] = []
    for f in find_fractals(mk):
        if not chosen:
            chosen.append(f)
            continue
        last = chosen[-1]
        if f.kind == last.kind:
            if (f.kind == "top" and f.price > last.price) or (f.kind == "bot" and f.price < last.price):
                chosen[-1] = f
            continue
        valid_dir = (last.kind == "bot" and f.price > last.price) or \
                    (last.kind == "top" and f.price < last.price)
        if f.idx - last.idx >= MIN_STROKE_GAP and valid_dir:
            chosen.append(f)

    strokes: List[Stroke] = []
    for a, b in zip(chosen, chosen[1:]):
        strokes.append(Stroke(
            direction="up" if a.kind == "bot" else "down",
            start_price=a.price,
            end_price=b.price,
            high=max(a.price, b.price),
            low=min(a.price, b.price),
            start_idx=mk[a.idx].end,
            end_idx=mk[b.idx].end,
            confirm_idx=b.confirm,
        ))
    return strokes


def build_pivots(strokes: List[Stroke]) -> List[Pivot]:
    """连续三笔重叠确定中枢 ZG/ZD，再向后延伸至不再重叠。"""
    out: List[Pivot] = []
    i = 0
    while i + 2 < len(strokes):
        seg = strokes[i:i + 3]
        zg = min(s.high for s in seg)
        zd = max(s.low for s in seg)
        if zg <= zd:
            i += 1
            continue
        j = i + 2
        while j + 1 < len(strokes) and strokes[j + 1].low < zg and strokes[j + 1].high > zd:
            j += 1
        out.append(Pivot(i, j, zg, zd, strokes[i].start_idx))
        i = j + 1
    return out


def find_third_buy(strokes: List[Stroke], pivots: List[Pivot],
                   require_last: bool = True) -> Optional[ThirdBuy]:
    """基于最后一个中枢判定三买：向上离开后回抽笔低点不回到中枢区间 [ZD, ZG]。

    require_last=True 时要求回抽笔就是当前最后一笔（即信号刚成立）。
    """
    if not strokes or not pivots:
        return None
    pivot = pivots[-1]
    k = pivot.i1 + 1
    # 若中枢后首笔整体位于中枢上方（向上跳空离开），回抽笔再后移一笔
    if k < len(strokes) - 1 and strokes[k].direction == "up" and strokes[k].low > pivot.zg:
        k += 1
    if k >= len(strokes):
        return None
    if require_last and k != len(strokes) - 1:
        return None
    pull = strokes[k]
    if pull.direction != "down":
        return None
    if pull.start_price <= pivot.zg:      # 离开段未向上突破中枢上沿
        return None
    if pull.low <= pivot.zg:              # 回抽回到中枢区间，不构成三买
        return None
    return ThirdBuy(pivot=pivot, pullback=pull, leave_high=pull.start_price)
