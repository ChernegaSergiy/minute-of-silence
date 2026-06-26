//! APNG frame decoder — exposes decoded frames to the frontend as raw RGBA base64.
//!
//! Uses the pure-Rust `png` crate which works identically on all platforms
//! (Windows, macOS, Linux/WebKitGTK) without relying on any browser or OS APIs.

use std::io::Cursor;

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use png::{BlendOp, DisposeOp};

/// Decoded APNG metadata returned alongside the frames.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApngInfo {
    pub width: u32,
    pub height: u32,
    /// Each entry is a base64-encoded raw RGBA pixel buffer (full canvas size).
    pub frames: Vec<String>,
}

/// Decode every frame of an APNG file into fully-composited RGBA canvases.
///
/// The function correctly handles:
/// - `blend_op`: Over / Source
/// - `dispose_op`: None / Background / Previous
///
/// Returns each composited frame as raw RGBA base64 so the frontend can feed
/// it directly into `ImageData` + `putImageData` — no system image decoder needed.
#[tauri::command]
pub fn decode_apng_frames(data: Vec<u8>) -> Result<ApngInfo, String> {
    let cursor = Cursor::new(&data);
    let decoder = png::Decoder::new(cursor);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;

    let info = reader.info();
    let width = info.width;
    let height = info.height;
    let canvas_size = (width * height * 4) as usize;

    // The current composited canvas (RGBA).
    let mut canvas = vec![0u8; canvas_size];
    // Saved copy used by DisposeOp::Previous.
    let mut previous_canvas = vec![0u8; canvas_size];

    let mut frames: Vec<String> = Vec::new();

    // Temporary pixel buffer — sized for the largest possible frame.
    let buf_size = reader
        .output_buffer_size()
        .unwrap_or(width as usize * height as usize * 4);
    let mut raw_buf = vec![0u8; buf_size];

    // How many frames the APNG declares (acTL chunk).
    // Falls back to 1 for plain (non-animated) PNGs.
    let num_frames = reader
        .info()
        .animation_control
        .map(|ac| ac.num_frames as usize)
        .unwrap_or(1);

    for _ in 0..num_frames {
        let frame_info = match reader.next_frame(&mut raw_buf) {
            Ok(fi) => fi,
            Err(e) => {
                // Fewer frames than acTL declared — stop gracefully.
                log::warn!("APNG: next_frame stopped early: {}", e);
                break;
            }
        };

        // The raw pixel buffer for this frame (may be a subframe).
        let frame_bytes = &raw_buf[..frame_info.buffer_size()];

        // APNG frame control (position / size / blend / dispose).
        // If absent this is a plain PNG — treat it as a single full frame.
        let (rect, blend_op, dispose_op) = if let Some(fc) = reader.info().frame_control {
            (
                FrameRect {
                    x: fc.x_offset,
                    y: fc.y_offset,
                    w: fc.width,
                    h: fc.height,
                },
                fc.blend_op,
                fc.dispose_op,
            )
        } else {
            (
                FrameRect {
                    x: 0,
                    y: 0,
                    w: width,
                    h: height,
                },
                BlendOp::Source,
                DisposeOp::None,
            )
        };

        // Convert the raw frame pixels to RGBA (the png crate gives us the
        // output colour type we asked for; we always ask for RGBA below —
        // actually the crate exposes the file's native colour type, so we
        // must handle the most common cases ourselves).
        let frame_rgba = to_rgba(frame_bytes, &frame_info.color_type, rect.w, rect.h)?;

        // Save the canvas before touching it (for DisposeOp::Previous).
        previous_canvas.clone_from(&canvas);

        // Composite the subframe onto the canvas.
        composite(&mut canvas, &frame_rgba, &rect, width, blend_op);

        frames.push(B64.encode(&canvas));

        // Apply dispose_op to prepare the canvas for the next frame.
        match dispose_op {
            DisposeOp::None => {
                // Keep the canvas as-is.
            }
            DisposeOp::Background => {
                // Clear the frame area to fully transparent black.
                fill_rect(&mut canvas, &rect, width, [0, 0, 0, 0]);
            }
            DisposeOp::Previous => {
                // Restore what was saved before this frame.
                canvas.clone_from(&previous_canvas);
            }
        }
    }

    Ok(ApngInfo {
        width,
        height,
        frames,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Bounding rectangle of an APNG subframe within the canvas.
struct FrameRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

/// Convert raw pixel bytes from any common PNG colour type to packed RGBA.
fn to_rgba(
    bytes: &[u8],
    color_type: &png::ColorType,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let pixel_count = (width * height) as usize;
    let mut out = Vec::with_capacity(pixel_count * 4);

    match color_type {
        png::ColorType::Rgba => {
            out.extend_from_slice(bytes);
        }
        png::ColorType::Rgb => {
            for chunk in bytes.chunks(3) {
                out.extend_from_slice(chunk);
                out.push(255);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for chunk in bytes.chunks(2) {
                let v = chunk[0];
                out.extend_from_slice(&[v, v, v, chunk[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for &v in bytes {
                out.extend_from_slice(&[v, v, v, 255]);
            }
        }
        png::ColorType::Indexed => {
            return Err("Indexed colour APNG is not yet supported".to_string());
        }
    }

    if out.len() != pixel_count * 4 {
        return Err(format!(
            "RGBA conversion produced {} bytes, expected {}",
            out.len(),
            pixel_count * 4
        ));
    }

    Ok(out)
}

/// Composite `src` (subframe) onto `dst` (full canvas) using the given blend op.
fn composite(dst: &mut [u8], src: &[u8], rect: &FrameRect, canvas_width: u32, blend_op: BlendOp) {
    for row in 0..rect.h {
        for col in 0..rect.w {
            let src_idx = ((row * rect.w + col) * 4) as usize;
            let dst_idx = (((rect.y + row) * canvas_width + (rect.x + col)) * 4) as usize;

            if src_idx + 4 > src.len() || dst_idx + 4 > dst.len() {
                continue;
            }

            let [sr, sg, sb, sa] = [
                src[src_idx],
                src[src_idx + 1],
                src[src_idx + 2],
                src[src_idx + 3],
            ];

            match blend_op {
                BlendOp::Source => {
                    // Replace destination pixel directly.
                    dst[dst_idx] = sr;
                    dst[dst_idx + 1] = sg;
                    dst[dst_idx + 2] = sb;
                    dst[dst_idx + 3] = sa;
                }
                BlendOp::Over => {
                    // Porter-Duff "over" compositing.
                    let da = dst[dst_idx + 3];
                    let sa_f = sa as f32 / 255.0;
                    let da_f = da as f32 / 255.0;
                    let out_a = sa_f + da_f * (1.0 - sa_f);
                    if out_a > 0.0 {
                        let blend = |s: u8, d: u8| -> u8 {
                            let s = s as f32 / 255.0;
                            let d = d as f32 / 255.0;
                            ((s * sa_f + d * da_f * (1.0 - sa_f)) / out_a * 255.0).round() as u8
                        };
                        dst[dst_idx] = blend(sr, dst[dst_idx]);
                        dst[dst_idx + 1] = blend(sg, dst[dst_idx + 1]);
                        dst[dst_idx + 2] = blend(sb, dst[dst_idx + 2]);
                        dst[dst_idx + 3] = (out_a * 255.0).round() as u8;
                    } else {
                        dst[dst_idx] = 0;
                        dst[dst_idx + 1] = 0;
                        dst[dst_idx + 2] = 0;
                        dst[dst_idx + 3] = 0;
                    }
                }
            }
        }
    }
}

/// Fill a rectangular region of the canvas with a constant RGBA value.
fn fill_rect(canvas: &mut [u8], rect: &FrameRect, canvas_width: u32, pixel: [u8; 4]) {
    for row in 0..rect.h {
        for col in 0..rect.w {
            let idx = (((rect.y + row) * canvas_width + (rect.x + col)) * 4) as usize;
            if idx + 4 <= canvas.len() {
                canvas[idx..idx + 4].copy_from_slice(&pixel);
            }
        }
    }
}
