@echo off
REM Entrada principal: a versao mantida do projeto e Rust.
call "%~dp0rust\build.bat" %*
exit /b %errorlevel%
