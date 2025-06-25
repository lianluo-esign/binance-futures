"""
AOI (Active Order Imbalance) 指标分析工具 - 演示版本
使用模拟数据演示AOI指标的计算、可视化和分析功能
"""

import sys
import os
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from datetime import datetime, timedelta
import logging
from typing import Dict, List, Tuple, Union
import json

# 配置中文字体和日志
plt.rcParams['font.sans-serif'] = ['SimHei', 'Microsoft YaHei']
plt.rcParams['axes.unicode_minus'] = False
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class AOIIndicator:
    """AOI (Active Order Imbalance) 指标计算器"""
    
    def __init__(self, window_type: str = 'time', window_size: Union[int, float] = 60):
        """
        初始化AOI指标计算器
        
        参数:
            window_type: 窗口类型 ('time', 'tick', 'volume')
            window_size: 窗口大小 (秒数/tick数/成交量)
        """
        self.window_type = window_type
        self.window_size = window_size
        self.trades_buffer = []
        self.aoi_history = []
        self.delta_volume_history = []
        self.timestamps = []
        
    def add_trade(self, timestamp: int, price: float, quantity: float, side: str):
        """添加交易数据"""
        trade = {
            'timestamp': timestamp,
            'price': price,
            'quantity': quantity,
            'side': side
        }
        self.trades_buffer.append(trade)
        
        # 根据窗口类型清理过期数据
        self._clean_buffer(timestamp)
        
        # 计算当前AOI
        aoi, delta_vol = self._calculate_aoi()
        
        self.aoi_history.append(aoi)
        self.delta_volume_history.append(delta_vol)
        self.timestamps.append(timestamp)
        
    def _clean_buffer(self, current_timestamp: int):
        """根据窗口类型清理过期数据"""
        if self.window_type == 'time':
            # 时间窗口：保留指定秒数内的数据
            cutoff_time = current_timestamp - (self.window_size * 1000)
            self.trades_buffer = [t for t in self.trades_buffer if t['timestamp'] >= cutoff_time]
        elif self.window_type == 'tick':
            # tick窗口：保留指定数量的最新交易
            if len(self.trades_buffer) > self.window_size:
                self.trades_buffer = self.trades_buffer[-int(self.window_size):]
        elif self.window_type == 'volume':
            # 成交量窗口：保留指定成交量的最新交易
            total_volume = sum(t['quantity'] for t in self.trades_buffer)
            while total_volume > self.window_size and len(self.trades_buffer) > 1:
                removed_trade = self.trades_buffer.pop(0)
                total_volume -= removed_trade['quantity']
                
    def _calculate_aoi(self) -> Tuple[float, float]:
        """计算AOI和Delta Volume"""
        if not self.trades_buffer:
            return 0.0, 0.0
            
        buy_volume = sum(t['quantity'] for t in self.trades_buffer if t['side'] == 'buy')
        sell_volume = sum(t['quantity'] for t in self.trades_buffer if t['side'] == 'sell')
        
        total_volume = buy_volume + sell_volume
        if total_volume == 0:
            return 0.0, 0.0
            
        aoi = (buy_volume - sell_volume) / total_volume
        delta_volume = buy_volume - sell_volume
        
        return aoi, delta_volume

    def get_current_aoi(self) -> float:
        """获取当前AOI值"""
        return self.aoi_history[-1] if self.aoi_history else 0.0

    def get_current_delta_volume(self) -> float:
        """获取当前Delta Volume"""
        return self.delta_volume_history[-1] if self.delta_volume_history else 0.0

    def get_statistics(self) -> Dict:
        """获取AOI统计信息"""
        if not self.aoi_history:
            return {}
            
        aoi_array = np.array(self.aoi_history)
        delta_array = np.array(self.delta_volume_history)
        
        return {
            'aoi_mean': np.mean(aoi_array),
            'aoi_std': np.std(aoi_array),
            'aoi_min': np.min(aoi_array),
            'aoi_max': np.max(aoi_array),
            'aoi_median': np.median(aoi_array),
            'delta_volume_mean': np.mean(delta_array),
            'delta_volume_std': np.std(delta_array),
            'delta_volume_sum': np.sum(delta_array),
            'positive_aoi_ratio': np.sum(aoi_array > 0) / len(aoi_array),
            'strong_buy_ratio': np.sum(aoi_array > 0.5) / len(aoi_array),
            'strong_sell_ratio': np.sum(aoi_array < -0.5) / len(aoi_array),
        }

def generate_mock_trade_data(duration_hours: int = 2, trades_per_minute: int = 10) -> pd.DataFrame:
    """
    生成模拟交易数据
    
    参数:
        duration_hours: 数据时长(小时)
        trades_per_minute: 每分钟交易数量
        
    返回:
        pd.DataFrame: 模拟交易数据
    """
    logger.info(f"生成 {duration_hours} 小时的模拟交易数据...")
    
    # 设置时间范围
    end_time = datetime.now()
    start_time = end_time - timedelta(hours=duration_hours)
    
    # 计算总交易数量
    total_minutes = duration_hours * 60
    total_trades = total_minutes * trades_per_minute
    
    # 生成时间戳
    timestamps = pd.date_range(start=start_time, end=end_time, periods=total_trades)
    
    # 生成价格走势（随机游走）
    base_price = 65000.0  # BTC基础价格
    price_changes = np.random.normal(0, 10, total_trades)  # 价格变化
    prices = base_price + np.cumsum(price_changes)
    
    # 生成交易量（对数正态分布）
    quantities = np.random.lognormal(mean=-2, sigma=1, size=total_trades)
    
    # 生成交易方向（带趋势偏向）
    # 使用价格变化来影响买卖概率
    buy_probabilities = 0.5 + 0.1 * np.tanh(price_changes / 20)  # 价格上涨时更多买单
    sides = ['buy' if np.random.random() < prob else 'sell' for prob in buy_probabilities]
    
    # 创建DataFrame
    trades_df = pd.DataFrame({
        'ts': [int(ts.timestamp() * 1000) for ts in timestamps],
        'timestamp': timestamps,
        'price': prices,
        'qty': quantities,
        'side': sides
    })
    
    logger.info(f"生成了 {len(trades_df)} 条模拟交易记录")
    return trades_df

def analyze_aoi_with_mock_data():
    """使用模拟数据进行AOI分析"""
    try:
        logger.info("开始AOI指标分析演示...")
        
        # 1. 生成模拟交易数据
        trades_df = generate_mock_trade_data(duration_hours=2, trades_per_minute=20)
        
        # 2. 定义多种窗口配置
        window_configs = [
            {'type': 'time', 'size': 60},    # 60秒时间窗口
            {'type': 'time', 'size': 300},   # 5分钟时间窗口
            {'type': 'tick', 'size': 100},   # 100笔交易窗口
            {'type': 'tick', 'size': 500},   # 500笔交易窗口
        ]
        
        # 3. 计算AOI指标
        results = []
        indicators = {}
        
        for config in window_configs:
            config_name = f"{config['type']}_{config['size']}"
            logger.info(f"计算AOI指标: {config_name}")
            
            # 创建指标计算器
            indicator = AOIIndicator(config['type'], config['size'])
            indicators[config_name] = indicator
            
            # 逐行处理交易数据
            for _, trade in trades_df.iterrows():
                indicator.add_trade(
                    timestamp=int(trade['ts']),
                    price=float(trade['price']),
                    quantity=float(trade['qty']),
                    side=trade['side']
                )
                
                # 记录结果
                results.append({
                    'timestamp': trade['ts'],
                    'datetime': trade['timestamp'],
                    'price': trade['price'],
                    f'aoi_{config_name}': indicator.get_current_aoi(),
                    f'delta_volume_{config_name}': indicator.get_current_delta_volume(),
                    'config': config_name
                })
        
        aoi_df = pd.DataFrame(results)
        
        # 4. 生成交易信号
        signals_df = aoi_df.copy()
        aoi_threshold = 0.6
        
        for config_name in indicators.keys():
            aoi_col = f'aoi_{config_name}'
            signal_col = f'signal_{config_name}'
            
            if aoi_col in signals_df.columns:
                conditions = [
                    signals_df[aoi_col] > aoi_threshold,
                    signals_df[aoi_col] < -aoi_threshold,
                ]
                choices = ['BUY', 'SELL']
                signals_df[signal_col] = np.select(conditions, choices, default='HOLD')
        
        # 5. 创建可视化图表
        create_aoi_visualizations(aoi_df, signals_df, indicators)
        
        # 6. 生成分析报告
        generate_analysis_report(aoi_df, signals_df, indicators)
        
        logger.info("AOI分析演示完成！")
        
    except Exception as e:
        logger.error(f"分析过程中发生错误: {str(e)}")
        import traceback
        traceback.print_exc()

def create_aoi_visualizations(aoi_df: pd.DataFrame, signals_df: pd.DataFrame, indicators: Dict):
    """创建AOI可视化图表"""
    logger.info("创建可视化图表...")
    
    # 创建输出目录
    output_dir = './aoi_analysis_output'
    os.makedirs(output_dir, exist_ok=True)
    
    # 1. AOI时间序列图
    fig, axes = plt.subplots(len(indicators), 1, figsize=(15, 4*len(indicators)))
    if len(indicators) == 1:
        axes = [axes]
        
    for i, config_name in enumerate(indicators.keys()):
        aoi_col = f'aoi_{config_name}'
        config_data = aoi_df[aoi_df['config'] == config_name]
        
        axes[i].plot(config_data['datetime'], config_data[aoi_col], 
                   label=f'AOI ({config_name})', linewidth=1)
        axes[i].axhline(y=0, color='black', linestyle='-', alpha=0.3)
        axes[i].axhline(y=0.6, color='red', linestyle='--', alpha=0.7, label='买入阈值')
        axes[i].axhline(y=-0.6, color='green', linestyle='--', alpha=0.7, label='卖出阈值')
        
        axes[i].set_title(f'AOI指标时间序列 - {config_name}')
        axes[i].set_ylabel('AOI值')
        axes[i].legend()
        axes[i].grid(True, alpha=0.3)
        
    plt.tight_layout()
    plt.savefig(os.path.join(output_dir, 'aoi_timeseries.png'), dpi=300, bbox_inches='tight')
    logger.info("AOI时间序列图已保存")
    
    # 2. 交易信号图
    fig, axes = plt.subplots(len(indicators), 1, figsize=(15, 4*len(indicators)))
    if len(indicators) == 1:
        axes = [axes]
        
    for i, config_name in enumerate(indicators.keys()):
        signal_col = f'signal_{config_name}'
        config_data = signals_df[signals_df['config'] == config_name]
        
        # 绘制价格线
        axes[i].plot(config_data['datetime'], config_data['price'], 
                   label='价格', color='black', alpha=0.7)
        
        # 标记买卖信号
        buy_signals = config_data[config_data[signal_col] == 'BUY']
        sell_signals = config_data[config_data[signal_col] == 'SELL']
        
        if not buy_signals.empty:
            axes[i].scatter(buy_signals['datetime'], buy_signals['price'], 
                          color='red', marker='^', s=30, label='买入信号', alpha=0.8)
                          
        if not sell_signals.empty:
            axes[i].scatter(sell_signals['datetime'], sell_signals['price'], 
                          color='green', marker='v', s=30, label='卖出信号', alpha=0.8)
        
        axes[i].set_title(f'交易信号 - {config_name}')
        axes[i].set_ylabel('价格')
        axes[i].legend()
        axes[i].grid(True, alpha=0.3)
        
    plt.tight_layout()
    plt.savefig(os.path.join(output_dir, 'trading_signals.png'), dpi=300, bbox_inches='tight')
    logger.info("交易信号图已保存")
    
    plt.show()

def generate_analysis_report(aoi_df: pd.DataFrame, signals_df: pd.DataFrame, indicators: Dict):
    """生成分析报告"""
    logger.info("生成分析报告...")
    
    report = {
        'analysis_time': datetime.now().strftime('%Y-%m-%d %H:%M:%S'),
        'data_period': {
            'start': aoi_df['datetime'].min().strftime('%Y-%m-%d %H:%M:%S'),
            'end': aoi_df['datetime'].max().strftime('%Y-%m-%d %H:%M:%S'),
            'duration_hours': (aoi_df['datetime'].max() - aoi_df['datetime'].min()).total_seconds() / 3600
        },
        'configurations': {},
        'signal_statistics': {}
    }
    
    # 为每个配置生成统计信息
    for config_name, indicator in indicators.items():
        stats = indicator.get_statistics()
        config_data = aoi_df[aoi_df['config'] == config_name]
        
        report['configurations'][config_name] = {
            'window_type': indicator.window_type,
            'window_size': indicator.window_size,
            'total_samples': len(config_data),
            'statistics': stats
        }
        
        # 信号统计
        signal_col = f'signal_{config_name}'
        if signal_col in signals_df.columns:
            config_signals = signals_df[signals_df['config'] == config_name]
            signal_counts = config_signals[signal_col].value_counts()
            report['signal_statistics'][config_name] = signal_counts.to_dict()
    
    # 打印报告摘要
    print("\n" + "="*60)
    print("AOI指标分析报告")
    print("="*60)
    print(f"分析时间: {report['analysis_time']}")
    print(f"数据周期: {report['data_period']['start']} 到 {report['data_period']['end']}")
    print(f"分析时长: {report['data_period']['duration_hours']:.2f} 小时")
    print()
    
    # 打印各配置的统计信息
    for config_name, config_info in report['configurations'].items():
        print(f"配置: {config_name}")
        print(f"  窗口类型: {config_info['window_type']}")
        print(f"  窗口大小: {config_info['window_size']}")
        print(f"  样本数量: {config_info['total_samples']}")
        
        stats = config_info['statistics']
        if stats:
            print(f"  AOI均值: {stats['aoi_mean']:.4f}")
            print(f"  AOI标准差: {stats['aoi_std']:.4f}")
            print(f"  AOI范围: [{stats['aoi_min']:.4f}, {stats['aoi_max']:.4f}]")
            print(f"  正向AOI比例: {stats['positive_aoi_ratio']:.2%}")
            print(f"  强买入信号比例: {stats['strong_buy_ratio']:.2%}")
            print(f"  强卖出信号比例: {stats['strong_sell_ratio']:.2%}")
            print(f"  累计Delta Volume: {stats['delta_volume_sum']:.2f}")
        print()
        
    # 打印信号统计
    print("交易信号统计:")
    for config_name, signal_counts in report['signal_statistics'].items():
        print(f"  {config_name}: {signal_counts}")
    print()
    
    # 保存报告到文件
    output_dir = './aoi_analysis_output'
    os.makedirs(output_dir, exist_ok=True)
    
    report_file = os.path.join(output_dir, 'aoi_analysis_report.json')
    with open(report_file, 'w', encoding='utf-8') as f:
        json.dump(report, f, ensure_ascii=False, indent=2, default=str)
    logger.info(f"详细报告已保存: {report_file}")
    
    # 保存数据
    aoi_data_file = os.path.join(output_dir, 'aoi_data.csv')
    aoi_df.to_csv(aoi_data_file, index=False, encoding='utf-8')
    logger.info(f"AOI数据已保存: {aoi_data_file}")
    
    signals_data_file = os.path.join(output_dir, 'signals_data.csv')
    signals_df.to_csv(signals_data_file, index=False, encoding='utf-8')
    logger.info(f"信号数据已保存: {signals_data_file}")
    
    print(f"所有分析结果已保存到: {output_dir}")

if __name__ == "__main__":
    analyze_aoi_with_mock_data()
