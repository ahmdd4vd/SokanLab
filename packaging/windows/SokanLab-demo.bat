@echo off
cd /d "%~dp0"
if not exist "sokanlab-desktop.exe" (
  echo sokanlab-desktop.exe tidak ditemukan.
  echo Extract seluruh isi ZIP ke satu folder, lalu jalankan lagi.
  pause
  exit /b 1
)
if not exist "data\layout-demo.json" (
  echo Layout demo tidak ditemukan; membuka chart default.
  sokanlab-desktop.exe
) else (
  sokanlab-desktop.exe --layout data\layout-demo.json
)
if errorlevel 1 (
  echo.
  echo SokanLab berhenti dengan kode %errorlevel%.
  pause
)
