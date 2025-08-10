// Virtual Clock Module - 虚拟时钟系统
//
// 本模块实现了高精度的虚拟时钟系统，负责：
// - 基于纳秒级时间戳的精确时间控制
// - 可配置的播放速度控制（支持0-1000x）
// - 线程安全的时间状态管理
// - 暂停/恢复/跳转功能
// - 现实时间与虚拟时间的精确映射
//
// 设计原则：
// 1. 高精度：使用纳秒级时间戳确保精确时间控制
// 2. 线程安全：使用Arc<Mutex>保证多线程安全
// 3. 零成本抽象：编译时优化，运行时零开销
// 4. 可测试性：提供mock支持便于单元测试
// 5. 内存安全：利用Rust所有权系统确保安全

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// 虚拟时钟配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualClockConfig {
    /// 初始播放速度倍数 (0.0 = 无延迟模式, >0.0 = 正常速度控制)
    pub initial_speed: f64,
    
    /// 最大播放速度
    pub max_speed: f64,
    
    /// 最小播放速度
    pub min_speed: f64,
    
    /// 是否启用纳秒级精度（false时使用微秒精度以提高性能）
    pub nanosecond_precision: bool,
    
    /// 时间漂移容差（纳秒）- 允许的时间误差范围
    pub drift_tolerance_ns: u64,
}

impl Default for VirtualClockConfig {
    fn default() -> Self {
        Self {
            initial_speed: 1.0,
            max_speed: 1000.0,
            min_speed: 0.1,
            nanosecond_precision: true,
            drift_tolerance_ns: 1_000_000, // 1ms tolerance
        }
    }
}

/// 虚拟时钟状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClockState {
    /// 停止状态
    Stopped,
    /// 运行状态
    Running,
    /// 暂停状态
    Paused,
}

/// 时间同步结果
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncResult {
    /// 事件应该立即发送
    SendNow,
    /// 事件还未到发送时间，返回需要等待的时间
    WaitFor(Duration),
    /// 事件已经过期（落后太多）
    Expired,
}

/// 虚拟时钟内部状态
#[derive(Debug, Clone)]
struct ClockState_ {
    /// 当前状态
    state: ClockState,
    
    /// 播放速度
    speed: f64,
    
    /// 虚拟时间起始点（纳秒时间戳）
    virtual_start_ns: Option<u64>,
    
    /// 实际时间起始点
    real_start_time: Option<Instant>,
    
    /// 当前虚拟时间（纳秒时间戳）
    current_virtual_ns: u64,
    
    /// 累积的暂停时间
    accumulated_pause_duration: Duration,
    
    /// 最近一次状态改变的时间
    last_state_change: Instant,
    
    /// 配置
    config: VirtualClockConfig,
}

/// 虚拟时钟实现
/// 
/// 这是一个高精度的虚拟时钟，能够根据配置的播放速度
/// 精确控制事件的发送时间。支持纳秒级精度和多种播放模式。
#[derive(Debug, Clone)]
pub struct VirtualClock {
    state: Arc<Mutex<ClockState_>>,
}

impl VirtualClock {
    /// 创建新的虚拟时钟
    pub fn new(config: VirtualClockConfig) -> Self {
        let state = ClockState_ {
            state: ClockState::Stopped,
            speed: config.initial_speed,
            virtual_start_ns: None,
            real_start_time: None,
            current_virtual_ns: 0,
            accumulated_pause_duration: Duration::ZERO,
            last_state_change: Instant::now(),
            config,
        };

        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// 启动时钟，设置虚拟时间起点
    pub fn start(&self, virtual_start_timestamp_ns: u64) -> Result<(), VirtualClockError> {
        let mut state = self.state.lock()
            .map_err(|_| VirtualClockError::LockFailure)?;

        let now = Instant::now();
        
        state.state = ClockState::Running;
        state.virtual_start_ns = Some(virtual_start_timestamp_ns);
        state.real_start_time = Some(now);
        state.current_virtual_ns = virtual_start_timestamp_ns;
        state.accumulated_pause_duration = Duration::ZERO;
        state.last_state_change = now;

        Ok(())
    }

    /// 停止时钟
    pub fn stop(&self) -> Result<(), VirtualClockError> {
        let mut state = self.state.lock()
            .map_err(|_| VirtualClockError::LockFailure)?;

        state.state = ClockState::Stopped;
        state.virtual_start_ns = None;
        state.real_start_time = None;
        state.accumulated_pause_duration = Duration::ZERO;
        state.last_state_change = Instant::now();

        Ok(())
    }

    /// 暂停时钟
    pub fn pause(&self) -> Result<(), VirtualClockError> {
        let mut state = self.state.lock()
            .map_err(|_| VirtualClockError::LockFailure)?;

        if state.state == ClockState::Running {
            // 更新当前虚拟时间到暂停点
            self.update_current_virtual_time_locked(&mut state)?;
            
            state.state = ClockState::Paused;
            state.last_state_change = Instant::now();
        }

        Ok(())
    }

    /// 恢复时钟
    pub fn resume(&self) -> Result<(), VirtualClockError> {
        let mut state = self.state.lock()
            .map_err(|_| VirtualClockError::LockFailure)?;

        if state.state == ClockState::Paused {
            let now = Instant::now();
            let pause_duration = now.duration_since(state.last_state_change);
            
            state.state = ClockState::Running;
            state.accumulated_pause_duration += pause_duration;
            state.last_state_change = now;
        }

        Ok(())
    }

    /// 设置播放速度
    pub fn set_speed(&self, speed: f64) -> Result<(), VirtualClockError> {
        let mut state = self.state.lock()
            .map_err(|_| VirtualClockError::LockFailure)?;

        // 验证速度范围
        if speed < 0.0 || (speed > 0.0 && speed < state.config.min_speed) || 
           speed > state.config.max_speed {
            return Err(VirtualClockError::InvalidSpeed { 
                speed, 
                min: state.config.min_speed, 
                max: state.config.max_speed 
            });
        }

        // 如果时钟正在运行，更新当前虚拟时间并重置基准
        if state.state == ClockState::Running {
            self.update_current_virtual_time_locked(&mut state)?;
            
            // 重置时间基准以新速度继续
            let now = Instant::now();
            state.real_start_time = Some(now);
            state.virtual_start_ns = Some(state.current_virtual_ns);
            state.accumulated_pause_duration = Duration::ZERO;
        }

        state.speed = speed;
        Ok(())
    }

    /// 跳转到指定虚拟时间
    pub fn seek_to(&self, virtual_timestamp_ns: u64) -> Result<(), VirtualClockError> {
        let mut state = self.state.lock()
            .map_err(|_| VirtualClockError::LockFailure)?;

        state.current_virtual_ns = virtual_timestamp_ns;
        
        // 如果时钟正在运行，重置时间基准
        if state.state == ClockState::Running {
            let now = Instant::now();
            state.real_start_time = Some(now);
            state.virtual_start_ns = Some(virtual_timestamp_ns);
            state.accumulated_pause_duration = Duration::ZERO;
            state.last_state_change = now;
        }

        Ok(())
    }

    /// 检查事件是否应该发送
    /// 
    /// 返回SyncResult指示事件处理方式：
    /// - SendNow: 立即发送
    /// - WaitFor(duration): 等待指定时间后再发送
    /// - Expired: 事件已过期
    pub fn should_send_event(&self, event_timestamp_ns: u64) -> Result<SyncResult, VirtualClockError> {
        let mut state = self.state.lock()
            .map_err(|_| VirtualClockError::LockFailure)?;

        match state.state {
            ClockState::Stopped => return Ok(SyncResult::SendNow),
            ClockState::Paused => return Ok(SyncResult::WaitFor(Duration::from_millis(10))),
            ClockState::Running => {
                // 速度为0表示无延迟模式
                if state.speed == 0.0 {
                    state.current_virtual_ns = event_timestamp_ns;
                    return Ok(SyncResult::SendNow);
                }

                // 更新当前虚拟时间
                self.update_current_virtual_time_locked(&mut state)?;

                // 计算事件与当前虚拟时间的差异
                let time_diff = event_timestamp_ns as i64 - state.current_virtual_ns as i64;

                if time_diff <= -(state.config.drift_tolerance_ns as i64) {
                    // 事件已经过期太久
                    return Ok(SyncResult::Expired);
                } else if time_diff <= (state.config.drift_tolerance_ns as i64) {
                    // 事件在容差范围内，可以立即发送
                    state.current_virtual_ns = event_timestamp_ns;
                    return Ok(SyncResult::SendNow);
                } else {
                    // 事件还未到时间，计算需要等待多长时间
                    let wait_virtual_ns = time_diff as u64;
                    let wait_real_ns = if state.speed > 0.0 {
                        (wait_virtual_ns as f64 / state.speed) as u64
                    } else {
                        0
                    };

                    let wait_duration = Duration::from_nanos(wait_real_ns);
                    return Ok(SyncResult::WaitFor(wait_duration));
                }
            }
        }
    }

    /// 获取当前虚拟时间（纳秒）
    pub fn current_virtual_time(&self) -> Result<u64, VirtualClockError> {
        let mut state = self.state.lock()
            .map_err(|_| VirtualClockError::LockFailure)?;

        if state.state == ClockState::Running {
            self.update_current_virtual_time_locked(&mut state)?;
        }

        Ok(state.current_virtual_ns)
    }

    /// 获取当前播放速度
    pub fn current_speed(&self) -> Result<f64, VirtualClockError> {
        let state = self.state.lock()
            .map_err(|_| VirtualClockError::LockFailure)?;
        Ok(state.speed)
    }

    /// 获取当前状态
    pub fn current_state(&self) -> Result<ClockState, VirtualClockError> {
        let state = self.state.lock()
            .map_err(|_| VirtualClockError::LockFailure)?;
        Ok(state.state)
    }

    /// 获取时钟统计信息
    pub fn get_statistics(&self) -> Result<VirtualClockStatistics, VirtualClockError> {
        let state = self.state.lock()
            .map_err(|_| VirtualClockError::LockFailure)?;

        let total_real_duration = if let Some(start_time) = state.real_start_time {
            start_time.elapsed()
        } else {
            Duration::ZERO
        };

        let effective_real_duration = total_real_duration.saturating_sub(state.accumulated_pause_duration);

        let virtual_duration = if let Some(virtual_start) = state.virtual_start_ns {
            Duration::from_nanos(state.current_virtual_ns.saturating_sub(virtual_start))
        } else {
            Duration::ZERO
        };

        Ok(VirtualClockStatistics {
            state: state.state,
            speed: state.speed,
            current_virtual_ns: state.current_virtual_ns,
            virtual_start_ns: state.virtual_start_ns,
            total_real_duration,
            effective_real_duration,
            virtual_duration,
            accumulated_pause_duration: state.accumulated_pause_duration,
            config: state.config.clone(),
        })
    }

    /// 内部方法：更新当前虚拟时间（需要已持有锁）
    fn update_current_virtual_time_locked(&self, state: &mut ClockState_) -> Result<(), VirtualClockError> {
        if state.state != ClockState::Running {
            return Ok(());
        }

        let Some(real_start) = state.real_start_time else {
            return Err(VirtualClockError::ClockNotStarted);
        };

        let Some(virtual_start) = state.virtual_start_ns else {
            return Err(VirtualClockError::ClockNotStarted);
        };

        let now = Instant::now();
        let total_elapsed = now.duration_since(real_start);
        let effective_elapsed = total_elapsed.saturating_sub(state.accumulated_pause_duration);

        if state.speed > 0.0 {
            let virtual_elapsed_ns = (effective_elapsed.as_nanos() as f64 * state.speed) as u64;
            state.current_virtual_ns = virtual_start.saturating_add(virtual_elapsed_ns);
        } else {
            // 速度为0时不更新时间
        }

        Ok(())
    }
}

/// 虚拟时钟统计信息
#[derive(Debug, Clone)]
pub struct VirtualClockStatistics {
    /// 当前状态
    pub state: ClockState,
    /// 当前速度
    pub speed: f64,
    /// 当前虚拟时间（纳秒）
    pub current_virtual_ns: u64,
    /// 虚拟时间起点（纳秒）
    pub virtual_start_ns: Option<u64>,
    /// 总实际运行时间
    pub total_real_duration: Duration,
    /// 有效实际运行时间（扣除暂停）
    pub effective_real_duration: Duration,
    /// 虚拟时间跨度
    pub virtual_duration: Duration,
    /// 累积暂停时间
    pub accumulated_pause_duration: Duration,
    /// 配置信息
    pub config: VirtualClockConfig,
}

/// 虚拟时钟错误类型
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum VirtualClockError {
    /// 锁获取失败
    #[error("Failed to acquire lock")]
    LockFailure,
    
    /// 时钟未启动
    #[error("Clock is not started")]
    ClockNotStarted,
    
    /// 无效的播放速度
    #[error("Invalid speed {speed}, must be 0.0 or between {min} and {max}")]
    InvalidSpeed { speed: f64, min: f64, max: f64 },
    
    /// 时间计算溢出
    #[error("Time calculation overflow")]
    TimeOverflow,
}

/// 高性能时钟trait - 用于性能关键的场景
/// 
/// 这个trait提供了更直接的接口，避免了锁的开销，
/// 适用于单线程或已经有外部同步的场景。
pub trait HighPerformanceClock {
    type Error;

    /// 快速检查事件是否应该发送（无锁版本）
    fn fast_should_send(&self, event_timestamp_ns: u64, current_real_time: Instant) -> Result<bool, Self::Error>;
    
    /// 快速更新虚拟时间
    fn fast_update_time(&mut self, current_real_time: Instant) -> Result<u64, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_virtual_clock_creation() {
        let config = VirtualClockConfig::default();
        let clock = VirtualClock::new(config.clone());
        
        let stats = clock.get_statistics().unwrap();
        assert_eq!(stats.state, ClockState::Stopped);
        assert_eq!(stats.speed, config.initial_speed);
    }

    #[test]
    fn test_clock_start_stop() {
        let clock = VirtualClock::new(VirtualClockConfig::default());
        
        // 启动时钟
        let start_time = 1000_000_000_000u64; // 1秒的纳秒时间戳
        assert!(clock.start(start_time).is_ok());
        
        let stats = clock.get_statistics().unwrap();
        assert_eq!(stats.state, ClockState::Running);
        assert_eq!(stats.virtual_start_ns, Some(start_time));
        
        // 停止时钟
        assert!(clock.stop().is_ok());
        
        let stats = clock.get_statistics().unwrap();
        assert_eq!(stats.state, ClockState::Stopped);
    }

    #[test]
    fn test_pause_resume() {
        let clock = VirtualClock::new(VirtualClockConfig::default());
        
        let start_time = 1000_000_000_000u64;
        clock.start(start_time).unwrap();
        
        // 暂停
        thread::sleep(Duration::from_millis(10));
        clock.pause().unwrap();
        assert_eq!(clock.current_state().unwrap(), ClockState::Paused);
        
        // 恢复
        thread::sleep(Duration::from_millis(10));
        clock.resume().unwrap();
        assert_eq!(clock.current_state().unwrap(), ClockState::Running);
    }

    #[test]
    fn test_speed_control() {
        let mut config = VirtualClockConfig::default();
        config.max_speed = 10.0;
        config.min_speed = 0.5;
        
        let clock = VirtualClock::new(config);
        
        // 测试有效速度
        assert!(clock.set_speed(2.0).is_ok());
        assert_eq!(clock.current_speed().unwrap(), 2.0);
        
        // 测试无效速度
        assert!(clock.set_speed(20.0).is_err()); // 超过最大值
        assert!(clock.set_speed(0.1).is_err());  // 小于最小值
        
        // 测试速度为0（无延迟模式）
        assert!(clock.set_speed(0.0).is_ok());
        assert_eq!(clock.current_speed().unwrap(), 0.0);
    }

    #[test]
    fn test_event_timing_speed_zero() {
        let clock = VirtualClock::new(VirtualClockConfig::default());
        
        let start_time = 1000_000_000_000u64;
        clock.start(start_time).unwrap();
        clock.set_speed(0.0).unwrap(); // 无延迟模式
        
        // 任何事件都应该立即发送
        let event_time = start_time + 1_000_000_000; // +1秒
        let result = clock.should_send_event(event_time).unwrap();
        assert_eq!(result, SyncResult::SendNow);
    }

    #[test]
    fn test_event_timing_normal_speed() {
        let clock = VirtualClock::new(VirtualClockConfig::default());
        
        let start_time = 1000_000_000_000u64;
        clock.start(start_time).unwrap();
        clock.set_speed(1.0).unwrap(); // 正常速度
        
        // 立即检查同一时间的事件 - 应该立即发送
        let result = clock.should_send_event(start_time).unwrap();
        assert_eq!(result, SyncResult::SendNow);
        
        // 检查未来事件 - 应该等待
        let future_event = start_time + 100_000_000; // +100ms
        let result = clock.should_send_event(future_event).unwrap();
        match result {
            SyncResult::WaitFor(duration) => {
                assert!(duration.as_millis() > 50); // 至少等待50ms
            }
            _ => panic!("Expected WaitFor result"),
        }
    }

    #[test]
    fn test_event_timing_2x_speed() {
        let clock = VirtualClock::new(VirtualClockConfig::default());
        
        let start_time = 1000_000_000_000u64;
        clock.start(start_time).unwrap();
        clock.set_speed(2.0).unwrap(); // 2x速度
        
        // 等待一小段时间让虚拟时间推进
        thread::sleep(Duration::from_millis(50));
        
        let future_event = start_time + 100_000_000; // +100ms的事件
        let result = clock.should_send_event(future_event).unwrap();
        
        // 在2x速度下，100ms的事件应该在大约50ms后可用
        // 由于我们已经等待了50ms，这个事件应该接近可发送状态
        match result {
            SyncResult::SendNow | SyncResult::WaitFor(_) => {
                // 都是合理的结果，取决于精确的时间
            }
            SyncResult::Expired => panic!("Event should not be expired"),
        }
    }

    #[test]
    fn test_seek_functionality() {
        let clock = VirtualClock::new(VirtualClockConfig::default());
        
        let start_time = 1000_000_000_000u64;
        clock.start(start_time).unwrap();
        
        // 跳转到未来时间
        let seek_time = start_time + 5_000_000_000; // +5秒
        assert!(clock.seek_to(seek_time).is_ok());
        
        let current_time = clock.current_virtual_time().unwrap();
        assert_eq!(current_time, seek_time);
    }

    #[test]
    fn test_statistics() {
        let clock = VirtualClock::new(VirtualClockConfig::default());
        
        let start_time = 1000_000_000_000u64;
        clock.start(start_time).unwrap();
        
        thread::sleep(Duration::from_millis(10));
        
        let stats = clock.get_statistics().unwrap();
        assert_eq!(stats.state, ClockState::Running);
        assert!(stats.total_real_duration.as_millis() >= 10);
        assert_eq!(stats.virtual_start_ns, Some(start_time));
    }
}