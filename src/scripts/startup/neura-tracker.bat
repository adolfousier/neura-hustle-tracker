@echo off
for /f "tokens=*" %%i in ('dir /b /ad /s "%USERPROFILE%" ^| findstr /i "neura-hustle-tracker"') do (
  cd /d "%%i"
  goto :found
)
:found
start cmd /k "timeout /t 30 /nobreak > nul && make run"