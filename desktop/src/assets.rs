use std::{
    borrow::Cow,
    sync::{Mutex, MutexGuard, OnceLock},
};

use gpui_kit::{
    App, AssetSource, ImageSource, Resource, Result, SharedString,
    assets::{Assets, icon_assets},
};

use crate::{
    clients::server::ServerClient,
    components::{
        analysis::{review_icon, review_icon_paths},
        board::{PIECE_IMAGES, end_picture, end_picture_paths},
        play::{bot_picture, bot_picture_paths, speed_picture_bytes, speed_picture_paths},
        puzzles::{puzzle_picture, puzzle_picture_paths},
        shell::{APP_LOGO, app_logo, nav_picture, nav_picture_paths},
        social::{social_picture, social_picture_paths},
    },
    services::social::avatar_of_path,
};

icon_assets!(
    ChessIcons,
    [
        ChessKnight,
        Puzzle,
        ChevronsLeft,
        ChevronsRight,
        Flag,
        ArrowUpDown,
        Microscope,
        Users,
        UserPlus,
        Swords,
        LogIn,
        LogOut,
        Zap,
        Timer,
        Trophy,
        Signal,
        RotateCcw,
        Fish,
        Upload,
        UserMinus,
        UserCheck,
        UserX,
        Ban,
        Mail,
        Monitor,
        Volume2
    ]
);

/// Where players' pictures come from, once the app knows its server.
static PICTURES: OnceLock<ServerClient> = OnceLock::new();
/// Pictures that could not be had: GPUI keeps a failed load for good, so they are asked again.
static MISSED_PICTURES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Players' pictures are fetched from `client` from now on, by the asset paths their hashes make.
pub fn load_pictures_from(client: ServerClient) {
    // Set once, by the first window; a second would be the same server.
    let _ = PICTURES.set(client);
}

/// The kit's icons, this app's Lucide icons, the piece images and the app's own pictures.
pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some((user_id, hash)) = avatar_of_path(path) {
            return Ok(player_picture(user_id, hash).map(Cow::Owned));
        }
        if let Some(piece) = PIECE_IMAGES.iter().find(|image| image.path == path) {
            return Ok(Some(Cow::Borrowed(piece.bytes)));
        }
        if let Some(icon) = review_icon(path)
            .or_else(|| nav_picture(path))
            .or_else(|| speed_picture_bytes(path))
            .or_else(|| bot_picture(path))
            .or_else(|| end_picture(path))
            .or_else(|| puzzle_picture(path))
            .or_else(|| social_picture(path))
            .or_else(|| app_logo(path))
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
                .chain(puzzle_picture_paths())
                .chain(social_picture_paths())
                .chain([APP_LOGO])
                .filter(|icon| icon.starts_with(path))
                .map(SharedString::from),
        );
        paths.extend(ChessIcons.list(path)?);
        paths.extend(Assets.list(path)?);
        Ok(paths)
    }
}

/// A player's picture from the server; none while it cannot be had, and the initial shows.
fn player_picture(user_id: &str, hash: &str) -> Option<Vec<u8>> {
    let picture = PICTURES.get()?.avatar_now(user_id, hash).ok();
    if picture.is_none() {
        missed_pictures().push(format!("avatars/{user_id}/{hash}"));
    }
    picture
}

/// The server is back: every picture that could not be had is asked for again.
pub fn retry_missed_pictures(cx: &mut App) {
    let missed = std::mem::take(&mut *missed_pictures());
    for path in missed {
        ImageSource::Resource(Resource::Embedded(path.into())).remove_asset(cx);
    }
    cx.refresh_windows();
}

/// The pictures missed so far; a panic elsewhere while holding them only loses a retry.
fn missed_pictures() -> MutexGuard<'static, Vec<String>> {
    MISSED_PICTURES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use gpui_kit::AssetSource as _;

    use super::AppAssets;
    use crate::{components::board::PIECE_IMAGES, enums::analysis::MoveClass};

    #[test]
    fn every_piece_picture_of_every_set_is_served() {
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
    fn every_rail_and_speed_picture_is_served() {
        for path in [
            "nav/play.png",
            "nav/puzzles.png",
            "nav/analyse.png",
            "nav/social.png",
            "speed/bullet.png",
            "speed/blitz.png",
            "speed/rapid.png",
            "speed/classical.svg",
            "bots/sporos.jpg",
            "bots/andreas.jpg",
            "end/winner.png",
            "end/checkmate_black.png",
            "end/timeout_white.png",
            "puzzles/battle.svg",
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

    #[test]
    fn a_picture_path_is_not_served_before_the_app_knows_its_server() {
        let served = AppAssets
            .load("avatars/u1/abc")
            .expect("loading never fails");
        assert_eq!(served, None, "the initial shows instead");
    }
}
