基于ringbuffer的纯粹的单线程无锁事件驱动架构的低延迟高频交易系统

1. 纯事件驱动 ：信号生成通过事件发布，不直接修改状态
2. 解耦合 ：SignalGenerator 与 OrderBook解耦
3. 异步处理 ：信号可以异步处理，提高性能
4. 统一时间系统 ：使用 Instant 保持一致性
5. 可扩展性 ：易于添加新的信号类型
6. 内存效率 ：避免在 OrderBook 中存储信号历史
7. UI界面的渲染规则为：UI基于orderbook的数据状态实时渲染，纯数据状态驱动渲染，而不是基于事件流触发。

需求1:
实时撤单/增加订单统计功能，根据tick trade的改变获取前后的orderbook的挂单，best ask和best bid的挂单
当有新的tick trade主动订单成交时触发检测 