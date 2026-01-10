use std::fs;
use std::path::PathBuf;
use tauri::Manager; // パス取得に必須
use serde::{Serialize, Deserialize};

// --- 構造体定義 ---
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AxisToken {
    pub id: String,
    pub text: String,
    pub timestamp: i64,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InteractionLog {
    pub id: String,
    pub session_id: String,
    pub timestamp: i64,
    pub user_tokens: Vec<AxisToken>,
    pub ai_response: String,
    pub provider_used: String,
}

// --- ヘルパー: パスの一元管理 ---
// 全ての機能がこの関数を使うことで、ファイルの不整合を防ぎます
fn get_history_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    
    // ディレクトリが無ければ作成
    if !app_dir.exists() {
        let _ = fs::create_dir_all(&app_dir);
    }
    
    Ok(app_dir.join("history.json"))
}

// --- 機能実装 ---

// 1. 全履歴の取得 (Fetch)
pub fn get_all_logs(app: &tauri::AppHandle) -> Result<Vec<InteractionLog>, String> {
    let path = get_history_path(app)?;
    
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    // JSONパースに失敗したら空配列を返す（クラッシュ防止）
    let logs: Vec<InteractionLog> = serde_json::from_str(&content).unwrap_or_default();
    
    Ok(logs)
}

// 2. ログの保存 (Save)
pub fn save_log(app: &tauri::AppHandle, log: &InteractionLog) -> Result<(), String> {
    let path = get_history_path(app)?;
    
    // 既存のログを読み込んで追加
    let mut logs = get_all_logs(app).unwrap_or_default();
    logs.push(log.clone());
    
    let json = serde_json::to_string_pretty(&logs).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

// 3. セッションの削除 (Delete)
pub fn delete_session_log(app: &tauri::AppHandle, target_session_id: &str) -> Result<(), String> {
    let path = get_history_path(app)?;
    
    // 既存のログを読み込む
    let logs = get_all_logs(app).unwrap_or_default();
    
    // 対象ID以外を残すフィルタリング
    let new_logs: Vec<InteractionLog> = logs.into_iter()
        .filter(|log| log.session_id != target_session_id)
        .collect();
        
    let json = serde_json::to_string_pretty(&new_logs).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    
    Ok(())
}