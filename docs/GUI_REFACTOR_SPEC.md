# FlowSight 完全重构方案规格文档
## 从Rust桌面应用到纯Next.js Web应用的完整重构

### 项目概述

**项目名称：** FlowSight Web  
**版本：** 2.0.0  
**重构目标：** 完全替换Rust架构，使用纯Next.js技术栈重新构建专业交易分析平台  
**技术栈：** Next.js 14 + TypeScript + Node.js + WebSocket + 实时数据可视化  
**架构模式：** 全栈JavaScript/TypeScript，无Rust依赖  

---

## 1. 项目分析与体检报告

### 1.1 重构驱动因素分析

#### 现有Rust架构的优势
- ✅ **高性能数据处理**：基于Rust的无锁事件系统，亚毫秒级延迟
- ✅ **实时数据流**：完善的WebSocket连接管理和自动重连机制
- ✅ **专业数据分析**：订单流分析、高频波动率、价格跳跃检测等专业功能
- ✅ **稳定的数据结构**：成熟的订单簿管理和事件驱动架构
- ✅ **丰富的性能监控**：完整的性能指标和健康监控系统

#### 重构的必要性
- 🔄 **技术栈统一**：使用JavaScript/TypeScript统一前后端开发
- 🌐 **Web原生优势**：无需安装，跨平台兼容，自动更新
- 👥 **团队协作效率**：降低技术门槛，更多开发者可以参与
- 🚀 **快速迭代**：Web技术栈的快速开发和部署能力
- 🔧 **生态系统丰富**：JavaScript生态的丰富组件和工具支持
- 💡 **现代化体验**：Web技术提供的现代交互和用户体验

### 1.2 新架构的响应式数据驱动设计

纯Next.js架构将采用现代Web技术实现响应式数据驱动：

```typescript
// 新的响应式数据流 (纯JavaScript/TypeScript)
币安WebSocket → Next.js API → 事件总线 → React状态 → UI渲染
     ↓              ↓           ↓           ↓         ↓
  实时市场数据    服务端处理   客户端事件   状态更新   组件重渲染
```

**新架构的响应式特征：**
- 基于Node.js的事件驱动架构（EventEmitter + 自定义EventBus）
- React + Zustand的现代状态管理
- WebSocket + Server-Sent Events双重实时通信
- React的声明式UI自动更新机制
- TypeScript类型安全保障

---

## 2. Web重构技术选型

### 2.1 前端框架选择：Next.js 14

**选择理由：**
- 🚀 **服务端渲染（SSR）**：首屏加载速度优化，SEO友好
- ⚡ **App Router**：现代化的路由系统，支持嵌套布局
- 🔄 **实时数据支持**：内置WebSocket支持，完美适配实时交易数据
- 📱 **响应式设计**：天然支持多设备适配
- 🛠️ **TypeScript原生支持**：类型安全，开发体验优秀
- 🎨 **丰富的UI生态**：可集成Tailwind CSS、Framer Motion等现代UI库

### 2.2 状态管理：Zustand + React Query

**Zustand用于本地状态：**
```typescript
// 订单簿状态管理
interface OrderBookStore {
  orderFlows: Map<number, OrderFlow>
  marketSnapshot: MarketSnapshot
  connectionStatus: ConnectionStatus
  updateOrderBook: (data: DepthUpdate) => void
  updateTrade: (trade: TradeData) => void
}
```

**React Query用于服务端状态：**
- WebSocket连接管理
- 数据缓存和同步
- 错误重试机制
- 后台数据更新

### 2.3 实时通信：WebSocket + Server-Sent Events

**双重实时通信策略：**
- **WebSocket**：币安API数据接收（保持现有逻辑）
- **Server-Sent Events**：服务端到客户端的状态推送
- **HTTP/2 Push**：关键数据的主动推送

### 2.4 数据可视化：React + D3.js + Canvas

**多层次可视化方案：**
- **React组件**：基础UI和布局
- **D3.js**：复杂数据可视化和动画
- **Canvas/WebGL**：高性能图表渲染
- **Framer Motion**：流畅的UI动画

---

## 3. 架构设计

### 3.1 纯Next.js全栈架构图

```mermaid
graph TB
    subgraph "浏览器客户端"
        A[React组件层] --> B[Zustand状态管理]
        B --> C[React Query缓存]
        C --> D[WebSocket客户端]
        A --> E[D3.js数据可视化]
        A --> F[Canvas高性能渲染]
        G[Service Worker] --> A
    end
    
    subgraph "Next.js全栈服务器"
        H[App Router页面] --> I[API Routes]
        I --> J[WebSocket服务器]
        J --> K[事件处理引擎]
        K --> L[订单簿管理器]
        L --> M[数据分析引擎]
        I --> N[Server Actions]
        O[中间件] --> I
    end
    
    subgraph "数据存储层"
        P[Redis实时缓存(可选)]
        R[内存数据结构]
        S[LRU缓存]
    end
    
    subgraph "外部API"
        AA[币安WebSocket API]
        BB[币安REST API]
    end
    
    subgraph "基础设施"
        U[Docker容器]
        V[Nginx负载均衡]
        W[监控告警]
    end
    
    D --> J
    J --> AA
    K --> P
    M --> R
    I --> BB
    
    U --> H
    V --> U
    W --> K
```

### 3.2 数据流设计

```typescript
// 响应式数据流
interface DataFlow {
  // 1. 数据接收层
  binanceWebSocket: WebSocketConnection
  
  // 2. 数据处理层
  eventProcessor: EventProcessor
  orderBookManager: OrderBookManager
  
  // 3. 状态管理层
  globalStore: GlobalStore
  orderBookStore: OrderBookStore
  
  // 4. UI渲染层
  components: ReactComponent[]
  visualizations: D3Visualization[]
}
```

### 3.3 组件架构

**界面布局设计：**
```
┌─────────────────────────────────────────────────────────────┐
│                    顶部状态栏 (5%)                           │
├─────────────────────┬───────────────────────────────────────┤
│                     │          右上：实时主动订单           │
│                     │        线型图 + 动态圆点显示          │
│    左侧：订单簿      │         (圆点大小 = 成交量)          │
│   (50% 宽度)        │              (45% 高度)              │
│                     ├───────────────────────────────────────┤
│                     │                                       │
│                     │        右下：Footprint足迹图          │
│                     │      5分钟K线内价格层级累计显示        │
│                     │              (50% 高度)              │
└─────────────────────┴───────────────────────────────────────┘
```

**组件架构：**
```
NEXTJS_REFACTOR/src/
├── app/                          # Next.js App Router
│   ├── layout.tsx               # 全局布局
│   ├── page.tsx                 # 主页面
│   └── api/                     # API路由
│       ├── websocket/           # WebSocket处理
│       └── market-data/         # 市场数据API
├── components/                   # React组件
│   ├── layout/                  # 布局组件
│   │   ├── MainLayout.tsx       # 主布局容器
│   │   ├── StatusBar.tsx        # 顶部状态栏
│   │   └── GridLayout.tsx       # 网格布局管理
│   ├── trading/                 # 交易相关组件
│   │   ├── OrderBookPanel.tsx   # 左侧订单簿面板
│   │   ├── ActiveOrderChart.tsx # 右上实时主动订单图表
│   │   └── FootprintChart.tsx   # 右下足迹图
│   ├── charts/                  # 图表组件
│   │   ├── LineChart.tsx        # 线型图基础组件
│   │   ├── DotChart.tsx         # 动态圆点图
│   │   ├── CandlestickChart.tsx # K线图组件
│   │   └── HeatmapChart.tsx     # 热力图组件
│   ├── ui/                      # 基础UI组件
│   └── visualizations/          # 数据可视化组件
├── lib/                         # 核心逻辑
│   ├── websocket/               # WebSocket管理
│   ├── data-processing/         # 数据处理
│   │   ├── footprint/           # 足迹图数据处理
│   │   ├── kline/               # K线数据处理
│   │   └── volume/              # 成交量分析
│   ├── stores/                  # 状态管理
│   └── utils/                   # 工具函数
├── types/                       # TypeScript类型定义
│   ├── trading.ts               # 交易相关类型
│   ├── charts.ts                # 图表相关类型
│   └── footprint.ts             # 足迹图类型
└── styles/                      # 样式文件
    ├── layout.css               # 布局样式
    ├── charts.css               # 图表样式
    └── components.css           # 组件样式
```

---

## 4. 纯Next.js核心功能实现方案

### 4.0 技术栈详细说明

**前端技术栈：**
- **Next.js 14**: App Router + Server Components + Client Components
- **React 18**: Concurrent Features + Suspense + Error Boundaries
- **TypeScript 5**: 严格类型检查 + 最新语法特性
- **Tailwind CSS**: 原子化CSS + 响应式设计
- **Framer Motion**: 高性能动画库
- **Zustand**: 轻量级状态管理
- **React Query**: 服务端状态管理和缓存

**后端技术栈（纯内存架构）：**
- **Node.js 18+**: 现代JavaScript运行时
- **Next.js API Routes**: RESTful API支持
- **WebSocket (ws)**: 实时双向通信
- **内存数据结构**: Map、Set、Array等原生数据结构存储实时数据
- **LRU Cache**: 内存缓存管理，自动清理过期数据
- **EventEmitter**: Node.js原生事件系统

**数据存储策略（无数据库）：**
- **实时数据**: 完全存储在内存中，重启后重新获取
- **历史数据**: 仅保留短期历史（如最近1000笔交易）用于技术指标计算
- **数据清理**: 定时清理过期数据，防止内存泄漏
- **状态恢复**: 应用重启后自动从币安API重新获取最新数据

**数据可视化技术栈：**
- **D3.js**: 复杂数据可视化
- **Canvas API**: 高性能图表渲染
- **WebGL**: GPU加速渲染
- **Chart.js**: 标准图表组件
- **React-Window**: 虚拟化长列表

### 4.1 主布局组件实现

```typescript
// components/layout/MainLayout.tsx - 三区域主布局
import React from 'react'
import { StatusBar } from './StatusBar'
import { OrderBookPanel } from '../trading/OrderBookPanel'
import { ActiveOrderChart } from '../trading/ActiveOrderChart'
import { FootprintChart } from '../trading/FootprintChart'

interface MainLayoutProps {
  symbol: string
}

export const MainLayout: React.FC<MainLayoutProps> = ({ symbol }) => {
  return (
    <div className="h-screen flex flex-col bg-black text-white">
      {/* 顶部状态栏 - 5% 高度 */}
      <div className="h-[5%] border-b border-gray-800">
        <StatusBar symbol={symbol} />
      </div>
      
      {/* 主内容区域 - 95% 高度 */}
      <div className="h-[95%] flex">
        {/* 左侧订单簿 - 50% 宽度 */}
        <div className="w-1/2 border-r border-gray-800">
          <OrderBookPanel symbol={symbol} />
        </div>
        
        {/* 右侧双区域 - 50% 宽度 */}
        <div className="w-1/2 flex flex-col">
          {/* 右上：实时主动订单图表 - 45% 高度 */}
          <div className="h-[45%] border-b border-gray-800 p-2">
            <ActiveOrderChart symbol={symbol} />
          </div>
          
          {/* 右下：足迹图 - 55% 高度 */}
          <div className="h-[55%] p-2">
            <FootprintChart symbol={symbol} />
          </div>
        </div>
      </div>
    </div>
  )
}
```

### 4.2 实时主动订单图表组件

```typescript
// components/trading/ActiveOrderChart.tsx - 线型图 + 动态圆点
import React, { useRef, useEffect, useMemo } from 'react'
import * as d3 from 'd3'
import { useOrderBookStore } from '@/lib/stores/orderbook-store'

interface ActiveOrderChartProps {
  symbol: string
}

interface TradePoint {
  timestamp: number
  price: number
  volume: number
  side: 'buy' | 'sell'
}

export const ActiveOrderChart: React.FC<ActiveOrderChartProps> = ({ symbol }) => {
  const svgRef = useRef<SVGSVGElement>(null)
  const { tradeHistory, currentPrice } = useOrderBookStore()
  
  // 处理交易数据，保留最近5分钟
  const recentTrades = useMemo(() => {
    const now = Date.now()
    const fiveMinutesAgo = now - 5 * 60 * 1000
    
    return tradeHistory
      .filter(trade => trade.timestamp >= fiveMinutesAgo)
      .map(trade => ({
        timestamp: trade.timestamp,
        price: trade.price,
        volume: trade.volume,
        side: trade.side
      }))
  }, [tradeHistory])

  useEffect(() => {
    if (!svgRef.current || recentTrades.length === 0) return

    const svg = d3.select(svgRef.current)
    const margin = { top: 20, right: 30, bottom: 40, left: 60 }
    const width = 400 - margin.left - margin.right
    const height = 200 - margin.bottom - margin.top

    // 清除之前的内容
    svg.selectAll('*').remove()

    const g = svg
      .append('g')
      .attr('transform', `translate(${margin.left},${margin.top})`)

    // 设置比例尺
    const xScale = d3.scaleTime()
      .domain(d3.extent(recentTrades, d => new Date(d.timestamp)) as [Date, Date])
      .range([0, width])

    const yScale = d3.scaleLinear()
      .domain(d3.extent(recentTrades, d => d.price) as [number, number])
      .nice()
      .range([height, 0])

    // 成交量比例尺（用于圆点大小）
    const volumeScale = d3.scaleSqrt()
      .domain([0, d3.max(recentTrades, d => d.volume) || 1])
      .range([2, 15]) // 圆点半径范围

    // 绘制价格线
    const line = d3.line<TradePoint>()
      .x(d => xScale(new Date(d.timestamp)))
      .y(d => yScale(d.price))
      .curve(d3.curveMonotoneX)

    // 分别绘制买单和卖单的线
    const buyTrades = recentTrades.filter(d => d.side === 'buy')
    const sellTrades = recentTrades.filter(d => d.side === 'sell')

    if (buyTrades.length > 1) {
      g.append('path')
        .datum(buyTrades)
        .attr('fill', 'none')
        .attr('stroke', '#10B981') // 绿色买单线
        .attr('stroke-width', 2)
        .attr('d', line)
    }

    if (sellTrades.length > 1) {
      g.append('path')
        .datum(sellTrades)
        .attr('fill', 'none')
        .attr('stroke', '#EF4444') // 红色卖单线
        .attr('stroke-width', 2)
        .attr('d', line)
    }

    // 绘制动态圆点（成交量大小）
    g.selectAll('.trade-dot')
      .data(recentTrades)
      .enter()
      .append('circle')
      .attr('class', 'trade-dot')
      .attr('cx', d => xScale(new Date(d.timestamp)))
      .attr('cy', d => yScale(d.price))
      .attr('r', d => volumeScale(d.volume))
      .attr('fill', d => d.side === 'buy' ? '#10B981' : '#EF4444')
      .attr('opacity', 0.7)
      .attr('stroke', d => d.side === 'buy' ? '#059669' : '#DC2626')
      .attr('stroke-width', 1)

    // 添加坐标轴
    g.append('g')
      .attr('transform', `translate(0,${height})`)
      .call(d3.axisBottom(xScale).tickFormat(d3.timeFormat('%H:%M')))
      .selectAll('text')
      .style('fill', '#9CA3AF')

    g.append('g')
      .call(d3.axisLeft(yScale).tickFormat(d3.format('.2f')))
      .selectAll('text')
      .style('fill', '#9CA3AF')

    // 添加当前价格线
    if (currentPrice) {
      g.append('line')
        .attr('x1', 0)
        .attr('x2', width)
        .attr('y1', yScale(currentPrice))
        .attr('y2', yScale(currentPrice))
        .attr('stroke', '#F59E0B')
        .attr('stroke-width', 2)
        .attr('stroke-dasharray', '5,5')
    }

  }, [recentTrades, currentPrice])

  return (
    <div className="h-full flex flex-col">
      <div className="flex justify-between items-center mb-2">
        <h3 className="text-sm font-medium text-gray-300">实时主动订单</h3>
        <div className="flex items-center space-x-4 text-xs">
          <div className="flex items-center">
            <div className="w-3 h-3 bg-green-500 rounded-full mr-1"></div>
            <span>主动买单</span>
          </div>
          <div className="flex items-center">
            <div className="w-3 h-3 bg-red-500 rounded-full mr-1"></div>
            <span>主动卖单</span>
          </div>
        </div>
      </div>
      
      <div className="flex-1">
        <svg
          ref={svgRef}
          width="100%"
          height="100%"
          viewBox="0 0 400 200"
          className="bg-gray-900 rounded"
        />
      </div>
      
      <div className="text-xs text-gray-400 mt-1">
        圆点大小 = 成交量 | 最近5分钟数据
      </div>
    </div>
  )
}
```

### 4.3 Footprint足迹图组件

```typescript
// components/trading/FootprintChart.tsx - 5分钟K线足迹图
import React, { useRef, useEffect, useMemo } from 'react'
import * as d3 from 'd3'
import { useOrderBookStore } from '@/lib/stores/orderbook-store'

interface FootprintChartProps {
  symbol: string
}

interface FootprintData {
  timestamp: number
  open: number
  high: number
  low: number
  close: number
  priceVolumes: Map<number, { buyVolume: number; sellVolume: number }>
}

interface PriceVolumeLevel {
  price: number
  buyVolume: number
  sellVolume: number
  totalVolume: number
  imbalance: number // 买卖比例
}

export const FootprintChart: React.FC<FootprintChartProps> = ({ symbol }) => {
  const svgRef = useRef<SVGSVGElement>(null)
  const { tradeHistory } = useOrderBookStore()
  
  // 生成5分钟K线足迹数据
  const footprintData = useMemo(() => {
    if (tradeHistory.length === 0) return []

    const fiveMinutes = 5 * 60 * 1000
    const now = Date.now()
    const startTime = now - 60 * 60 * 1000 // 最近1小时数据
    
    // 按5分钟分组
    const klineGroups = new Map<number, any[]>()
    
    tradeHistory
      .filter(trade => trade.timestamp >= startTime)
      .forEach(trade => {
        const klineTime = Math.floor(trade.timestamp / fiveMinutes) * fiveMinutes
        if (!klineGroups.has(klineTime)) {
          klineGroups.set(klineTime, [])
        }
        klineGroups.get(klineTime)!.push(trade)
      })

    // 生成足迹数据
    const footprints: FootprintData[] = []
    
    klineGroups.forEach((trades, timestamp) => {
      if (trades.length === 0) return

      const prices = trades.map(t => t.price)
      const open = trades[0].price
      const close = trades[trades.length - 1].price
      const high = Math.max(...prices)
      const low = Math.min(...prices)

      // 按价格层级聚合成交量
      const priceVolumes = new Map<number, { buyVolume: number; sellVolume: number }>()
      
      trades.forEach(trade => {
        // 价格聚合到0.5美元
        const aggregatedPrice = Math.round(trade.price * 2) / 2
        
        if (!priceVolumes.has(aggregatedPrice)) {
          priceVolumes.set(aggregatedPrice, { buyVolume: 0, sellVolume: 0 })
        }
        
        const level = priceVolumes.get(aggregatedPrice)!
        if (trade.side === 'buy') {
          level.buyVolume += trade.volume
        } else {
          level.sellVolume += trade.volume
        }
      })

      footprints.push({
        timestamp,
        open,
        high,
        low,
        close,
        priceVolumes
      })
    })

    return footprints.sort((a, b) => a.timestamp - b.timestamp)
  }, [tradeHistory])

  useEffect(() => {
    if (!svgRef.current || footprintData.length === 0) return

    const svg = d3.select(svgRef.current)
    const margin = { top: 20, right: 60, bottom: 40, left: 60 }
    const width = 600 - margin.left - margin.right
    const height = 300 - margin.bottom - margin.top

    // 清除之前的内容
    svg.selectAll('*').remove()

    const g = svg
      .append('g')
      .attr('transform', `translate(${margin.left},${margin.top})`)

    // 设置比例尺
    const xScale = d3.scaleBand()
      .domain(footprintData.map(d => d.timestamp.toString()))
      .range([0, width])
      .padding(0.1)

    const allPrices: number[] = []
    footprintData.forEach(d => {
      allPrices.push(d.high, d.low)
      d.priceVolumes.forEach((_, price) => allPrices.push(price))
    })

    const yScale = d3.scaleLinear()
      .domain(d3.extent(allPrices) as [number, number])
      .nice()
      .range([height, 0])

    // 获取最大成交量用于颜色比例尺
    let maxVolume = 0
    footprintData.forEach(d => {
      d.priceVolumes.forEach(level => {
        maxVolume = Math.max(maxVolume, level.buyVolume + level.sellVolume)
      })
    })

    const volumeColorScale = d3.scaleSequential(d3.interpolateBlues)
      .domain([0, maxVolume])

    // 绘制每个K线的足迹
    footprintData.forEach((kline, klineIndex) => {
      const x = xScale(kline.timestamp.toString())!
      const klineWidth = xScale.bandwidth()

      // 绘制K线框架
      const candleGroup = g.append('g')
        .attr('class', 'candle-group')

      // K线主体
      const bodyHeight = Math.abs(yScale(kline.open) - yScale(kline.close))
      const bodyY = Math.min(yScale(kline.open), yScale(kline.close))
      const isGreen = kline.close >= kline.open

      candleGroup.append('rect')
        .attr('x', x)
        .attr('y', bodyY)
        .attr('width', klineWidth)
        .attr('height', bodyHeight)
        .attr('fill', 'none')
        .attr('stroke', isGreen ? '#10B981' : '#EF4444')
        .attr('stroke-width', 1)

      // K线影线
      candleGroup.append('line')
        .attr('x1', x + klineWidth / 2)
        .attr('x2', x + klineWidth / 2)
        .attr('y1', yScale(kline.high))
        .attr('y2', yScale(kline.low))
        .attr('stroke', isGreen ? '#10B981' : '#EF4444')
        .attr('stroke-width', 1)

      // 绘制价格层级的成交量
      const priceLevels: PriceVolumeLevel[] = []
      kline.priceVolumes.forEach((volume, price) => {
        const totalVolume = volume.buyVolume + volume.sellVolume
        const imbalance = totalVolume > 0 ? volume.buyVolume / totalVolume : 0.5
        
        priceLevels.push({
          price,
          buyVolume: volume.buyVolume,
          sellVolume: volume.sellVolume,
          totalVolume,
          imbalance
        })
      })

      // 按价格排序
      priceLevels.sort((a, b) => b.price - a.price)

      // 绘制每个价格层级
      priceLevels.forEach(level => {
        if (level.totalVolume === 0) return

        const levelY = yScale(level.price)
        const levelHeight = 3 // 每个价格层级的高度

        // 背景矩形（总成交量）
        const volumeIntensity = level.totalVolume / maxVolume
        candleGroup.append('rect')
          .attr('x', x + 1)
          .attr('y', levelY - levelHeight / 2)
          .attr('width', klineWidth - 2)
          .attr('height', levelHeight)
          .attr('fill', volumeColorScale(level.totalVolume))
          .attr('opacity', 0.3)

        // 买卖量分布
        const buyWidth = (klineWidth - 2) * level.imbalance
        const sellWidth = (klineWidth - 2) * (1 - level.imbalance)

        // 买单部分（左侧，绿色）
        if (level.buyVolume > 0) {
          candleGroup.append('rect')
            .attr('x', x + 1)
            .attr('y', levelY - levelHeight / 2)
            .attr('width', buyWidth)
            .attr('height', levelHeight)
            .attr('fill', '#10B981')
            .attr('opacity', 0.6)
        }

        // 卖单部分（右侧，红色）
        if (level.sellVolume > 0) {
          candleGroup.append('rect')
            .attr('x', x + 1 + buyWidth)
            .attr('y', levelY - levelHeight / 2)
            .attr('width', sellWidth)
            .attr('height', levelHeight)
            .attr('fill', '#EF4444')
            .attr('opacity', 0.6)
        }

        // 添加成交量文本（如果空间足够）
        if (level.totalVolume > maxVolume * 0.1) {
          candleGroup.append('text')
            .attr('x', x + klineWidth / 2)
            .attr('y', levelY + 1)
            .attr('text-anchor', 'middle')
            .attr('font-size', '8px')
            .attr('fill', '#FFFFFF')
            .text(level.totalVolume.toFixed(0))
        }
      })
    })

    // 添加坐标轴
    g.append('g')
      .attr('transform', `translate(0,${height})`)
      .call(d3.axisBottom(xScale)
        .tickFormat((d, i) => {
          const timestamp = parseInt(d as string)
          return d3.timeFormat('%H:%M')(new Date(timestamp))
        })
        .tickValues(xScale.domain().filter((_, i) => i % 2 === 0)) // 只显示偶数索引的刻度
      )
      .selectAll('text')
      .style('fill', '#9CA3AF')
      .style('font-size', '10px')

    g.append('g')
      .call(d3.axisLeft(yScale).tickFormat(d3.format('.1f')))
      .selectAll('text')
      .style('fill', '#9CA3AF')
      .style('font-size', '10px')

    // 添加右侧价格轴
    g.append('g')
      .attr('transform', `translate(${width},0)`)
      .call(d3.axisRight(yScale).tickFormat(d3.format('.1f')))
      .selectAll('text')
      .style('fill', '#9CA3AF')
      .style('font-size', '10px')

  }, [footprintData])

  return (
    <div className="h-full flex flex-col">
      <div className="flex justify-between items-center mb-2">
        <h3 className="text-sm font-medium text-gray-300">Footprint 足迹图</h3>
        <div className="flex items-center space-x-4 text-xs">
          <div className="flex items-center">
            <div className="w-3 h-2 bg-green-500 mr-1"></div>
            <span>主动买入</span>
          </div>
          <div className="flex items-center">
            <div className="w-3 h-2 bg-red-500 mr-1"></div>
            <span>主动卖出</span>
          </div>
          <div className="flex items-center">
            <div className="w-3 h-2 bg-blue-300 mr-1"></div>
            <span>成交量密度</span>
          </div>
        </div>
      </div>
      
      <div className="flex-1">
        <svg
          ref={svgRef}
          width="100%"
          height="100%"
          viewBox="0 0 600 300"
          className="bg-gray-900 rounded"
        />
      </div>
      
      <div className="text-xs text-gray-400 mt-1">
        5分钟K线 | 每个价格层级显示主动买卖量累计 | 颜色深度 = 成交量大小
      </div>
    </div>
  )
}
```

### 4.4 足迹图数据处理引擎

```typescript
// lib/data-processing/footprint/footprint-engine.ts
export interface FootprintLevel {
  price: number
  buyVolume: number
  sellVolume: number
  buyCount: number
  sellCount: number
  timestamp: number
}

export interface FootprintCandle {
  timestamp: number
  open: number
  high: number
  low: number
  close: number
  volume: number
  levels: Map<number, FootprintLevel>
}

export class FootprintEngine {
  private candles = new Map<number, FootprintCandle>()
  private readonly timeframe: number // 毫秒
  private readonly priceStep: number // 价格聚合步长

  constructor(timeframeMinutes: number = 5, priceStep: number = 0.5) {
    this.timeframe = timeframeMinutes * 60 * 1000
    this.priceStep = priceStep
  }

  // 添加交易数据
  addTrade(price: number, volume: number, side: 'buy' | 'sell', timestamp: number): void {
    const candleTime = this.getCandleTime(timestamp)
    const aggregatedPrice = this.aggregatePrice(price)

    // 获取或创建K线
    let candle = this.candles.get(candleTime)
    if (!candle) {
      candle = {
        timestamp: candleTime,
        open: price,
        high: price,
        low: price,
        close: price,
        volume: 0,
        levels: new Map()
      }
      this.candles.set(candleTime, candle)
    }

    // 更新K线OHLC
    candle.high = Math.max(candle.high, price)
    candle.low = Math.min(candle.low, price)
    candle.close = price
    candle.volume += volume

    // 获取或创建价格层级
    let level = candle.levels.get(aggregatedPrice)
    if (!level) {
      level = {
        price: aggregatedPrice,
        buyVolume: 0,
        sellVolume: 0,
        buyCount: 0,
        sellCount: 0,
        timestamp
      }
      candle.levels.set(aggregatedPrice, level)
    }

    // 更新层级数据
    if (side === 'buy') {
      level.buyVolume += volume
      level.buyCount += 1
    } else {
      level.sellVolume += volume
      level.sellCount += 1
    }
    level.timestamp = timestamp
  }

  // 获取指定时间范围的足迹数据
  getFootprintData(startTime: number, endTime: number): FootprintCandle[] {
    const result: FootprintCandle[] = []
    
    this.candles.forEach((candle, timestamp) => {
      if (timestamp >= startTime && timestamp <= endTime) {
        result.push(candle)
      }
    })

    return result.sort((a, b) => a.timestamp - b.timestamp)
  }

  // 获取最新的N个K线
  getLatestCandles(count: number): FootprintCandle[] {
    const sortedCandles = Array.from(this.candles.values())
      .sort((a, b) => b.timestamp - a.timestamp)
      .slice(0, count)
      .reverse()

    return sortedCandles
  }

  // 清理过期数据
  cleanup(maxAge: number): void {
    const cutoffTime = Date.now() - maxAge
    
    this.candles.forEach((candle, timestamp) => {
      if (timestamp < cutoffTime) {
        this.candles.delete(timestamp)
      }
    })
  }

  // 获取K线时间戳
  private getCandleTime(timestamp: number): number {
    return Math.floor(timestamp / this.timeframe) * this.timeframe
  }

  // 聚合价格到指定步长
  private aggregatePrice(price: number): number {
    return Math.round(price / this.priceStep) * this.priceStep
  }

  // 计算价格层级的失衡度
  calculateImbalance(level: FootprintLevel): number {
    const totalVolume = level.buyVolume + level.sellVolume
    if (totalVolume === 0) return 0.5
    return level.buyVolume / totalVolume
  }

  // 获取成交量分布统计
  getVolumeDistribution(candle: FootprintCandle): {
    totalBuyVolume: number
    totalSellVolume: number
    maxLevelVolume: number
    priceRange: number
    volumeWeightedPrice: number
  } {
    let totalBuyVolume = 0
    let totalSellVolume = 0
    let maxLevelVolume = 0
    let volumeWeightedSum = 0
    let totalVolume = 0

    const prices: number[] = []
    
    candle.levels.forEach(level => {
      totalBuyVolume += level.buyVolume
      totalSellVolume += level.sellVolume
      const levelVolume = level.buyVolume + level.sellVolume
      maxLevelVolume = Math.max(maxLevelVolume, levelVolume)
      
      volumeWeightedSum += level.price * levelVolume
      totalVolume += levelVolume
      prices.push(level.price)
    })

    const priceRange = prices.length > 0 ? Math.max(...prices) - Math.min(...prices) : 0
    const volumeWeightedPrice = totalVolume > 0 ? volumeWeightedSum / totalVolume : candle.close

    return {
      totalBuyVolume,
      totalSellVolume,
      maxLevelVolume,
      priceRange,
      volumeWeightedPrice
    }
  }
}
```

### 4.2 高性能数据可视化

```typescript
// PriceChart.tsx - 使用Canvas优化性能
const PriceChart: React.FC<PriceChartProps> = ({ data, width, height }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const animationRef = useRef<number>()
  
  // 使用Web Workers处理大量数据
  const worker = useMemo(() => {
    return new Worker('/workers/chart-data-processor.js')
  }, [])
  
  // Canvas渲染优化
  const renderChart = useCallback((ctx: CanvasRenderingContext2D) => {
    // 使用requestAnimationFrame优化渲染
    const render = () => {
      ctx.clearRect(0, 0, width, height)
      
      // 绘制价格线
      drawPriceLine(ctx, data.prices)
      
      // 绘制成交量柱状图
      drawVolumeBar(ctx, data.volumes)
      
      // 绘制技术指标
      drawIndicators(ctx, data.indicators)
      
      animationRef.current = requestAnimationFrame(render)
    }
    
    render()
  }, [data, width, height])
  
  return (
    <canvas
      ref={canvasRef}
      width={width}
      height={height}
      className="price-chart"
    />
  )
}
```

### 4.3 Next.js API Routes实现WebSocket服务器

```typescript
// app/api/websocket/route.ts - Next.js WebSocket服务器
import { NextRequest } from 'next/server'
import { WebSocketServer } from 'ws'
import { EventEmitter } from 'events'

class BinanceWebSocketProxy extends EventEmitter {
  private wss: WebSocketServer
  private binanceConnections = new Map<string, WebSocket>()
  private clientConnections = new Set<WebSocket>()

  constructor(port: number) {
    super()
    this.wss = new WebSocketServer({ port })
    this.setupServer()
  }

  private setupServer(): void {
    this.wss.on('connection', (clientWs) => {
      this.clientConnections.add(clientWs)
      
      clientWs.on('message', async (message) => {
        try {
          const { action, symbol } = JSON.parse(message.toString())
          
          if (action === 'subscribe') {
            await this.subscribeToBinance(symbol)
          } else if (action === 'unsubscribe') {
            this.unsubscribeFromBinance(symbol)
          }
        } catch (error) {
          console.error('WebSocket message error:', error)
        }
      })
      
      clientWs.on('close', () => {
        this.clientConnections.delete(clientWs)
      })
    })
  }

  private async subscribeToBinance(symbol: string): Promise<void> {
    if (this.binanceConnections.has(symbol)) return

    const streams = [
      `${symbol.toLowerCase()}@depth20@100ms`,
      `${symbol.toLowerCase()}@trade`,
      `${symbol.toLowerCase()}@bookTicker`
    ]

    const wsUrl = `wss://fstream.binance.com/stream?streams=${streams.join('/')}`
    const binanceWs = new WebSocket(wsUrl)

    binanceWs.onopen = () => {
      this.binanceConnections.set(symbol, binanceWs)
      this.broadcastToClients({ type: 'connected', symbol })
    }

    binanceWs.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)
        
        // 处理币安数据并转发给所有客户端
        this.processMarketData(data)
        this.broadcastToClients(data)
      } catch (error) {
        console.error('Binance data parse error:', error)
      }
    }

    binanceWs.onclose = () => {
      this.binanceConnections.delete(symbol)
      this.handleBinanceReconnect(symbol)
    }

    binanceWs.onerror = (error) => {
      console.error('Binance WebSocket error:', error)
      this.emit('error', { symbol, error })
    }
  }

  private processMarketData(data: any): void {
    if (!data.stream || !data.data) return

    const stream = data.stream
    const marketData = data.data

    if (stream.includes('depth')) {
      this.emit('depth:update', marketData)
    } else if (stream.includes('trade')) {
      this.emit('trade:executed', marketData)
    } else if (stream.includes('bookTicker')) {
      this.emit('ticker:update', marketData)
    }
  }

  private broadcastToClients(data: any): void {
    const message = JSON.stringify(data)
    
    this.clientConnections.forEach(client => {
      if (client.readyState === WebSocket.OPEN) {
        client.send(message)
      }
    })
  }

  private async handleBinanceReconnect(symbol: string): Promise<void> {
    // 指数退避重连
    let attempts = 0
    const maxAttempts = 5

    while (attempts < maxAttempts) {
      const delay = Math.pow(2, attempts) * 1000
      await new Promise(resolve => setTimeout(resolve, delay))
      
      try {
        await this.subscribeToBinance(symbol)
        break
      } catch (error) {
        attempts++
        console.error(`Reconnect attempt ${attempts} failed:`, error)
      }
    }
  }
}

// 全局WebSocket代理实例
let wsProxy: BinanceWebSocketProxy

export async function GET(request: NextRequest) {
  if (!wsProxy) {
    wsProxy = new BinanceWebSocketProxy(8080)
  }

  return new Response('WebSocket server running on port 8080', {
    status: 200
  })
}
```

### 4.4 纯TypeScript订单簿管理器

```typescript
// lib/orderbook/manager.ts - 完全重写的订单簿管理器
import { EventEmitter } from 'events'

interface OrderFlow {
  price: number
  bidVolume: number
  askVolume: number
  activeBuyVolume: number
  activeSellVolume: number
  historicalBuyVolume: number
  historicalSellVolume: number
  timestamp: number
}

interface MarketSnapshot {
  symbol: string
  bestBid: number | null
  bestAsk: number | null
  currentPrice: number | null
  spread: number
  realizedVolatility: number
  jumpSignal: number
  orderBookImbalance: number
  volumeWeightedMomentum: number
  timestamp: number
}

export class OrderBookManager extends EventEmitter {
  private orderFlows = new Map<number, OrderFlow>()
  private symbol: string
  private priceHistory: Array<{ price: number; timestamp: number }> = []
  private tradeHistory: Array<{ price: number; volume: number; side: 'buy' | 'sell'; timestamp: number }> = []
  private volatilityWindow = 10000 // 10秒窗口
  private jumpThreshold = 2.5 // Z-score阈值

  constructor(symbol: string) {
    super()
    this.symbol = symbol
    this.startPeriodicCleanup()
  }

  // 处理深度更新
  updateDepth(data: any): void {
    const timestamp = Date.now()
    
    // 处理买单
    if (data.b && Array.isArray(data.b)) {
      data.b.forEach(([priceStr, quantityStr]: [string, string]) => {
        const price = parseFloat(priceStr)
        const quantity = parseFloat(quantityStr)
        this.updateOrderFlow(price, quantity, 'bid', timestamp)
      })
    }

    // 处理卖单
    if (data.a && Array.isArray(data.a)) {
      data.a.forEach(([priceStr, quantityStr]: [string, string]) => {
        const price = parseFloat(priceStr)
        const quantity = parseFloat(quantityStr)
        this.updateOrderFlow(price, quantity, 'ask', timestamp)
      })
    }

    this.emit('depth:updated', this.getMarketSnapshot())
  }

  // 处理交易数据
  updateTrade(data: any): void {
    const price = parseFloat(data.p)
    const quantity = parseFloat(data.q)
    const isBuyerMaker = data.m
    const side = isBuyerMaker ? 'sell' : 'buy'
    const timestamp = Date.now()

    // 更新交易历史
    this.tradeHistory.push({ price, volume: quantity, side, timestamp })
    
    // 保持交易历史在合理范围内
    if (this.tradeHistory.length > 1000) {
      this.tradeHistory = this.tradeHistory.slice(-500)
    }

    // 更新价格历史
    this.priceHistory.push({ price, timestamp })
    if (this.priceHistory.length > 1000) {
      this.priceHistory = this.priceHistory.slice(-500)
    }

    // 更新订单流的主动交易量
    const orderFlow = this.getOrCreateOrderFlow(price)
    if (side === 'buy') {
      orderFlow.activeBuyVolume += quantity
      orderFlow.historicalBuyVolume += quantity
    } else {
      orderFlow.activeSellVolume += quantity
      orderFlow.historicalSellVolume += quantity
    }
    orderFlow.timestamp = timestamp

    // 计算技术指标
    this.calculateRealizedVolatility()
    this.calculateJumpSignal()
    this.calculateOrderBookImbalance()

    this.emit('trade:updated', {
      price,
      quantity,
      side,
      timestamp,
      snapshot: this.getMarketSnapshot()
    })
  }

  // 处理BookTicker数据
  updateBookTicker(data: any): void {
    const bestBidPrice = parseFloat(data.b)
    const bestAskPrice = parseFloat(data.a)
    const bestBidQty = parseFloat(data.B)
    const bestAskQty = parseFloat(data.A)

    // 清理无效的订单数据
    this.cleanInvalidOrders(bestBidPrice, bestAskPrice)

    this.emit('ticker:updated', {
      bestBid: bestBidPrice,
      bestAsk: bestAskPrice,
      bestBidQty,
      bestAskQty,
      spread: bestAskPrice - bestBidPrice
    })
  }

  private updateOrderFlow(price: number, quantity: number, side: 'bid' | 'ask', timestamp: number): void {
    const orderFlow = this.getOrCreateOrderFlow(price)
    
    if (side === 'bid') {
      orderFlow.bidVolume = quantity
    } else {
      orderFlow.askVolume = quantity
    }
    
    orderFlow.timestamp = timestamp

    // 如果数量为0，表示该价格层级被移除
    if (quantity === 0) {
      if (orderFlow.bidVolume === 0 && orderFlow.askVolume === 0 && 
          orderFlow.activeBuyVolume === 0 && orderFlow.activeSellVolume === 0) {
        this.orderFlows.delete(price)
      }
    }
  }

  private getOrCreateOrderFlow(price: number): OrderFlow {
    let orderFlow = this.orderFlows.get(price)
    
    if (!orderFlow) {
      orderFlow = {
        price,
        bidVolume: 0,
        askVolume: 0,
        activeBuyVolume: 0,
        activeSellVolume: 0,
        historicalBuyVolume: 0,
        historicalSellVolume: 0,
        timestamp: Date.now()
      }
      this.orderFlows.set(price, orderFlow)
    }
    
    return orderFlow
  }

  private calculateRealizedVolatility(): number {
    if (this.priceHistory.length < 2) return 0

    const now = Date.now()
    const windowStart = now - this.volatilityWindow
    
    // 过滤窗口内的价格数据
    const windowPrices = this.priceHistory
      .filter(p => p.timestamp >= windowStart)
      .map(p => p.price)

    if (windowPrices.length < 2) return 0

    // 计算对数收益率
    const returns: number[] = []
    for (let i = 1; i < windowPrices.length; i++) {
      const logReturn = Math.log(windowPrices[i] / windowPrices[i - 1])
      if (isFinite(logReturn)) {
        returns.push(logReturn)
      }
    }

    if (returns.length === 0) return 0

    // 计算标准差
    const mean = returns.reduce((sum, r) => sum + r, 0) / returns.length
    const variance = returns.reduce((sum, r) => sum + Math.pow(r - mean, 2), 0) / returns.length
    
    return Math.sqrt(variance) * 10000 // 放大以便观察
  }

  private calculateJumpSignal(): number {
    if (this.priceHistory.length < 30) return 0

    const recentPrices = this.priceHistory.slice(-30).map(p => p.price)
    const returns: number[] = []
    
    for (let i = 1; i < recentPrices.length; i++) {
      const logReturn = Math.log(recentPrices[i] / recentPrices[i - 1])
      if (isFinite(logReturn)) {
        returns.push(logReturn)
      }
    }

    if (returns.length === 0) return 0

    const mean = returns.reduce((sum, r) => sum + r, 0) / returns.length
    const std = Math.sqrt(returns.reduce((sum, r) => sum + Math.pow(r - mean, 2), 0) / returns.length)
    
    if (std === 0) return 0

    // 计算最新收益率的Z-score
    const latestReturn = returns[returns.length - 1]
    const zScore = Math.abs((latestReturn - mean) / std)
    
    return zScore > this.jumpThreshold ? zScore : 0
  }

  private calculateOrderBookImbalance(): number {
    let totalBidVolume = 0
    let totalAskVolume = 0

    this.orderFlows.forEach(orderFlow => {
      totalBidVolume += orderFlow.bidVolume
      totalAskVolume += orderFlow.askVolume
    })

    const totalVolume = totalBidVolume + totalAskVolume
    if (totalVolume === 0) return 0.5

    return totalBidVolume / totalVolume
  }

  private cleanInvalidOrders(bestBid: number, bestAsk: number): void {
    const spread = bestAsk - bestBid
    const buffer = spread * 0.1 // 10%缓冲区

    this.orderFlows.forEach((orderFlow, price) => {
      // 清理明显不合理的挂单
      if (price > bestAsk + buffer && orderFlow.bidVolume > 0) {
        orderFlow.bidVolume = 0
      }
      if (price < bestBid - buffer && orderFlow.askVolume > 0) {
        orderFlow.askVolume = 0
      }
    })
  }

  private startPeriodicCleanup(): void {
    setInterval(() => {
      const now = Date.now()
      const maxAge = 300000 // 5分钟

      // 清理过期的主动交易量
      this.orderFlows.forEach(orderFlow => {
        if (now - orderFlow.timestamp > 5000) { // 5秒后清理主动交易量
          orderFlow.activeBuyVolume = 0
          orderFlow.activeSellVolume = 0
        }
      })

      // 清理完全空的订单流
      this.orderFlows.forEach((orderFlow, price) => {
        if (orderFlow.bidVolume === 0 && orderFlow.askVolume === 0 && 
            orderFlow.activeBuyVolume === 0 && orderFlow.activeSellVolume === 0 &&
            now - orderFlow.timestamp > maxAge) {
          this.orderFlows.delete(price)
        }
      })
    }, 1000) // 每秒清理一次
  }

  // 获取市场快照
  getMarketSnapshot(): MarketSnapshot {
    const bestBid = this.getBestBid()
    const bestAsk = this.getBestAsk()
    const currentPrice = this.getCurrentPrice()
    
    return {
      symbol: this.symbol,
      bestBid,
      bestAsk,
      currentPrice,
      spread: bestBid && bestAsk ? bestAsk - bestBid : 0,
      realizedVolatility: this.calculateRealizedVolatility(),
      jumpSignal: this.calculateJumpSignal(),
      orderBookImbalance: this.calculateOrderBookImbalance(),
      volumeWeightedMomentum: this.calculateVolumeWeightedMomentum(),
      timestamp: Date.now()
    }
  }

  private getBestBid(): number | null {
    let bestBid = null
    this.orderFlows.forEach((orderFlow, price) => {
      if (orderFlow.bidVolume > 0 && (bestBid === null || price > bestBid)) {
        bestBid = price
      }
    })
    return bestBid
  }

  private getBestAsk(): number | null {
    let bestAsk = null
    this.orderFlows.forEach((orderFlow, price) => {
      if (orderFlow.askVolume > 0 && (bestAsk === null || price < bestAsk)) {
        bestAsk = price
      }
    })
    return bestAsk
  }

  private getCurrentPrice(): number | null {
    if (this.priceHistory.length === 0) return null
    return this.priceHistory[this.priceHistory.length - 1].price
  }

  private calculateVolumeWeightedMomentum(): number {
    if (this.tradeHistory.length < 10) return 0

    const recentTrades = this.tradeHistory.slice(-20)
    let buyVolume = 0
    let sellVolume = 0

    recentTrades.forEach(trade => {
      if (trade.side === 'buy') {
        buyVolume += trade.volume
      } else {
        sellVolume += trade.volume
      }
    })

    const totalVolume = buyVolume + sellVolume
    if (totalVolume === 0) return 0

    return (buyVolume - sellVolume) / totalVolume
  }

  // 获取聚合后的订单流数据
  getAggregatedOrderFlows(precision: number = 1.0, maxLevels: number = 40): OrderFlow[] {
    const aggregated = new Map<number, OrderFlow>()

    this.orderFlows.forEach(orderFlow => {
      const aggregatedPrice = Math.floor(orderFlow.price / precision) * precision
      
      if (!aggregated.has(aggregatedPrice)) {
        aggregated.set(aggregatedPrice, {
          price: aggregatedPrice,
          bidVolume: 0,
          askVolume: 0,
          activeBuyVolume: 0,
          activeSellVolume: 0,
          historicalBuyVolume: 0,
          historicalSellVolume: 0,
          timestamp: orderFlow.timestamp
        })
      }

      const agg = aggregated.get(aggregatedPrice)!
      agg.bidVolume += orderFlow.bidVolume
      agg.askVolume += orderFlow.askVolume
      agg.activeBuyVolume += orderFlow.activeBuyVolume
      agg.activeSellVolume += orderFlow.activeSellVolume
      agg.historicalBuyVolume += orderFlow.historicalBuyVolume
      agg.historicalSellVolume += orderFlow.historicalSellVolume
      agg.timestamp = Math.max(agg.timestamp, orderFlow.timestamp)
    })

    // 转换为数组并排序
    const result = Array.from(aggregated.values())
      .sort((a, b) => b.price - a.price) // 从高价到低价

    // 限制返回的层级数量
    return result.slice(0, maxLevels)
  }
}
```

---

## 5. 性能优化策略

### 5.1 前端性能优化

**虚拟化渲染：**
```typescript
// 使用react-window处理大量数据
import { FixedSizeList as List } from 'react-window'

const VirtualizedOrderBook = ({ items }) => (
  <List
    height={800}
    itemCount={items.length}
    itemSize={24}
    itemData={items}
  >
    {OrderBookRow}
  </List>
)
```

**内存优化：**
```typescript
// 使用WeakMap避免内存泄漏
const componentCache = new WeakMap()

// 使用Object.freeze防止意外修改
const immutableData = Object.freeze(marketData)
```

**渲染优化：**
```typescript
// 使用React.memo减少不必要的重渲染
const OrderBookRow = React.memo(({ price, volume, side }) => {
  return (
    <div className={`order-row ${side}`}>
      <span>{price}</span>
      <span>{volume}</span>
    </div>
  )
}, (prevProps, nextProps) => {
  return prevProps.price === nextProps.price && 
         prevProps.volume === nextProps.volume
})
```

### 5.2 数据处理优化

**Web Workers：**
```typescript
// chart-data-processor.worker.ts
self.onmessage = function(e) {
  const { rawData, aggregationLevel } = e.data
  
  // 在Worker中处理大量数据计算
  const processedData = aggregateMarketData(rawData, aggregationLevel)
  
  self.postMessage(processedData)
}
```

**数据缓存：**
```typescript
// 使用LRU缓存优化数据访问
import LRU from 'lru-cache'

const dataCache = new LRU<string, MarketData>({
  max: 1000,
  ttl: 1000 * 60 * 5 // 5分钟TTL
})
```

---

## 6. 用户体验增强

### 6.1 响应式设计

```css
/* 移动端适配 */
@media (max-width: 768px) {
  .order-book-table {
    font-size: 12px;
    
    .column {
      min-width: 60px;
    }
    
    .price-column {
      font-weight: bold;
      background: rgba(255, 255, 255, 0.1);
    }
  }
}

/* 平板适配 */
@media (min-width: 769px) and (max-width: 1024px) {
  .order-book-table {
    font-size: 14px;
  }
}
```

### 6.2 交互体验优化

**流畅动画：**
```typescript
// 使用Framer Motion实现流畅动画
import { motion, AnimatePresence } from 'framer-motion'

const OrderBookRow = ({ price, volume, isNew }) => (
  <motion.div
    initial={isNew ? { opacity: 0, x: -20 } : false}
    animate={{ opacity: 1, x: 0 }}
    exit={{ opacity: 0, x: 20 }}
    transition={{ duration: 0.2 }}
    className="order-row"
  >
    <span>{price}</span>
    <span>{volume}</span>
  </motion.div>
)
```

**智能提示：**
```typescript
// 使用Tooltip提供上下文信息
const PriceCell = ({ price, volume, timestamp }) => (
  <Tooltip
    content={
      <div>
        <p>价格: {price}</p>
        <p>成交量: {volume}</p>
        <p>时间: {new Date(timestamp).toLocaleTimeString()}</p>
      </div>
    }
  >
    <span className="price-cell">{price}</span>
  </Tooltip>
)
```

---

## 7. 部署与运维

### 7.1 容器化部署

```dockerfile
# Dockerfile
FROM node:18-alpine AS builder

WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production

COPY . .
RUN npm run build

FROM node:18-alpine AS runner
WORKDIR /app

COPY --from=builder /app/public ./public
COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static

EXPOSE 3000
CMD ["node", "server.js"]
```

### 7.2 监控与日志

```typescript
// monitoring.ts
import { createLogger } from 'winston'
import { PrometheusRegistry } from 'prom-client'

const logger = createLogger({
  level: 'info',
  format: winston.format.json(),
  transports: [
    new winston.transports.File({ filename: 'error.log', level: 'error' }),
    new winston.transports.File({ filename: 'combined.log' })
  ]
})

// 性能指标收集
const registry = new PrometheusRegistry()
const websocketConnections = new Gauge({
  name: 'websocket_connections_total',
  help: 'Total number of WebSocket connections'
})
```

---

## 8. 迁移计划

### 8.1 阶段性迁移

**第一阶段（4周）：基础架构**
- [ ] Next.js项目初始化
- [ ] 基础组件开发
- [ ] WebSocket连接管理
- [ ] 状态管理系统

**第二阶段（6周）：核心功能**
- [ ] 订单簿可视化
- [ ] 实时数据处理
- [ ] 性能优化
- [ ] 响应式设计

**第三阶段（4周）：高级功能**
- [ ] 数据分析功能
- [ ] 用户体验优化
- [ ] 测试与调试
- [ ] 部署上线

### 8.2 完全重写策略

**无需数据迁移，完全重新实现：**

```typescript
// 纯TypeScript实现的核心数据结构
interface OrderFlow {
  price: number
  bidVolume: number
  askVolume: number
  activeBuyVolume: number
  activeSellVolume: number
  timestamp: number
}

interface MarketSnapshot {
  symbol: string
  bestBid: number
  bestAsk: number
  currentPrice: number
  realizedVolatility: number
  jumpSignal: number
  orderBookImbalance: number
  timestamp: number
}

// Node.js实现的订单簿管理器
class OrderBookManager {
  private orderFlows = new Map<number, OrderFlow>()
  private eventEmitter = new EventEmitter()
  
  updateDepth(data: DepthUpdate): void {
    // 纯JavaScript实现的订单簿更新逻辑
    data.bids.forEach(([price, quantity]) => {
      this.updateOrderFlow(parseFloat(price), parseFloat(quantity), 'bid')
    })
    
    data.asks.forEach(([price, quantity]) => {
      this.updateOrderFlow(parseFloat(price), parseFloat(quantity), 'ask')
    })
    
    this.eventEmitter.emit('orderbook:updated', this.getSnapshot())
  }
  
  private updateOrderFlow(price: number, quantity: number, side: 'bid' | 'ask'): void {
    const orderFlow = this.orderFlows.get(price) || {
      price,
      bidVolume: 0,
      askVolume: 0,
      activeBuyVolume: 0,
      activeSellVolume: 0,
      timestamp: Date.now()
    }
    
    if (side === 'bid') {
      orderFlow.bidVolume = quantity
    } else {
      orderFlow.askVolume = quantity
    }
    
    orderFlow.timestamp = Date.now()
    this.orderFlows.set(price, orderFlow)
  }
}
```

---

## 9. 风险评估与缓解

### 9.1 技术风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| 性能不达标 | 高 | 中 | 提前性能测试，使用Web Workers |
| WebSocket连接不稳定 | 高 | 低 | 完善重连机制，备用连接 |
| 数据同步问题 | 中 | 中 | 实现数据校验，错误恢复 |

### 9.2 业务风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| 用户接受度低 | 高 | 低 | 渐进式迁移，保持功能一致性 |
| 开发周期延长 | 中 | 中 | 敏捷开发，MVP优先 |

---

## 10. 成功指标

### 10.1 性能指标
- **首屏加载时间**：< 2秒
- **数据更新延迟**：< 100ms
- **内存使用**：< 200MB
- **CPU使用率**：< 30%

### 10.2 用户体验指标
- **响应时间**：< 16ms (60fps)
- **错误率**：< 0.1%
- **可用性**：> 99.9%
- **用户满意度**：> 4.5/5

---

## 11. 总结

通过将FlowSight从Rust桌面应用重构为基于Next.js的现代Web应用，我们将获得：

✅ **更好的可访问性**：无需安装，浏览器即可使用  
✅ **更强的扩展性**：丰富的Web生态系统支持  
✅ **更优的用户体验**：现代化的交互设计  
✅ **更简单的部署**：云端部署，自动更新  
✅ **更好的协作性**：支持多用户、云端同步  

同时保持原有的核心优势：
- 实时数据处理能力
- 专业的交易分析功能
- 高性能的数据可视化
- 稳定的系统架构

这个重构方案将FlowSight打造成一个现代化、高性能、用户友好的专业交易分析平台。