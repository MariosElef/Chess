use gpui_kit::{
    component::{h_flex, v_flex},
    *,
};

use crate::{components::social::square_avatar, models::account::Face};

/// How big the player's picture is over their number.
const PICTURE: Pixels = px(72.);

/// The player's picture with one number under it, large: a rush's score, a rating, a count.
#[derive(IntoElement)]
pub struct PlayerHeader {
    face: Option<Face>,
    number: String,
}

impl PlayerHeader {
    pub fn new(face: Option<Face>, number: impl Into<String>) -> Self {
        Self {
            face,
            number: number.into(),
        }
    }
}

impl RenderOnce for PlayerHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let picture = self
            .face
            .map(|face| square_avatar(&face.name, face.picture.as_deref(), PICTURE, cx));
        v_flex()
            .w_full()
            .items_center()
            .gap_3()
            .py_2()
            .children(picture)
            .child(
                h_flex()
                    .text_3xl()
                    .font_weight(FontWeight::EXTRA_BOLD)
                    .child(self.number),
            )
    }
}
