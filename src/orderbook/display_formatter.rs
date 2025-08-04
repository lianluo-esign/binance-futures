use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use ordered_float::OrderedFloat;
use crate::orderbook::OrderFlow;

/// 根据价格精度聚合订单簿数据，固定使用1美元精度向下取整
/// precision: 价格精度（USD增量），强制为1.0以保证向下取整到整美元
pub fn aggregate_price_levels(
    order_flows: &BTreeMap<OrderedFloat<f64>, OrderFlow>,
    precision: f64,
) -> BTreeMap<OrderedFloat<f64>, OrderFlow> {
    // 强制使用1美元精度，忽略传入的precision参数
    let dollar_precision = 1.0;
    
    let mut aggregated: BTreeMap<OrderedFloat<f64>, OrderFlow> = BTreeMap::new();

    for (price_key, order_flow) in order_flows {
        let original_price = price_key.0;

        // 向下取整到1美元精度
        // 例如：110000.0 -> 110000.0, 110000.5 -> 110000.0, 110000.9 -> 110000.0
        let aggregated_price = (original_price / dollar_precision).floor() * dollar_precision;
        let aggregated_key = OrderedFloat(aggregated_price);

        // 获取或创建聚合价格级别
        let aggregated_flow = aggregated.entry(aggregated_key).or_insert_with(OrderFlow::new);

        // 聚合买卖价格和数量（累加所有在同一美元价位的挂单量）
        aggregated_flow.bid_ask.bid += order_flow.bid_ask.bid;
        aggregated_flow.bid_ask.ask += order_flow.bid_ask.ask;
        aggregated_flow.bid_ask.timestamp = aggregated_flow.bid_ask.timestamp.max(order_flow.bid_ask.timestamp);

        // 聚合交易记录
        aggregated_flow.history_trade_record.buy_volume += order_flow.history_trade_record.buy_volume;
        aggregated_flow.history_trade_record.sell_volume += order_flow.history_trade_record.sell_volume;
        aggregated_flow.history_trade_record.timestamp = aggregated_flow.history_trade_record.timestamp.max(order_flow.history_trade_record.timestamp);

        aggregated_flow.realtime_trade_record.buy_volume += order_flow.realtime_trade_record.buy_volume;
        aggregated_flow.realtime_trade_record.sell_volume += order_flow.realtime_trade_record.sell_volume;
        aggregated_flow.realtime_trade_record.timestamp = aggregated_flow.realtime_trade_record.timestamp.max(order_flow.realtime_trade_record.timestamp);

        // 聚合撤单记录
        aggregated_flow.realtime_cancel_records.bid_cancel += order_flow.realtime_cancel_records.bid_cancel;
        aggregated_flow.realtime_cancel_records.ask_cancel += order_flow.realtime_cancel_records.ask_cancel;
        aggregated_flow.realtime_cancel_records.timestamp = aggregated_flow.realtime_cancel_records.timestamp.max(order_flow.realtime_cancel_records.timestamp);

        // 聚合增加订单
        aggregated_flow.realtime_increase_order.bid += order_flow.realtime_increase_order.bid;
        aggregated_flow.realtime_increase_order.ask += order_flow.realtime_increase_order.ask;
        aggregated_flow.realtime_increase_order.timestamp = aggregated_flow.realtime_increase_order.timestamp.max(order_flow.realtime_increase_order.timestamp);
    }

    aggregated
}

/// 增强的价格聚合函数，处理bid/ask冲突
/// 当best bid和best ask价格聚合到同一层级时，将bid数据向下聚合
pub fn aggregate_price_levels_with_conflict_resolution(
    order_flows: &BTreeMap<OrderedFloat<f64>, OrderFlow>,
    best_bid_price: Option<f64>,
    best_ask_price: Option<f64>,
    precision: f64,
) -> BTreeMap<OrderedFloat<f64>, OrderFlow> {
    let dollar_precision = 1.0;
    let mut aggregated: BTreeMap<OrderedFloat<f64>, OrderFlow> = BTreeMap::new();

    // 检查是否存在bid/ask冲突
    let has_conflict = if let (Some(best_bid), Some(best_ask)) = (best_bid_price, best_ask_price) {
        let best_bid_aggregated = (best_bid / dollar_precision).floor() * dollar_precision;
        let best_ask_aggregated = (best_ask / dollar_precision).floor() * dollar_precision;
        (best_bid_aggregated - best_ask_aggregated).abs() < 0.01 // 在同一聚合层级
    } else {
        false
    };

    for (price_key, order_flow) in order_flows {
        let original_price = price_key.0;
        let has_bid = order_flow.bid_ask.bid > 0.0;
        let has_ask = order_flow.bid_ask.ask > 0.0;

        // 计算聚合价格
        let mut aggregated_price = (original_price / dollar_precision).floor() * dollar_precision;
        
        // 如果存在冲突且当前价格有bid数据，将bid向下聚合一个层级
        if has_conflict && has_bid {
            if let Some(best_bid) = best_bid_price {
                let best_bid_aggregated = (best_bid / dollar_precision).floor() * dollar_precision;
                // 如果当前价格的bid会聚合到与best ask相同的层级，则向下聚合
                if (aggregated_price - best_bid_aggregated).abs() < 0.01 {
                    aggregated_price -= dollar_precision; // 向下移动1美元
                }
            }
        }

        let aggregated_key = OrderedFloat(aggregated_price);
        let aggregated_flow = aggregated.entry(aggregated_key).or_insert_with(OrderFlow::new);

        // 聚合数据
        aggregated_flow.bid_ask.bid += order_flow.bid_ask.bid;
        aggregated_flow.bid_ask.ask += order_flow.bid_ask.ask;
        aggregated_flow.bid_ask.timestamp = aggregated_flow.bid_ask.timestamp.max(order_flow.bid_ask.timestamp);

        // 聚合其他数据
        aggregated_flow.history_trade_record.buy_volume += order_flow.history_trade_record.buy_volume;
        aggregated_flow.history_trade_record.sell_volume += order_flow.history_trade_record.sell_volume;
        aggregated_flow.history_trade_record.timestamp = aggregated_flow.history_trade_record.timestamp.max(order_flow.history_trade_record.timestamp);

        aggregated_flow.realtime_trade_record.buy_volume += order_flow.realtime_trade_record.buy_volume;
        aggregated_flow.realtime_trade_record.sell_volume += order_flow.realtime_trade_record.sell_volume;
        aggregated_flow.realtime_trade_record.timestamp = aggregated_flow.realtime_trade_record.timestamp.max(order_flow.realtime_trade_record.timestamp);

        aggregated_flow.realtime_cancel_records.bid_cancel += order_flow.realtime_cancel_records.bid_cancel;
        aggregated_flow.realtime_cancel_records.ask_cancel += order_flow.realtime_cancel_records.ask_cancel;
        aggregated_flow.realtime_cancel_records.timestamp = aggregated_flow.realtime_cancel_records.timestamp.max(order_flow.realtime_cancel_records.timestamp);

        aggregated_flow.realtime_increase_order.bid += order_flow.realtime_increase_order.bid;
        aggregated_flow.realtime_increase_order.ask += order_flow.realtime_increase_order.ask;
        aggregated_flow.realtime_increase_order.timestamp = aggregated_flow.realtime_increase_order.timestamp.max(order_flow.realtime_increase_order.timestamp);
    }

    aggregated
}

/// 根据价格精度聚合交易价格，固定使用1美元精度向下取整
pub fn aggregate_trade_price(price: f64, precision: f64) -> f64 {
    // 强制使用1美元精度，忽略传入的precision参数
    let dollar_precision = 1.0;
    (price / dollar_precision).floor() * dollar_precision
}

/// 详细的订单数据模拟函数 - 与备份文件保持一致
pub fn simulate_order_data_detailed(price: f64, current_price: f64) -> (f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
    // 使用价格作为种子生成伪随机数据
    let mut hasher = DefaultHasher::new();
    ((price * 1000.0) as u64).hash(&mut hasher);
    let seed = hasher.finish();

    let distance = (price - current_price).abs();
    let base_volume = if distance < 1.0 { 10.0 } else { 5.0 / (distance + 1.0) };

    // 根据距离当前价格的远近调整成交量
    let bid_vol = if price < current_price {
        base_volume * (1.0 + (seed % 100) as f64 / 100.0)
    } else {
        base_volume * 0.3 * (1.0 + (seed % 50) as f64 / 100.0)
    };

    let ask_vol = if price > current_price {
        base_volume * (1.0 + ((seed >> 8) % 100) as f64 / 100.0)
    } else {
        base_volume * 0.3 * (1.0 + ((seed >> 8) % 50) as f64 / 100.0)
    };

    // 交易量
    let buy_trade_vol = if (seed >> 16) % 10 < 3 { bid_vol * 0.1 } else { 0.0 };
    let sell_trade_vol = if (seed >> 20) % 10 < 3 { ask_vol * 0.1 } else { 0.0 };

    // 撤单量
    let bid_cancel_vol = if (seed >> 24) % 20 < 2 { bid_vol * 0.2 } else { 0.0 };
    let ask_cancel_vol = if (seed >> 28) % 20 < 2 { ask_vol * 0.2 } else { 0.0 };

    // 增单量
    let bid_increase_vol = if (seed >> 32) % 15 < 2 { base_volume * 0.3 } else { 0.0 };
    let ask_increase_vol = if (seed >> 36) % 15 < 2 { base_volume * 0.3 } else { 0.0 };

    // 历史交易量
    let history_buy_vol = buy_trade_vol * 5.0;
    let history_sell_vol = sell_trade_vol * 5.0;

    (bid_vol, ask_vol, buy_trade_vol, sell_trade_vol, bid_cancel_vol, ask_cancel_vol,
     bid_increase_vol, ask_increase_vol, history_buy_vol, history_sell_vol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bid_ask_conflict_resolution() {
        let mut order_flows = BTreeMap::new();
        
        // 创建测试数据：bid和ask价格会聚合到同一层级
        let mut flow1 = OrderFlow::new();
        flow1.bid_ask.bid = 10.0; // 在110001.1的bid
        flow1.bid_ask.ask = 0.0;
        order_flows.insert(OrderedFloat(110001.1), flow1);
        
        let mut flow2 = OrderFlow::new();
        flow2.bid_ask.bid = 0.0;
        flow2.bid_ask.ask = 5.0; // 在110001.5的ask
        order_flows.insert(OrderedFloat(110001.5), flow2);
        
        // 测试场景：best_bid=110001.1, best_ask=110001.5
        // 它们都会聚合到110001层级，应该发生冲突
        let result = aggregate_price_levels_with_conflict_resolution(
            &order_flows,
            Some(110001.1),
            Some(110001.5),
            1.0
        );
        
        // 验证结果：bid应该被向下聚合到110000层级
        assert!(result.contains_key(&OrderedFloat(110000.0))); // bid被聚合到这里
        assert!(result.contains_key(&OrderedFloat(110001.0))); // ask仍在这里
        
        // 验证数据正确性
        let bid_level = &result[&OrderedFloat(110000.0)];
        let ask_level = &result[&OrderedFloat(110001.0)];
        
        assert_eq!(bid_level.bid_ask.bid, 10.0);
        assert_eq!(bid_level.bid_ask.ask, 0.0);
        assert_eq!(ask_level.bid_ask.bid, 0.0);
        assert_eq!(ask_level.bid_ask.ask, 5.0);
    }

    #[test] 
    fn test_no_conflict_normal_aggregation() {
        let mut order_flows = BTreeMap::new();
        
        // 创建测试数据：bid和ask价格不会冲突
        let mut flow1 = OrderFlow::new();
        flow1.bid_ask.bid = 10.0;
        flow1.bid_ask.ask = 0.0;
        order_flows.insert(OrderedFloat(110000.5), flow1);
        
        let mut flow2 = OrderFlow::new();
        flow2.bid_ask.bid = 0.0;
        flow2.bid_ask.ask = 5.0;
        order_flows.insert(OrderedFloat(110002.5), flow2);
        
        // 测试场景：best_bid=110000.5, best_ask=110002.5
        // 它们会聚合到不同层级，不应该冲突
        let result = aggregate_price_levels_with_conflict_resolution(
            &order_flows,
            Some(110000.5),
            Some(110002.5),
            1.0
        );
        
        // 验证结果：正常聚合，不发生冲突
        assert!(result.contains_key(&OrderedFloat(110000.0))); // bid聚合到这里
        assert!(result.contains_key(&OrderedFloat(110002.0))); // ask聚合到这里
        
        let bid_level = &result[&OrderedFloat(110000.0)];
        let ask_level = &result[&OrderedFloat(110002.0)];
        
        assert_eq!(bid_level.bid_ask.bid, 10.0);
        assert_eq!(ask_level.bid_ask.ask, 5.0);
    }
}