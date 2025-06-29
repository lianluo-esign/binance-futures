#!/usr/bin/env python3
"""
创建示例音频文件的脚本
需要安装: pip install numpy scipy

这个脚本会生成简单的提示音作为示例音频文件
"""

import numpy as np
import wave
import os

def create_beep(frequency, duration, sample_rate=44100, amplitude=0.3):
    """创建一个简单的提示音"""
    t = np.linspace(0, duration, int(sample_rate * duration), False)
    # 生成正弦波
    wave_data = amplitude * np.sin(2 * np.pi * frequency * t)
    
    # 添加淡入淡出效果
    fade_samples = int(0.1 * sample_rate)  # 0.1秒淡入淡出
    if len(wave_data) > 2 * fade_samples:
        # 淡入
        wave_data[:fade_samples] *= np.linspace(0, 1, fade_samples)
        # 淡出
        wave_data[-fade_samples:] *= np.linspace(1, 0, fade_samples)
    
    return wave_data

def save_wav(filename, wave_data, sample_rate=44100):
    """保存WAV文件"""
    # 确保目录存在
    os.makedirs(os.path.dirname(filename), exist_ok=True)
    
    # 转换为16位整数
    wave_data_int = (wave_data * 32767).astype(np.int16)
    
    with wave.open(filename, 'w') as wav_file:
        wav_file.setnchannels(1)  # 单声道
        wav_file.setsampwidth(2)  # 16位
        wav_file.setframerate(sample_rate)
        wav_file.writeframes(wave_data_int.tobytes())

def main():
    print("创建示例音频文件...")
    
    # 创建买单音效 - 较高音调，上升音调
    print("创建买单音效...")
    buy_beep1 = create_beep(800, 0.2)  # 800Hz, 0.2秒
    buy_beep2 = create_beep(1000, 0.2)  # 1000Hz, 0.2秒
    buy_sound = np.concatenate([buy_beep1, buy_beep2])
    save_wav("src/audio/buy.wav", buy_sound)
    
    # 创建卖单音效 - 较低音调，下降音调
    print("创建卖单音效...")
    sell_beep1 = create_beep(600, 0.2)  # 600Hz, 0.2秒
    sell_beep2 = create_beep(400, 0.2)  # 400Hz, 0.2秒
    sell_sound = np.concatenate([sell_beep1, sell_beep2])
    save_wav("src/audio/sell.wav", sell_sound)
    
    print("示例音频文件创建完成！")
    print("文件位置:")
    print("- src/audio/buy.wav")
    print("- src/audio/sell.wav")
    print("\n注意: 这些是WAV格式的示例文件。")
    print("如果需要MP3格式，请使用音频转换工具进行转换。")

if __name__ == "__main__":
    try:
        main()
    except ImportError as e:
        print(f"缺少依赖: {e}")
        print("请安装所需的Python包:")
        print("pip install numpy scipy")
    except Exception as e:
        print(f"创建音频文件时出错: {e}")
