#!/usr/bin/env bash
# SokanLab — audit offline: pastikan tidak ada kode jaringan, tidak ada dependensi
# jaringan di graph build platform ini, dan smoke impor/backtest/render berjalan
# tanpa akses jaringan. Jalankan dari akar repo: scripts/check-offline.sh
set -euo pipefail
cd "$(dirname "$0")/.."

fail=0
export PATH="$HOME/.cargo/bin:$PATH"

echo "== 1) audit kode: pola jaringan di apps/ + crates/ =="
if hits=$(grep -rn --include='*.rs' -E 'TcpStream|TcpListener|UdpSocket|reqwest::|hyper::|tokio::net' apps crates); then
  echo "$hits"
  echo "GAGAL: ditemukan penggunaan jaringan di kode"
  fail=1
else
  echo "ok: 0 hit"
fi

echo "== 2) audit dependensi: klien jaringan di graph build =="
if ! command -v cargo >/dev/null 2>&1; then
  echo "lewati: cargo tidak ada di PATH"
else
  if hits=$(cargo tree --workspace 2>/dev/null | grep -E 'reqwest|hyper|ureq|isahc|surf v|curl v|openssl-sys|native-tls|rustls|tokio|async-std|socket2|^mio v' ); then
    echo "$hits"
    echo "GAGAL: dependensi jaringan ditemukan di graph build"
    fail=1
  else
    echo "ok: 0 klien jaringan"
  fi
fi

echo "== 3) smoke offline: impor → backtest → render =="
bin=""
for cand in ./target/release/sokanlab-desktop ./target/debug/sokanlab-desktop; do
  if [ -x "$cand" ]; then bin="$cand"; break; fi
done
if [ -z "$bin" ]; then
  echo "GAGAL: biner belum dibuild (jalankan cargo build)"
  exit 1
fi
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
"$bin" import --file fixtures/valid-3bars.csv --db "$tmp/ci.db" --out "$tmp/ci.parquet" >/dev/null
"$bin" backtest --parquet "$tmp/ci.parquet" --strategy buy_and_hold --out "$tmp/bt" >/dev/null
"$bin" --render --out "$tmp/shot.png" --bars 120 --size 640x400 >/dev/null
echo "ok: impor + backtest + render selesai tanpa jaringan"

if [ "$fail" -ne 0 ]; then
  echo "AUDIT OFFLINE: GAGAL"
  exit 1
fi
echo "AUDIT OFFLINE: LULUS"
