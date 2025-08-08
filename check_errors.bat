@echo off
cargo check 2>&1 | findstr /C:"error[E"