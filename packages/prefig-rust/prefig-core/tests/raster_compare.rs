//! Compare two SVG documents by drawing each one and checking that the two
//! pictures look the same.
//!
//! A few figures cannot be checked by comparing the SVG files directly. When a
//! `<shape>` combines other shapes with a boolean operation (union, difference,
//! and so on), the Rust port uses the `geo` library and Python uses `shapely`.
//! The two libraries trace the same outline but list its corner points in a
//! different order and place them a hair apart (up to about a seventh of a
//! unit). The SVG text is therefore not the same even though the drawn figure
//! is. Comparing the finished pictures instead of the files sidesteps that.
//!
//! Drawing is done by `rsvg-convert`, the librsvg command line tool the dev
//! container installs. Every comparison runs in that one container with the
//! same library, so the pictures come out the same each time.

use std::io::Write;
use std::process::{Command, Stdio};

/// Draw an SVG document and return its size and pixels (four bytes per pixel:
/// red, green, blue, alpha).
fn draw(svg: &str) -> Result<(usize, usize, Vec<u8>), String> {
    let mut child = Command::new("rsvg-convert")
        .arg("--format=png")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start rsvg-convert (is it installed?): {e}"))?;

    // The SVG is small (a few kilobytes), well under the pipe buffer, so writing
    // it all before reading the picture back cannot deadlock.
    child
        .stdin
        .take()
        .unwrap()
        .write_all(svg.as_bytes())
        .map_err(|e| format!("could not send the SVG to rsvg-convert: {e}"))?;

    let out = child
        .wait_with_output()
        .map_err(|e| format!("rsvg-convert did not finish: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "rsvg-convert failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    let decoder = png::Decoder::new(out.stdout.as_slice());
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("could not read the PNG rsvg-convert produced: {e}"))?;
    let mut pixels = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut pixels)
        .map_err(|e| format!("could not read the PNG rsvg-convert produced: {e}"))?;
    if info.color_type != png::ColorType::Rgba {
        return Err(format!(
            "expected rsvg-convert to produce RGBA pixels, got {:?}",
            info.color_type
        ));
    }
    pixels.truncate(info.buffer_size());
    Ok((info.width as usize, info.height as usize, pixels))
}

/// How different two drawn figures may be and still count as the same picture.
///
/// A pixel counts as changed when one of its color or alpha values shifts by
/// more than this much (out of 255). The boolean-shape outlines land less than
/// a pixel apart, so all that ever changes is the faint soft edge drawing
/// leaves along a curve. Measured against every current shape figure, no pixel
/// changes by more than 16 from that soft edge, so 16 sits just past the noise
/// while still catching a real color change.
const CHANNEL_TOLERANCE: u8 = 16;

/// At most this share of the pixels may be changed. Every current shape figure
/// changes zero pixels at the tolerance above; this leeway is only so a future
/// change to the drawing library that nudges a few edge pixels does not fail
/// the test. A real change to a shape would move its whole outline and repaint
/// far more than this.
const MAX_CHANGED_FRACTION: f64 = 0.005;

/// Compare two SVG documents as pictures. Returns a list of differences, empty
/// when the two pictures match.
pub fn compare(a: &str, b: &str) -> Vec<String> {
    let (aw, ah, ap) = match draw(a) {
        Ok(v) => v,
        Err(e) => return vec![e],
    };
    let (bw, bh, bp) = match draw(b) {
        Ok(v) => v,
        Err(e) => return vec![e],
    };
    if (aw, ah) != (bw, bh) {
        return vec![format!("picture size {aw}x{ah} != {bw}x{bh}")];
    }

    let changed = ap
        .chunks_exact(4)
        .zip(bp.chunks_exact(4))
        .filter(|(pa, pb)| {
            pa.iter()
                .zip(pb.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE)
        })
        .count();

    let total = aw * ah;
    let fraction = changed as f64 / total as f64;
    if fraction > MAX_CHANGED_FRACTION {
        return vec![format!(
            "pictures differ: {changed} of {total} pixels changed ({:.3}%), \
             more than the allowed {:.3}%",
            fraction * 100.0,
            MAX_CHANGED_FRACTION * 100.0
        )];
    }
    Vec::new()
}
