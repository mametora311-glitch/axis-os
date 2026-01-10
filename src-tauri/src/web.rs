// src-tauri/src/web.rs
use reqwest::header::USER_AGENT;
use scraper::{Html, Selector};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub snippet: String,
}

pub async fn search_duckduckgo(query: &str) -> Result<Vec<SearchResult>, String> {
    // クエリの前後の空白を除去し、URLエンコード（念のため）
    let url = format!("https://html.duckduckgo.com/html/?q={}", query.trim());
    
    println!("🌐 [Grok] Searching: [{}]", query.trim());

    let client = reqwest::Client::new();
    let res = client.get(&url)
        // 最新のChromeのふりをする
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let html_text = res.text().await.map_err(|e| format!("Read error: {}", e))?;
    
    // デバッグ: 本当にHTMLが取れているか確認したければコメントアウトを外す
    // println!("📄 HTML Length: {}", html_text.len());

    let document = Html::parse_document(&html_text);

    let result_selector = Selector::parse(".result").unwrap();
    let title_selector = Selector::parse(".result__a").unwrap();
    let snippet_selector = Selector::parse(".result__snippet").unwrap();

    let mut results = Vec::new();

    for element in document.select(&result_selector) {
        let title = match element.select(&title_selector).next() {
            Some(el) => el.text().collect::<Vec<_>>().join(""),
            None => continue,
        };
        
        let link = match element.select(&title_selector).next() {
            Some(el) => el.value().attr("href").unwrap_or("").to_string(),
            None => continue,
        };

        let snippet = match element.select(&snippet_selector).next() {
            Some(el) => el.text().collect::<Vec<_>>().join(""),
            None => "No description".to_string(),
        };

        if !title.is_empty() {
            results.push(SearchResult { title, link, snippet });
        }
        if results.len() >= 5 { break; }
    }

    if results.is_empty() {
        println!("⚠️ [Grok] No results found. (Maybe blocked?)");
    } else {
        println!("✅ [Grok] Success! Found {} links.", results.len());
        // 最初の1件のタイトルを表示して確認
        if let Some(first) = results.first() {
             println!("   Top result: {}", first.title);
        }
    }

    Ok(results)
}

// src-tauri/src/web.rs の既存コードの末尾に追加

// ★追加: Grokipedia検索（テスト用ダミー実装）
// 常に「空の結果」を返すことで、lib.rs 側のフォールバック処理(DDGへの切り替え)を作動させる
pub async fn search_grokipedia(query: &str) -> Result<Vec<SearchResult>, String> {
    println!("📚 Grokipedia Search: '{}' (Simulating...)", query);
    
    // ここに将来的に本物のAPI実装を入れる
    // 今は「該当なし」として空のベクタを返す
    let results: Vec<SearchResult> = Vec::new();

    Ok(results)
}