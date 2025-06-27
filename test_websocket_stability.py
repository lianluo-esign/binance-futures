#!/usr/bin/env python3
"""
WebSocket连接稳定性测试脚本
用于监控Rust应用程序的WebSocket连接状态
"""

import subprocess
import time
import re
import sys
from datetime import datetime, timedelta

def run_test():
    print("=" * 60)
    print("WebSocket连接稳定性测试")
    print("=" * 60)
    print(f"开始时间: {datetime.now()}")
    print("测试目标: 验证WebSocket连接在长时间运行后的稳定性")
    print("预期运行时间: 至少2小时")
    print("=" * 60)
    
    # 启动Rust应用程序
    print("启动Rust应用程序...")
    try:
        process = subprocess.Popen(
            ["cargo", "run"],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            universal_newlines=True,
            bufsize=1
        )
    except Exception as e:
        print(f"启动应用程序失败: {e}")
        return False
    
    start_time = datetime.now()
    last_message_time = start_time
    connection_lost_count = 0
    reconnect_success_count = 0
    total_messages = 0
    
    # 监控模式
    print("开始监控WebSocket连接状态...")
    print("监控指标:")
    print("- 连接状态")
    print("- 消息接收频率")
    print("- 重连次数")
    print("- 连接断开次数")
    print("-" * 60)
    
    try:
        while True:
            current_time = datetime.now()
            runtime = current_time - start_time
            
            # 读取应用程序输出
            try:
                line = process.stdout.readline()
                if line:
                    line = line.strip()
                    
                    # 检测关键事件
                    if "WebSocket连接成功" in line:
                        print(f"[{current_time.strftime('%H:%M:%S')}] ✅ 连接成功")
                        reconnect_success_count += 1
                        
                    elif "WebSocket连接已关闭" in line or "连接已断开" in line:
                        print(f"[{current_time.strftime('%H:%M:%S')}] ❌ 连接断开")
                        connection_lost_count += 1
                        
                    elif "重连成功" in line:
                        print(f"[{current_time.strftime('%H:%M:%S')}] 🔄 重连成功")
                        reconnect_success_count += 1
                        
                    elif "重连失败" in line:
                        print(f"[{current_time.strftime('%H:%M:%S')}] ⚠️  重连失败")
                        
                    elif "收到" in line and "消息" in line:
                        # 检测消息接收
                        total_messages += 1
                        last_message_time = current_time
                        
                        # 每1000条消息报告一次
                        if total_messages % 1000 == 0:
                            print(f"[{current_time.strftime('%H:%M:%S')}] 📊 已接收 {total_messages} 条消息")
                    
                    elif "ping" in line.lower() or "pong" in line.lower():
                        # 心跳检测
                        if "ping" in line.lower():
                            print(f"[{current_time.strftime('%H:%M:%S')}] 💓 收到ping")
                        elif "pong" in line.lower():
                            print(f"[{current_time.strftime('%H:%M:%S')}] 💓 发送pong")
                
                # 检查进程是否还在运行
                if process.poll() is not None:
                    print(f"[{current_time.strftime('%H:%M:%S')}] ⚠️  应用程序意外退出")
                    break
                    
            except Exception as e:
                print(f"读取输出时出错: {e}")
                break
            
            # 每30秒输出一次状态报告
            if runtime.total_seconds() % 30 < 1:
                time_since_last_message = current_time - last_message_time
                print(f"\n[{current_time.strftime('%H:%M:%S')}] 📈 状态报告:")
                print(f"  运行时间: {runtime}")
                print(f"  总消息数: {total_messages}")
                print(f"  连接断开次数: {connection_lost_count}")
                print(f"  重连成功次数: {reconnect_success_count}")
                print(f"  距离上次消息: {time_since_last_message}")
                
                # 检查是否长时间无消息
                if time_since_last_message > timedelta(minutes=10):
                    print(f"  ⚠️  警告: 超过10分钟未收到消息!")
                
                print("-" * 40)
            
            # 检查是否达到测试目标（2小时）
            if runtime > timedelta(hours=2):
                print(f"\n🎉 测试完成! 运行时间: {runtime}")
                print("最终统计:")
                print(f"  总消息数: {total_messages}")
                print(f"  连接断开次数: {connection_lost_count}")
                print(f"  重连成功次数: {reconnect_success_count}")
                
                if connection_lost_count == 0:
                    print("✅ 测试通过: 连接保持稳定")
                elif reconnect_success_count >= connection_lost_count:
                    print("⚠️  测试部分通过: 连接有断开但重连成功")
                else:
                    print("❌ 测试失败: 连接不稳定")
                
                break
            
            time.sleep(0.1)  # 短暂休眠避免CPU占用过高
            
    except KeyboardInterrupt:
        print(f"\n用户中断测试，运行时间: {datetime.now() - start_time}")
        
    finally:
        # 清理进程
        if process.poll() is None:
            print("正在关闭应用程序...")
            process.terminate()
            time.sleep(2)
            if process.poll() is None:
                process.kill()
        
        print("测试结束")
        return True

if __name__ == "__main__":
    run_test()
