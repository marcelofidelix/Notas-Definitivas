@echo off
REM ===========================================================================
REM build.bat - Compilacao do Notas Definitivas no Windows (MinGW / GCC)
REM ---------------------------------------------------------------------------
REM Uso (no CMD ou PowerShell, dentro desta pasta):
REM   build.bat            compila          -> notas.exe
REM   build.bat windows    idem
REM   build.bat run        compila e executa
REM   build.bat clean      remove notas.exe
REM
REM Requisito: gcc do MinGW-w64 no PATH.
REM ===========================================================================
setlocal
cd /d "%~dp0"

set "SRC=notas.c"
if exist "src\notas.c" set "SRC=src\notas.c"

set "DO_RUN=0"
if /I "%~1"=="run" set "DO_RUN=1"

if /I "%~1"=="clean" goto clean

set "SQLITE_FLAGS="
if exist "tools\sqlite3-mingw\include\sqlite3.h" (
    set "SQLITE_FLAGS=-Itools\sqlite3-mingw\include -Ltools\sqlite3-mingw\lib"
)

echo ==^> Compilando para Windows (gcc, %SRC%)
REM v6.29.0+: as notas vivem em um banco SQLite (notas.db), entao a
REM libsqlite3 precisa estar disponivel para o gcc do MinGW.
gcc -O2 -Wall -Wextra "%SRC%" -o notas.exe %SQLITE_FLAGS% -lsqlite3 -lkernel32 -luser32 -lshell32
if errorlevel 1 (
    echo ERRO: falha na compilacao. Verifique se o gcc ^(MinGW-w64^) esta no PATH.
    exit /b 1
)
echo     OK -^> notas.exe

if "%DO_RUN%"=="1" notas.exe
goto end

:clean
if exist notas.exe del /q notas.exe
echo Binario notas.exe removido.

:end
endlocal
