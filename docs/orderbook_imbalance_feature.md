# 交易指标功能

## 功能概述

在FlowSight交易界面的右侧新增了两个高度为100像素的指标区域：
1. **Orderbook Imbalance指标** - 实时展示订单簿的多空压力情况
2. **Trade Imbalance指标** - 基于500ms滑动窗口的交易不平衡分析

## 功能特性

## Orderbook Imbalance指标

### 1. 实时数据显示
- **买单占比**: 显示当前最佳买单挂单量占总挂单量的百分比
- **卖单占比**: 显示当前最佳卖单挂单量占总挂单量的百分比
- **数据来源**: 从OrderBookManager的市场快照中获取bid_volume_ratio和ask_volume_ratio

### 2. 可视化条形图
- **横向条形图**: 使用横向条形图直观展示买卖双方的力量对比
- **买单条形图**: 蓝色条形图从左侧开始，长度代表买单占比
- **卖单条形图**: 红色条形图从右侧开始，长度代表卖单占比
- **中心分割线**: 白色分割线标示50%平衡点

### 3. 多空压力指示
- **多头压力** (🟢): 当买单占比 - 卖单占比 > 10%时显示
- **空头压力** (🔴): 当买单占比 - 卖单占比 < -10%时显示  
- **均衡状态** (⚪): 当差值在±10%范围内时显示
- **差值显示**: 实时显示买卖占比的具体差值

### 4. 界面设计
- **固定高度**: 100像素固定高度，不影响其他组件布局
- **深色主题**: 深色背景配色，与整体界面风格一致
- **清晰边框**: 带有边框和内边距，视觉层次分明
- **左对齐布局**: 与上方价格图表左边对齐，形成统一的视觉效果
- **无左边距**: 移除左边距以实现与价格图表的完美对齐
- **颜色方案**:
  - 买单: 蓝色 (120, 180, 255)
  - 卖单: 红色 (255, 120, 120)
  - 多头压力: 绿色 (120, 255, 120)
  - 空头压力: 红色 (255, 120, 120)

## Trade Imbalance指标

### 1. 计算公式
- **TI = (#BuyTrades - #SellTrades) / TotalTrades**
- **滑动窗口**: 500毫秒实时滑动窗口
- **数据来源**: 实时交易流数据，区分买单和卖单

### 2. 可视化展示
- **横向条形图**: 以中心线为基准的双向条形图
- **正值显示**: 绿色条形图向右延伸，表示买单多于卖单
- **负值显示**: 红色条形图向左延伸，表示卖单多于买单
- **刻度标记**: -1, -0.5, 0, 0.5, 1的刻度线标记
- **中心分割线**: 白色粗线标示平衡点

### 3. 交易压力分级
- **强买压** (🟢): TI > 0.3，绿色显示
- **轻买压** (🟡): 0.1 < TI ≤ 0.3，黄色显示
- **均衡** (⚪): -0.1 ≤ TI ≤ 0.1，灰色显示
- **轻卖压** (🟠): -0.3 ≤ TI < -0.1，橙色显示
- **强卖压** (🔴): TI < -0.3，红色显示

### 4. 界面设计
- **固定高度**: 100像素固定高度
- **左对齐布局**: 与Orderbook Imbalance指标左边对齐
- **实时更新**: 每笔交易后立即更新500ms窗口数据
- **数值显示**: 显示精确的TI值和百分比

## 布局优化

### 对齐改进
为了提供更好的视觉体验，进行了以下布局优化：

1. **价格图表margin移除**: 尝试移除价格图表的内置margin（注：egui Plot组件不支持margin方法）
2. **指标区域左对齐**:
   - 移除左边距 (`left: 0.0`)
   - 保持其他边距 (`right: 8.0, top: 8.0, bottom: 8.0`)
   - 实现与上方价格图表的左边对齐

### 视觉一致性
- 三个组件（价格图表、Orderbook Imbalance、Trade Imbalance）形成统一的左对齐布局
- 消除视觉上的不协调感
- 提供更专业的交易界面体验

### 高度分配
- **价格图表**: 动态高度（总高度 - 200像素）
- **Orderbook Imbalance**: 固定100像素
- **Trade Imbalance**: 固定100像素

## 技术实现

### 数据流程

#### Orderbook Imbalance数据流程
1. WebSocket接收订单簿深度数据
2. OrderBookManager处理数据并计算bid_volume_ratio和ask_volume_ratio
3. 数据存储在MarketSnapshot中
4. GUI组件通过app.get_market_snapshot()获取实时数据
5. render_orderbook_imbalance()方法渲染可视化界面

#### Trade Imbalance数据流程
1. WebSocket接收实时交易数据
2. OrderBookManager.handle_trade()处理交易数据
3. 调用update_trade_imbalance()更新500ms滑动窗口
4. 计算TI值：(#BuyTrades - #SellTrades) / TotalTrades
5. GUI组件通过get_trade_imbalance()获取实时TI值
6. render_trade_imbalance()方法渲染可视化界面

### 关键代码位置

#### Orderbook Imbalance
- **数据计算**: `src/orderbook/manager.rs` - `calculate_volume_ratio()`方法
- **GUI渲染**: `src/gui/unified_orderbook_widget.rs` - `render_orderbook_imbalance()`方法

#### Trade Imbalance
- **数据计算**: `src/orderbook/manager.rs` - `update_trade_imbalance()`方法
- **滑动窗口**: `trade_imbalance_window: VecDeque<(u64, bool)>`
- **GUI渲染**: `src/gui/unified_orderbook_widget.rs` - `render_trade_imbalance()`方法

#### 布局集成
- **价格图表**: 右侧上部，动态高度
- **Orderbook Imbalance**: 右侧中部，固定100像素高度
- **Trade Imbalance**: 右侧下部，固定100像素高度

### 性能优化
- **Orderbook Imbalance**: 使用实时市场快照数据，避免重复计算
- **Trade Imbalance**: 使用VecDeque高效管理500ms滑动窗口
- **内存管理**: 自动清理过期数据，防止内存泄漏
- **轻量级渲染**: 不影响主要订单簿表格性能
- **固定布局尺寸**: 避免动态布局计算开销

## 使用说明

1. 启动FlowSight应用程序
2. 连接到Binance WebSocket数据流
3. 在界面右侧下方可以看到"📊 Orderbook Imbalance"指标面板
4. 观察横向条形图的变化来判断市场多空压力
5. 参考压力指示文字来辅助交易决策

## 应用场景

- **入场时机**: 当出现明显多头或空头压力时，可作为入场信号参考
- **风险控制**: 均衡状态时谨慎操作，避免在不确定市场中交易
- **趋势确认**: 结合价格走势，验证多空力量是否与价格方向一致
- **反转信号**: 极端不平衡后的快速回归可能预示短期反转
