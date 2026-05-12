//! 批量导出模块：把"色彩流水线 + 用户导出配置"组合成一张可落盘的图片。
//!
//! 设计要点：
//! - **全分辨率处理**：不像预览那样先缩放，保证导出图色彩计算精度最大化；
//! - **Resize 在最后**：只有 `ExportSettings::resize` 显式要求时才下采样，使用 Lanczos3 内核保边；
//! - **水印兜底**：水印需要 TTF 字体，找不到系统字体时**静默跳过**而不是报错，
//!   避免在 Linux 容器等无字体环境下整个导出失败；
//! - **同名安全**：输出文件已存在时自动追加 `_1` `_2` 后缀，**绝不覆盖**用户文件。

use crate::error::{AppError, Result};
use crate::processing::{self, FilterSettings};
use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::{ImageBuffer, Rgb, RgbImage};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 导出参数。前端 [`ExportSettings`](../../../src/types.ts) 类型与此字段一一对应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSettings {
    pub format: ExportFormat,
    pub quality: u8,
    pub destination: Destination,
    pub resize: Option<ResizeSpec>,
    pub strip_gps: bool,
    pub watermark: Option<Watermark>,
    pub filename_template: Option<String>,
}

/// 支持的输出格式。HEIF 暂未列入：image crate 写 HEIF 需要 libheif，
/// 跨平台打包复杂度较高，先支持四个最常用的。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Jpeg,
    Png,
    Tiff,
    Webp,
}

/// 输出目录：可以放在原文件旁的子文件夹（默认 `FujiSim_Export`），
/// 也可以指定一个绝对路径（批量导出到统一仓库）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Destination {
    Subfolder { name: String },
    Path { path: PathBuf },
}

/// 缩放规格。LongEdge 是常见的"按最长边缩到 N 像素"，Percent 是简单百分比。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResizeSpec {
    LongEdge(u32),
    Percent(u32),
}

/// 水印参数（仅文字水印，图片水印未来扩展）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watermark {
    pub text: String,
    pub position: WatermarkPosition,
    pub opacity: f32,
    pub size: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatermarkPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            format: ExportFormat::Jpeg,
            quality: 92,
            destination: Destination::Subfolder {
                name: "FujiSim_Export".into(),
            },
            resize: None,
            strip_gps: false,
            watermark: None,
            filename_template: None,
        }
    }
}

/// 把 `Destination` 解析成具体的目录路径，必要时创建该目录。
pub fn resolve_destination_dir(src: &Path, dest: &Destination) -> Result<PathBuf> {
    let dir = match dest {
        Destination::Subfolder { name } => {
            let parent = src.parent().ok_or_else(|| AppError::other("no parent"))?;
            parent.join(name)
        }
        Destination::Path { path } => path.clone(),
    };
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// 导出单张图片到 `out_dir`。
///
/// 流程：解码 → 色彩流水线（全分辨率）→ Resize（可选）→ 16-bit 转 8-bit → 水印 → 编码落盘。
/// 文件名规则：`<原名>_<胶片预设名>.<ext>`，同名时追加 `_1` `_2` 后缀。
pub fn export_one(
    src_path: &Path,
    out_dir: &Path,
    filter: &FilterSettings,
    export: &ExportSettings,
) -> Result<PathBuf> {
    let src = processing::load_image_rgb16(src_path)?;
    let processed = processing::process_image(&src, filter)?;
    let final_image = match &export.resize {
        Some(ResizeSpec::LongEdge(le)) => {
            let (w, h) = processed.dimensions();
            let scale = (*le as f32) / (w.max(h) as f32);
            if scale >= 1.0 {
                processed
            } else {
                let nw = (w as f32 * scale).round() as u32;
                let nh = (h as f32 * scale).round() as u32;
                image::imageops::resize(&processed, nw, nh, image::imageops::FilterType::Lanczos3)
            }
        }
        Some(ResizeSpec::Percent(p)) => {
            let (w, h) = processed.dimensions();
            let s = (*p as f32) / 100.0;
            let nw = (w as f32 * s).round().max(1.0) as u32;
            let nh = (h as f32 * s).round().max(1.0) as u32;
            image::imageops::resize(&processed, nw, nh, image::imageops::FilterType::Lanczos3)
        }
        None => processed,
    };

    let mut rgb8: RgbImage = ImageBuffer::new(final_image.width(), final_image.height());
    for (x, y, px) in final_image.enumerate_pixels() {
        rgb8.put_pixel(
            x,
            y,
            Rgb([(px.0[0] >> 8) as u8, (px.0[1] >> 8) as u8, (px.0[2] >> 8) as u8]),
        );
    }

    if let Some(wm) = &export.watermark {
        draw_watermark(&mut rgb8, wm);
    }

    let stem = src_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled");
    let ext = match export.format {
        ExportFormat::Jpeg => "jpg",
        ExportFormat::Png => "png",
        ExportFormat::Tiff => "tif",
        ExportFormat::Webp => "webp",
    };
    let suffix = format!("_{}", sanitize(&filter.base_simulation));
    let mut out = out_dir.join(format!("{stem}{suffix}.{ext}"));
    let mut i = 1;
    while out.exists() {
        out = out_dir.join(format!("{stem}{suffix}_{i}.{ext}"));
        i += 1;
    }

    match export.format {
        ExportFormat::Jpeg => {
            let mut writer = std::fs::File::create(&out)?;
            let encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, export.quality);
            rgb8.write_with_encoder(encoder)?;
        }
        ExportFormat::Png => rgb8.save_with_format(&out, image::ImageFormat::Png)?,
        ExportFormat::Tiff => rgb8.save_with_format(&out, image::ImageFormat::Tiff)?,
        ExportFormat::Webp => rgb8.save_with_format(&out, image::ImageFormat::WebP)?,
    }
    Ok(out)
}

/// 文件名清理：把非 ASCII 字母数字字符替换为下划线，避免预设名里的空格/标点污染文件名。
fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// 加载一份可用的 TTF 字体。
/// 优先在 macOS / Windows / Linux 上扫描几个常见路径，找到就返回字体字节数组。
/// 都找不到时返回 `None`，调用方应跳过水印绘制（功能降级而非失败）。
fn load_system_font() -> Option<Vec<u8>> {
    const CANDIDATES: &[&str] = &[
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Geneva.ttf",
        "/Library/Fonts/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ];
    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            if FontRef::try_from_slice(&bytes).is_ok() {
                return Some(bytes);
            }
        }
    }
    None
}

/// 在已经 8-bit 的图片上绘制文字水印。
///
/// 实现：用 `ab_glyph` 把 TTF glyph 光栅化为覆盖度（0..1），
/// 然后按 `opacity * coverage` 与原像素做线性 alpha 混合。
/// 不修改图像尺寸，水印直接"画上去"。
fn draw_watermark(img: &mut RgbImage, wm: &Watermark) {
    let font_bytes = match load_system_font() {
        Some(bytes) => bytes,
        None => return,
    };
    let font = match FontRef::try_from_slice(&font_bytes) {
        Ok(f) => f,
        Err(_) => return,
    };
    let scale = PxScale::from(wm.size as f32);
    let scaled = font.as_scaled(scale);
    let mut width = 0f32;
    for c in wm.text.chars() {
        let glyph_id = font.glyph_id(c);
        width += scaled.h_advance(glyph_id);
    }
    let height = scaled.ascent() - scaled.descent();
    let pad = (wm.size as i32) / 3;
    let (img_w, img_h) = img.dimensions();
    let (x0, y0) = match wm.position {
        WatermarkPosition::TopLeft => (pad, pad),
        WatermarkPosition::TopRight => (img_w as i32 - width as i32 - pad, pad),
        WatermarkPosition::BottomLeft => (pad, img_h as i32 - height as i32 - pad),
        WatermarkPosition::BottomRight => (
            img_w as i32 - width as i32 - pad,
            img_h as i32 - height as i32 - pad,
        ),
        WatermarkPosition::Center => (
            (img_w as i32 - width as i32) / 2,
            (img_h as i32 - height as i32) / 2,
        ),
    };
    let opacity = wm.opacity.clamp(0.0, 1.0);
    let mut pen_x = x0 as f32;
    for c in wm.text.chars() {
        let glyph_id = font.glyph_id(c);
        let glyph = glyph_id.with_scale_and_position(scale, ab_glyph::point(pen_x, y0 as f32 + scaled.ascent()));
        if let Some(outline) = font.outline_glyph(glyph) {
            let bb = outline.px_bounds();
            outline.draw(|gx, gy, coverage| {
                let px = bb.min.x as i32 + gx as i32;
                let py = bb.min.y as i32 + gy as i32;
                if px >= 0 && py >= 0 && (px as u32) < img_w && (py as u32) < img_h {
                    let pixel = img.get_pixel_mut(px as u32, py as u32);
                    let a = coverage * opacity;
                    pixel.0[0] = (pixel.0[0] as f32 * (1.0 - a) + 255.0 * a) as u8;
                    pixel.0[1] = (pixel.0[1] as f32 * (1.0 - a) + 255.0 * a) as u8;
                    pixel.0[2] = (pixel.0[2] as f32 * (1.0 - a) + 255.0 * a) as u8;
                }
            });
        }
        pen_x += scaled.h_advance(glyph_id);
    }
}
