use rdev::{listen, Event, EventType};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// 全局键鼠活动监听器
pub struct ActivityMonitor {
    last_activity: Arc<AtomicU64>,
    is_running: Arc<AtomicBool>,
}

impl ActivityMonitor {
    pub fn new() -> Self {
        Self {
            last_activity: Arc::new(AtomicU64::new(Self::current_timestamp())),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// 启动监听线程
    pub fn start(&self) {
        if self.is_running.load(Ordering::SeqCst) {
            return;
        }

        self.is_running.store(true, Ordering::SeqCst);
        let last_activity = self.last_activity.clone();
        let is_running = self.is_running.clone();

        std::thread::spawn(move || {
            let callback = move |event: Event| {
                match event.event_type {
                    EventType::KeyPress(_)
                    | EventType::KeyRelease(_)
                    | EventType::ButtonPress(_)
                    | EventType::ButtonRelease(_)
                    | EventType::MouseMove { .. }
                    | EventType::Wheel { .. } => {
                        last_activity.store(Self::current_timestamp(), Ordering::SeqCst);
                    }
                }
            };

            // 监听全局事件
            if let Err(e) = listen(callback) {
                println!("Activity monitor error: {:?}", e);
                is_running.store(false, Ordering::SeqCst);
            }
        });
    }

    /// 停止监听
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// 检查是否在指定秒数内无活动
    pub fn is_inactive_for(&self, secs: u64) -> bool {
        let last = self.last_activity.load(Ordering::SeqCst);
        let now = Self::current_timestamp();
        now.saturating_sub(last) >= secs
    }

    /// 重置最后活动时间为当前时间
    pub fn reset_activity(&self) {
        self.last_activity
            .store(Self::current_timestamp(), Ordering::SeqCst);
    }
}

impl Default for ActivityMonitor {
    fn default() -> Self {
        Self::new()
    }
}
