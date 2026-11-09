use gpui_kit::App;

use super::SessionSource;
use crate::{models::account::Face, services::social::face_of};

/// The signed-in player's face, for their own bar; none while signed out.
pub fn my_face(session: &SessionSource, cx: &App) -> Option<Face> {
    session(cx).map(|session| {
        let user = session.user;
        face_of(&user.id, &user.username, user.avatar.as_deref())
    })
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui_kit::TestAppContext;

    use super::my_face;
    use crate::{testing::ann, views::live::SessionSource};

    #[gpui_kit::test]
    fn signed_out_there_is_no_face_and_signed_in_there_is(cx: &mut TestAppContext) {
        let nobody: SessionSource = Rc::new(|_| None);
        assert_eq!(cx.update(|cx| my_face(&nobody, cx)), None);
        let mut session = ann(false);
        session.user.avatar = Some("h1".to_owned());
        let ann: SessionSource = Rc::new(move |_| Some(session.clone()));
        let face = cx.update(|cx| my_face(&ann, cx)).expect("a face");
        assert_eq!(face.name, "ann");
        assert_eq!(face.picture.as_deref(), Some("avatars/ann/h1"));
    }
}
