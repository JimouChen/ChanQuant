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


def _fetch_eastmoney(code: str, start: str, end: str, adjust: str) -> Optional[pd.DataFrame]:
    df = _ak().stock_zh_a_hist(symbol=code, period="daily", start_date=start,
                               end_date=end, adjust=adjust)
    if df is None or df.empty:
        return None
    df = df.rename(columns={"日期": "dt", "开盘": "o", "收盘": "c", "最高": "h",
                            "最低": "l", "成交量": "v", "成交额": "amt", "换手率": "turn"})
    return df[COLUMNS].copy()


def _fetch_sina(code: str, start: str, end: str, adjust: str) -> Optional[pd.DataFrame]:
    symbol = ("sh" if code.startswith("6") else "sz") + code
    df = _ak().stock_zh_a_daily(symbol=symbol, start_date=start, end_date=end, adjust=adjust)
    if df is None or df.empty:
        return None
    return pd.DataFrame({
        "dt": df["date"], "o": df["open"], "c": df["close"], "h": df["high"],
        "l": df["low"], "v": df["volume"], "amt": df["amount"],
        "turn": df["turnover"] * 100,
    })


def fetch_daily(code: str, start: str, end: str, adjust: str = "qfq",
                use_cache: bool = True, source: str = "eastmoney") -> Optional[pd.DataFrame]:
    """单只股票日线，返回统一字段并缓存；source 支持 eastmoney / sina。"""
    source_prefix = "" if source == "eastmoney" else f"{source}_"
    path = CACHE_DIR / f"{code}_{source_prefix}{adjust}_{start}_{end}.pkl"
    if use_cache:
        cache_paths = [path] if path.exists() else []
        if not cache_paths and CACHE_DIR.exists():
            for candidate in CACHE_DIR.glob(f"{code}_{source_prefix}{adjust}_*_*.pkl"):
                try:
                    cached_start, cached_end = candidate.stem.rsplit("_", 2)[-2:]
                    if cached_start <= start and cached_end >= end:
                        cache_paths.append(candidate)
                except ValueError:
                    continue
        for cached_path in sorted(cache_paths):
            try:
                cached = pd.read_pickle(cached_path)
                if cached_path == path:
                    return cached
                mask = ((cached["dt"] >= pd.to_datetime(start))
                        & (cached["dt"] <= pd.to_datetime(end)))
                return cached.loc[mask].copy()
            except Exception:
                pass

    fetcher = _fetch_sina if source == "sina" else _fetch_eastmoney
    for _ in range(3):
        try:
            df = fetcher(code, start, end, adjust)
            if df is None:
                return None
            df = df.copy()
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
                  min_bars: int = 130, source: str = "eastmoney") -> Dict[str, pd.DataFrame]:
    """并发拉取股票池日线，过滤数据不足的标的。"""
    codes = list(codes)

    def task(code):
        try:
            return code, fetch_daily(code, start, end, adjust=adjust,
                                     use_cache=use_cache, source=source)
        except Exception:
            return code, None

    bars: Dict[str, pd.DataFrame] = {}
    with ThreadPoolExecutor(max_workers=workers) as ex:
        for code, df in ex.map(task, codes):
            if df is not None and len(df) >= min_bars:
                bars[code] = df
    return bars
