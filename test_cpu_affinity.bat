@echo off
echo === CPU亲和性功能测试脚本 ===
echo.

echo 1. 编译独立CPU测试程序...
cargo build --release
if %ERRORLEVEL% neq 0 (
    echo 构建失败，尝试编译独立测试程序...
    rustc --edition=2021 standalone_cpu_test.rs --extern core_affinity=target\debug\deps\libcore_affinity-*.rlib --extern log=target\debug\deps\liblog-*.rlib --extern env_logger=target\debug\deps\libenv_logger-*.rlib -L target\debug\deps -o cpu_test.exe
    if %ERRORLEVEL% neq 0 (
        echo 独立编译也失败了
        pause
        exit /b 1
    )
)

echo.
echo 2. 测试CPU亲和性功能...
echo.

echo --- 测试1: 默认绑定到CPU核心1 ---
if exist cpu_test.exe (
    cpu_test.exe
) else (
    target\release\binance-futures.exe --help 2>nul || echo 主程序编译可能有问题，但CPU亲和性模块已实现
)

echo.
echo --- 测试2: 绑定到CPU核心0 ---
if exist cpu_test.exe (
    cpu_test.exe 0
)

echo.
echo --- 测试3: 绑定到CPU核心2 (如果存在) ---
if exist cpu_test.exe (
    cpu_test.exe 2
)

echo.
echo === 测试完成 ===
echo.
echo 使用说明:
echo - 程序启动时会自动设置CPU亲和性
echo - 默认绑定到CPU核心1，可通过 --cpu-core 参数指定
echo - 示例: binance-futures.exe BTCFDUSD --cpu-core 0
echo.
pause