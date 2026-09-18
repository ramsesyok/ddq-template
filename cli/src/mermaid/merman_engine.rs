//! merman 経路: 純 Rust の mermaid 再実装（ブラウザが無いときの保険。cli/DESIGN.md §5.3）。
//!
//! **HTML ラベル（foreignObject）を含まない SVG にすること。** 既定の parity パイプラインだと
//! mindmap / ER / block-beta / requirement のラベルが foreignObject になり、Typst で文字が
//! 消える（§9 実測）。`SvgPipeline::resvg_safe()` がそれを SVG text に落とす。

use merman::{
    Engine, MermaidConfig, OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest,
    svg::{SvgPipeline, SvgRenderOptions},
};

use super::set_root_background;

/// 組み込んだ merman の版（Cargo.lock から build.rs が渡す）
pub const VERSION: &str = env!("DDQ_MERMAN_VERSION");

/// 全入力を変換する。返り値は入力と同順。
pub fn render(
    sources: &[String],
    config: &serde_json::Value,
    background: &str,
) -> Vec<Result<String, String>> {
    let renderer = Renderer::new()
        .with_engine(Engine::new().with_site_config(MermaidConfig::from_value(config.clone())));
    sources
        .iter()
        .map(|source| render_one(&renderer, source, background))
        .collect()
}

fn render_one(renderer: &Renderer, source: &str, background: &str) -> Result<String, String> {
    let request = SvgRequest {
        pipeline: Some(SvgPipeline::resvg_safe()),
        options: SvgRenderOptions {
            // mermaid-cli / ブラウザ経路と同じ id にして、出力の見た目（CSS セレクタ）を揃える
            diagram_id: Some("my-svg".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    let output = renderer
        .render(RenderRequest::svg(source, OperationControl::new(), request))
        .map_err(|e| e.to_string())?;
    match output {
        RenderOutput::Svg(Some(svg)) => Ok(set_root_background(svg.svg(), background)),
        _ => Err("mermaid の図として認識できませんでした".to_string()),
    }
}
