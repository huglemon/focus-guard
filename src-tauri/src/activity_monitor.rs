use rdev::{listen, Event, EventType};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "macos")]
use std::process::Command;

/// 检查是否有辅助功能权限（macOS）
#[cfg(target_os = "macos")]
pub fn check_accessibility_permission() -> bool {
    // 使用 AppleScript 检查辅助功能权限
    let output = Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to return (exists process 1)"])
        .output();

    match output {
        Ok(result) => result.status.success(),
        Err(_) => false,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility_permission() -> bool {
    true
}

/// 请求辅助功能权限（macOS）- 打开系统设置
#[cfg(target_os = "macos")]
pub fn request_accessibility_permission() {
    // 打开系统设置的输入监控页面（rdev 需要这个权限）
    let _ = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
        .spawn();
}

#[cfg(not(target_os = "macos"))]
pub fn request_accessibility_permission() {
    // 非 macOS 平台不需要
}

/// 最近的活动类型
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum LastActivity {
    None,
    Key,
    Mouse,
    Move,
    Wheel,
}

impl std::fmt::Display for LastActivity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LastActivity::None => write!(f, "-"),
            LastActivity::Key => write!(f, "键"),
            LastActivity::Mouse => write!(f, "点"),
            LastActivity::Move => write!(f, "动"),
            LastActivity::Wheel => write!(f, "滚"),
        }
    }
}

/// 全局键鼠活动监听器（按需监控）
pub struct ActivityMonitor {
    last_activity: Arc<AtomicU64>,
    is_running: Arc<AtomicBool>,
    is_monitoring: Arc<AtomicBool>,
    last_activity_type: Arc<Mutex<LastActivity>>,
}

impl ActivityMonitor {
    pub fn new() -> Self {
        Self {
            last_activity: Arc::new(AtomicU64::new(Self::current_timestamp())),
            is_running: Arc::new(AtomicBool::new(false)),
            is_monitoring: Arc::new(AtomicBool::new(false)),
            last_activity_type: Arc::new(Mutex::new(LastActivity::None)),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// 启动监听线程（后台运行，但只在 is_monitoring 为 true 时记录活动）
    pub fn start(&self) {
        if self.is_running.load(Ordering::SeqCst) {
            return;
        }

        self.is_running.store(true, Ordering::SeqCst);
        let last_activity = self.last_activity.clone();
        let is_running = self.is_running.clone();
        let is_monitoring = self.is_monitoring.clone();
        let last_activity_type = self.last_activity_type.clone();

        std::thread::spawn(move || {
            println!("[ActivityMonitor] 监听线程已启动");

            let callback = move |event: Event| {
                // 只在监控模式下记录活动
                if is_monitoring.load(Ordering::SeqCst) {
                    match event.event_type {
                        EventType::KeyPress(_) | EventType::KeyRelease(_) => {
                            *last_activity_type.lock().unwrap() = LastActivity::Key;
                            last_activity.store(Self::current_timestamp(), Ordering::SeqCst);
                        }
                        EventType::ButtonPress(_) | EventType::ButtonRelease(_) => {
                            *last_activity_type.lock().unwrap() = LastActivity::Mouse;
                            last_activity.store(Self::current_timestamp(), Ordering::SeqCst);
                        }
                        EventType::MouseMove { .. } => {
                            *last_activity_type.lock().unwrap() = LastActivity::Move;
                            last_activity.store(Self::current_timestamp(), Ordering::SeqCst);
                        }
                        EventType::Wheel { .. } => {
                            *last_activity_type.lock().unwrap() = LastActivity::Wheel;
                            last_activity.store(Self::current_timestamp(), Ordering::SeqCst);
                        }
                    }
                }
            };

            println!("[ActivityMonitor] 开始调用 rdev::listen()...");
            match listen(callback) {
                Ok(_) => println!("[ActivityMonitor] listen() 正常退出"),
                Err(e) => {
                    println!("[ActivityMonitor] listen() 错误: {:?}", e);
                    is_running.store(false, Ordering::SeqCst);
                }
            }
        });
    }

    /// 开始监控（按需调用，发送久坐提醒后调用）
    pub fn start_monitoring(&self) {
        println!("[ActivityMonitor] 开始监控键鼠活动");
        self.is_monitoring.store(true, Ordering::SeqCst);
        *self.last_activity_type.lock().unwrap() = LastActivity::None;
        self.reset_activity();
    }

    /// 停止监控
    pub fn stop_monitoring(&self) {
        println!("[ActivityMonitor] 停止监控键鼠活动");
        self.is_monitoring.store(false, Ordering::SeqCst);
    }

    /// 是否正在监控
    #[allow(dead_code)]
    pub fn is_monitoring(&self) -> bool {
        self.is_monitoring.load(Ordering::SeqCst)
    }

    /// 获取最近的活动类型
    #[allow(dead_code)]
    pub fn get_last_activity_type(&self) -> LastActivity {
        self.last_activity_type.lock().unwrap().clone()
    }

    /// 停止监听线程
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        self.is_monitoring.store(false, Ordering::SeqCst);
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
