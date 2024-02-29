@echo off

if "%1"=="" goto end_parse_arguments

if /i "%1"=="-h" goto display_help
if /i "%1"=="--help" goto display_help

:display_help
echo Usage: Tpaste is a command-line tool that upload terminal output to a web service and provides its link. It requires authentication or registration with a new account.
echo.
echo Options:
echo   -h, --help    Print help information.
echo.
exit /b 0

:end_parse_arguments

cd /d C:\Users\dell\OneDrive\Desktop\RUST\project\client
src\main.exe
