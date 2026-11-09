use std::{fs, path::Path};

/// The largest picture the server takes; a larger file is refused before it is sent.
const LARGEST_PICTURE: u64 = 5 * 1024 * 1024;

/// Why a picture file could not be sent.
#[derive(Debug)]
pub enum PictureFileError {
    TooLarge,
    /// Missing, or not ours to read.
    Unreadable,
}

/// The bytes of the picture file at `path`, if it is small enough to send.
pub fn read_picture(path: &Path) -> Result<Vec<u8>, PictureFileError> {
    let size = fs::metadata(path)
        .map_err(|_| PictureFileError::Unreadable)?
        .len();
    if size > LARGEST_PICTURE {
        return Err(PictureFileError::TooLarge);
    }
    fs::read(path).map_err(|_| PictureFileError::Unreadable)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{PictureFileError, read_picture};

    #[test]
    fn a_small_file_is_read_and_a_missing_one_is_not() {
        let path = std::env::temp_dir().join("chess-picture-test.bin");
        fs::write(&path, b"picture").expect("written");
        assert_eq!(read_picture(&path).expect("read"), b"picture");
        fs::remove_file(&path).expect("removed");
        assert!(matches!(
            read_picture(&path),
            Err(PictureFileError::Unreadable)
        ));
    }
}
