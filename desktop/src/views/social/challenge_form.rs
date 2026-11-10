use std::rc::Rc;

use chess_domain::TimeControl;
use gpui_kit::{
    component::{ActiveTheme as _, WindowExt as _, h_flex, v_flex},
    *,
};

use crate::{
    components::social::{ChallengePicker, avatar},
    i18n::tr,
    models::social::Friend,
    services::social::face_of,
};

type OnSend = Rc<dyn Fn(TimeControl, bool, &mut Window, &mut App)>;

/// The Challenge modal: who, then a time control and rated or not, as the Play lobby asks.
pub struct ChallengeForm {
    friend: Friend,
    choice: TimeControl,
    rated: bool,
    /// Signed in with an account; a guest's games are never rated.
    can_rate: bool,
    on_send: OnSend,
}

impl ChallengeForm {
    pub fn new(
        friend: Friend,
        (choice, rated, can_rate): (TimeControl, bool, bool),
        on_send: impl Fn(TimeControl, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            friend,
            choice,
            rated,
            can_rate,
            on_send: Rc::new(on_send),
        }
    }

    /// Opens the form in a modal over the window, starting from the last time control picked.
    pub fn open(
        friend: Friend,
        terms: (TimeControl, bool, bool),
        on_send: impl Fn(TimeControl, bool, &mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut App,
    ) {
        let form = cx.new(|_| Self::new(friend, terms, on_send));
        window.open_dialog(cx, move |dialog, _, cx| {
            // Themed as the app's panels, like New group.
            dialog
                .w(px(560.))
                .close_button(false)
                .bg(cx.theme().group_box)
                .rounded(cx.theme().radius)
                .overflow_hidden()
                .pt_0()
                .pb_0()
                .gap_0()
                .child(form.clone())
        });
    }

    /// Sends the challenge and puts the modal away.
    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.hand_on(window, cx);
        window.close_dialog(cx);
    }

    /// The time picked and the rated preference, to whoever opened the modal.
    fn hand_on(&self, window: &mut Window, cx: &mut App) {
        (self.on_send)(self.choice, self.rated, window, cx);
    }

    /// The friend's picture and name, as the modal's title band.
    fn header(&self, cx: &App) -> Div {
        let friend = &self.friend;
        let face = face_of(&friend.id, &friend.username, friend.avatar.as_deref());
        h_flex()
            .gap_3()
            .px_5()
            .py_4()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(avatar(&face.name, face.picture.as_deref(), px(40.), cx))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .child(tr!("social.challenge.title", name = self.friend.username)),
            )
    }
}

impl Render for ChallengeForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The form outlives its buttons unless the window is closing.
        let form = cx.entity().downgrade();
        let (choose, rated, send) = (form.clone(), form.clone(), form);
        let picker = ChallengePicker::new(
            (self.choice, self.rated, self.can_rate),
            move |choice, _, cx| {
                let _ = choose.update(cx, |form, cx| {
                    form.choice = choice;
                    cx.notify();
                });
            },
            move |on, _, cx| {
                let _ = rated.update(cx, |form, cx| {
                    form.rated = on;
                    cx.notify();
                });
            },
            (
                move |window, cx: &mut App| {
                    let _ = send.update(cx, |form, cx| form.send(window, cx));
                },
                |window, cx: &mut App| window.close_dialog(cx),
            ),
        );
        v_flex()
            .child(self.header(cx))
            .child(div().px_5().py_4().child(picker))
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

    use chess_domain::TimeControl;
    use gpui_kit::TestAppContext;

    use super::ChallengeForm;
    use crate::models::social::Friend;

    #[gpui_kit::test]
    fn sending_hands_on_the_time_picked_and_the_rated_preference(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let sent: Rc<RefCell<Option<(TimeControl, bool)>>> = Rc::default();
        let heard = Rc::clone(&sent);
        let bob = Friend {
            id: "bob".to_owned(),
            username: "bob".to_owned(),
            avatar: None,
            online: true,
            away_s: None,
            ratings: BTreeMap::new(),
        };
        let (form, cx) = cx.add_window_view(move |_, _| {
            ChallengeForm::new(
                bob,
                (TimeControl::new(3, 2), true, false),
                move |time, rated, _, _| {
                    *heard.borrow_mut() = Some((time, rated));
                },
            )
        });
        form.update(cx, |form, _| form.choice = TimeControl::new(10, 0));
        form.update_in(cx, |form, window, cx| form.hand_on(window, cx));
        assert_eq!(
            *sent.borrow(),
            Some((TimeControl::new(10, 0), true)),
            "the preference; Social keeps a guest unrated"
        );
    }
}
