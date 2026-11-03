@echo off
REM ===========================================================================
REM build.bat - Compila a versao em Rust do Notas Definitivas (Windows)
REM ---------------------------------------------------------------------------
REM Uso (dentro da pasta rust\):
REM   build.bat            compila em release  -> ..\notas_rs.exe
REM   build.bat run        compila e executa
REM   build.bat test       roda os testes (unitarios + integracao + fuzz)
REM   build.bat clean      remove target\ e ..\notas_rs.exe
REM
REM Requisitos: Rust (rustup) e um linker para o alvo escolhido:
REM   - stable-x86_64-pc-windows-gnu : gcc do MinGW-w64 no PATH, ou
REM   - stable-x86_64-pc-windows-msvc: Visual Studio Build Tools (C++).
REM O SQLite e compilado junto (precisa de um compilador C do mesmo alvo).
REM ===========================================================================
setlocal
cd /d "%~dp0"

where cargo >nul 2>&1 || set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo >nul 2>&1 || (echo ERRO: cargo nao encontrado. Instale o Rust: https://rustup.rs & exit /b 1)

if /I "%~1"=="clean" goto clean
if /I "%~1"=="test" goto test

echo ==^> Compilando (release)
cargo build --release
if errorlevel 1 (
    echo ERRO: falha na compilacao. Veja a secao "Problemas conhecidos" em README.md.
    exit /b 1
)
copy /y "target\release\notas.exe" "..\notas_rs.exe" >nul
if errorlevel 1 (
    echo AVISO: nao foi possivel atualizar ..\notas_rs.exe ^(o programa esta aberto?^).
    echo        Feche-o e rode build.bat de novo. O binario novo esta em rust\target\release\notas.exe
    exit /b 1
)
echo     OK -^> ..\notas_rs.exe
if /I "%~1"=="run" "..\notas_rs.exe"
goto end

:test
cargo test --release
goto end

:clean
if exist target rmdir /s /q target
if exist "..\notas_rs.exe" del /q "..\notas_rs.exe"
echo Artefatos removidos.

:end
endlocal
