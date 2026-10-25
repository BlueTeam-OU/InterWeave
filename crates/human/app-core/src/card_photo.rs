// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A card's photo, at both ends (HUMAN-CHAT.md §Contact card, ADR-0059
//! decision 5, plan §29 step 2).
//!
//! **The sender** re-encodes the photo a person chooses (contact-cards
//! design 5.2, "The photo"): decoded within bounds, turned the way its
//! orientation says, cropped to its largest centred square and scaled to
//! exactly 256 × 256 (a smaller square keeps its size, never enlarged),
//! laid over white where it is transparent, and written again as a new
//! JPEG at the highest quality from 90 down to 20 that the card allows.
//! The `image` crate's encoder writes every component at full resolution
//! (4:4:4), not the 4:2:0 design 5.2's figures were measured at;
//! architect-cto accepted it (2026-10-25: nothing on the wire names a
//! subsampling). Measured with this preparation on 2026-10-25: seven
//! photographs (850 x 772 to 3024 x 4032) land at quality 85 to 90, from
//! 13,123 to 24,555 bytes; uniform noise at 256 px is 55,940 bytes at 50,
//! 47,781 at 40 and 21,616 at 20, and lands at 21. Below about 40 for an
//! ordinary photograph is the measure that brings an encoder with 4:2:0. A new
//! encoding carries the pixels and nothing else — no EXIF, XMP or GPS
//! block, no embedded thumbnail, no text chunk, no file name — because
//! nothing of the original file but its pixels reaches the encoder.
//!
//! **The receiver** decodes a card's photo only after the person accepts
//! the card, with a memory-safe decoder held to 256 × 256: Accept keeps
//! the photo as received only when it decodes so, and a photo that fails
//! to decode, or is larger, is dropped and the card kept without it
//! (`cards::kept_photo`); the view decodes the kept bytes again to show
//! them.
//! Both decoders are the `image` crate's pure-Rust ones (`png`,
//! `zune-jpeg`), already in the graph under the toolkit.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageDecoder as _, ImageReader, Limits};
use interweave_human_chat_protocol::{CardPhoto, MAX_CARD_PHOTO_BYTES};

/// A card's photo is at most this many pixels on a side.
pub const MAX_CARD_PHOTO_SIDE: u32 = 256;
/// A photo a person chooses may be at most this many pixels on a side
/// before it is scaled: past it, the decode is refused rather than run.
pub const MAX_CHOSEN_SIDE: u32 = 16_384;
/// What a decode may allocate, the decoded image included.
const MAX_CHOSEN_ALLOC: u64 = 256 * 1024 * 1024;
/// The JPEG qualities tried, highest first, until the photo fits.
const QUALITIES: std::ops::RangeInclusive<u8> = 20..=90;

/// The re-encoded photo, ready for a card: always `image/jpeg`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPhoto {
    /// `image/jpeg`.
    pub media_type: &'static str,
    /// The new encoding.
    pub bytes: Vec<u8>,
    /// Its size in pixels.
    pub width: u32,
    /// Its size in pixels.
    pub height: u32,
}

/// Why a chosen photo cannot go on a card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhotoError {
    /// Not an image this client reads (PNG or JPEG), or a damaged one.
    Unreadable,
    /// Larger than [`MAX_CHOSEN_SIDE`] on a side, or past the decode's
    /// allocation bound.
    TooLarge,
    /// No quality brings it under the card's byte bound.
    DoesNotFit,
}

/// Re-encode the photo a person chose for their card.
///
/// # Errors
/// [`PhotoError`] when it cannot be read within bounds or made to fit.
pub fn prepare_photo(chosen: &[u8]) -> Result<PreparedPhoto, PhotoError> {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_CHOSEN_SIDE);
    limits.max_image_height = Some(MAX_CHOSEN_SIDE);
    limits.max_alloc = Some(MAX_CHOSEN_ALLOC);
    let image = decode(chosen, limits, true)?;
    let side = image.width().min(image.height());
    let square = image.crop_imm(
        (image.width() - side) / 2,
        (image.height() - side) / 2,
        side,
        side,
    );
    let square = if side > MAX_CARD_PHOTO_SIDE {
        square.resize_exact(
            MAX_CARD_PHOTO_SIDE,
            MAX_CARD_PHOTO_SIDE,
            image::imageops::FilterType::Triangle,
        )
    } else {
        square
    };
    // JPEG has no alpha: what is transparent is laid over white.
    let rgba = square.to_rgba8();
    let rgb = DynamicImage::ImageRgb8(image::RgbImage::from_fn(
        rgba.width(),
        rgba.height(),
        |column, row| {
            let [red, green, blue, alpha] = rgba.get_pixel(column, row).0;
            let alpha = u16::from(alpha);
            let over = |channel: u8| {
                u8::try_from((u16::from(channel) * alpha + 255 * (255 - alpha)) / 255)
                    .unwrap_or(u8::MAX)
            };
            image::Rgb([over(red), over(green), over(blue)])
        },
    ));
    for quality in QUALITIES.rev() {
        let mut bytes = Vec::new();
        rgb.write_with_encoder(JpegEncoder::new_with_quality(&mut bytes, quality))
            .map_err(|_| PhotoError::Unreadable)?;
        if bytes.len() <= MAX_CARD_PHOTO_BYTES {
            return Ok(PreparedPhoto {
                media_type: "image/jpeg",
                bytes,
                width: rgb.width(),
                height: rgb.height(),
            });
        }
    }
    Err(PhotoError::DoesNotFit)
}

/// A received photo, decoded after Accept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownPhoto {
    /// Its size in pixels, at most [`MAX_CARD_PHOTO_SIDE`] on a side.
    pub width: u32,
    /// Its size in pixels.
    pub height: u32,
    /// Its pixels, RGBA, row by row.
    pub rgba: Vec<u8>,
}

/// Decode an accepted card's photo, or `None` when it does not decode
/// within 256 × 256: the card is kept without it (HUMAN-CHAT.md).
#[must_use]
pub fn decode_received(photo: &CardPhoto) -> Option<ShownPhoto> {
    decode_kept(&photo.bytes)
}

/// Decode a photo the store kept -- an accepted card's, or the person's
/// own card's, already re-encoded -- for display, by the same bound as at
/// Accept (STATE.md: the bytes kept are decoded again at display).
#[must_use]
pub fn decode_kept(bytes: &[u8]) -> Option<ShownPhoto> {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_CARD_PHOTO_SIDE);
    limits.max_image_height = Some(MAX_CARD_PHOTO_SIDE);
    // 256 × 256 RGBA is 256 KiB; a decoder needs little more.
    limits.max_alloc = Some(4 * 1024 * 1024);
    let image = decode(bytes, limits, false).ok()?;
    let rgba = image.to_rgba8();
    Some(ShownPhoto {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

fn decode(bytes: &[u8], limits: Limits, orient: bool) -> Result<DynamicImage, PhotoError> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| PhotoError::Unreadable)?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
    ) {
        return Err(PhotoError::Unreadable);
    }
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|e| limit_or_unreadable(&e))?;
    // A phone writes a photo turned and says so in its metadata; the
    // pixels are turned here, before the metadata is left behind.
    let orientation = if orient {
        decoder.orientation().ok()
    } else {
        None
    };
    let mut image = DynamicImage::from_decoder(decoder).map_err(|e| limit_or_unreadable(&e))?;
    if let Some(orientation) = orientation {
        image.apply_orientation(orientation);
    }
    Ok(image)
}

fn limit_or_unreadable(e: &image::ImageError) -> PhotoError {
    match e {
        image::ImageError::Limits(_) => PhotoError::TooLarge,
        _ => PhotoError::Unreadable,
    }
}
