use std::borrow::Cow;

use gpui_kit::{
    AssetSource, Result, SharedString,
    assets::{Assets, icon_assets},
};

use crate::components::{
    analysis::{review_icon, review_icon_paths},
    board::{PIECE_IMAGES, end_picture, end_picture_paths},
    play::{bot_picture, bot_picture_paths, speed_picture_bytes, speed_picture_paths},
    shell::{nav_picture, nav_picture_paths},
};

icon_assets!(
    ChessIcons,
    [
        ChessKnight,
        Puzzle,
        ChevronsLeft,
        ChevronsRight,
        Flag,
        Handshake,
        ArrowUpDown,
        Microscope,
        Users,
        UserPlus,
        Swords,
        Sparkles
    ]
);

/// The kit's default icons, the Lucide icons only this app uses, the piece images, the game
/// review's icons, and the sidebar's and the speeds' pictures.
pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(piece) = PIECE_IMAGES.iter().find(|image| image.path == path) {
            return Ok(Some(Cow::Borrowed(piece.bytes)));
        }
        if let Some(icon) = review_icon(path)
            .or_else(|| nav_picture(path))
            .or_else(|| speed_picture_bytes(path))
            .or_else(|| bot_picture(path))
            .or_else(|| end_picture(path))
        {
            return Ok(Some(Cow::Borrowed(icon)));
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
        paths.extend(
            review_icon_paths()
                .chain(nav_picture_paths())
                .chain(speed_picture_paths())
                .chain(bot_picture_paths())
                .chain(end_picture_paths())
                .filter(|icon| icon.starts_with(path))
                .map(SharedString::from),
        );
        paths.extend(ChessIcons.list(path)?);
        paths.extend(Assets.list(path)?);
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::AssetSource as _;

    use super::AppAssets;
    use crate::{components::board::PIECE_IMAGES, enums::analysis::MoveClass};

    #[test]
    fn every_piece_picture_of_both_sets_is_served() {
        for image in PIECE_IMAGES {
            let served = AppAssets.load(image.path).expect("loading never fails");
            assert_eq!(served.as_deref(), Some(image.bytes), "{}", image.path);
        }
        assert!(
            PIECE_IMAGES
                .iter()
                .any(|image| image.path == "pieces/modern/wK.png")
        );
    }

    #[test]
    fn every_sidebar_and_speed_picture_is_served() {
        for path in [
            "nav/play.png",
            "nav/puzzles.png",
            "nav/analyse.png",
            "nav/social.png",
            "speed/bullet.png",
            "speed/blitz.png",
            "speed/rapid.png",
            "bots/sporos.jpg",
            "bots/andreas.jpg",
            "end/winner.png",
            "end/checkmate_black.png",
            "end/timeout_white.png",
        ] {
            let served = AppAssets.load(path).expect("loading never fails");
            assert!(served.is_some_and(|bytes| !bytes.is_empty()), "{path}");
        }
    }

    #[test]
    fn every_review_icon_is_served() {
        for class in MoveClass::ALL {
            let served = AppAssets
                .load(class.icon_path())
                .expect("loading never fails");
            assert!(
                served.is_some_and(|bytes| bytes.starts_with(b"\x89PNG")),
                "{class:?}"
            );
        }
    }
}
