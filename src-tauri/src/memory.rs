// src-tauri/src/memory.rs
//
// Axis 用メモリストア（json + meta）
// - entry: input/output 分離
// - meta : kind / importance / tags / stickies / search_text
//
// 検索はフルスキャン + 簡易スコアリング（MVP）

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AttachmentRef {
    pub object_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub mime: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct IoBlock {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub attachments: Vec<AttachmentRef>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemoryEntry {
    pub id: String,
    pub session_id: String,
    pub timestamp_ms: i64,
    pub input: IoBlock,
    pub output: IoBlock,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Stickies {
    // 大分類 / 中分類 / 小分類
    #[serde(default)]
    pub l: String,
    #[serde(default)]
    pub m: String,
    #[serde(default)]
    pub s: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MemoryKind {
    ShortTerm,
    LongTerm,
    Meta,
    Sealed,
}

impl Default for MemoryKind {
    fn default() -> Self {
        MemoryKind::ShortTerm
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MemoryMeta {
    pub id: String,
    #[serde(default)]
    pub kind: MemoryKind,
    #[serde(default)]
    pub importance: f32, // 0.0 ..= 1.0
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub stickies: Option<Stickies>,
    #[serde(default)]
    pub source: String, // "llm" / "memory" / "manual" など
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub references: Vec<String>,
    #[serde(default)]
    pub sealed_reason: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    #[serde(default)]
    pub search_text: String, // input+output+添付テキストなどを詰めた検索面
}

#[derive(Debug, Clone)]
pub struct MemoryHit {
    pub id: String,
    pub score: f32,
    pub entry: MemoryEntry,
}

// ---------- パス関連 ----------

// src-tauri/src/memory.rs

fn memory_root(app: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let root = app_dir.join("axis_memory");
    if !root.exists() {
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    }

    // ★ ここで保存先を1回はログに出す（起動直後の確認用）
    println!("[memory] root dir = {:?}", &root);

    Ok(root)
}


fn entries_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let d = memory_root(app)?.join("entries");
    if !d.exists() {
        fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    }
    Ok(d)
}

fn entry_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    Ok(entries_dir(app)?.join(format!("{}.json", id)))
}

fn meta_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    Ok(entries_dir(app)?.join(format!("{}.meta.json", id)))
}

// ---------- 保存/読み込み ----------

fn validate_meta(meta: &MemoryMeta) -> Result<(), String> {
    if !(0.0..=1.0).contains(&meta.importance) {
        return Err("importance must be within 0.0..=1.0".to_string());
    }
    if matches!(meta.kind, MemoryKind::Sealed)
        && meta.sealed_reason.as_deref().unwrap_or("").is_empty()
    {
        return Err("sealed_reason is required when kind=SEALED".to_string());
    }
    Ok(())
}

pub fn save_entry_and_meta(
    app: &AppHandle,
    entry: &MemoryEntry,
    meta: &MemoryMeta,
) -> Result<(), String> {
    validate_meta(meta)?;

    let ep = entry_path(app, &entry.id)?;
    let mp = meta_path(app, &entry.id)?;

    let entry_json = serde_json::to_string_pretty(entry).map_err(|e| e.to_string())?;
    let meta_json = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;

    fs::write(ep, entry_json).map_err(|e| e.to_string())?;
    fs::write(mp, meta_json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_entry(app: &AppHandle, id: &str) -> Result<MemoryEntry, String> {
    let ep = entry_path(app, id)?;
    let s = fs::read_to_string(ep).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|e| e.to_string())
}

#[allow(dead_code)]
pub fn load_meta(app: &AppHandle, id: &str) -> Result<MemoryMeta, String> {
    let mp = meta_path(app, id)?;
    let s = fs::read_to_string(mp).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|e| e.to_string())
}

fn list_meta(app: &AppHandle) -> Result<Vec<MemoryMeta>, String> {
    let dir = entries_dir(app)?;
    let mut out = Vec::new();

    let rd = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for e in rd {
        if let Ok(e) = e {
            let p = e.path();
            if p.is_file() {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with(".meta.json") {
                        if let Ok(s) = fs::read_to_string(&p) {
                            if let Ok(m) = serde_json::from_str::<MemoryMeta>(&s) {
                                out.push(m);
                            }
                        }
                    }
                }
            }
        }
    }

    // 新しいもの順に
    out.sort_by(|a, b| b.updated_at_ms.cmp(&a.updated_at_ms));
    Ok(out)
}

// ---------- 検索ロジック(MVP) ----------

fn normalize_text(s: &str) -> String {
    s.to_lowercase().replace('\u{3000}', " ").trim().to_string()
}

// 超簡易トークナイザ（英数字 & 日本語）
fn tokenize(s: &str) -> Vec<String> {
    let s = normalize_text(s);
    let mut toks: Vec<String> = Vec::new();
    let mut cur = String::new();

    fn class_of(c: char) -> u8 {
        if c.is_ascii_alphanumeric() {
            1
        } else if ('\u{3040}'..='\u{30ff}').contains(&c) || ('\u{4e00}'..='\u{9fff}').contains(&c) {
            2
        } else {
            0
        }
    }

    let mut last_class: u8 = 0;
    for ch in s.chars() {
        let cl = class_of(ch);
        if cl == 0 {
            if !cur.is_empty() {
                toks.push(cur.clone());
                cur.clear();
            }
            last_class = 0;
            continue;
        }
        if last_class != 0 && cl != last_class {
            if !cur.is_empty() {
                toks.push(cur.clone());
                cur.clear();
            }
        }
        cur.push(ch);
        last_class = cl;
    }
    if !cur.is_empty() {
        toks.push(cur);
    }

    toks.into_iter()
        .filter(|t| t.len() >= 2 || t.chars().all(|c| c.is_ascii_digit()))
        .collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f32;
    let uni = a.union(b).count() as f32;
    if uni == 0.0 {
        0.0
    } else {
        inter / uni
    }
}

fn tag_overlap(tags: &[String], query_tokens: &[String]) -> i32 {
    let mut n = 0;
    for tag in tags {
        let tag_l = normalize_text(tag);
        for qt in query_tokens {
            if tag_l.contains(qt) || qt.contains(&tag_l) {
                n += 1;
                break;
            }
        }
    }
    n
}

fn recency_boost(updated_at_ms: i64) -> f32 {
    let now = Utc::now().timestamp_millis();
    let age_ms = (now - updated_at_ms).max(0) as f32;
    let age_days = age_ms / 1000.0 / 60.0 / 60.0 / 24.0;
    let b = 1.0 - (age_days / 30.0);
    b.clamp(0.0, 1.0)
}

// 上位K件のメモリヒットを返す
pub fn search_top_k(app: &AppHandle, query: &str, limit: usize) -> Result<Vec<MemoryHit>, String> {
    let q = normalize_text(query);
    if q.is_empty() {
        return Ok(vec![]);
    }

    let q_tokens = tokenize(&q);
    let q_set: HashSet<String> = q_tokens.iter().cloned().collect();

    let metas = list_meta(app)?;
    let mut hits: Vec<MemoryHit> = Vec::new();

    for meta in metas {
        if matches!(meta.kind, MemoryKind::Sealed) {
            continue;
        }

        if meta.search_text.is_empty() {
            continue;
        }

        // ざっくりフィルタ
        if !q_tokens.iter().any(|t| meta.search_text.contains(t))
            && tag_overlap(&meta.tags, &q_tokens) == 0
        {
            continue;
        }

        let t_tokens = tokenize(&meta.search_text);
        let t_set: HashSet<String> = t_tokens.into_iter().collect();

        let jac = jaccard(&q_set, &t_set);
        let ov = tag_overlap(&meta.tags, &q_tokens) as f32;

        let mut score = 0.0;
        score += jac * 5.0;
        score += ov * 1.5;
        score += meta.importance.clamp(0.0, 1.0) * 2.0;
        score += recency_boost(meta.updated_at_ms) * 1.0;

        if meta.search_text.contains(&q) {
            score += 2.0;
        }

        if score <= 0.0 {
            continue;
        }

        if let Ok(entry) = load_entry(app, &meta.id) {
            hits.push(MemoryHit {
                id: meta.id.clone(),
                score,
                entry,
            });
        }
    }

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(limit);
    Ok(hits)
}

pub fn search_best_for_query(app: &AppHandle, query: &str) -> Result<Option<MemoryHit>, String> {
    Ok(search_top_k(app, query, 1)?.into_iter().next())
}

// LLM 用の [Relevant Memories] セクション文字列
pub fn build_memory_context(app: &AppHandle, query: &str, limit: usize) -> Result<String, String> {
    let hits = search_top_k(app, query, limit)?;
    if hits.is_empty() {
        return Ok(String::new());
    }

    let mut lines: Vec<String> = Vec::new();
    for h in hits {
        let q_snip: String = h.entry.input.text.chars().take(80).collect();
        let a_snip: String = h.entry.output.text.chars().take(120).collect();
        lines.push(format!(
            "- (score={:.2}) Q: {} / A: {}",
            h.score, q_snip, a_snip
        ));
    }

    Ok(format!("\n[Relevant Memories]\n{}", lines.join("\n")))
}

// ask_axis から使う「1対話の保存」ヘルパ
pub fn save_interaction(
    app: &AppHandle,
    session_id: &str,
    input_text: &str,
    output_text: &str,
    source: &str,
    provider: &str,
    references: Vec<String>,
) -> Result<(), String> {
    let now = Utc::now().timestamp_millis();
    let id = format!("{}-{}", session_id, now);

    let entry = MemoryEntry {
        id: id.clone(),
        session_id: session_id.to_string(),
        timestamp_ms: now,
        input: IoBlock {
            text: input_text.to_string(),
            attachments: vec![], // TODO: 添付を Axis から渡すように拡張
        },
        output: IoBlock {
            text: output_text.to_string(),
            attachments: vec![],
        },
    };

    let search_text = normalize_text(&format!("{}\n{}\n", input_text, output_text));

    let meta = MemoryMeta {
        id,
        kind: MemoryKind::ShortTerm,
        importance: 0.5,
        tags: vec![],   // TODO: 付箋/タグ UI から付与
        stickies: None, // TODO: 大/中/小分類をここに入れる
        source: source.to_string(),
        provider: Some(provider.to_string()),
        references,
        sealed_reason: None,
        created_at_ms: now,
        updated_at_ms: now,
        search_text,
    };

    save_entry_and_meta(app, &entry, &meta)?;

    // ★ 保存されたことをログ
    println!(
        "[memory] saved id={} session={} source={} provider={}",
        meta.id, session_id, source, provider
    );

    Ok(())
}
