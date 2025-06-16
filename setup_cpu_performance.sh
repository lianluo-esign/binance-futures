#!/bin/bash
# aws_run.sh

# 设置CPU调度器为性能模式
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# 禁用透明大页
echo never | sudo tee /sys/kernel/mm/transparent_hugepage/enabled

# 设置网络中断亲和性到CPU0
for irq in $(grep eth0 /proc/interrupts | cut -d: -f1); do
    echo 1 | sudo tee /proc/irq/$irq/smp_affinity
done

# 运行程序在CPU1上
#skset -c 1 chrt -f 99 ./target/release/binance-futures