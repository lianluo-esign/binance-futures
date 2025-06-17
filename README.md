基于优化的低延迟的ringbuffer的纯粹的单线程无锁事件驱动架构的低延迟高频交易系统

1. 纯事件驱动 ：信号生成通过事件发布，不直接修改状态
2. 解耦合 ：SignalGenerator 与 OrderBook解耦
3. 异步处理 ：信号可以异步处理，提高性能
4. 统一时间系统 ：使用 Instant 保持一致性
5. 可扩展性 ：易于添加新的信号类型
6. 内存效率 ：避免在 OrderBook 中存储信号历史
7. UI界面的渲染规则为：UI基于orderbook的数据状态实时渲染，纯数据状态驱动渲染，而不是基于事件流触发。
8. ringbuffer支持充分利用L1/L2缓存的功能

需求1:
实时计算撤单/增加订单功能，每次当前新的depth update的获取，和上一次update的时候的订单薄的数据中的order_flow数据的snapshot 做diff， 如果当前的order_flow的bid变多了 就是增加了挂单，如果bid变少了 就是撤单了，

并且对当前的tick trade价格的同价位的order做减法去除主动订单的消耗得出真正的撤单和增加订单的数量。实时撤单保存在realtime_order_cancel

增加订单保存在realtime_increase_order


需求2:
首先对handle_book_ticker的事件进行处理，增加对bookTicker的快照，也就是每次更新完orderflow之后临时保存一份bookticker的数据到booktickerSnapshot，后面要用到。

当tick trade 被触发，如果是一个主动的buy trade且成交量大于booktickerSnapshot的best ask量，则判定为订单冲击，产生一个订单冲击买入的信号投放到事件缓冲区。 

如果是一个主动的sell trade且成交量大于booktickerSnapshot的best bid量，则判定为订单冲击，产生一个订单冲击卖出的信号。
