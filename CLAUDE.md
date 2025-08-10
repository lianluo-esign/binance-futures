注: 禁止删除本文件!!!!!!!!!!!

# CLAUDE 开发规则

## Rust OOP 最佳实践指南

### 1. 优先使用组合而非继承
- **原则**: 使用struct组合和trait实现，避免复杂的继承链
- **实现**: 
  - 将功能分解为独立的组件
  - 通过组合构建复杂对象
  - 使用依赖注入而非继承
- **示例**:
  ```rust
  // ✅ 好的做法：组合
  pub struct Car {
      engine: Engine,
      gps: Option<GPS>,
      audio: AudioSystem,
  }
  
  // ❌ 避免的做法：试图模拟继承
  // 不要创建复杂的trait继承链
  ```

### 2. 通过Traits实现多态
- **原则**: 使用trait对象和泛型实现运行时和编译时多态
- **实现**:
  - 定义清晰的trait接口
  - 使用`Box<dyn Trait>`实现动态分发
  - 使用泛型`<T: Trait>`实现静态分发
  - 提供默认实现减少重复代码
- **示例**:
  ```rust
  // 定义行为trait
  pub trait Drawable {
      fn draw(&self);
      fn area(&self) -> f64;
      
      // 默认实现
      fn describe(&self) {
          println!("Area: {}", self.area());
      }
  }
  
  // 动态多态
  fn render_shapes(shapes: &[Box<dyn Drawable>]) { /* ... */ }
  
  // 静态多态 (零成本)
  fn process_shape<T: Drawable>(shape: &T) { /* ... */ }
  ```

### 3. 使用模块系统实现封装
- **原则**: 通过模块和可见性控制实现良好的封装
- **实现**:
  - 将相关功能组织在同一模块中
  - 使用`pub`、`pub(crate)`、`pub(super)`控制可见性
  - 隐藏内部实现细节
  - 提供清晰的公共API
- **文件组织**:
  ```
  src/
  ├── lib.rs                 # 公共API导出
  ├── core/                  # 核心功能模块
  │   ├── mod.rs
  │   ├── engine.rs
  │   └── types.rs
  ├── gui/                   # GUI相关模块
  │   ├── mod.rs
  │   ├── widgets/
  │   │   ├── mod.rs
  │   │   ├── button.rs
  │   │   └── table.rs
  │   └── renderers/
  └── utils/                 # 工具模块
  ```

### 4. 利用类型系统实现编译时检查
- **原则**: 利用Rust强大的类型系统在编译时捕获错误
- **实现**:
  - 使用newtype模式创建强类型
  - 利用泛型约束确保类型安全
  - 使用枚举表示状态和错误
  - 使用类型状态模式编码业务规则
- **示例**:
  ```rust
  // 强类型包装
  #[derive(Debug, Clone, Copy)]
  pub struct Price(f64);
  
  #[derive(Debug, Clone, Copy)]  
  pub struct Volume(f64);
  
  // 类型状态模式
  pub struct Order<State> {
      id: String,
      amount: Price,
      _state: PhantomData<State>,
  }
  
  pub struct Pending;
  pub struct Confirmed;
  
  impl Order<Pending> {
      pub fn confirm(self) -> Order<Confirmed> { /* ... */ }
  }
  ```

### 5. 结合所有权系统确保内存安全
- **原则**: 利用所有权、借用和生命周期确保内存安全
- **实现**:
  - 明确数据所有权归属
  - 使用引用避免不必要的克隆
  - 利用RAII模式自动管理资源
  - 使用智能指针处理共享所有权
- **示例**:
  ```rust
  pub struct ResourceManager {
      resources: Vec<Resource>,
  }
  
  impl ResourceManager {
      // 移动所有权
      pub fn add_resource(&mut self, resource: Resource) {
          self.resources.push(resource);
      }
      
      // 借用引用
      pub fn get_resource(&self, id: usize) -> Option<&Resource> {
          self.resources.get(id)
      }
      
      // 可变借用
      pub fn update_resource(&mut self, id: usize, f: impl FnOnce(&mut Resource)) {
          if let Some(resource) = self.resources.get_mut(id) {
              f(resource);
          }
      }
  }
  ```

### 6. 使用泛型实现零成本抽象
- **原则**: 使用泛型在编译时实现多态，避免运行时开销
- **实现**:
  - 优先使用泛型而非trait对象
  - 使用关联类型简化复杂泛型
  - 合理使用where子句提高可读性
  - 利用泛型特化优化性能
- **示例**:
  ```rust
  // 泛型trait with关联类型
  pub trait Repository {
      type Item;
      type Error;
      
      fn save(&mut self, item: Self::Item) -> Result<(), Self::Error>;
      fn find(&self, id: u32) -> Result<Option<Self::Item>, Self::Error>;
  }
  
  // 泛型实现
  pub fn process_items<R, I>(repo: &mut R, items: Vec<I>) -> Result<(), R::Error>
  where
      R: Repository<Item = I>,
      I: Clone + Debug,
  {
      for item in items {
          repo.save(item)?;
      }
      Ok(())
  }
  ```

### 7. 文件大小和组织规则
- **原则**: 保持单个文件少于1000行，合理拆分代码
- **实现**:
  - 单个文件不超过1000行代码
  - 按功能职责拆分模块
  - 使用`mod.rs`组织子模块
  - 相关的类型和实现放在同一文件
- **拆分策略**:
  ```rust
  // volume_profile/mod.rs - 模块入口
  pub mod widget;      // 小于1000行
  pub mod renderer;    // 小于1000行
  pub mod manager;     // 小于1000行
  pub mod types;       // 共享类型定义
  
  pub use widget::VolumeProfileWidget;
  pub use renderer::VolumeProfileRenderer;
  pub use manager::VolumeProfileManager;
  ```

## 代码质量标准

### 错误处理
- 使用`Result<T, E>`处理可能失败的操作
- 使用 `thiserror` 定义自定义错误类型
- 使用`?`操作符简化错误传播
- 实现错误恢复和重试机制
- 记录错误日志和监控指标

### 文档和测试
- 为所有公共API提供文档注释
- 编写单元测试验证功能正确性
- 使用集成测试验证模块间协作
- 使用 `criterion` 进行性能基准测试
- 使用 `proptest` 进行属性测试
- 在每次功能完成后禁止写markdown文档

### 性能考虑
- 避免不必要的克隆和分配
- 使用`&str`而非`String`作为只读参数
- 考虑使用`Cow<str>`处理可能需要所有权的字符串
- 使用零拷贝技术处理大数据
- 实现对象池和内存池减少分配
- 优化热路径，使用 `#[inline]` 和 `#[inline(always)]`

### 并发和线程安全
- 优先使用无锁数据结构
- 正确使用 `Send` 和 `Sync` trait
- 避免死锁和数据竞争
- 使用 `parking_lot` 替代标准库锁获得更好性能

### 开发工具链
- **代码质量工具**:
  ```toml
  # Cargo.toml 开发依赖
  [dev-dependencies]
  criterion = "0.5"        # 性能基准测试
  proptest = "1.0"         # 属性测试
  mockall = "0.12"         # Mock 框架
  ```
- **CI/CD 检查清单**:
  - 代码格式检查 (`cargo fmt --check`)
  - 静态分析 (`cargo clippy -- -D warnings`)
  - 安全审计 (`cargo audit`)
  - 性能回归测试
  - 测试覆盖率检查
- **性能监控**:
  ```rust
  // 内置性能指标
  pub struct PerformanceMetrics {
      pub event_processing_latency: Histogram,
      pub memory_usage: Gauge,
      pub orderbook_update_rate: Counter,
      pub websocket_reconnect_count: Counter,
  }
  ```

### 8. 异步编程和并发安全
- **原则**: 基于 tokio 构建高性能异步系统，确保线程安全
- **实现**:
  - 使用 `Arc<Mutex<T>>` 或 `Arc<RwLock<T>>` 处理共享状态
  - 优先使用无锁数据结构（如 RingBuffer）
  - 避免在异步函数中使用阻塞操作
  - 使用 channel 进行异步组件间通信
- **示例**:
  ```rust
  // 实时订单流处理
  pub struct OrderFlowProcessor {
      ring_buffer: Arc<LockFreeRingBuffer<OrderEvent>>,
      event_bus: Arc<EventBus>,
  }
  
  impl OrderFlowProcessor {
      pub async fn process_depth_update(&self, update: DepthUpdate) -> Result<()> {
          // 无锁写入环形缓冲区
          self.ring_buffer.write(OrderEvent::DepthUpdate(update))?;
          
          // 异步发布事件
          self.event_bus.publish(Event::OrderFlowChanged).await?;
          Ok(())
      }
  }
  ```

### 9. 金融交易系统特定规范
- **精度和数值计算**:
  - 使用 `OrderedFloat<f64>` 处理价格，确保可排序性
  - 避免浮点数的直接比较
  - 统一时间戳格式（纳秒级 u64）
  - 实现价格和数量的强类型包装
- **实时数据处理模式**:
  - 使用差分计算减少数据传输
  - 实现订单流分析和冲击检测
  - 保持状态一致性和原子性
- **示例**:
  ```rust
  // 强类型金融数据
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
  pub struct Price(OrderedFloat<f64>);
  
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]  
  pub struct Quantity(OrderedFloat<f64>);
  
  // 订单流差分计算
  pub struct OrderFlowDiffer {
      previous_snapshot: Option<OrderBookSnapshot>,
  }
  
  impl OrderFlowDiffer {
      pub fn calculate_diff(&mut self, current: OrderBookSnapshot) -> OrderFlowDiff {
          let diff = match &self.previous_snapshot {
              Some(prev) => self.compute_incremental_changes(prev, &current),
              None => OrderFlowDiff::initial(current.clone()),
          };
          self.previous_snapshot = Some(current);
          diff
      }
  }
  ```

### 10. 企业级错误处理
- **分层错误设计**:
  - 使用 `thiserror` 定义领域特定错误
  - 实现错误的上下文传播
  - 区分可恢复和不可恢复错误
  - 提供错误指标和监控
- **错误恢复策略**:
  - 实现指数退避重试
  - 使用断路器模式防止级联失败
  - 记录错误日志和指标
- **示例**:
  ```rust
  // 分层错误定义
  #[derive(Debug, thiserror::Error)]
  pub enum TradeSystemError {
      #[error("WebSocket connection failed: {0}")]
      ConnectionError(#[from] WebSocketError),
      
      #[error("Data processing error: {message}")]
      ProcessingError { 
          message: String, 
          recoverable: bool 
      },
      
      #[error("Configuration error: {0}")]
      ConfigError(#[from] ConfigError),
  }
  
  // 自动重试策略
  pub struct ResilientWebSocketManager {
      retry_policy: ExponentialBackoff,
      circuit_breaker: CircuitBreaker,
  }
  ```

### 11. 高频交易系统性能优化
- **内存布局优化**:
  - 使用 `#[repr(C)]` 控制结构体布局
  - 字段按大小降序排列减少填充
  - 使用 `Box<[T]>` 存储固定大小数据
  - 实现内存池减少分配开销
- **CPU 亲和性和线程优化**:
  - 将关键线程绑定到专用 CPU 核心
  - 设置实时优先级提高响应速度
  - 避免 false sharing 和缓存竞争
- **示例**:
  ```rust
  // 缓存友好的订单簿设计
  #[repr(C)]
  pub struct PriceLevel {
      price: Price,        // 8 bytes
      quantity: Quantity,  // 8 bytes
      timestamp: u64,      // 8 bytes
      order_count: u32,    // 4 bytes
      _padding: u32,       // 4 bytes padding for alignment
  }
  
  // CPU 绑定策略
  pub fn setup_high_performance_threading() -> Result<()> {
      // 将主处理线程绑定到专用 CPU 核心
      core_affinity::set_for_current(CoreId { id: 0 });
      
      // 设置实时优先级
      set_thread_priority(ThreadPriority::Max)?;
      Ok(())
  }
  ```

### 12. 配置管理和依赖注入
- **分层配置系统**:
  - 使用 serde 和 toml 实现配置序列化
  - 支持环境变量覆盖
  - 实现配置验证和默认值
  - 支持热重载（非关键配置）
- **依赖注入容器**:
  - 使用 Arc 包装共享服务
  - 通过构造函数注入依赖
  - 实现服务生命周期管理
- **示例**:
  ```rust
  // 类型安全的配置系统
  #[derive(Debug, Deserialize, Validate)]
  pub struct TradingConfig {
      #[validate(range(min = 1, max = 65535))]
      pub websocket_port: u16,
      
      #[validate(custom = "validate_symbol")]
      pub symbol: String,
      
      #[serde(default = "default_buffer_size")]
      pub ring_buffer_size: usize,
  }
  
  // 简单的服务容器
  pub struct ServiceContainer {
      market_service: Arc<dyn MarketService + Send + Sync>,
      event_bus: Arc<EventBus>,
      config: Arc<TradingConfig>,
  }
  ```

### 13. 币安期货系统特定模式
- **订单流分析模式**:
  - 实时计算订单流差分
  - 检测大单冲击和吸筹
  - 分析市场微观结构
- **事件驱动架构**:
  - 使用高性能事件总线
  - 实现背压和流控制
  - 支持事件回放和审计
- **示例**:
  ```rust
  // 订单冲击检测
  pub trait OrderFlowAnalyzer {
      fn analyze_depth_change(&self, prev: &OrderBook, current: &OrderBook) -> OrderFlowSignal;
      fn detect_order_impact(&self, trade: &Trade, ticker: &BookTicker) -> Option<ImpactSignal>;
  }
  
  impl OrderImpactDetector {
      pub fn detect_impact(&self, trade: &Trade, snapshot: &BookTickerSnapshot) -> Option<ImpactSignal> {
          match trade.side {
              TradeSide::Buy if trade.quantity > snapshot.best_ask_qty => {
                  Some(ImpactSignal::BuyImpact { 
                      excess_quantity: trade.quantity - snapshot.best_ask_qty,
                      timestamp: trade.timestamp 
                  })
              }
              TradeSide::Sell if trade.quantity > snapshot.best_bid_qty => {
                  Some(ImpactSignal::SellImpact { 
                      excess_quantity: trade.quantity - snapshot.best_bid_qty,
                      timestamp: trade.timestamp 
                  })
              }
              _ => None,
          }
      }
  }
  ```

### sub agent的使用
- **code-writer 调用场景**:
  - 新功能模块实现（>200行代码）
  - 复杂算法实现（订单流分析、信号计算等）
  - 性能关键路径优化
  - 多文件重构任务（>3个文件）
- **code-reviewer 调用场景**:
  - 架构设计的合理性检查
  - 功能模块的完成程度及逻辑可解释性
  - 金融计算逻辑验证
  - 并发安全性审查
  - 性能瓶颈识别
  - 内存泄漏和资源管理检查
- **logic-light-fire 调用场景**:
  - 技术方案选型和架构设计
  - 性能优化策略分析
  - 创新算法设计
  - 系统瓶颈诊断和解决方案

### 在开发新需求时最大化向下兼容
- 在保持当前业务逻辑的基础上来开发新的业务逻辑代码功能

### 只允许一个main.rs文件
- 在开发新功能或者修改main函数的时候 请确保整个系统只有一个main.rs入口文件

## 项目结构模板

```
src/
├── lib.rs                          # 库入口，导出公共API
├── core/                           # 核心业务逻辑
│   ├── mod.rs
│   ├── types.rs                    # 共享类型定义
│   ├── traits.rs                   # 核心trait定义
│   └── engine.rs                   # 核心引擎逻辑
├── data/                           # 数据层
│   ├── mod.rs
│   ├── models/                     # 数据模型
│   │   ├── mod.rs
│   │   ├── order.rs
│   │   └── market.rs
│   └── repositories/               # 数据访问层
│       ├── mod.rs
│       └── market_repository.rs
├── gui/                            # GUI层
│   ├── mod.rs
│   ├── widgets/                    # UI组件
│   │   ├── mod.rs
│   │   ├── volume_profile.rs
│   │   └── orderbook.rs
│   └── renderers/                  # 渲染器
│       ├── mod.rs
│       └── terminal_renderer.rs
├── services/                       # 业务服务层
│   ├── mod.rs
│   ├── market_service.rs
│   └── trading_service.rs
└── utils/                          # 工具函数
    ├── mod.rs
    ├── formatting.rs
    └── validation.rs
```

## 代码审查检查清单

### 架构和设计
- [ ] 是否使用组合而非继承？
- [ ] 是否正确实现了trait多态？
- [ ] 模块封装是否合理？
- [ ] 类型系统是否充分利用？
- [ ] 是否遵循单一职责原则？

### 内存和性能
- [ ] 所有权和借用是否正确？
- [ ] 是否使用了零成本抽象？
- [ ] 是否避免了不必要的内存分配？
- [ ] 热路径是否经过优化？
- [ ] 是否正确处理了内存对齐？

### 并发和安全
- [ ] 是否线程安全？
- [ ] 是否存在数据竞争风险？
- [ ] 是否正确使用了同步原语？
- [ ] 是否处理了所有错误情况？
- [ ] 是否有潜在的死锁？

### 代码质量
- [ ] 文件是否小于1000行？
- [ ] 错误处理是否完善？
- [ ] 是否有充分的测试覆盖？
- [ ] 文档是否完整准确？
- [ ] 是否遵循项目编码规范？

### 金融系统特定
- [ ] 数值计算是否精确？
- [ ] 是否处理了市场数据异常？
- [ ] 是否有适当的限流和熔断？
- [ ] 是否记录了关键操作日志？
- [ ] 是否支持数据回放和审计？