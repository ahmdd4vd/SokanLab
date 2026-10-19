@echo off
setlocal EnableExtensions
cd /d "%~dp0"
if not exist "sokanlab-desktop.exe" (
  echo sokanlab-desktop.exe tidak ditemukan.
  echo Extract seluruh isi ZIP ke satu folder, lalu jalankan lagi.
  pause
  exit /b 1
)

echo.
echo === Impor CSV ke SokanLab ===
echo Format: timestamp,open,high,low,close,volume (semua integer)
echo.
set "FILE="
set /p "FILE=Path CSV (drag ke sini lalu Enter): "
if not defined FILE goto batal
set "FILE=%FILE:"=%"
rem Rapikan spasi/CR sisa drag-and-drop (aman di cmd Windows maupun Wine).
for /f "tokens=* delims=" %%A in ("%FILE%") do set "FILE=%%A"
if not exist "%FILE%" (
  echo File tidak ditemukan: %FILE%
  pause
  exit /b 1
)

set "SYMBOL="
set /p "SYMBOL=Simbol (contoh BTC/USDT) [BTC/USDT]: "
if not defined SYMBOL set "SYMBOL=BTC/USDT"
set "TF="
set /p "TF=Timeframe 1m/1h/1D [1h]: "
if not defined TF set "TF=1h"

set "SYM=%SYMBOL:/=%"

rem Dataset baru harus menjadi satu-satunya entri katalog agar langsung aktif.
if exist "data\catalog.db" (
  if exist "data\catalog-sample.db" (
    del "data\catalog.db"
  ) else (
    ren "data\catalog.db" "catalog-sample.db"
  )
)
if exist "data\BTCUSDT-1h.parquet" (
  if not exist "data\BTCUSDT-1h-sample.parquet" (
    ren "data\BTCUSDT-1h.parquet" "BTCUSDT-1h-sample.parquet"
  )
)
rem Layout contoh memakai timestamp data contoh; sisihkan agar chart baru fit.
if exist "data\layout.json" (
  if not exist "data\layout-contoh.json" (
    ren "data\layout.json" "layout-contoh.json"
  )
)

sokanlab-desktop.exe import --file "%FILE%" --symbol "%SYMBOL%" --timeframe "%TF%" --db "data\catalog.db" --out "data\%SYM%-%TF%.parquet"
if errorlevel 1 goto gagal

echo.
echo Impor selesai. Dataset baru akan dimuat saat SokanLab dibuka.
echo Layout contoh disimpan sebagai data\layout-contoh.json.
echo.
pause
exit /b 0

:gagal
echo.
echo Impor gagal. Periksa format CSV dan pesan error di atas.
if not exist "data\catalog.db" (
  if exist "data\catalog-sample.db" ren "data\catalog-sample.db" "catalog.db"
)
if not exist "data\BTCUSDT-1h.parquet" (
  if exist "data\BTCUSDT-1h-sample.parquet" ren "data\BTCUSDT-1h-sample.parquet" "BTCUSDT-1h.parquet"
)
if not exist "data\layout.json" (
  if exist "data\layout-contoh.json" ren "data\layout-contoh.json" "layout.json"
)
pause
exit /b 1

:batal
echo Dibatalkan.
pause
exit /b 0
