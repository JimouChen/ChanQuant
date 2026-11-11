# -*- coding: utf-8 -*-
"""A股日线数据加载与本地缓存（akshare + 前复权）。"""
import os
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Dict, Iterable, Optional

import pandas as pd

MAIN_BOARD_PREFIX = ("600", "601", "603", "605", "000", "001", "002", "003")
CACHE_DIR = Path(os.environ.get("CHANQUANT_CACHE", Path.home() / ".cache" / "chanquant" / "daily"))
COLUMNS = ["dt", "o", "c", "h", "l", "v", "amt", "turn"]


def _ak():
    import akshare as ak
    return ak


def main_board_universe(exclude_st: bool = True) -> pd.DataFrame:
    """沪深主板股票池（剔除 ST / 退市），返回 code、name 两列。"""
    lst = _ak().stock_info_a_code_name()
    lst.columns = ["code", "name"]
    mask = lst["code"].str.startswith(MAIN_BOARD_PREFIX)
    if exclude_st:
        mask &= ~lst["name"].str.contains("ST|退", na=False)
    return lst[mask].reset_index(drop=True)


def fetch_daily(code: str, start: str, end: str, adjust: str = "qfq",
                use_cache: bool = True) -> Optional[pd.DataFrame]:
    """单只股票日线，返回 [dt, o, c, h, l, v, amt, turn]，带本地 pkl 缓存。"""
    path = CACHE_DIR / f"{code}_{adjust}_{start}_{end}.pkl"
    if use_cache and path.exists():
        try:
            return pd.read_pickle(path)
        except Exception:
            pass

    ak = _ak()
    for _ in range(3):
        try:
            df = ak.stock_zh_a_hist(symbol=code, period="daily", start_date=start,
                                    end_date=end, adjust=adjust)
            if df is None or len(df) == 0:
                return None
            df = df.rename(columns={"日期": "dt", "开盘": "o", "收盘": "c", "最高": "h",
                                    "最低": "l", "成交量": "v", "成交额": "amt", "换手率": "turn"})
            df = df[COLUMNS].copy()
            df["dt"] = pd.to_datetime(df["dt"])
            if use_cache:
                CACHE_DIR.mkdir(parents=True, exist_ok=True)
                tmp = path.with_suffix(".tmp")
                df.to_pickle(tmp)
                os.replace(tmp, path)
            return df
        except Exception:
            time.sleep(0.8)
    return None


def load_universe(codes: Iterable[str], start: str, end: str, workers: int = 16,
                  adjust: str = "qfq", use_cache: bool = True,
                  min_bars: int = 130) -> Dict[str, pd.DataFrame]:
    """并发拉取股票池日线，过滤数据不足的标的。"""
    codes = list(codes)

    def task(code):
        try:
            return code, fetch_daily(code, start, end, adjust=adjust, use_cache=use_cache)
        except Exception:
            return code, None

    bars: Dict[str, pd.DataFrame] = {}
    with ThreadPoolExecutor(max_workers=workers) as ex:
        for code, df in ex.map(task, codes):
            if df is not None and len(df) >= min_bars:
                bars[code] = df
    return bars
