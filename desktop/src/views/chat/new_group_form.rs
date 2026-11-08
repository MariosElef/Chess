use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _, WindowExt as _,
        button::{Button, ButtonVariants},
        h_flex,
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use crate::{
    components::{
        panel::{PanelHeader, START_GREEN, StartButton},
        shell::MESSAGES_PICTURE,
        social::avatar,
    },
    models::social::Friend,
};

type OnCreate = Rc<dyn Fn(String, Vec<String>, &mut Window, &mut App)>;

/// The New group modal's form: a name, friends ticked by clicking, and Create once both are set.
pub struct NewGroupForm {
    name: Entity<InputState>,
    _name_changes: Subscription,
    friends: Vec<Friend>,
    picked: Vec<String>,
    on_create: OnCreate,
}

impl NewGroupForm {
    pub fn new(
        friends: Vec<Friend>,
        on_create: impl Fn(String, Vec<String>, &mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Group name"));
        // Create waits for a name, so it follows the typing.
        let _name_changes = cx.subscribe(&name, |_, _, _: &InputEvent, cx| cx.notify());
        Self {
            name,
            _name_changes,
            friends,
            picked: Vec::new(),
            on_create: Rc::new(on_create),
        }
    }

    /// Opens the form in a modal over the window.
    pub fn open(
        friends: Vec<Friend>,
        on_create: impl Fn(String, Vec<String>, &mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut App,
    ) {
        let form = cx.new(|cx| Self::new(friends, on_create, window, cx));
        let focus = form.read(cx).name.read(cx).focus_handle(cx);
        window.open_dialog(cx, move |dialog, _, cx| {
            // Themed as the app's panels: their colour and corners, a title band, no gaps.
            dialog
                .w(px(440.))
                .close_button(false)
                .bg(cx.theme().group_box)
                .rounded(cx.theme().radius)
                .overflow_hidden()
                .pt_0()
                .pb_0()
                .gap_0()
                .child(form.clone())
        });
        focus.focus(window, cx);
    }

    fn toggle(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.picked.iter().any(|picked| picked == id) {
            self.picked.retain(|picked| picked != id);
        } else {
            self.picked.push(id.to_owned());
        }
        cx.notify();
    }

    fn group_name(&self, cx: &App) -> String {
        self.name.read(cx).value().trim().to_owned()
    }

    fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.group_name(cx);
        (self.on_create)(name, self.picked.clone(), window, cx);
        window.close_dialog(cx);
    }

    /// A friend to tick: their picture, their name, and a check once picked.
    fn friend_row(&self, friend: &Friend, cx: &mut Context<Self>) -> Stateful<Div> {
        let picked = self.picked.contains(&friend.id);
        let theme = cx.theme();
        let (hover, border) = (theme.accent, theme.border);
        let green = rgb(START_GREEN);
        let id = friend.id.clone();
        let check = div()
            .size_5()
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .border_2()
            .map(|this| {
                if picked {
                    this.border_color(green)
                        .bg(green)
                        .child(Icon::from(IconName::Check).size_3().text_color(white()))
                } else {
                    this.border_color(border)
                }
            });
        h_flex()
            .id(SharedString::from(format!("group-pick-{}", friend.id)))
            .gap_3()
            .px_2()
            .py_1p5()
            .rounded(px(6.))
            .cursor_pointer()
            .hover(move |this| this.bg(hover))
            .child(avatar(&friend.username, px(32.), cx))
            .child(
                div()
                    .flex_1()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(friend.username.clone()),
            )
            .child(check)
            .on_click(cx.listener(move |form, _, _, cx| form.toggle(&id, cx)))
    }
}

impl Render for NewGroupForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<Stateful<Div>> = self
            .friends
            .iter()
            .map(|friend| self.friend_row(friend, cx))
            .collect();
        let ready = !self.group_name(cx).is_empty() && !self.picked.is_empty();
        let form = cx.entity().downgrade();
        let create = StartButton::new("create-group", "Create group", move |window, cx| {
            let _ = form.update(cx, |form, cx| form.create(window, cx));
        })
        .disabled(!ready);
        let muted = cx.theme().muted_foreground;
        let empty = rows.is_empty();
        let body = v_flex()
            .gap_4()
            .px_5()
            .py_4()
            .child(Input::new(&self.name).large())
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child(format!("Friends in the group ({})", self.picked.len())),
                    )
                    .when(empty, |this| {
                        this.child(
                            div()
                                .py_2()
                                .text_sm()
                                .text_color(muted)
                                .child("Add friends on Social first."),
                        )
                    })
                    .child(
                        div()
                            .id("group-friends")
                            .max_h(px(280.))
                            .overflow_y_scroll()
                            .child(v_flex().children(rows)),
                    ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Button::new("cancel-group")
                            .ghost()
                            .large()
                            .label("Cancel")
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    )
                    .child(div().flex_1().child(create)),
            );
        v_flex()
            .child(PanelHeader::new(MESSAGES_PICTURE, "New group"))
            .child(body)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use gpui_kit::TestAppContext;

    use super::NewGroupForm;
    use crate::models::social::Friend;

    fn friend(name: &str) -> Friend {
        Friend {
            id: name.to_owned(),
            username: name.to_owned(),
            online: true,
            away_s: None,
            ratings: BTreeMap::new(),
        }
    }

    #[gpui_kit::test]
    fn friends_are_ticked_and_unticked_and_a_group_needs_a_name(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (form, cx) = cx.add_window_view(|window, cx| {
            NewGroupForm::new(
                vec![friend("bob"), friend("cat")],
                |_, _, _, _| {},
                window,
                cx,
            )
        });
        form.update(cx, |form, cx| form.toggle("bob", cx));
        form.update(cx, |form, cx| form.toggle("cat", cx));
        form.update(cx, |form, cx| form.toggle("bob", cx));
        form.read_with(cx, |form, cx| {
            assert_eq!(form.picked, ["cat"]);
            assert!(form.group_name(cx).is_empty(), "no name typed yet");
        });
        form.update_in(cx, |form, window, cx| {
            form.name
                .update(cx, |input, cx| input.set_value("  Club  ", window, cx));
        });
        form.read_with(cx, |form, cx| assert_eq!(form.group_name(cx), "Club"));
    }
}
