#!/usr/bin/env python3
"""Package a SokanLab preview build into a portable archive (.zip / .tar.gz).

Hanya memakai pustaka standar Python, jadi jalan sama di runner Linux, macOS,
dan Windows. Selalu menyertakan berkas atribusi + `config/default.toml`
(GUI membacanya relatif terhadap working directory).

Isi paket:
    <name>/sokanlab-desktop[.exe]
    <name>/config/default.toml
    <name>/LICENSE, NOTICE, THIRD-PARTY.md, README.md
    <name>/docs/{BACKTEST,TV-PARITY,WINDOWS-PREVIEW}.md
    <name>/data/*                  (bila --data-dir diberikan)
    <name>/*                      (bila --extra diberikan, mis. skrip .bat)
    <name>/SHA256SUMS.txt          (hash semua berkas di atas, kecuali dirinya)

Contoh:
    python3 scripts/package-preview.py \
        --name sokanlab-0.1.0-preview-linux-x86_64 \
        --bin target/x86_64-unknown-linux-gnu/release/sokanlab-desktop \
        --format tar.gz --out dist --data-dir stage/data
"""
import argparse
import hashlib
import shutil
import sys
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROOT_FILES = ["LICENSE", "NOTICE", "THIRD-PARTY.md", "README.md"]
DOCS = ["BACKTEST.md", "TV-PARITY.md", "WINDOWS-PREVIEW.md"]


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def stage(args, staging: Path) -> None:
    if staging.exists():
        shutil.rmtree(staging)
    staging.mkdir(parents=True)

    bin_src = Path(args.bin)
    if not bin_src.is_file():
        sys.exit(f"biner tidak ditemukan: {bin_src}")
    bin_name = "sokanlab-desktop.exe" if bin_src.suffix.lower() == ".exe" else "sokanlab-desktop"
    bin_dst = staging / bin_name
    shutil.copy2(bin_src, bin_dst)
    bin_dst.chmod(0o755)

    cfg = ROOT / "config" / "default.toml"
    if not cfg.is_file():
        sys.exit(f"config tidak ditemukan: {cfg}")
    (staging / "config").mkdir()
    shutil.copy2(cfg, staging / "config" / "default.toml")

    for name in ROOT_FILES:
        src = ROOT / name
        if src.is_file():
            shutil.copy2(src, staging / name)

    (staging / "docs").mkdir()
    for name in DOCS:
        src = ROOT / "docs" / name
        if src.is_file():
            shutil.copy2(src, staging / "docs" / name)

    if args.extra:
        for item in args.extra:
            src = Path(item)
            if not src.is_file():
                sys.exit(f"--extra tidak ditemukan: {src}")
            shutil.copy2(src, staging / src.name)

    if args.data_dir:
        data_src = Path(args.data_dir)
        if not data_src.is_dir():
            sys.exit(f"--data-dir tidak ditemukan: {data_src}")
        data_dst = staging / "data"
        data_dst.mkdir()
        for item in sorted(data_src.iterdir()):
            if item.is_file():
                shutil.copy2(item, data_dst / item.name)

    # SHA256SUMS.txt: hash semua berkas relatif, diurutkan, sebelum berkasnya sendiri.
    lines = []
    for f in sorted(staging.rglob("*")):
        if f.is_file():
            lines.append(f"{sha256(f)}  {f.relative_to(staging).as_posix()}")
    (staging / "SHA256SUMS.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")


def archive(staging: Path, out_dir: Path, name: str, fmt: str) -> Path:
    out_dir.mkdir(parents=True, exist_ok=True)
    if fmt == "zip":
        dest = out_dir / f"{name}.zip"
        with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
            for f in sorted(staging.rglob("*")):
                if not f.is_file():
                    continue
                arc = f"{name}/{f.relative_to(staging).as_posix()}"
                info = zipfile.ZipInfo.from_file(f, arc)
                info.compress_type = zipfile.ZIP_DEFLATED  # writestr memakai ini, bukan default ZipFile
                mode = 0o755 if f.stat().st_mode & 0o111 else 0o644
                info.external_attr = (mode << 16) | 0o100000  # file biasa + mode unix
                with f.open("rb") as fh:
                    z.writestr(info, fh.read())
        return dest

    dest = out_dir / f"{name}.tar.gz"
    with tarfile.open(dest, "w:gz", format=tarfile.GNU_FORMAT) as t:
        for f in sorted(staging.rglob("*")):
            if not f.is_file():
                continue
            info = t.gettarinfo(str(f), arcname=f"{name}/{f.relative_to(staging).as_posix()}")
            info.uname = info.gname = "root"
            info.uid = info.gid = 0
            info.mode = 0o755 if f.stat().st_mode & 0o111 else 0o644
            with f.open("rb") as fh:
                t.addfile(info, fh)
    return dest


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--name", required=True, help="nama folder/arsip paket")
    ap.add_argument("--bin", required=True, help="path biner sokanlab-desktop")
    ap.add_argument("--format", required=True, choices=["zip", "tar.gz"])
    ap.add_argument("--out", required=True, help="direktori keluaran arsip")
    ap.add_argument("--data-dir", default=None, help="direktori berisi data contoh (opsional)")
    ap.add_argument("--extra", nargs="*", default=[], help="berkas tambahan di root paket")
    ap.add_argument("--work", default=None, help="direktori staging (default: <out>/<name>)")
    args = ap.parse_args()

    out_dir = Path(args.out)
    staging = Path(args.work) if args.work else out_dir / args.name
    stage(args, staging)
    dest = archive(staging, out_dir, args.name, args.format)
    digest = sha256(dest)
    (dest.parent / f"{dest.name}.sha256").write_text(f"{digest}  {dest.name}\n", encoding="utf-8")
    print(f"paket={dest} ({dest.stat().st_size} byte)")
    print(f"sha256={digest}")


if __name__ == "__main__":
    main()
