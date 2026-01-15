// src-tauri/src/db.rs
use chrono::Utc;
use rusqlite::{params, Connection, Result};
use std::{fs, path::Path};

pub struct AxisDatabase {
    conn: Connection,
}

impl AxisDatabase {
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            let _ = fs::create_dir_all(parent);
        }

        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            -- 1) セッション（UUID文字列）
            CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            -- 2) メッセージ（UUID文字列で紐付け）
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,          -- user / assistant / system
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
            );

            -- 3) 単語(文字)インデックス（高速 recall 用）
            -- tokenize='trigram' は「日本語/スペース無し」でも拾いやすい
            CREATE VIRTUAL TABLE IF NOT EXISTS message_index
            USING fts5(content, session_id UNINDEXED, tokenize='trigram');

            -- 4) 信念
            CREATE TABLE IF NOT EXISTS beliefs (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );

            -- 5) 目標
            CREATE TABLE IF NOT EXISTS goals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                priority INTEGER DEFAULT 0,
                due_at INTEGER,
                created_at INTEGER NOT NULL
            );

            -- 6) NotebookLM風 資料
            CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT UNIQUE,
                summary TEXT,
                content_text TEXT,
                created_at INTEGER NOT NULL
            );

            -- 7) 付箋（大中小）
            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                doc_id INTEGER,
                category_l TEXT,
                category_m TEXT,
                category_s TEXT,
                FOREIGN KEY(doc_id) REFERENCES documents(id) ON DELETE CASCADE
            );
            "#,
        )?;

        Ok(Self { conn })
    }

    fn now_ms() -> i64 {
        Utc::now().timestamp_millis()
    }

    fn upsert_session(&self, session_id: &str) -> Result<()> {
        let now = Self::now_ms();
        let title = format!("session {}", session_id.chars().take(8).collect::<String>());

        self.conn.execute(
            r#"
            INSERT INTO sessions(session_id, title, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?3)
            ON CONFLICT(session_id) DO UPDATE SET updated_at = excluded.updated_at
            "#,
            params![session_id, title, now],
        )?;
        Ok(())
    }

    // lib.rs が呼んでるやつ（赤線の根）
    pub fn save_interaction(&self, session_id: &str, role: &str, content: &str) -> Result<()> {
        self.upsert_session(session_id)?;

        let now = Self::now_ms();
        self.conn.execute(
            r#"
            INSERT INTO messages(session_id, role, content, created_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![session_id, role, content, now],
        )?;

        // FTS にも入れる（recall はこっちを引く）
        self.conn.execute(
            r#"INSERT INTO message_index(content, session_id) VALUES (?1, ?2)"#,
            params![content, session_id],
        )?;

        Ok(())
    }

    // lib.rs が呼んでるやつ（赤線の根）
    #[allow(dead_code)]
    pub fn search_similar_logs(&self, query: &str) -> Result<Vec<String>> {
        // FTS5のクエリ構文で事故りやすい文字を軽く潰す（最低限）
        let cleaned: String = query
            .chars()
            .map(|c| match c {
                '"' => ' ',             // クオートは除去
                '*' | ':' | '-' => ' ', // FTS演算子になりがち
                _ => c,
            })
            .collect();

        let fts_query = format!("\"{}\"", cleaned.trim());

        // rank列は使わない。bm25()でスコアリング（小さいほど良い）
        let mut stmt = self.conn.prepare(
            "SELECT content
         FROM message_index
         WHERE message_index MATCH ?1
         ORDER BY bm25(message_index)
         LIMIT 3",
        )?;

        let rows = stmt.query_map([fts_query], |row| row.get::<_, String>(0))?;

        let mut results = Vec::new();
        for r in rows {
            if let Ok(content) = r {
                results.push(content);
            }
        }
        Ok(results)
    }
}
