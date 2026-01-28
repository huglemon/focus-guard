use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone)]
pub enum UpdateStatus {
    Checking,
    Available { version: String },
    NotAvailable,
    Downloading { progress: u64, total: u64 },
    Ready,
    Error(String),
}

/// 检查更新
pub async fn check_for_update(app: &AppHandle) -> Result<Option<String>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;

    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            println!("Update available: {}", version);
            Ok(Some(version))
        }
        Ok(None) => {
            println!("No update available");
            Ok(None)
        }
        Err(e) => {
            println!("Update check failed: {}", e);
            Err(e.to_string())
        }
    }
}

/// 下载并安装更新
pub async fn download_and_install(app: &AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;

    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("No update available")?;

    println!("Downloading update: {}", update.version);

    // 下载更新
    let mut downloaded = 0;

    update
        .download_and_install(
            |chunk, _content_length| {
                downloaded += chunk;
                println!("Downloaded: {} bytes", downloaded);
            },
            || {
                println!("Download complete, installing...");
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    println!("Update installed, restart required");
    Ok(())
}

/// 获取当前版本
pub fn get_current_version(app: &AppHandle) -> String {
    app.package_info().version.to_string()
}
