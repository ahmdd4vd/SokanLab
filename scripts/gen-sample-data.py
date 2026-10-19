#!/usr/bin/env python3
"""Generate deterministic sample data + layouts for SokanLab preview packages.

No network access: everything is computed locally with a fixed RNG seed, so the
same seed always yields the same CSV (byte-identical across machines/platforms).

Usage:
    python3 scripts/gen-sample-data.py --out /path/to/data

Writes into the output directory:
    contoh-BTCUSDT-1h.csv   2400 bar 1 jam (timestamp,open,high,low,close,volume)
    layout.json             layout bersih (SMA/EMA/VWAP + pane RSI/MACD)
    layout-demo.json        layout demo (10 alat gambar)

Impor CSV ke katalog/Parquet dilakukan terpisah oleh biner, mis.:
    sokanlab-desktop import --file data/contoh-BTCUSDT-1h.csv \
        --symbol BTC/USDT --timeframe 1h --db data/catalog.db --out data/BTCUSDT-1h.parquet
"""
import argparse
import json
import math
import random
from datetime import datetime, timezone
from pathlib import Path

N = 2400
STEP = 3600  # 1 jam
TS0 = 1767225600  # 2026-01-01 00:00:00 UTC
SEED = 20261019


def build_bars():
    rng = random.Random(SEED)
    bars = []
    p = 10000.0
    for i in range(N):
        target = 10000.0 + 280.0 * math.sin(i / 180.0) + 120.0 * math.sin(i / 60.0)
        shock = 0.0
        if 1450 <= i < 1470:
            shock = -45.0
        if 1900 <= i < 1930:
            shock = 38.0
        o = max(10, int(round(p)))
        c = max(10, int(round(o + 0.06 * (target - o) + rng.gauss(0, 17) + shock)))
        hi = max(o, c) + rng.randint(2, 22)
        lo = max(1, min(o, c) - rng.randint(2, 22))
        vol = rng.randint(120, 6800)
        bars.append((TS0 + i * STEP, o, hi, lo, c, vol))
        p = float(c)
    return bars


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", default="data", help="direktori keluaran (default: data)")
    args = ap.parse_args()

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    bars = build_bars()
    csv_path = out / "contoh-BTCUSDT-1h.csv"
    with csv_path.open("w", newline="\n") as f:
        f.write("timestamp,open,high,low,close,volume\n")
        for b in bars:
            f.write(",".join(str(x) for x in b) + "\n")

    ts = [b[0] * 1000 for b in bars]
    lo = [b[3] for b in bars]
    hi = [b[2] for b in bars]
    cl = [b[4] for b in bars]
    start = N - 430

    def a(i, price):
        return {"t": ts[i], "p": int(round(price))}

    def win(lo_i, hi_i):
        return min(lo[lo_i:hi_i]), max(hi[lo_i:hi_i])

    lo1, hi1 = win(start + 20, start + 130)
    lo2, hi2 = win(start + 140, start + 250)
    lo3, hi3 = win(start + 260, N)
    view_lo = min(lo[start:])
    view_hi = max(hi[start:])
    pad = int((view_hi - view_lo) * 0.06)

    vp = {
        "t_min": ts[start],
        "t_max": ts[-1] + 3600 * 1000,
        "p_min": view_lo - pad,
        "p_max": view_hi + pad,
        "scale": "linear",
        "invert": False,
    }

    clean = {
        "schema": 7,
        "theme": "dark",
        "viewport": vp,
        "tools": {"magnet": "weak", "stay": False},
        "overlays": {"sma": 21, "ema": 55, "vwap": True},
        "panes": [{"kind": "rsi", "period": 14}, {"kind": "macd", "fast": 12, "slow": 26, "signal": 9}],
        "watchlist": ["BTC/USDT-1h"],
        "drawings": [
            {"kind": "trend", "a": a(start + 40, lo1), "b": a(start + 210, hi2),
             "style": {"color": "#7fb2d9", "width": 2, "dash": "solid"}},
            {"kind": "fib", "a": a(start + 145, lo2), "b": a(start + 265, hi3),
             "extend": "right", "style": {"color": "#f2b04d", "width": 1, "dash": "solid"}},
            {"kind": "hline", "price": int(round(hi3 + 18)),
             "style": {"color": "#e06c75", "width": 1, "dash": "dashed"}},
            {"kind": "text", "a": a(start + 300, hi3 + 45), "text": "SokanLab — preview Windows",
             "font": "l", "align": "center", "style": {"color": "#d7dae0"}},
        ],
    }

    demo = {
        "schema": 7,
        "theme": "dark",
        "viewport": vp,
        "tools": {"magnet": "off", "stay": False},
        "watchlist": ["BTC/USDT-1h"],
        "drawings": [
            {"kind": "regression", "a": a(start + 20, cl[start + 20]), "b": a(start + 400, cl[start + 400]),
             "extend": "right", "style": {"color": "#8fc98f", "width": 2, "dash": "solid"}},
            {"kind": "fib", "a": a(start + 40, lo1), "b": a(start + 140, hi1),
             "style": {"color": "#f2b04d", "width": 1, "dash": "solid"}},
            {"kind": "channel", "a": a(start + 160, lo2), "b": a(start + 260, lo2 + 60),
             "c": a(start + 210, hi2), "variant": "parallel",
             "style": {"color": "#56b6c2", "width": 1, "dash": "solid"}},
            {"kind": "elliott_impulse", "pts": [
                a(start + 180, lo2), a(start + 215, (lo2 + hi2) / 2), a(start + 235, lo2 + 35),
                a(start + 275, hi3 - 10), a(start + 310, (hi2 + hi3) / 2)],
             "style": {"color": "#d19a66", "width": 2, "dash": "solid"}},
            {"kind": "rect", "a": a(start + 320, lo3 + 20), "b": a(start + 380, lo3 + 140),
             "style": {"color": "#c678dd", "width": 1, "dash": "dashed"}, "opacity": 55},
            {"kind": "highlighter", "pts": [a(start + 200, lo2 + 10), a(start + 280, lo2 + 25)],
             "style": {"color": "#f2b04d", "width": 3, "dash": "solid"}, "opacity": 45},
            {"kind": "brush", "pts": [
                a(start + 390, hi3 - 60), a(start + 396, hi3 - 35), a(start + 402, hi3 - 50),
                a(start + 408, hi3 - 15)],
             "style": {"color": "#e06c75", "width": 2, "dash": "solid"}},
            {"kind": "sticker", "a": a(start + 405, hi3 + 10), "icon": "star",
             "style": {"color": "#e5c07b", "width": 2, "dash": "solid"}},
            {"kind": "callout", "a": a(start + 60, lo1 - 20), "text": "klik kanan objek → properti",
             "font": "s", "align": "left", "style": {"color": "#d7dae0", "width": 1, "dash": "solid"}},
            {"kind": "text", "a": a(start + 210, hi1 + 70), "text": "contoh alat gambar SokanLab",
             "font": "l", "align": "center", "style": {"color": "#d7dae0", "width": 1, "dash": "solid"}},
        ],
    }

    (out / "layout.json").write_text(json.dumps(clean, ensure_ascii=False, indent=1) + "\n")
    (out / "layout-demo.json").write_text(json.dumps(demo, ensure_ascii=False, indent=1) + "\n")

    print(f"bars={N} range={datetime.fromtimestamp(TS0, timezone.utc)}.."
          f"{datetime.fromtimestamp(TS0 + (N - 1) * STEP, timezone.utc)}")
    print(f"price range {min(lo)}..{max(hi)} csv={csv_path.stat().st_size} bytes")
    print(f"objects clean={len(clean['drawings'])} demo={len(demo['drawings'])}")


if __name__ == "__main__":
    main()
