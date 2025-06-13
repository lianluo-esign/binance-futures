use crate::event::{Event, EventType, TimerType};
use crate::orderbook::{OrderBookData, ImbalanceSignal, BigOrder};
use crate::ringbuffer::EventRingBuffer;
use std::time::Instant;
use ordered_float::OrderedFloat;

// 信号事件类型
#[derive(Clone, Debug)]
pub enum SignalEventType {
    ImbalanceDetected {
        signal_type: String,
        ratio: f64,
        timestamp: Instant,
    },
    BigOrderDetected {
        order_type: String,
        volume: f64,
        price: f64,
        timestamp: Instant,
    },
    IcebergDetected {
        signal_type: String,
        volume: f64,
        timestamp: Instant,
    },
    CancelImbalanceDetected {
        signal_type: String,
        ratio: f64,
        timestamp: Instant,
    },
}

pub struct SignalGenerator {
    last_signal_check: Instant,
    signal_interval: std::time::Duration,
    imbalance_threshold: f64,
    big_order_threshold: f64,
    iceberg_detection_window: std::time::Duration,
    event_buffer: EventRingBuffer,
}

impl SignalGenerator {
    pub fn new(event_buffer: EventRingBuffer) -> Self {
        Self {
            last_signal_check: Instant::now(),
            signal_interval: std::time::Duration::from_millis(1000),
            imbalance_threshold: 0.7,
            big_order_threshold: 10.0,
            iceberg_detection_window: std::time::Duration::from_millis(30000),
            event_buffer,
        }
    }

    // 事件驱动的信号生成
    pub fn process_timer_event(&mut self, orderbook: &OrderBookData) {
        let current_time = Instant::now();
        
        if current_time.duration_since(self.last_signal_check) < self.signal_interval {
            return;
        }
        
        self.last_signal_check = current_time;
        
        self.detect_imbalance_signals(orderbook, current_time);
        self.detect_big_order_signals(orderbook, current_time);
        self.detect_iceberg_signals(orderbook, current_time);
        self.detect_cancel_signals(orderbook, current_time);
    }

    fn detect_imbalance_signals(&self, orderbook: &OrderBookData, current_time: Instant) {
        let mut total_bid_volume = 0.0;
        let mut total_ask_volume = 0.0;
        let mut bid_levels = 0;
        let mut ask_levels = 0;
        
        for (_price, order_flow) in &orderbook.order_flows {
            if order_flow.bid_ask.bid > 0.0 && bid_levels < 10 {
                total_bid_volume += order_flow.bid_ask.bid;
                bid_levels += 1;
            }
            if order_flow.bid_ask.ask > 0.0 && ask_levels < 10 {
                total_ask_volume += order_flow.bid_ask.ask;
                ask_levels += 1;
            }
        }

        let total_volume = total_bid_volume + total_ask_volume;
        if total_volume > 0.0 {
            let bid_ratio = total_bid_volume / total_volume;
            let ask_ratio = total_ask_volume / total_volume;

            if bid_ratio > self.imbalance_threshold {
                let signal_event = Event::new(EventType::Signal(SignalEventType::ImbalanceDetected {
                    signal_type: "buy".to_string(),
                    ratio: bid_ratio,
                    timestamp: current_time,
                }));
                let _ = self.event_buffer.push(signal_event);
            } else if ask_ratio > self.imbalance_threshold {
                let signal_event = Event::new(EventType::Signal(SignalEventType::ImbalanceDetected {
                    signal_type: "sell".to_string(),
                    ratio: ask_ratio,
                    timestamp: current_time,
                }));
                let _ = self.event_buffer.push(signal_event);
            }
        }
    }

    fn detect_big_order_signals(&self, orderbook: &OrderBookData, current_time: Instant) {
        for (price, order_flow) in &orderbook.order_flows {
            if order_flow.bid_ask.bid > self.big_order_threshold {
                let signal_event = Event::new(EventType::Signal(SignalEventType::BigOrderDetected {
                    order_type: "buy".to_string(),
                    volume: order_flow.bid_ask.bid,
                    price: price.0,
                    timestamp: current_time,
                }));
                let _ = self.event_buffer.push(signal_event);
            }
            if order_flow.bid_ask.ask > self.big_order_threshold {
                let signal_event = Event::new(EventType::Signal(SignalEventType::BigOrderDetected {
                    order_type: "sell".to_string(),
                    volume: order_flow.bid_ask.ask,
                    price: price.0,
                    timestamp: current_time,
                }));
                let _ = self.event_buffer.push(signal_event);
            }
        }
    }

    fn detect_iceberg_signals(&self, orderbook: &OrderBookData, current_time: Instant) {
        for (price, order_flow) in &orderbook.order_flows {
            // 检查买单冰山订单
            if order_flow.bid_ask.bid > 5.0 {
                let history_buy_volume = order_flow.history_trade_record.buy_volume;
                
                if history_buy_volume > order_flow.bid_ask.bid * 2.0 {
                    let signal_event = Event::new(EventType::Signal(SignalEventType::IcebergDetected {
                        signal_type: "buy".to_string(),
                        volume: order_flow.bid_ask.bid,
                        timestamp: current_time,
                    }));
                    let _ = self.event_buffer.push(signal_event);
                }
            }
            
            // 检查卖单冰山订单
            if order_flow.bid_ask.ask > 5.0 {
                let history_sell_volume = order_flow.history_trade_record.sell_volume;
                
                if history_sell_volume > order_flow.bid_ask.ask * 2.0 {
                    let signal_event = Event::new(EventType::Signal(SignalEventType::IcebergDetected {
                        signal_type: "sell".to_string(),
                        volume: order_flow.bid_ask.ask,
                        timestamp: current_time,
                    }));
                    let _ = self.event_buffer.push(signal_event);
                }
            }
        }
    }

    fn detect_cancel_signals(&self, orderbook: &OrderBookData, current_time: Instant) {
        let mut total_bid_cancel = 0.0;
        let mut total_ask_cancel = 0.0;
        
        for (_price, order_flow) in &orderbook.order_flows {
            total_bid_cancel += order_flow.realtime_cancel_records.bid_cancel;
            total_ask_cancel += order_flow.realtime_cancel_records.ask_cancel;
        }
        
        let total_cancel = total_bid_cancel + total_ask_cancel;
        if total_cancel > 10.0 {
            if total_bid_cancel > total_ask_cancel * 2.0 {
                let signal_event = Event::new(EventType::Signal(SignalEventType::CancelImbalanceDetected {
                    signal_type: "bid_cancel".to_string(),
                    ratio: total_bid_cancel / total_cancel,
                    timestamp: current_time,
                }));
                let _ = self.event_buffer.push(signal_event);
            } else if total_ask_cancel > total_bid_cancel * 2.0 {
                let signal_event = Event::new(EventType::Signal(SignalEventType::CancelImbalanceDetected {
                    signal_type: "ask_cancel".to_string(),
                    ratio: total_ask_cancel / total_cancel,
                    timestamp: current_time,
                }));
                let _ = self.event_buffer.push(signal_event);
            }
        }
    }
}