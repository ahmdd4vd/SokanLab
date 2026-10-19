@echo off
cd /d "%~dp0"
if not exist "sokanlab-desktop.exe" (
  echo sokanlab-desktop.exe tidak ditemukan.
  echo Extract seluruh isi ZIP ke satu folder, lalu jalankan lagi.
  pause
  exit /b 1
)
set WGPU_BACKEND=opengl
sokanlab-desktop.exe
if errorlevel 1 (
  echo.
  echo SokanLab berhenti dengan kode %errorlevel%.
  echo Coba update driver GPU, atau jalankan SokanLab.bat (backend default).
  pause
)
