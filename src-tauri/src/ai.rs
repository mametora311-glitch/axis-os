// src-tauri/src/ai.rs

use serde_json::json;
use std::env;
use reqwest::Client;

// --- 共通: OpenAI互換 API呼び出し (汎用) ---
pub async fn call_openai_compatible(
    url: &str,
    api_key_env: &str,
    model_name: &str,
    system_prompt: &str,
    user_input: &str
) -> Result<String, String> {
    let api_key = env::var(api_key_env).map_err(|_| format!("{} missing", api_key_env))?;
    
    let client = Client::new();
    
    // ★修正: temperatureパラメータを削除しました。
    // o1系(gpt-5-nano等)はtemperature指定不可、他モデルもデフォルト(1.0等)で動作します。
    let body = json!({
        "model": model_name,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_input }
        ]
        // "temperature": 0.3  <-- 削除！これが犯人でした
    });

    let res = client.post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?;

    let status = res.status();
    let text = res.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("API Error [{}]: {}", status, text));
    }

    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("JSON Parse Error: {}", e))?;
    
    if let Some(err) = json.get("error") {
        return Err(format!("API Returned Error: {:?}", err));
    }

    json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| format!("No content in response: {}", text))
        .map(|s| s.to_string())
}

// --- Google Gemini 呼び出し (汎用) ---
pub async fn call_google(model_name: &str, system_prompt: &str, user_input: &str) -> Result<String, String> {
    let api_key = env::var("GEMINI_API_KEY").map_err(|_| "GEMINI_API_KEY missing".to_string())?;
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", model_name, api_key);

    let body = json!({
        "system_instruction": { "parts": [{ "text": system_prompt }] },
        "contents": [{ "parts": [{ "text": user_input }] }]
    });

    let client = Client::new();
    let res = client.post(&url).json(&body).send().await.map_err(|e| e.to_string())?;
    
    let status = res.status();
    let text = res.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Gemini Error [{}]: {}", status, text));
    }

    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("JSON Parse Error: {}", e))?;
    
    // Geminiのエラーレスポンスハンドリングも念のため強化
    if let Some(err) = json.get("error") {
        return Err(format!("Gemini API Error: {:?}", err));
    }

    json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| format!("No content in Gemini response: {}", text))
        .map(|s| s.to_string())
}

// --- ショートカット関数 ---
pub async fn call_openai(model: &str, sys: &str, user: &str) -> Result<String, String> {
    call_openai_compatible("https://api.openai.com/v1/chat/completions", "OPENAI_API_KEY", model, sys, user).await
}

pub async fn call_grok(model: &str, sys: &str, user: &str) -> Result<String, String> {
    call_openai_compatible("https://api.x.ai/v1/chat/completions", "XAI_API_KEY", model, sys, user).await
}