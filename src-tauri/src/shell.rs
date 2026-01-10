// src-tauri/src/shell.rs
// v0.4.1 Fix: "Liar Logic" Removal (AppID Search + Explorer Launch)

use std::process::Command;
use std::thread;
use std::time::Duration;
use enigo::{Enigo, Key, Keyboard, Settings, Direction};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// 1. アプリ起動 (AppID経由の確実な起動)
pub fn execute_command(app_req: &str) -> String {
    let request = app_req.trim();
    let request_lower = request.to_lowercase();
    
    // 優先: よく使うシステムコマンド (これらは動いているはず)
    match request_lower.as_str() {
        "calc" | "calculator" | "電卓" => return launch_simple("calc", "Calculator"),
        "notepad" | "memo" | "メモ" | "メモ帳" => return launch_simple("notepad", "Notepad"),
        "explorer" | "folder" | "エクスプローラー" => return launch_simple("explorer", "File Explorer"),
        "cmd" | "terminal" | "ターミナル" => return launch_simple_args("start", vec!["cmd"], "Terminal"),
        "taskmgr" | "タスクマネージャー" => return launch_simple("taskmgr", "Task Manager"),
        _ => {} 
    };

    // --- ユニバーサル検索ロジック変更 ---
    // PowerShellで「起動」するのではなく、「AppID」だけを取得する。
    // ※ AppID = Windowsがアプリを管理するための絶対住所
    let ps_script = format!(
        "$app = Get-StartApps | Where-Object {{ $_.Name -like '*{}*' }} | Select-Object -First 1; \
         if ($app) {{ Write-Output $app.AppID }} else {{ Write-Output 'NOT_FOUND' }}",
        request
    );

    let output = Command::new("powershell")
        .args(&["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script])
        .creation_flags(0x08000000) 
        .output();

    match output {
        Ok(o) => {
            let app_id = String::from_utf8_lossy(&o.stdout).trim().to_string();
            
            if app_id == "NOT_FOUND" || app_id.is_empty() {
                // 見つからない場合は正直に言う
                format!("Failed: Application '{}' not found in Start Menu.", request)
            } else {
                // ★ここが修正点: explorer.exe に AppID を渡して起動させる
                // これで「裏でこっそり失敗する」のを防ぐ
                let launch_cmd = format!("shell:AppsFolder\\{}", app_id);
                
                // explorer.exe は通常ウインドウを表示してくれる
                match Command::new("explorer").arg(&launch_cmd).spawn() {
                    Ok(_) => format!("Success: Launched '{}' (ID: {}).", request, app_id),
                    Err(e) => format!("Error: Found ID {} but failed to launch. {}", app_id, e)
                }
            }
        },
        Err(e) => format!("Error executing shell search: {}", e),
    }
}

// 補助関数
fn launch_simple(cmd: &str, name: &str) -> String {
    Command::new("cmd").args(&["/C", "start", "", cmd]).spawn()
        .map(|_| format!("Success: Launched {}.", name))
        .unwrap_or_else(|e| format!("Error launching {}: {}", name, e))
}

fn launch_simple_args(cmd: &str, args: Vec<&str>, name: &str) -> String {
    Command::new(cmd).args(&args).spawn()
        .map(|_| format!("Success: Launched {}.", name))
        .unwrap_or_else(|e| format!("Error launching {}: {}", name, e))
}

// --- 以下、入力・キー操作系（変更なし） ---
pub fn type_text(text: &str, target_window: Option<&str>) -> String {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    
    if let Some(target) = target_window {
        let ps_script = format!(
            "$ws = New-Object -ComObject WScript.Shell; \
             $p = Get-Process | Where-Object {{ $_.MainWindowTitle -like '*{}*' -or $_.ProcessName -like '*{}*' }} | Select-Object -First 1; \
             if ($p) {{ $ws.AppActivate($p.Id) }}", 
            target, target
        );
        let _ = Command::new("powershell")
            .args(&["-NoProfile", "-WindowStyle", "Hidden", "-ExecutionPolicy", "Bypass", "-Command", &ps_script])
            .creation_flags(0x08000000).output();
        thread::sleep(Duration::from_millis(1000));
    } else {
        thread::sleep(Duration::from_millis(2000)); 
    }

    if let Err(e) = enigo.text(text) { return format!("Error typing text: {}", e); }
    
    if let Some(t) = target_window {
        format!("Focused '{}' and Typed: '{}'", t, text)
    } else {
        format!("Typed: '{}'", text)
    }
}

pub fn press_key(key_name: &str) -> String {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    thread::sleep(Duration::from_millis(300));
    let result = match key_name.to_lowercase().as_str() {
        "enter" | "return" => enigo.key(Key::Return, Direction::Click),
        "tab" => enigo.key(Key::Tab, Direction::Click),
        "space" => enigo.key(Key::Space, Direction::Click),
        "backspace" => enigo.key(Key::Backspace, Direction::Click),
        "windows" | "super" | "meta" => enigo.key(Key::Meta, Direction::Click),
        "escape" | "esc" => enigo.key(Key::Escape, Direction::Click),
        _ => return "Error: Unknown key.".to_string(),
    };
    match result { Ok(_) => format!("Pressed: [{}]", key_name), Err(e) => format!("Error: {}", e) }
}