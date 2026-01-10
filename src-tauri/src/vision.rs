// src-tauri/src/vision.rs

use screenshots::Screen;
use std::io::Cursor;
use image::ImageOutputFormat;
use base64::{Engine as _, engine::general_purpose};

// 画面を撮影してBase64文字列で返す関数
pub fn take_screenshot() -> Result<String, String> {
    // 1. 全モニタを検知
    let screens = Screen::all().map_err(|e| e.to_string())?;
    
    // マルチモニタ対応: とりあえずメイン画面(最初の画面)を取得
    let screen = screens.first().ok_or("No screen found")?;
    
    // 2. キャプチャ実行
    let image = screen.capture().map_err(|e| e.to_string())?;
    
    // 3. メモリ上でPNGに変換
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);
    
    // screenshotsクレートの画像を imageクレートで扱えるように変換してPNG保存
    // (RGBAデータをPNGへエンコード)
    image.write_to(&mut cursor, ImageOutputFormat::Png)
        .map_err(|e| e.to_string())?;
        
    // 4. Base64エンコード (data:image/png;base64,... の形式はフロントでつける)
    let base64_str = general_purpose::STANDARD.encode(buffer);
    
    Ok(base64_str)
}