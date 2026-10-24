// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The address code (ADR-0062 decision 4; `design/2026-10-22-add-a-peer.md`
//! §5.4, §5.5): drawn for Show, read for a scan.
//!
//! A code carries an add's lines as text, one per line, and nothing else:
//! what it says is judged by the model's rules and then the runtime's,
//! never here. Reading a file is held to bounds a hostile image cannot
//! move: its bytes, sides and the decoder's allocation are capped before
//! a pixel is decoded ([`read_image`]), and the code's text by the QR
//! format itself ([`rqrr::MAX_PAYLOAD_SIZE`]). A pasted image is not: the
//! clipboard crate reads the clipboard's bytes whole and decodes them
//! under the image crate's default allocation bound (512 MiB) before this
//! client sees a pixel, and [`read_pasted`] can only refuse its sides
//! afterwards -- a known limit, stated in the PR that built it (#409).

use std::io::Cursor;

use image::{ImageReader, Limits};
use qrcode::{Color, EcLevel, QrCode};
use slint::{Image, Rgb8Pixel, SharedPixelBuffer};

/// The quiet zone a scanner needs around the code, in modules (ISO/IEC
/// 18004 asks four).
pub const QUIET_MODULES: usize = 4;
/// The most bytes of an image file read for a code.
pub const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;
/// The most pixels on a side of an image read for a code: past it the
/// decode is refused rather than run.
pub const MAX_IMAGE_SIDE: u32 = 8192;
/// What a decode may allocate, the decoded image included.
const MAX_IMAGE_ALLOC: u64 = 256 * 1024 * 1024;

/// What reading an image for a code came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeRead {
    /// The first code that decodes: its text.
    Text(String),
    /// No code decodes in the image.
    NoCode,
}

/// Why an image could not be looked at for a code at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageUnread {
    /// Not a PNG or JPEG, or a damaged one.
    Unreadable,
    /// Past [`MAX_IMAGE_BYTES`], [`MAX_IMAGE_SIDE`] or the decode's
    /// allocation bound.
    TooLarge,
}

/// The code for `lines`, one per line: dark modules on white with the
/// quiet zone, one pixel a module, for the view to scale without
/// smoothing. Error correction is the highest level the text fits, so a
/// short list survives a smudged screen; `None` only for text no code
/// holds, which the model's sixteen lines never are.
#[must_use]
pub fn drawn(lines: &[String]) -> Option<Image> {
    let text = lines.join("\n");
    let code = [EcLevel::M, EcLevel::L]
        .into_iter()
        .find_map(|level| QrCode::with_error_correction_level(&text, level).ok())?;
    let modules = code.width();
    let side = modules + 2 * QUIET_MODULES;
    let side32 = u32::try_from(side).ok()?;
    let mut buffer = SharedPixelBuffer::<Rgb8Pixel>::new(side32, side32);
    let white = Rgb8Pixel::new(255, 255, 255);
    let dark = Rgb8Pixel::new(0, 0, 0);
    let pixels = buffer.make_mut_slice();
    pixels.fill(white);
    for (i, color) in code.to_colors().into_iter().enumerate() {
        if color == Color::Dark {
            let (y, x) = (i / modules + QUIET_MODULES, i % modules + QUIET_MODULES);
            pixels[y * side + x] = dark;
        }
    }
    Some(Image::from_rgb8(buffer))
}

/// Read the first code in a greyscale image, `luma` row by row, 0 black.
/// A buffer shorter than its sides holds no code.
#[must_use]
pub fn read_luma(width: usize, height: usize, luma: &[u8]) -> CodeRead {
    if width.checked_mul(height).is_none_or(|n| n > luma.len()) {
        return CodeRead::NoCode;
    }
    let mut image =
        rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| luma[y * width + x]);
    image
        .detect_grids()
        .into_iter()
        .find_map(|grid| grid.decode().ok().map(|(_, text)| text))
        .map_or(CodeRead::NoCode, CodeRead::Text)
}

/// Read the first code in an RGBA image, `rgba` row by row, four bytes a
/// pixel, as a clipboard hands one over. A pixel's alpha is laid over
/// white, so a transparent background reads as the paper it stands for.
#[must_use]
pub fn read_rgba(width: usize, height: usize, rgba: &[u8]) -> CodeRead {
    if width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(4))
        .is_none_or(|n| n > rgba.len())
    {
        return CodeRead::NoCode;
    }
    let luma: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .take(width * height)
        .map(|&[r, g, b, a]| luma_over_white(r, g, b, a))
        .collect();
    read_luma(width, height, &luma)
}

/// Read the first code in an image pasted from the clipboard, already
/// decoded by the clipboard crate: refused past [`MAX_IMAGE_SIDE`], as a
/// file is, before anything more is allocated for it.
///
/// # Errors
/// [`ImageUnread::TooLarge`] past the side bound.
pub fn read_pasted(width: usize, height: usize, rgba: &[u8]) -> Result<CodeRead, ImageUnread> {
    let side = usize::try_from(MAX_IMAGE_SIDE).unwrap_or(usize::MAX);
    if width > side || height > side {
        return Err(ImageUnread::TooLarge);
    }
    Ok(read_rgba(width, height, rgba))
}

/// Read the first code in a PNG or JPEG file's bytes, within bounds.
///
/// # Errors
/// [`ImageUnread`] when the bytes are no image this client reads, or
/// are past a bound.
pub fn read_image(bytes: &[u8]) -> Result<CodeRead, ImageUnread> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(ImageUnread::TooLarge);
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_SIDE);
    limits.max_image_height = Some(MAX_IMAGE_SIDE);
    limits.max_alloc = Some(MAX_IMAGE_ALLOC);
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| ImageUnread::Unreadable)?;
    // Only the two formats the dialog offers, whatever else the crate
    // was built to read.
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
    ) {
        return Err(ImageUnread::Unreadable);
    }
    reader.limits(limits);
    let decoded = reader.decode().map_err(|error| match error {
        image::ImageError::Limits(_) => ImageUnread::TooLarge,
        _ => ImageUnread::Unreadable,
    })?;
    let rgba = decoded.to_rgba8();
    let (width, height) = (rgba.width() as usize, rgba.height() as usize);
    Ok(read_rgba(width, height, rgba.as_raw()))
}

/// Rec. 601 luma of a pixel laid over white.
fn luma_over_white(red: u8, green: u8, blue: u8, alpha: u8) -> u8 {
    let (red, green, blue) = (u32::from(red), u32::from(green), u32::from(blue));
    let alpha = u32::from(alpha);
    let luma = (299 * red + 587 * green + 114 * blue) / 1000;
    let over = (luma * alpha + 255 * (255 - alpha)) / 255;
    u8::try_from(over).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]
    use super::*;

    fn luma_of(image: &Image) -> (usize, usize, Vec<u8>) {
        let buffer = image.to_rgb8().expect("an rgb image");
        let (w, h) = (buffer.width() as usize, buffer.height() as usize);
        let luma = buffer.as_slice().iter().map(|p| p.r).collect();
        (w, h, luma)
    }

    /// Scaled up as a screen would draw it, so the reader sees modules
    /// several pixels wide.
    fn scaled(w: usize, h: usize, luma: &[u8], k: usize) -> (usize, usize, Vec<u8>) {
        let mut out = Vec::with_capacity(w * h * k * k);
        for y in 0..h * k {
            for x in 0..w * k {
                out.push(luma[(y / k) * w + x / k]);
            }
        }
        (w * k, h * k, out)
    }

    fn sixteen() -> Vec<String> {
        (0..16)
            .map(|i| {
                format!(
                    "/ip4/203.0.113.{i}/tcp/4001/p2p/12D3KooWAbCdEfGhJkLmNpQrStUvWxYz123456789AbCdEfGhJkLmN/p2p-circuit/p2p/12D3KooWZyXwVuTsRqPoNmLkJhGfEdCbA987654321ZyXwVuTsRqPoNmL"
                )
            })
            .collect()
    }

    /// What Show draws reads back as exactly its lines, sixteen circuit
    /// routes included, and has its quiet zone white on every side.
    #[test]
    fn a_drawn_code_reads_back_as_its_lines() {
        for lines in [
            vec!["/ip4/192.168.1.20/tcp/4001/p2p/x".to_owned()],
            sixteen(),
        ] {
            let image = drawn(&lines).expect("fits");
            let (w, h, luma) = luma_of(&image);
            for i in 0..w {
                for q in 0..QUIET_MODULES {
                    assert_eq!(luma[q * w + i], 255, "top quiet zone");
                    assert_eq!(luma[(h - 1 - q) * w + i], 255, "bottom quiet zone");
                    assert_eq!(luma[i * w + q], 255, "left quiet zone");
                    assert_eq!(luma[i * w + w - 1 - q], 255, "right quiet zone");
                }
            }
            let (w, h, big) = scaled(w, h, &luma, 4);
            assert_eq!(read_luma(w, h, &big), CodeRead::Text(lines.join("\n")));
        }
    }

    /// An image with no code, and a buffer shorter than its sides, read
    /// as no code rather than failing.
    #[test]
    fn no_code_and_a_short_buffer_read_as_no_code() {
        assert_eq!(read_luma(64, 64, &[255; 64 * 64]), CodeRead::NoCode);
        assert_eq!(read_luma(64, 64, &[0; 10]), CodeRead::NoCode);
        assert_eq!(read_rgba(64, 64, &[0; 10]), CodeRead::NoCode);
    }

    /// A PNG of a code reads back; a transparent background stands for
    /// white; anything else, or past a bound, is refused before decoding.
    #[test]
    fn an_image_file_is_read_within_bounds() {
        let lines = vec!["/ip4/192.168.1.20/tcp/4001/p2p/peer".to_owned()];
        let (w, h, luma) = luma_of(&drawn(&lines).expect("fits"));
        let (w, h, luma) = scaled(w, h, &luma, 4);
        // Dark modules opaque, light ones transparent.
        let rgba: Vec<u8> = luma
            .iter()
            .flat_map(|&l| if l == 0 { [0, 0, 0, 255] } else { [0, 0, 0, 0] })
            .collect();
        let mut png = Vec::new();
        image::RgbaImage::from_raw(
            u32::try_from(w).expect("w"),
            u32::try_from(h).expect("h"),
            rgba,
        )
        .expect("an image")
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("encoded");
        assert_eq!(read_image(&png), Ok(CodeRead::Text(lines.join("\n"))));

        assert_eq!(read_image(b"not an image"), Err(ImageUnread::Unreadable));
        let gif = b"GIF89a\x01\x00\x01\x00\x00\x00\x00;";
        assert_eq!(
            read_image(gif),
            Err(ImageUnread::Unreadable),
            "PNG and JPEG only"
        );
        assert_eq!(
            read_image(&vec![0; MAX_IMAGE_BYTES + 1]),
            Err(ImageUnread::TooLarge)
        );
        let mut wide = Vec::new();
        image::GrayImage::new(MAX_IMAGE_SIDE + 1, 1)
            .write_to(&mut Cursor::new(&mut wide), image::ImageFormat::Png)
            .expect("encoded");
        assert_eq!(read_image(&wide), Err(ImageUnread::TooLarge));
        // Within the sides, but past the decode's allocation: refused
        // as too large, where the crate's own default would let it
        // through to a decode that fails as damaged (#409 F7).
        assert_eq!(
            read_image(&png_header(8192, 8000, 16, 6)),
            Err(ImageUnread::TooLarge)
        );
    }

    /// A PNG of the given header and no pixel data to speak of: what a decoder sizes
    /// its allocation from before it reads a pixel.
    fn png_header(width: u32, height: u32, depth: u8, colour: u8) -> Vec<u8> {
        fn chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
            out.extend_from_slice(&u32::try_from(data.len()).expect("len").to_be_bytes());
            let start = out.len();
            out.extend_from_slice(&kind);
            out.extend_from_slice(data);
            let crc = crc32(&out[start..]);
            out.extend_from_slice(&crc.to_be_bytes());
        }
        fn crc32(bytes: &[u8]) -> u32 {
            let mut crc = !0_u32;
            for &b in bytes {
                crc ^= u32::from(b);
                for _ in 0..8 {
                    crc = if crc & 1 == 1 {
                        (crc >> 1) ^ 0xEDB8_8320
                    } else {
                        crc >> 1
                    };
                }
            }
            !crc
        }
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[depth, colour, 0, 0, 0]);
        chunk(&mut png, *b"IHDR", &ihdr);
        // One empty zlib stream: data enough for a decoder to start.
        chunk(&mut png, *b"IDAT", &[0x78, 0x9c, 0x03, 0x00, 0, 0, 0, 1]);
        chunk(&mut png, *b"IEND", &[]);
        png
    }

    /// A pasted image is refused past the side bound, as a file is, and
    /// read within it (#409 C1).
    #[test]
    fn a_pasted_image_is_refused_past_the_side_bound() {
        let side = MAX_IMAGE_SIDE as usize;
        assert_eq!(read_pasted(side + 1, 1, &[]), Err(ImageUnread::TooLarge));
        assert_eq!(read_pasted(1, side + 1, &[]), Err(ImageUnread::TooLarge));
        assert_eq!(read_pasted(4, 4, &[255; 64]), Ok(CodeRead::NoCode));
    }
}
