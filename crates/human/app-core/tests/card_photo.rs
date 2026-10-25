// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A card's photo at both ends (plan §29 step 2, exit gate (b): "a photo
//! chosen with a GPS block and a camera serial leaves the sender with
//! neither"; `human-client-ui.md` §13: "the sent photo carries no image
//! metadata"; "a received photo is not decoded before Accept" is the
//! facade's, and the 256 × 256 bound at decode is here).

#![allow(clippy::expect_used, clippy::panic)]

use std::io::Cursor;

use image::{DynamicImage, ImageFormat, RgbImage};
use interweave_human_app_core::card_photo::{
    MAX_CARD_PHOTO_SIDE, PhotoError, decode_received, prepare_photo,
};
use interweave_human_chat_protocol::{CardPhoto, MAX_CARD_PHOTO_BYTES};

const SERIAL: &[u8] = b"CAMSERIAL-0042";
const GPS: &[u8] = b"GPSLatitudeRef";
const XMP_NS: &[u8] = b"http://ns.adobe.com/xap/1.0/";

fn picture(width: u32, height: u32) -> DynamicImage {
    // Noise, so the encoder has something to spend bytes on.
    let mut n: u32 = 0x9e37_79b9;
    DynamicImage::ImageRgb8(RgbImage::from_fn(width, height, |x, y| {
        n = n.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        image::Rgb([x.to_le_bytes()[0], y.to_le_bytes()[0], n.to_be_bytes()[0]])
    }))
}

fn encode(image: &DynamicImage, format: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image.write_to(&mut out, format).expect("encodes");
    out.into_inner()
}

/// An EXIF APP1 segment: orientation (tag 0x0112) and, in an ASCII
/// value, a GPS tag name and a camera serial.
fn exif_app1(orientation: u16) -> Vec<u8> {
    let mut tiff = b"II*\0".to_vec();
    tiff.extend_from_slice(&8u32.to_le_bytes());
    let text: Vec<u8> = [GPS, b" ", SERIAL, b"\0"].concat();
    // Two entries: orientation, and ImageDescription (0x010e) pointing at
    // the text past the IFD.
    tiff.extend_from_slice(&2u16.to_le_bytes());
    tiff.extend_from_slice(&0x0112u16.to_le_bytes());
    tiff.extend_from_slice(&3u16.to_le_bytes());
    tiff.extend_from_slice(&1u32.to_le_bytes());
    tiff.extend_from_slice(&u32::from(orientation).to_le_bytes());
    tiff.extend_from_slice(&0x010eu16.to_le_bytes());
    tiff.extend_from_slice(&2u16.to_le_bytes());
    tiff.extend_from_slice(&u32::try_from(text.len()).expect("short").to_le_bytes());
    let at = 8 + 2 + 2 * 12 + 4;
    tiff.extend_from_slice(&u32::try_from(at).expect("short").to_le_bytes());
    tiff.extend_from_slice(&0u32.to_le_bytes());
    tiff.extend_from_slice(&text);
    let mut body = b"Exif\0\0".to_vec();
    body.extend_from_slice(&tiff);
    app1(&body)
}

fn app1(body: &[u8]) -> Vec<u8> {
    let mut seg = vec![0xFF, 0xE1];
    seg.extend_from_slice(&u16::try_from(body.len() + 2).expect("short").to_be_bytes());
    seg.extend_from_slice(body);
    seg
}

/// A JPEG with EXIF and XMP segments after its SOI.
fn jpeg_with_metadata(image: &DynamicImage, orientation: u16) -> Vec<u8> {
    let plain = encode(image, ImageFormat::Jpeg);
    assert_eq!(&plain[..2], &[0xFF, 0xD8], "a JPEG");
    let xmp = app1(&[XMP_NS, b"\0<x:xmpmeta>", GPS, b" ", SERIAL, b"</x:xmpmeta>"].concat());
    [&plain[..2], &exif_app1(orientation), &xmp, &plain[2..]].concat()
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn a_photo_chosen_with_a_gps_block_and_a_camera_serial_leaves_the_sender_with_neither() {
    let chosen = jpeg_with_metadata(&picture(1024, 768), 1);
    for planted in [SERIAL, GPS, b"Exif".as_slice(), XMP_NS] {
        assert!(
            contains(&chosen, planted),
            "the control: the chosen file carries it"
        );
    }
    let sent = prepare_photo(&chosen).expect("prepared");
    for planted in [SERIAL, GPS, b"Exif".as_slice(), XMP_NS] {
        assert!(
            !contains(&sent.bytes, planted),
            "{} survived the re-encoding",
            String::from_utf8_lossy(planted)
        );
    }
    assert_eq!(sent.media_type, "image/jpeg");
    assert!(
        sent.bytes.len() <= MAX_CARD_PHOTO_BYTES,
        "{} bytes",
        sent.bytes.len()
    );
    assert_eq!(
        (sent.width, sent.height),
        (256, 256),
        "its largest centred square, exactly 256 a side"
    );
}

#[test]
fn a_png_text_chunk_is_not_carried_either() {
    let mut png = encode(&picture(300, 300), ImageFormat::Png);
    // A tEXt chunk before IEND (the last 12 bytes).
    let data = b"Author\0Alice Example";
    let mut chunk = u32::try_from(data.len())
        .expect("short")
        .to_be_bytes()
        .to_vec();
    let typed: Vec<u8> = [b"tEXt".as_slice(), data].concat();
    chunk.extend_from_slice(&typed);
    chunk.extend_from_slice(&crc32(&typed).to_be_bytes());
    let at = png.len() - 12;
    png.splice(at..at, chunk);
    assert!(contains(&png, b"Alice Example"), "the control");
    let sent = prepare_photo(&png).expect("prepared");
    assert!(!contains(&sent.bytes, b"Alice Example"));
    assert!(sent.width <= MAX_CARD_PHOTO_SIDE && sent.height <= MAX_CARD_PHOTO_SIDE);
}

/// A picture `width` wide, red on its left half and blue on its right.
fn halves(width: u32, height: u32) -> DynamicImage {
    DynamicImage::ImageRgb8(RgbImage::from_fn(width, height, |x, _| {
        if x < width / 2 {
            image::Rgb([255, 0, 0])
        } else {
            image::Rgb([0, 0, 255])
        }
    }))
}

/// The prepared photo's pixel at `(x, y)`.
fn pixel(sent: &interweave_human_app_core::card_photo::PreparedPhoto, x: u32, y: u32) -> [u8; 3] {
    image::load_from_memory(&sent.bytes)
        .expect("decodes")
        .to_rgb8()
        .get_pixel(x, y)
        .0
}

fn reddish([r, _, b]: [u8; 3]) -> bool {
    r > 200 && b < 60
}

#[test]
fn a_photo_is_turned_as_its_orientation_says_before_the_orientation_is_dropped() {
    // EXIF orientation 6: the stored pixels are to be turned 90° clockwise,
    // so the red left half is the top of the photo the person saw.
    let chosen = jpeg_with_metadata(&halves(400, 200), 6);
    let sent = prepare_photo(&chosen).expect("prepared");
    assert_eq!((sent.width, sent.height), (200, 200), "a 200 square, kept");
    assert!(reddish(pixel(&sent, 100, 20)), "red on top");
    assert!(!reddish(pixel(&sent, 100, 180)), "blue below");
    let upright = jpeg_with_metadata(&halves(400, 200), 1);
    let sent = prepare_photo(&upright).expect("prepared");
    assert!(
        reddish(pixel(&sent, 20, 100)),
        "the control: red on the left"
    );
    assert!(!reddish(pixel(&sent, 180, 100)));
}

/// Design 5.2: the largest centred square, exactly 256 a side; a smaller
/// one at its own size, never enlarged; what is transparent laid over
/// white. Mutations: the crop taken from a corner; a small photo
/// enlarged; transparency left black.
#[test]
fn a_chosen_photo_is_its_centred_square_at_256_and_laid_over_white() {
    // Green, red and blue thirds: the centred square is all red but its
    // edges.
    let thirds = DynamicImage::ImageRgb8(RgbImage::from_fn(600, 300, |x, _| match x {
        0..200 => image::Rgb([0, 255, 0]),
        200..400 => image::Rgb([255, 0, 0]),
        _ => image::Rgb([0, 0, 255]),
    }));
    let sent = prepare_photo(&encode(&thirds, ImageFormat::Png)).expect("prepared");
    assert_eq!((sent.width, sent.height), (256, 256));
    assert!(
        reddish(pixel(&sent, 128, 128)),
        "the middle third at its centre"
    );
    assert!(!reddish(pixel(&sent, 2, 128)), "green at its left edge");
    assert!(!reddish(pixel(&sent, 253, 128)), "blue at its right edge");

    let small = prepare_photo(&encode(&picture(100, 60), ImageFormat::Png)).expect("prepared");
    assert_eq!((small.width, small.height), (60, 60), "never enlarged");

    let clear = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        40,
        40,
        image::Rgba([0, 0, 0, 0]),
    ));
    let sent = prepare_photo(&encode(&clear, ImageFormat::Png)).expect("prepared");
    let [r, g, b] = pixel(&sent, 20, 20);
    assert!(
        r > 245 && g > 245 && b > 245,
        "white, not black: {r} {g} {b}"
    );
}

#[test]
fn what_is_not_a_readable_png_or_jpeg_within_bounds_is_refused() {
    assert_eq!(
        prepare_photo(b"not an image at all"),
        Err(PhotoError::Unreadable)
    );
    let gif = b"GIF89a\x01\x00\x01\x00\x00\x00\x00;";
    assert_eq!(prepare_photo(gif), Err(PhotoError::Unreadable));
    // A PNG whose header claims 20000 x 20000: refused before the decode
    // allocates for it.
    let mut png = encode(&picture(2, 2), ImageFormat::Png);
    png[16..20].copy_from_slice(&20_000u32.to_be_bytes());
    png[20..24].copy_from_slice(&20_000u32.to_be_bytes());
    let ihdr_crc = crc32(&png[12..29]);
    png[29..33].copy_from_slice(&ihdr_crc.to_be_bytes());
    assert_eq!(prepare_photo(&png), Err(PhotoError::TooLarge));
}

#[test]
fn a_received_photo_decodes_only_within_256_by_256() {
    let at = encode(&picture(256, 256), ImageFormat::Png);
    let shown = decode_received(&CardPhoto {
        media_type: "image/png",
        bytes: at,
    })
    .expect("decodes");
    assert_eq!((shown.width, shown.height), (256, 256));
    assert_eq!(shown.rgba.len(), 256 * 256 * 4);
    for (w, h) in [(257, 10), (10, 257)] {
        let over = encode(&picture(w, h), ImageFormat::Png);
        assert_eq!(
            decode_received(&CardPhoto {
                media_type: "image/png",
                bytes: over,
            }),
            None,
            "{w} x {h} is dropped, the card kept without it"
        );
    }
    assert_eq!(
        decode_received(&CardPhoto {
            media_type: "image/jpeg",
            bytes: b"\xFF\xD8 broken".to_vec(),
        }),
        None
    );
}

#[test]
fn a_received_photo_whose_header_lies_about_its_size_is_dropped_before_the_decode() {
    // TM-53: a header claiming 20000 x 20000 at a few bytes is dropped,
    // the card kept without it. The decode limit itself is pinned by the
    // 257-pixel cases above: this stream also fails without it.
    let mut lie = encode(&picture(2, 2), ImageFormat::Png);
    lie[16..20].copy_from_slice(&20_000u32.to_be_bytes());
    lie[20..24].copy_from_slice(&20_000u32.to_be_bytes());
    let ihdr_crc = crc32(&lie[12..29]);
    lie[29..33].copy_from_slice(&ihdr_crc.to_be_bytes());
    assert_eq!(
        decode_received(&CardPhoto {
            media_type: "image/png",
            bytes: lie,
        }),
        None
    );
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for b in bytes {
        crc ^= u32::from(*b);
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
