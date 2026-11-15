use crate::model::Bar;

pub const MIN_STROKE_GAP: usize = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct MergedBar {
    pub high: f64,
    pub low: f64,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fractal {
    pub idx: usize,
    pub kind: FractalKind,
    pub price: f64,
    pub confirm: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractalKind {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub direction: Direction,
    pub start_price: f64,
    pub end_price: f64,
    pub high: f64,
    pub low: f64,
    pub start_idx: usize,
    pub end_idx: usize,
    pub confirm_idx: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pivot {
    pub i0: usize,
    pub i1: usize,
    pub zg: f64,
    pub zd: f64,
    pub start_idx: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThirdBuy {
    pub pivot: Pivot,
    pub pullback: Stroke,
    pub pullback_idx: usize,
    pub leave_high: f64,
}

pub fn merge_bars(bars: &[Bar]) -> Vec<MergedBar> {
    let mut out: Vec<MergedBar> = Vec::with_capacity(bars.len());
    for (i, bar) in bars.iter().enumerate() {
        if out.is_empty() {
            out.push(MergedBar {
                high: bar.h,
                low: bar.l,
                start: i,
                end: i,
            });
            continue;
        }
        let len = out.len();
        let last = &out[len - 1];
        let contained =
            (bar.h <= last.high && bar.l >= last.low) || (bar.h >= last.high && bar.l <= last.low);
        if !contained {
            out.push(MergedBar {
                high: bar.h,
                low: bar.l,
                start: i,
                end: i,
            });
            continue;
        }
        let going_up = len < 2 || out[len - 1].high > out[len - 2].high;
        let last = &mut out[len - 1];
        if going_up {
            last.high = last.high.max(bar.h);
            last.low = last.low.max(bar.l);
        } else {
            last.high = last.high.min(bar.h);
            last.low = last.low.min(bar.l);
        }
        last.end = i;
    }
    out
}

pub fn find_fractals(merged: &[MergedBar]) -> Vec<Fractal> {
    let mut fractals = Vec::new();
    for i in 1..merged.len().saturating_sub(1) {
        let (a, b, c) = (&merged[i - 1], &merged[i], &merged[i + 1]);
        if b.high > a.high && b.high > c.high && b.low > a.low && b.low > c.low {
            fractals.push(Fractal {
                idx: i,
                kind: FractalKind::Top,
                price: b.high,
                confirm: c.start,
            });
        } else if b.low < a.low && b.low < c.low && b.high < a.high && b.high < c.high {
            fractals.push(Fractal {
                idx: i,
                kind: FractalKind::Bottom,
                price: b.low,
                confirm: c.start,
            });
        }
    }
    fractals
}

pub fn build_strokes(merged: &[MergedBar]) -> Vec<Stroke> {
    let mut chosen: Vec<Fractal> = Vec::new();
    for fractal in find_fractals(merged) {
        let Some(last) = chosen.last() else {
            chosen.push(fractal);
            continue;
        };
        if fractal.kind == last.kind {
            let more_extreme = match fractal.kind {
                FractalKind::Top => fractal.price > last.price,
                FractalKind::Bottom => fractal.price < last.price,
            };
            if more_extreme {
                let index = chosen.len() - 1;
                chosen[index] = fractal;
            }
            continue;
        }
        let valid_direction = match last.kind {
            FractalKind::Bottom => fractal.price > last.price,
            FractalKind::Top => fractal.price < last.price,
        };
        if fractal.idx - last.idx >= MIN_STROKE_GAP && valid_direction {
            chosen.push(fractal);
        }
    }

    chosen
        .windows(2)
        .map(|pair| {
            let (a, b) = (&pair[0], &pair[1]);
            Stroke {
                direction: if a.kind == FractalKind::Bottom {
                    Direction::Up
                } else {
                    Direction::Down
                },
                start_price: a.price,
                end_price: b.price,
                high: a.price.max(b.price),
                low: a.price.min(b.price),
                start_idx: merged[a.idx].end,
                end_idx: merged[b.idx].end,
                confirm_idx: b.confirm,
            }
        })
        .collect()
}

pub fn build_pivots(strokes: &[Stroke]) -> Vec<Pivot> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 < strokes.len() {
        let segment = &strokes[i..i + 3];
        let zg = segment
            .iter()
            .map(|stroke| stroke.high)
            .fold(f64::INFINITY, f64::min);
        let zd = segment
            .iter()
            .map(|stroke| stroke.low)
            .fold(f64::NEG_INFINITY, f64::max);
        if zg <= zd {
            i += 1;
            continue;
        }
        let mut j = i + 2;
        while j + 1 < strokes.len() && strokes[j + 1].low < zg && strokes[j + 1].high > zd {
            j += 1;
        }
        out.push(Pivot {
            i0: i,
            i1: j,
            zg,
            zd,
            start_idx: strokes[i].start_idx,
        });
        i = j + 1;
    }
    out
}

pub fn find_third_buy(
    strokes: &[Stroke],
    pivots: &[Pivot],
    require_last: bool,
) -> Option<ThirdBuy> {
    let pivot = pivots.last()?.clone();
    let mut k = pivot.i1 + 1;
    if k < strokes.len().saturating_sub(1)
        && strokes[k].direction == Direction::Up
        && strokes[k].low > pivot.zg
    {
        k += 1;
    }
    if k >= strokes.len() || (require_last && k != strokes.len() - 1) {
        return None;
    }
    let pullback = strokes[k].clone();
    if pullback.direction != Direction::Down
        || pullback.start_price <= pivot.zg
        || pullback.low <= pivot.zg
    {
        return None;
    }
    Some(ThirdBuy {
        leave_high: pullback.start_price,
        pullback,
        pullback_idx: k,
        pivot,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merged(high: f64, low: f64, index: usize) -> MergedBar {
        MergedBar {
            high,
            low,
            start: index,
            end: index,
        }
    }

    fn bar(high: f64, low: f64) -> Bar {
        Bar {
            dt: chrono::NaiveDate::default(),
            o: 0.0,
            c: 0.0,
            h: high,
            l: low,
            v: 0.0,
            amt: 0.0,
            turn: 0.0,
        }
    }

    #[test]
    fn merges_containment_in_trend_direction() {
        let bars = vec![bar(10.0, 5.0), bar(12.0, 6.0), bar(11.0, 7.0)];
        let result = merge_bars(&bars);
        assert_eq!(result.len(), 2);
        assert_eq!(
            (result[1].high, result[1].low, result[1].end),
            (12.0, 7.0, 2)
        );
    }

    #[test]
    fn confirms_fractal_at_right_bar_start() {
        let bars = vec![
            merged(10.0, 5.0, 0),
            merged(12.0, 7.0, 1),
            merged(11.0, 6.0, 2),
        ];
        let fractals = find_fractals(&bars);
        assert_eq!(fractals.len(), 1);
        assert_eq!(fractals[0].kind, FractalKind::Top);
        assert_eq!(fractals[0].confirm, 2);
    }

    fn stroke(direction: Direction, start: f64, end: f64, index: usize) -> Stroke {
        Stroke {
            direction,
            start_price: start,
            end_price: end,
            high: start.max(end),
            low: start.min(end),
            start_idx: index,
            end_idx: index + 1,
            confirm_idx: index + 1,
        }
    }

    #[test]
    fn extends_pivot_while_ranges_overlap() {
        let strokes = vec![
            stroke(Direction::Up, 10.0, 14.0, 0),
            stroke(Direction::Down, 14.0, 11.0, 2),
            stroke(Direction::Up, 11.0, 13.0, 4),
            stroke(Direction::Down, 13.0, 12.0, 6),
            stroke(Direction::Up, 14.0, 20.0, 8),
        ];
        let pivots = build_pivots(&strokes);
        assert_eq!(pivots.len(), 1);
        assert_eq!((pivots[0].i1, pivots[0].zg, pivots[0].zd), (3, 13.0, 11.0));
    }

    #[test]
    fn third_buy_requires_latest_pullback_when_requested() {
        let strokes = vec![
            stroke(Direction::Up, 10.0, 14.0, 0),
            stroke(Direction::Down, 14.0, 11.0, 2),
            stroke(Direction::Up, 11.0, 13.0, 4),
            stroke(Direction::Up, 14.0, 18.0, 6),
            stroke(Direction::Down, 18.0, 15.0, 8),
            stroke(Direction::Up, 15.0, 19.0, 10),
        ];
        let pivots = build_pivots(&strokes[..3]);
        assert!(find_third_buy(&strokes, &pivots, true).is_none());
        assert!(find_third_buy(&strokes[..5], &pivots, true).is_some());
    }
}
