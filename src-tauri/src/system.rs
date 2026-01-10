use sysinfo::{System, RefreshKind, CpuRefreshKind, MemoryRefreshKind};
use serde::Serialize;
use std::thread;
use std::time::Duration;

#[derive(Serialize)]
pub struct SystemStats {
    pub cpu_usage: u8,
    pub memory_used: u64,
    pub memory_total: u64,
    pub battery_level: u8,
    pub is_charging: bool,
}

pub fn get_system_stats() -> SystemStats {
    // 必要な情報だけリフレッシュするように設定
    let mut sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything())
    );

    // ★修正箇所: 0.30系では refresh_cpu() を使う
    sys.refresh_cpu(); 
    
    // CPU使用率を正確に取るため少し待つ
    thread::sleep(Duration::from_millis(100));
    sys.refresh_cpu();
    
    sys.refresh_memory();

    let cpu_count = sys.cpus().len() as f32;
    let cpu_total_usage: f32 = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).sum();
    let cpu_avg = if cpu_count > 0.0 { cpu_total_usage / cpu_count } else { 0.0 };

    let mem_used = sys.used_memory();
    let mem_total = sys.total_memory();

    SystemStats {
        cpu_usage: cpu_avg as u8,
        memory_used: mem_used,
        memory_total: mem_total,
        battery_level: 100, // デスクトップ想定
        is_charging: true,
    }
}

// src-tauri/src/system.rs の既存コードの下に追加

use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// 起動中のアプリ一覧（ウィンドウタイトル）を取得
pub fn get_running_apps() -> Vec<String> {
    let ps_script = "Get-Process | Where-Object { $_.MainWindowTitle -ne '' } | Select-Object -ExpandProperty MainWindowTitle";
    
    let output = Command::new("powershell")
        .args(&["-NoProfile", "-WindowStyle", "Hidden", "-Command", ps_script])
        .creation_flags(0x08000000)
        .output();

    match output {
        Ok(o) => {
            let res = String::from_utf8_lossy(&o.stdout);
            res.lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect()
        },
        Err(_) => vec![],
    }
}