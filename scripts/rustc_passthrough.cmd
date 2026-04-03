@echo off
REM Forward Cargo's rustc-wrapper contract straight to the real rustc binary.
REM Cargo passes the rustc path as the first argument followed by the original
REM rustc arguments, so the wrapper just shifts once and execs rustc directly.
setlocal EnableDelayedExpansion
set "RUSTC=%~1"
shift
set "ARGS="
:collect_args
if "%~1"=="" goto run_rustc
set "ARGS=!ARGS! %1"
shift
goto collect_args

:run_rustc
call "%RUSTC%" !ARGS!
