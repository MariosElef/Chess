use std::io::Cursor;

use image::{
    DynamicImage, GenericImageView as _, ImageDecoder, ImageError, ImageFormat, ImageReader,
    ImageResult, Limits, Rgb, RgbImage, Rgba, RgbaImage,
    codecs::jpeg::JpegEncoder,
    imageops::{self, FilterType},
    metadata::Orientation,
};
use sha2::{Digest as _, Sha256};

/// Every picture is kept as a square this many pixels across.
pub const SIDE: u32 = 256;
/// No picture wider or taller than this is decoded: one that claims more is refused unread.
const LARGEST_SIDE: u32 = 8_192;
/// The most memory one picture may take, every copy counted, whatever its file's size.
const LARGEST_DECODE_BYTES: u64 = 256 * 1024 * 1024;
/// Bytes in a pixel once the picture is 8-bit RGBA, as it is cut and scaled.
const RGBA_BYTES: u64 = 4;
/// JPEG quality: a 256-pixel face at about 20 KB.
const QUALITY: u8 = 85;

/// The picture in `bytes` (JPEG, PNG, WebP or GIF), cut to its middle square and kept as a JPEG.
pub fn square_jpeg(bytes: &[u8]) -> Result<Vec<u8>, ImageError> {
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    reader.limits(picture_limits());
    let mut decoder = reader.into_decoder()?;
    // A phone stores a portrait photo on its side and says so: it is turned upright.
    let orientation = decoder.orientation()?;
    decoder.set_limits(budget_for(&decoder, orientation)?)?;
    let mut picture = DynamicImage::from_decoder(decoder)?;
    picture.apply_orientation(orientation);
    let square = on_white(&middle_square(&picture.into_rgba8()));
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, QUALITY).encode_image(&square)?;
    Ok(jpeg)
}

/// The limits with every full-size copy counted: the decode, a turned one, the 8-bit one.
fn budget_for(decoder: &impl ImageDecoder, orientation: Orientation) -> ImageResult<Limits> {
    let (width, height) = decoder.dimensions();
    let mut limits = picture_limits();
    limits.reserve(decoder.total_bytes())?;
    if turns(orientation) {
        limits.reserve(decoder.total_bytes())?;
    }
    limits.reserve(u64::from(width) * u64::from(height) * RGBA_BYTES)?;
    Ok(limits)
}

/// Whether turning upright makes a new picture: a quarter turn swaps width and height.
fn turns(orientation: Orientation) -> bool {
    match orientation {
        Orientation::Rotate90
        | Orientation::Rotate270
        | Orientation::Rotate90FlipH
        | Orientation::Rotate270FlipH => true,
        Orientation::NoTransforms
        | Orientation::Rotate180
        | Orientation::FlipHorizontal
        | Orientation::FlipVertical => false,
    }
}

/// The middle square, scaled from where it lies: no copy is cut, and a thin picture stays small.
fn middle_square(picture: &RgbaImage) -> RgbaImage {
    let (width, height) = picture.dimensions();
    let side = width.min(height);
    let square = picture.view((width - side) / 2, (height - side) / 2, side, side);
    imageops::resize(&*square, SIDE, SIDE, FilterType::Lanczos3)
}

/// A JPEG has no see-through parts: those of a PNG, WebP or GIF are laid on white.
fn on_white(picture: &RgbaImage) -> RgbImage {
    RgbImage::from_fn(picture.width(), picture.height(), |x, y| {
        let Rgba([red, green, blue, alpha]) = *picture.get_pixel(x, y);
        Rgb([red, green, blue].map(|channel| over_white(channel, alpha)))
    })
}

/// One colour channel at `alpha` opacity over white.
fn over_white(channel: u8, alpha: u8) -> u8 {
    let (channel, alpha) = (u16::from(channel), u16::from(alpha));
    let blended = (channel * alpha + 255 * (255 - alpha)) / 255;
    u8::try_from(blended).expect("a blend of two bytes stays a byte")
}

/// The picture's name: the start of its SHA-256, so a new picture always has a new URL.
pub fn picture_hash(jpeg: &[u8]) -> String {
    Sha256::digest(jpeg)
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// What a decoder may take on: `LARGEST_SIDE` either way, `LARGEST_DECODE_BYTES` in all.
fn picture_limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(LARGEST_SIDE);
    limits.max_image_height = Some(LARGEST_SIDE);
    limits.max_alloc = Some(LARGEST_DECODE_BYTES);
    limits
}

/// What a stored picture is, for the answer's content type.
pub fn stored_format() -> ImageFormat {
    ImageFormat::Jpeg
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{ImageFormat, RgbImage};

    use super::{SIDE, picture_hash, square_jpeg};

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        RgbImage::from_pixel(width, height, image::Rgb([200, 40, 40]))
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("a png");
        bytes
    }

    #[test]
    fn any_picture_becomes_a_square_jpeg() {
        let jpeg = square_jpeg(&png(640, 300)).expect("it reads");
        let read = image::load_from_memory_with_format(&jpeg, ImageFormat::Jpeg).expect("a jpeg");
        assert_eq!((read.width(), read.height()), (SIDE, SIDE));
    }

    #[test]
    fn see_through_parts_turn_white() {
        assert_eq!(super::over_white(0, 0), 255);
        assert_eq!(super::over_white(10, 255), 10);
        assert_eq!(super::over_white(0, 128), 127);
    }

    #[test]
    fn a_phone_photo_is_taken_whatever_its_size() {
        let jpeg = square_jpeg(&png(5712, 4284)).expect("an iPhone's 24 MP fits");
        let read = image::load_from_memory_with_format(&jpeg, ImageFormat::Jpeg).expect("a jpeg");
        assert_eq!((read.width(), read.height()), (SIDE, SIDE));
    }

    #[test]
    fn a_thin_strip_is_cut_before_it_is_scaled() {
        let jpeg = square_jpeg(&png(1, 4096)).expect("it reads");
        let read = image::load_from_memory_with_format(&jpeg, ImageFormat::Jpeg).expect("a jpeg");
        assert_eq!((read.width(), read.height()), (SIDE, SIDE));
    }

    #[test]
    fn what_is_not_a_picture_is_refused() {
        assert!(square_jpeg(b"not a picture").is_err());
    }

    #[test]
    fn the_hash_names_the_bytes() {
        let jpeg = square_jpeg(&png(10, 10)).expect("it reads");
        assert_eq!(picture_hash(&jpeg), picture_hash(&jpeg));
        assert_eq!(picture_hash(&jpeg).len(), 32);
        assert_ne!(picture_hash(&jpeg), picture_hash(b"other"));
    }
}
