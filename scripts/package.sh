#!/usr/bin/env bash
# SokanLab — paket rilis lokal: biner release + LICENSE/NOTICE/README → tar.gz + SHA-256.
# Paket per-OS dibuat dengan menjalankan skrip ini di OS terkait (atau via CI).
# Windows: jalankan di Git Bash/WSL.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

cargo build --release

ver=$(./target/release/sokanlab-desktop --version | awk '{print $2}')
os=$(uname -s | tr '[:upper:]' '[:lower:]')
arch=$(uname -m)
name="sokanlab-${ver}-${os}-${arch}"
dist="dist/${name}"

rm -rf "${dist}"
mkdir -p "${dist}" "${dist}/config"
cp target/release/sokanlab-desktop "${dist}/"
for f in LICENSE NOTICE THIRD-PARTY.md README.md; do
  [ -f "$f" ] && cp "$f" "${dist}/"
done
# GUI membaca config/default.toml relatif terhadap working directory; ikutkan
# di paket agar biner dapat dijalankan dari hasil ekstraksi.
[ -f config/default.toml ] && cp config/default.toml "${dist}/config/"
mkdir -p "${dist}/docs"
[ -f docs/BACKTEST.md ] && cp docs/BACKTEST.md "${dist}/docs/"

tar -C dist -czf "dist/${name}.tar.gz" "${name}"
(
  cd dist
  sha256sum "${name}.tar.gz" > "${name}.tar.gz.sha256"
)

echo "paket: dist/${name}.tar.gz"
cat "dist/${name}.tar.gz.sha256"
