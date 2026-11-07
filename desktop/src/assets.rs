use std::borrow::Cow;

use gpui_kit::{
    AssetSource, Result, SharedString,
    assets::{Assets, icon_assets},
};

use crate::board::PIECE_IMAGES;

icon_assets!(ChessIcons, [ChessKnight, Puzzle]);

/// The kit's default icons, the Lucide icons only this app uses, and the piece images.
pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(piece) = PIECE_IMAGES.iter().find(|image| image.path == path) {
            return Ok(Some(Cow::Borrowed(piece.bytes)));
        }
        match ChessIcons.load(path)? {
            Some(icon) => Ok(Some(icon)),
            None => Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths: Vec<SharedString> = PIECE_IMAGES
            .iter()
            .filter(|image| image.path.starts_with(path))
            .map(|image| image.path.into())
            .collect();
        paths.extend(ChessIcons.list(path)?);
        paths.extend(Assets.list(path)?);
        Ok(paths)
    }
}
