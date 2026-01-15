use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct ModelScore {
    pub code: f32,
    pub reasoning: f32,
    pub math: f32,
    pub general_qa: f32,
    pub planning: f32,
    pub multimodal: f32,
    pub speed: f32,
    pub cost: f32,
}

pub type ModelProfiles = HashMap<String, ModelScore>;

fn load_profiles() -> ModelProfiles {
    // ビルド時に同ディレクトリのJSONを埋め込む
    const RAW: &str = include_str!("model_profiles.json");

    serde_json::from_str(RAW).unwrap_or_else(|e| {
        println!("[model_profiles] JSON parse error: {e}");
        HashMap::new()
    })
}

/// Commander にそのまま渡せるテキストブロックを生成
pub fn build_profiles_prompt() -> String {
    let profiles = load_profiles();
    if profiles.is_empty() {
        return "  (no model profiles loaded)".to_string();
    }

    let mut out = String::new();
    for (name, s) in profiles {
        use std::fmt::Write;
        let _ = writeln!(&mut out, "- {}:", name);
        let _ = writeln!(&mut out, "  code: {}", s.code);
        let _ = writeln!(&mut out, "  reasoning: {}", s.reasoning);
        let _ = writeln!(&mut out, "  math: {}", s.math);
        let _ = writeln!(&mut out, "  general_qa: {}", s.general_qa);
        let _ = writeln!(&mut out, "  planning: {}", s.planning);
        let _ = writeln!(&mut out, "  multimodal: {}", s.multimodal);
        let _ = writeln!(&mut out, "  speed: {}", s.speed);
        let _ = writeln!(&mut out, "  cost: {}", s.cost);
        let _ = writeln!(&mut out);
    }
    out
}
