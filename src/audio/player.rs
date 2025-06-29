use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use rodio::{Decoder, OutputStream, Sink};
use std::sync::{Arc, Mutex};
use std::thread;

/// 音频播放器
pub struct AudioPlayer {
    _stream: OutputStream,
    sink: Arc<Mutex<Sink>>,
}

impl AudioPlayer {
    /// 创建新的音频播放器
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        
        Ok(AudioPlayer {
            _stream: stream,
            sink: Arc::new(Mutex::new(sink)),
        })
    }

    /// 播放音频文件
    pub fn play_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?;
        
        if let Ok(sink) = self.sink.lock() {
            sink.append(source);
        }
        
        Ok(())
    }

    /// 异步播放音频文件（完全非阻塞）
    pub fn play_file_async<P: AsRef<Path> + Send + 'static>(&self, path: P) {
        let sink = Arc::clone(&self.sink);

        // 使用独立线程确保完全非阻塞
        thread::spawn(move || {
            // 快速失败，避免长时间阻塞
            match File::open(&path) {
                Ok(file) => {
                    match Decoder::new(BufReader::new(file)) {
                        Ok(source) => {
                            // 使用普通lock，但在独立线程中，不会阻塞主线程
                            if let Ok(sink) = sink.lock() {
                                sink.append(source);
                            }
                        }
                        Err(_) => {
                            // 音频解码失败，静默忽略
                        }
                    }
                }
                Err(_) => {
                    // 音频文件打开失败，静默忽略
                }
            }
        });
    }

    /// 停止播放
    pub fn stop(&self) {
        if let Ok(sink) = self.sink.lock() {
            sink.stop();
        }
    }

    /// 暂停播放
    pub fn pause(&self) {
        if let Ok(sink) = self.sink.lock() {
            sink.pause();
        }
    }

    /// 恢复播放
    pub fn resume(&self) {
        if let Ok(sink) = self.sink.lock() {
            sink.play();
        }
    }

    /// 设置音量 (0.0 - 1.0)
    pub fn set_volume(&self, volume: f32) {
        if let Ok(sink) = self.sink.lock() {
            sink.set_volume(volume.clamp(0.0, 1.0));
        }
    }
}

/// 全局音频播放器实例
static mut GLOBAL_AUDIO_PLAYER: Option<AudioPlayer> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// 获取全局音频播放器实例
pub fn get_global_audio_player() -> Option<&'static AudioPlayer> {
    unsafe {
        INIT.call_once(|| {
            if let Ok(player) = AudioPlayer::new() {
                GLOBAL_AUDIO_PLAYER = Some(player);
            }
        });
        GLOBAL_AUDIO_PLAYER.as_ref()
    }
}

/// 播放买单音效
pub fn play_buy_sound() {
    if let Some(player) = get_global_audio_player() {
        player.play_file_async("src/audio/buy.mp3");
    }
}

/// 播放卖单音效
pub fn play_sell_sound() {
    if let Some(player) = get_global_audio_player() {
        player.play_file_async("src/audio/sell.mp3");
    }
}

/// 播放ΔTick Pressure信号音效
pub fn play_tick_pressure_sound(is_buy: bool) {
    if is_buy {
        play_buy_sound();
    } else {
        play_sell_sound();
    }
}
