"""
AOI (Active Order Imbalance) 指标分析工具
实现主动成交不平衡指标的计算、可视化和回测分析
"""

import sys
import os
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from datetime import datetime, timedelta
import logging
from typing import Dict, List, Optional, Tuple, Union
from connect_mongodb import MongoDBConnector

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
        """
        添加交易数据

        参数:
            timestamp: 时间戳(毫秒)
            price: 价格
            quantity: 数量
            side: 交易方向 ('buy' 或 'sell')
        """
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
        """
        计算AOI和Delta Volume

        返回:
            Tuple[float, float]: (AOI值, Delta Volume)
        """
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

class AOIAnalyzer:
    """AOI指标分析器 - 提供数据获取、分析和可视化功能"""

    def __init__(self, mongo_connector: MongoDBConnector):
        """
        初始化分析器

        参数:
            mongo_connector: MongoDB连接器
        """
        self.mongo = mongo_connector
        self.indicators = {}  # 存储不同配置的指标计算器

    def load_trade_data(self, symbol: str, start_time: datetime, end_time: datetime, use_objectid: bool = True) -> pd.DataFrame:
        """
        从MongoDB加载交易数据

        参数:
            symbol: 交易对符号
            start_time: 开始时间
            end_time: 结束时间
            use_objectid: 是否使用ObjectId进行时间查询（默认True，性能更好）

        返回:
            pd.DataFrame: 交易数据
        """
        logger.info(f"加载 {symbol} 交易数据: {start_time} 到 {end_time}")

        # 直接查询MongoDB集合
        collection_name = f"{symbol}_trades"
        collection = self.mongo.get_collection(collection_name)

        # 构建查询条件
        if use_objectid:
            # 使用ObjectId进行时间查询（利用ObjectId内置的时间戳，性能更好）
            from bson import ObjectId

            # ObjectId包含时间戳，直接用于时间范围查询
            start_objectid = ObjectId.from_datetime(start_time)
            end_objectid = ObjectId.from_datetime(end_time)

            query = {
                "_id": {
                    "$gte": start_objectid,
                    "$lte": end_objectid
                }
            }

            logger.info(f"使用ObjectId查询，时间范围: {start_time} 到 {end_time}")
            logger.info(f"ObjectId范围: {start_objectid} 到 {end_objectid}")

        else:
            # 使用T字段进行时间查询
            # 为T字段创建索引以提高查询性能
            try:
                logger.info("检查并创建T字段索引...")
                collection.create_index("T", background=True)
                logger.info("T字段索引创建完成")
            except Exception as e:
                logger.warning(f"索引创建失败或已存在: {e}")

            start_timestamp = int(start_time.timestamp() * 1000)
            end_timestamp = int(end_time.timestamp() * 1000)

            query = {
                "T": {
                    "$gte": start_timestamp,
                    "$lte": end_timestamp
                }
            }

            logger.info(f"使用T字段查询，时间范围: {start_time} 到 {end_time}")
            logger.info(f"时间戳范围: {start_timestamp} 到 {end_timestamp}")

        # 先统计符合条件的记录数
        total_count = collection.count_documents(query)
        logger.info(f"符合条件的记录总数: {total_count:,}")

        if total_count == 0:
            logger.warning("未找到符合条件的交易数据")
            return pd.DataFrame()

        # 查询数据
        logger.info("开始查询数据...")
        if use_objectid:
            cursor = collection.find(query).sort("_id", 1)  # 按ObjectId排序（天然时间顺序）
        else:
            cursor = collection.find(query).sort("T", 1)   # 按T字段排序

        trades = list(cursor)
        logger.info("数据查询完成")

        if not trades:
            logger.warning("未找到交易数据")
            return pd.DataFrame()

        logger.info(f"从MongoDB获取到 {len(trades)} 条原始交易记录")

        # 转换为DataFrame
        df = pd.DataFrame(trades)

        # 数据清洗和格式化
        if 'T' in df.columns:
            df['ts'] = df['T']  # 保持兼容性
            df['timestamp'] = pd.to_datetime(df['T'], unit='ms')
            df = df.sort_values('timestamp')

        # 映射字段名称以适配现有代码
        if 'p' in df.columns:
            df['price'] = df['p'].astype(float)
        if 'q' in df.columns:
            df['qty'] = df['q'].astype(float)

        # 根据'm'字段确定交易方向
        # m=true表示买方成交(taker买入)，m=false表示卖方成交(taker卖出)
        if 'm' in df.columns:
            df['side'] = df['m'].apply(lambda x: 'buy' if x else 'sell')

        # 确保必要的列存在
        required_columns = ['price', 'qty', 'side', 'ts']
        missing_columns = [col for col in required_columns if col not in df.columns]
        if missing_columns:
            logger.error(f"缺少必要的列: {missing_columns}")
            logger.info(f"可用的列: {list(df.columns)}")
            return pd.DataFrame()

        # 数据类型转换
        df['price'] = pd.to_numeric(df['price'], errors='coerce')
        df['qty'] = pd.to_numeric(df['qty'], errors='coerce')

        # 移除无效数据
        df = df.dropna(subset=['price', 'qty'])

        logger.info(f"成功加载 {len(df)} 条有效交易记录")
        logger.info(f"价格范围: {df['price'].min():.2f} - {df['price'].max():.2f}")
        logger.info(f"数量范围: {df['qty'].min():.6f} - {df['qty'].max():.6f}")
        logger.info(f"买单比例: {(df['side'] == 'buy').mean():.2%}")

        return df

    def calculate_aoi_series(self, trades_df: pd.DataFrame,
                           window_configs: List[Dict]) -> pd.DataFrame:
        """
        计算多种窗口配置的AOI时间序列

        参数:
            trades_df: 交易数据DataFrame
            window_configs: 窗口配置列表，例如:
                [{'type': 'time', 'size': 60}, {'type': 'tick', 'size': 100}]

        返回:
            pd.DataFrame: 包含各种AOI指标的时间序列
        """
        if trades_df.empty:
            return pd.DataFrame()

        results = []

        for config in window_configs:
            config_name = f"{config['type']}_{config['size']}"
            logger.info(f"计算AOI指标: {config_name}")

            # 创建指标计算器
            indicator = AOIIndicator(config['type'], config['size'])
            self.indicators[config_name] = indicator

            # 逐行处理交易数据，添加进度显示
            total_trades = len(trades_df)
            progress_interval = max(1, total_trades // 100)  # 每1%显示进度

            for i, (_, trade) in enumerate(trades_df.iterrows()):
                indicator.add_trade(
                    timestamp=int(trade['ts']),
                    price=float(trade['price']),
                    quantity=float(trade['qty']),
                    side=trade['side']
                )

                # 每隔一定间隔记录结果（减少内存使用）
                if i % 1000 == 0 or i == total_trades - 1:  # 每1000条或最后一条记录
                    results.append({
                        'timestamp': trade['ts'],
                        'datetime': trade['timestamp'] if 'timestamp' in trade else pd.to_datetime(trade['ts'], unit='ms'),
                        'price': trade['price'],
                        f'aoi_{config_name}': indicator.get_current_aoi(),
                        f'delta_volume_{config_name}': indicator.get_current_delta_volume(),
                        'config': config_name
                    })

                # 显示进度
                if i % progress_interval == 0:
                    progress = (i + 1) / total_trades * 100
                    logger.info(f"  处理进度: {progress:.1f}% ({i+1}/{total_trades})")

        return pd.DataFrame(results)

    def generate_signals(self, aoi_df: pd.DataFrame,
                        aoi_threshold: float = 0.6,
                        delta_threshold: float = None) -> pd.DataFrame:
        """
        基于AOI指标生成交易信号

        参数:
            aoi_df: AOI数据DataFrame
            aoi_threshold: AOI阈值
            delta_threshold: Delta Volume阈值

        返回:
            pd.DataFrame: 包含信号的数据
        """
        signals_df = aoi_df.copy()

        # 为每个配置生成信号
        for config_name in self.indicators.keys():
            aoi_col = f'aoi_{config_name}'
            delta_col = f'delta_volume_{config_name}'
            signal_col = f'signal_{config_name}'

            if aoi_col in signals_df.columns:
                # 基于AOI阈值生成信号
                conditions = [
                    signals_df[aoi_col] > aoi_threshold,
                    signals_df[aoi_col] < -aoi_threshold,
                ]
                choices = ['BUY', 'SELL']
                signals_df[signal_col] = np.select(conditions, choices, default='HOLD')

                # 如果设置了delta阈值，进一步过滤信号
                if delta_threshold is not None and delta_col in signals_df.columns:
                    buy_mask = (signals_df[signal_col] == 'BUY') & (signals_df[delta_col] < delta_threshold)
                    sell_mask = (signals_df[signal_col] == 'SELL') & (signals_df[delta_col] > -delta_threshold)
                    signals_df.loc[buy_mask | sell_mask, signal_col] = 'HOLD'

        return signals_df

    def create_visualizations(self, aoi_df: pd.DataFrame,
                            signals_df: pd.DataFrame = None,
                            save_path: str = None) -> Dict:
        """
        创建AOI指标可视化图表

        参数:
            aoi_df: AOI数据
            signals_df: 信号数据
            save_path: 保存路径

        返回:
            Dict: 图表对象字典
        """
        charts = {}

        # 1. AOI时间序列图
        fig_aoi = self._create_aoi_timeseries_chart(aoi_df)
        charts['aoi_timeseries'] = fig_aoi

        # 2. Delta Volume图
        fig_delta = self._create_delta_volume_chart(aoi_df)
        charts['delta_volume'] = fig_delta

        # 3. AOI分布直方图
        fig_dist = self._create_aoi_distribution_chart(aoi_df)
        charts['aoi_distribution'] = fig_dist

        # 4. 如果有信号数据，创建信号图
        if signals_df is not None:
            fig_signals = self._create_signals_chart(signals_df)
            charts['signals'] = fig_signals

        # 5. 相关性热力图
        fig_corr = self._create_correlation_heatmap(aoi_df)
        charts['correlation'] = fig_corr

        # 保存图表
        if save_path:
            self._save_charts(charts, save_path)

        return charts

    def _create_aoi_timeseries_chart(self, aoi_df: pd.DataFrame):
        """创建AOI时间序列图"""
        fig, axes = plt.subplots(len(self.indicators), 1, figsize=(15, 4*len(self.indicators)))
        if len(self.indicators) == 1:
            axes = [axes]

        for i, config_name in enumerate(self.indicators.keys()):
            aoi_col = f'aoi_{config_name}'
            if aoi_col in aoi_df.columns:
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
        return fig

    def _create_delta_volume_chart(self, aoi_df: pd.DataFrame):
        """创建Delta Volume图"""
        fig, axes = plt.subplots(len(self.indicators), 1, figsize=(15, 4*len(self.indicators)))
        if len(self.indicators) == 1:
            axes = [axes]

        for i, config_name in enumerate(self.indicators.keys()):
            delta_col = f'delta_volume_{config_name}'
            if delta_col in aoi_df.columns:
                config_data = aoi_df[aoi_df['config'] == config_name]

                # 绘制Delta Volume柱状图
                colors = ['red' if x > 0 else 'green' for x in config_data[delta_col]]
                axes[i].bar(config_data['datetime'], config_data[delta_col],
                          color=colors, alpha=0.7, width=0.8)
                axes[i].axhline(y=0, color='black', linestyle='-', alpha=0.5)

                axes[i].set_title(f'Delta Volume - {config_name}')
                axes[i].set_ylabel('Delta Volume')
                axes[i].grid(True, alpha=0.3)

        plt.tight_layout()
        return fig

    def _create_aoi_distribution_chart(self, aoi_df: pd.DataFrame):
        """创建AOI分布直方图"""
        fig, axes = plt.subplots(1, len(self.indicators), figsize=(5*len(self.indicators), 6))
        if len(self.indicators) == 1:
            axes = [axes]

        for i, config_name in enumerate(self.indicators.keys()):
            aoi_col = f'aoi_{config_name}'
            if aoi_col in aoi_df.columns:
                config_data = aoi_df[aoi_df['config'] == config_name]

                axes[i].hist(config_data[aoi_col], bins=50, alpha=0.7,
                           color='skyblue', edgecolor='black')
                axes[i].axvline(x=0, color='black', linestyle='-', alpha=0.5)
                axes[i].axvline(x=0.6, color='red', linestyle='--', alpha=0.7)
                axes[i].axvline(x=-0.6, color='green', linestyle='--', alpha=0.7)

                axes[i].set_title(f'AOI分布 - {config_name}')
                axes[i].set_xlabel('AOI值')
                axes[i].set_ylabel('频次')
                axes[i].grid(True, alpha=0.3)

        plt.tight_layout()
        return fig

    def _create_signals_chart(self, signals_df: pd.DataFrame):
        """创建交易信号图"""
        fig, axes = plt.subplots(len(self.indicators), 1, figsize=(15, 4*len(self.indicators)))
        if len(self.indicators) == 1:
            axes = [axes]

        for i, config_name in enumerate(self.indicators.keys()):
            signal_col = f'signal_{config_name}'
            aoi_col = f'aoi_{config_name}'

            if signal_col in signals_df.columns and aoi_col in signals_df.columns:
                config_data = signals_df[signals_df['config'] == config_name]

                # 绘制价格线
                axes[i].plot(config_data['datetime'], config_data['price'],
                           label='价格', color='black', alpha=0.7)

                # 标记买卖信号
                buy_signals = config_data[config_data[signal_col] == 'BUY']
                sell_signals = config_data[config_data[signal_col] == 'SELL']

                if not buy_signals.empty:
                    axes[i].scatter(buy_signals['datetime'], buy_signals['price'],
                                  color='red', marker='^', s=50, label='买入信号', alpha=0.8)

                if not sell_signals.empty:
                    axes[i].scatter(sell_signals['datetime'], sell_signals['price'],
                                  color='green', marker='v', s=50, label='卖出信号', alpha=0.8)

                axes[i].set_title(f'交易信号 - {config_name}')
                axes[i].set_ylabel('价格')
                axes[i].legend()
                axes[i].grid(True, alpha=0.3)

        plt.tight_layout()
        return fig

    def _create_correlation_heatmap(self, aoi_df: pd.DataFrame):
        """创建相关性热力图"""
        # 提取所有AOI列
        aoi_columns = [col for col in aoi_df.columns if col.startswith('aoi_')]

        if len(aoi_columns) < 2:
            return None

        # 创建透视表
        pivot_data = {}
        for config_name in self.indicators.keys():
            config_data = aoi_df[aoi_df['config'] == config_name]
            if not config_data.empty:
                pivot_data[config_name] = config_data[f'aoi_{config_name}'].values

        if len(pivot_data) < 2:
            return None

        # 计算相关性矩阵
        corr_df = pd.DataFrame(pivot_data).corr()

        # 绘制热力图
        fig, ax = plt.subplots(figsize=(8, 6))
        im = ax.imshow(corr_df.values, cmap='coolwarm', aspect='auto', vmin=-1, vmax=1)

        # 设置标签
        ax.set_xticks(range(len(corr_df.columns)))
        ax.set_yticks(range(len(corr_df.index)))
        ax.set_xticklabels(corr_df.columns, rotation=45)
        ax.set_yticklabels(corr_df.index)

        # 添加数值标注
        for i in range(len(corr_df.index)):
            for j in range(len(corr_df.columns)):
                text = ax.text(j, i, f'{corr_df.iloc[i, j]:.2f}',
                             ha="center", va="center", color="black")

        ax.set_title('AOI指标相关性热力图')
        plt.colorbar(im)
        plt.tight_layout()
        return fig

    def _save_charts(self, charts: Dict, save_path: str):
        """保存图表到文件"""
        import os
        os.makedirs(save_path, exist_ok=True)

        for chart_name, fig in charts.items():
            if fig is not None:
                file_path = os.path.join(save_path, f'{chart_name}.png')
                fig.savefig(file_path, dpi=300, bbox_inches='tight')
                logger.info(f"图表已保存: {file_path}")

    def generate_report(self, aoi_df: pd.DataFrame,
                       signals_df: pd.DataFrame = None) -> Dict:
        """
        生成AOI分析报告

        参数:
            aoi_df: AOI数据
            signals_df: 信号数据

        返回:
            Dict: 分析报告
        """
        report = {
            'analysis_time': datetime.now().strftime('%Y-%m-%d %H:%M:%S'),
            'data_period': {
                'start': aoi_df['datetime'].min().strftime('%Y-%m-%d %H:%M:%S'),
                'end': aoi_df['datetime'].max().strftime('%Y-%m-%d %H:%M:%S'),
                'duration_hours': (aoi_df['datetime'].max() - aoi_df['datetime'].min()).total_seconds() / 3600
            },
            'configurations': {},
            'summary': {}
        }

        # 为每个配置生成统计信息
        for config_name, indicator in self.indicators.items():
            stats = indicator.get_statistics()
            config_data = aoi_df[aoi_df['config'] == config_name]

            report['configurations'][config_name] = {
                'window_type': indicator.window_type,
                'window_size': indicator.window_size,
                'total_samples': len(config_data),
                'statistics': stats
            }

        # 生成信号统计
        if signals_df is not None:
            signal_stats = {}
            for config_name in self.indicators.keys():
                signal_col = f'signal_{config_name}'
                if signal_col in signals_df.columns:
                    config_signals = signals_df[signals_df['config'] == config_name]
                    signal_counts = config_signals[signal_col].value_counts()
                    signal_stats[config_name] = signal_counts.to_dict()

            report['signal_statistics'] = signal_stats

        return report

def main():
    """主函数 - 演示AOI指标分析"""
    try:
        # 连接MongoDB
        mongo = MongoDBConnector(
            host='localhost',
            port=27017,
            db_name='crypto_data'
        )

        # 创建分析器 
        analyzer = AOIAnalyzer(mongo)

        # 设置分析参数
        symbol = 'btcusdt'  # 对应集合名称 btcusdt_trades
        end_time = datetime.now()
        start_time = end_time - timedelta(days=3)  # 分析最近3天数据

        # 定义多种窗口配置
        window_configs = [
            {'type': 'time', 'size': 60},    # 60秒时间窗口
            {'type': 'time', 'size': 300},   # 5分钟时间窗口
            {'type': 'tick', 'size': 100},   # 100笔交易窗口
            {'type': 'tick', 'size': 500},   # 500笔交易窗口
        ]

        logger.info("开始AOI指标分析...")

        # 1. 加载交易数据
        trades_df = analyzer.load_trade_data(symbol, start_time, end_time)

        if trades_df.empty:
            logger.error("没有找到交易数据，请检查数据库连接和数据")
            return

        logger.info(f"加载了 {len(trades_df)} 条交易记录")

        # 2. 计算AOI指标
        aoi_df = analyzer.calculate_aoi_series(trades_df, window_configs)

        if aoi_df.empty:
            logger.error("AOI计算失败")
            return

        logger.info("AOI指标计算完成")

        # 3. 生成交易信号
        signals_df = analyzer.generate_signals(aoi_df, aoi_threshold=0.6)
        logger.info("交易信号生成完成")

        # 4. 创建可视化图表
        charts = analyzer.create_visualizations(aoi_df, signals_df, save_path='./aoi_analysis_output')
        logger.info("可视化图表创建完成")

        # 5. 生成分析报告
        report = analyzer.generate_report(aoi_df, signals_df)

        # 6. 打印报告摘要
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
        if 'signal_statistics' in report:
            print("交易信号统计:")
            for config_name, signal_counts in report['signal_statistics'].items():
                print(f"  {config_name}: {signal_counts}")
            print()

        # 7. 保存报告到文件
        import json
        import os

        output_dir = './aoi_analysis_output'
        os.makedirs(output_dir, exist_ok=True)

        # 保存详细报告
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
        print("分析完成！")

        # 显示图表
        plt.show()

    except Exception as e:
        logger.error(f"分析过程中发生错误: {str(e)}")
        import traceback
        traceback.print_exc()
    finally:
        # 关闭MongoDB连接
        if 'mongo' in locals():
            mongo.close()

if __name__ == "__main__":
    main()