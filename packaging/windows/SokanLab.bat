@echo off
cd /d "%~dp0"
if not exist "sokanlab-desktop.exe" (
  echo sokanlab-desktop.exe tidak ditemukan.
  echo Extract seluruh isi ZIP ke satu folder, lalu jalankan lagi.
  pause
  exit /b 1
)
sokanlab-desktop.exe
if errorlevel 1 (
  echo.
  echo SokanLab berhenti dengan kode %errorlevel%.
  echo Coba jalankan dari Command Prompt untuk melihat pesan lengkap.
  pause
)
