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
- 定义自定义错误类型
- 使用`?`操作符简化错误传播

### 文档和测试
- 为所有公共API提供文档注释
- 编写单元测试验证功能正确性
- 使用集成测试验证模块间协作

### 性能考虑
- 避免不必要的克隆和分配
- 使用`&str`而非`String`作为只读参数
- 考虑使用`Cow<str>`处理可能需要所有权的字符串

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

- [ ] 是否使用组合而非继承？
- [ ] 是否正确实现了trait多态？
- [ ] 模块封装是否合理？
- [ ] 类型系统是否充分利用？
- [ ] 所有权和借用是否正确？
- [ ] 是否使用了零成本抽象？
- [ ] 文件是否小于1000行？
- [ ] 错误处理是否完善？