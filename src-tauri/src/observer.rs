// src-tauri/src/observer.rs
use tauri::{AppHandle, Emitter};
use std::process::Command;
use std::thread;
use std::time::Duration;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// 監視ループの開始
pub fn spawn_observer(app: AppHandle) {
    thread::spawn(move || {
        let mut last_window_title = String::new();
        let mut same_window_count = 0; // 滞在時間の計測用

        loop {
            // 5秒おきにチェック
            thread::sleep(Duration::from_secs(5));

            let current_title = get_active_window_title();
            
            // ウィンドウが変わった場合
            if current_title != last_window_title && !current_title.is_empty() {
                println!("👀 [Observer] Focus changed to: {}", current_title);
                
                // 特定のキーワードに反応する「空気を読む」ロジック
                if current_title.contains("Error") || current_title.contains("エラー") {
                    send_event(&app, "Error Detected", &format!("Looks like an error occurred in '{}'. Need help?", current_title));
                } else if current_title.contains("Visual Studio Code") || current_title.contains("VSCode") {
                     // 頻繁に出るとうざいので、たまに言うなどの制御が必要だが、一旦テスト用に
                     // send_event(&app, "Coding Mode", "System optimization for coding... ready.");
                }

                last_window_title = current_title.clone();
                same_window_count = 0;
            } else {
                // 同じウィンドウを見続けている場合
                same_window_count += 1;
                
                // 5秒 * 12回 = 60秒 (1分) 経過
                if same_window_count == 12 {
                    // YouTubeなどをダラダラ見ている時にチクリと言う
                    if current_title.contains("YouTube") || current_title.contains("Netflix") {
                         send_event(&app, "Suggestion", "You've been watching content for a while. focus_mode check?");
                    }
                }
            }
        }
    });
}

// フロントエンドに通知を送る
fn send_event(app: &AppHandle, topic: &str, message: &str) {
    // "axis-observer-event" というイベント名で発信
    let _ = app.emit("axis-observer-event", format!("[{}] {}", topic, message));
}

// PowerShellを使ってアクティブウィンドウのタイトルを取得
fn get_active_window_title() -> String {
    // C#のWin32APIラッパーをインライン定義して叩く（最速・確実）
    let ps_script = r#"
      Add-Type @"
        using System;
        using System.Runtime.InteropServices;
        public class Win32 {
          [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
          [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder text, int count);
        }
"@
      $hwnd = [Win32]::GetForegroundWindow()
      $sb = New-Object System.Text.StringBuilder 256
      [Win32]::GetWindowText($hwnd, $sb, 256) > $null
      $sb.ToString()
    "#;

    let output = Command::new("powershell")
        .args(&["-NoProfile", "-WindowStyle", "Hidden", "-Command", ps_script])
        .creation_flags(0x08000000)
        .output();

    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => String::new(),
    }
}