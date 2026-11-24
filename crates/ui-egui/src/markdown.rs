//! The Markdown editor's colours, and the links of the formatted view.

use diarycraft_engine::markdown::{LinkTarget, Mark, highlight, link_target};
use egui::text::{LayoutJob, TextFormat};
use std::path::{Path, PathBuf};

/// The text of the editor laid out with its Markdown coloured.
pub fn layout_job(ui: &egui::Ui, text: &str, wrap_width: f32) -> LayoutJob {
    let visuals = ui.visuals();
    let body = egui::TextStyle::Body.resolve(ui.style());
    let mono = egui::FontId::monospace(body.size * 0.95);
    let plain = TextFormat::simple(body.clone(), visuals.text_color());
    let mut job = LayoutJob::default();
    for span in highlight(text) {
        let Some(piece) = text.get(span.range) else { continue };
        let format = match span.mark {
            Mark::Plain => plain.clone(),
            Mark::FrontMatter => TextFormat::simple(mono.clone(), visuals.weak_text_color()),
            Mark::Heading(level) => {
                let scale = match level {
                    1 => 1.6,
                    2 => 1.4,
                    3 => 1.2,
                    _ => 1.05,
                };
                TextFormat::simple(egui::FontId::proportional(body.size * scale), visuals.strong_text_color())
            }
            Mark::Syntax | Mark::Rule => TextFormat::simple(body.clone(), visuals.weak_text_color()),
            Mark::ListMarker => TextFormat::simple(body.clone(), visuals.hyperlink_color),
            Mark::Strong => TextFormat::simple(body.clone(), visuals.strong_text_color()),
            Mark::Emphasis => TextFormat { italics: true, ..plain.clone() },
            Mark::Code | Mark::CodeBlock => TextFormat {
                background: visuals.code_bg_color,
                ..TextFormat::simple(mono.clone(), visuals.text_color())
            },
            Mark::Link => TextFormat::simple(body.clone(), visuals.hyperlink_color),
            Mark::Url => TextFormat {
                underline: egui::Stroke::new(1.0, visuals.weak_text_color()),
                ..TextFormat::simple(body.clone(), visuals.weak_text_color())
            },
            Mark::Quote => TextFormat { italics: true, ..TextFormat::simple(body.clone(), visuals.weak_text_color()) },
        };
        job.append(piece, 0.0, format);
    }
    job.wrap.max_width = wrap_width;
    job
}

/// What a click on a link of the formatted view opens: a web address as it is, or a file of the
/// diary, found from the folder of the entry and never outside the diary.
pub fn resolve_link(destination: &str, entry_dir: &Path, root: &Path) -> Option<PathBuf> {
    match link_target(destination)? {
        LinkTarget::Web(url) => Some(PathBuf::from(url)),
        LinkTarget::Relative(parts) => {
            let path = parts.iter().fold(entry_dir.to_path_buf(), |p, part| p.join(part));
            path.starts_with(root).then_some(path)
        }
    }
}
