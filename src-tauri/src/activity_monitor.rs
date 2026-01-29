use rdev::{listen, Event, EventType};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "macos")]
use std::process::Command;

/// 检查是否有输入监控权限（macOS）
/// 注意：这个检查不是100%准确，最可靠的方法是尝试监听并检查是否有事件
#[cfg(target_os = "macos")]
pub fn check_accessibility_permission() -> bool {
    // 尝试使用 tccutil 检查输入监控权限
    // 但这个方法也不完全可靠，所以我们主要依赖 rdev 的错误处理
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
    event_count: Arc<AtomicU64>,           // 事件计数器，用于调试
    monitoring_start_time: Arc<AtomicU64>, // 开始监控的时间
}

impl ActivityMonitor {
    pub fn new() -> Self {
        Self {
            last_activity: Arc::new(AtomicU64::new(Self::current_timestamp())),
            is_running: Arc::new(AtomicBool::new(false)),
            is_monitoring: Arc::new(AtomicBool::new(false)),
            last_activity_type: Arc::new(Mutex::new(LastActivity::None)),
            event_count: Arc::new(AtomicU64::new(0)),
            monitoring_start_time: Arc::new(AtomicU64::new(0)),
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
        let event_count = self.event_count.clone();

        std::thread::spawn(move || {
            println!("[ActivityMonitor] 监听线程已启动");

            let callback = move |event: Event| {
                // 只在监控模式下记录活动
                if is_monitoring.load(Ordering::SeqCst) {
                    let activity_type = match event.event_type {
                        EventType::KeyPress(_) | EventType::KeyRelease(_) => {
                            Some(LastActivity::Key)
                        }
                        EventType::ButtonPress(_) | EventType::ButtonRelease(_) => {
                            Some(LastActivity::Mouse)
                        }
                        EventType::MouseMove { .. } => {
                            Some(LastActivity::Move)
                        }
                        EventType::Wheel { .. } => {
                            Some(LastActivity::Wheel)
                        }
                    };

                    if let Some(activity) = activity_type {
                        let count = event_count.fetch_add(1, Ordering::SeqCst) + 1;
                        *last_activity_type.lock().unwrap() = activity;
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        last_activity.store(now, Ordering::SeqCst);

                        // 每100个事件打印一次日志
                        if count % 100 == 0 {
                            println!("[ActivityMonitor] 已接收 {} 个事件，最后活动时间: {}", count, now);
                        }
                    }
                }
            };

            println!("[ActivityMonitor] 开始调用 rdev::listen()...");
            match listen(callback) {
                Ok(_) => println!("[ActivityMonitor] listen() 正常退出"),
                Err(e) => {
                    println!("[ActivityMonitor] listen() 错误: {:?}", e);
                    println!("[ActivityMonitor] 请确保已授予输入监控权限（系统设置 > 隐私与安全性 > 输入监控）");
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
        self.event_count.store(0, Ordering::SeqCst);
        self.monitoring_start_time.store(Self::current_timestamp(), Ordering::SeqCst);
        // 不再调用 reset_activity()，让 last_activity 保持之前的值
        // 这样如果没有收到任何事件，has_activity_since_monitoring_started() 会返回 false
    }

    /// 停止监控
    pub fn stop_monitoring(&self) {
        let count = self.event_count.load(Ordering::SeqCst);
        println!("[ActivityMonitor] 停止监控键鼠活动，共接收 {} 个事件", count);
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
    /// 注意：这个方法检查的是从开始监控以来是否有活动
    pub fn is_inactive_for(&self, secs: u64) -> bool {
        let last = self.last_activity.load(Ordering::SeqCst);
        let monitoring_start = self.monitoring_start_time.load(Ordering::SeqCst);
        let event_count = self.event_count.load(Ordering::SeqCst);

        // 如果没有收到任何事件，认为用户无活动（可能是权限问题或用户真的没活动）
        if event_count == 0 {
            println!("[ActivityMonitor] 检查活动状态：未收到任何事件，可能需要检查输入监控权限");
            // 如果没有收到事件，我们无法确定用户是否有活动
            // 保守起见，返回 false（认为用户有活动），避免误重置
            return false;
        }

        // 检查最后活动时间是否在监控开始之后
        if last <= monitoring_start {
            // 最后活动时间在监控开始之前，说明监控期间没有活动
            println!("[ActivityMonitor] 检查活动状态：监控期间无活动（last={}, start={}）", last, monitoring_start);
            return true;
        }

        // 检查最后活动距今是否超过指定秒数
        let now = Self::current_timestamp();
        let inactive = now.saturating_sub(last) >= secs;
        println!("[ActivityMonitor] 检查活动状态：last={}, now={}, secs={}, inactive={}", last, now, secs, inactive);
        inactive
    }

    /// 重置最后活动时间为当前时间
    pub fn reset_activity(&self) {
        self.last_activity
            .store(Self::current_timestamp(), Ordering::SeqCst);
    }

    /// 获取监控期间的事件数量
    #[allow(dead_code)]
    pub fn get_event_count(&self) -> u64 {
        self.event_count.load(Ordering::SeqCst)
    }
}

impl Default for ActivityMonitor {
    fn default() -> Self {
        Self::new()
    }
}
