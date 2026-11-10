use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Sizable as _, WindowExt as _,
        button::{Button, ButtonVariants},
        h_flex,
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use crate::{
    components::{panel::PanelHeader, shell::MESSAGES_PICTURE, social::avatar},
    enums::chat::GroupChange,
    i18n::tr,
    models::{
        chat::Conversation,
        social::{Friend, UserRef},
    },
    services::social::avatar_path,
};

type OnChange = Rc<dyn Fn(GroupChange, &mut Window, &mut App)>;

/// The Group modal: members, the owner renaming and removing, anyone adding friends and leaving.
pub struct GroupForm {
    group: Conversation,
    /// The signed-in player's id.
    me: String,
    friends: Vec<Friend>,
    name: Entity<InputState>,
    _name_changes: Subscription,
    on_change: OnChange,
    /// The server refused the last change.
    problem: Option<String>,
    /// The player is no longer in the group: only Close is left.
    gone: bool,
    /// The group was renamed: the name field takes the new name when next drawn.
    refill_name: bool,
    /// A change is on its way: the buttons wait for the server's answer, so none goes twice.
    sending: bool,
}

impl GroupForm {
    pub fn new(
        (group, me, friends): (Conversation, String, Vec<Friend>),
        on_change: impl Fn(GroupChange, &mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title = group.title.clone();
        let name = cx.new(|cx| InputState::new(window, cx).default_value(title));
        // Save waits for a new name, so it follows the typing.
        let _name_changes = cx.subscribe(&name, |_, _, _: &InputEvent, cx| cx.notify());
        Self {
            group,
            me,
            friends,
            name,
            _name_changes,
            on_change: Rc::new(on_change),
            problem: None,
            gone: false,
            refill_name: false,
            sending: false,
        }
    }

    /// Opens the group's modal over the window.
    pub fn open(
        (group, me, friends): (Conversation, String, Vec<Friend>),
        on_change: impl Fn(GroupChange, &mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let form = cx.new(|cx| Self::new((group, me, friends), on_change, window, cx));
        let shown = form.clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            // Themed as the app's panels, like New group.
            dialog
                .w(px(440.))
                .close_button(false)
                .bg(cx.theme().group_box)
                .rounded(cx.theme().radius)
                .overflow_hidden()
                .pt_0()
                .pb_0()
                .gap_0()
                .child(shown.clone())
        });
        form
    }

    /// The group the modal is about.
    pub fn group_id(&self) -> &str {
        &self.group.id
    }

    /// The group as the server now lists it; none once the player is out of it.
    pub fn follow(&mut self, group: Option<Conversation>, cx: &mut Context<Self>) {
        self.sending = false;
        match group {
            Some(group) => {
                // A rename shows in the field too, unless a new name is being typed there.
                let typed = self.name.read(cx).value().trim().to_owned();
                self.refill_name |= typed == self.group.title && group.title != self.group.title;
                self.group = group;
            }
            None => self.gone = true,
        }
        cx.notify();
    }

    /// The server refused a change: the modal says why.
    pub fn failed(&mut self, problem: String, cx: &mut Context<Self>) {
        self.problem = Some(problem);
        self.sending = false;
        cx.notify();
    }

    fn owner(&self) -> bool {
        self.group.owner_id.as_deref() == Some(self.me.as_str())
    }

    fn new_name(&self, cx: &App) -> Option<String> {
        let name = self.name.read(cx).value().trim().to_owned();
        (!name.is_empty() && name != self.group.title).then_some(name)
    }

    /// Passes the change on; the modal shows it once the server's list has it.
    fn change(&mut self, change: GroupChange, window: &mut Window, cx: &mut Context<Self>) {
        if self.sending {
            return;
        }
        self.sending = true;
        self.problem = None;
        if change == GroupChange::Leave {
            window.close_dialog(cx);
        }
        (self.on_change)(change, window, cx);
        cx.notify();
    }

    /// A member: their picture and name, Owner or You beside it, and Remove for the owner.
    fn member_row(&self, member: &UserRef, cx: &mut Context<Self>) -> Div {
        let (id, name) = (member.id.as_str(), member.username.as_str());
        let muted = cx.theme().muted_foreground;
        let tag = if self.group.owner_id.as_deref() == Some(id) {
            Some(tr!("chat.group.owner"))
        } else if id == self.me {
            Some(tr!("chat.group.you"))
        } else {
            None
        };
        let removable = self.owner() && id != self.me;
        let to_remove = id.to_owned();
        person_line((name, avatar_path(id, member.avatar.as_deref())), cx)
            .when_some(tag, |this, tag| {
                this.child(div().text_xs().text_color(muted).child(tag))
            })
            .when(removable, |this| {
                this.child(
                    Button::new(SharedString::from(format!("group-remove-{id}")))
                        .ghost()
                        .small()
                        .label(tr!("chat.group.remove"))
                        .disabled(self.sending)
                        .on_click(cx.listener(move |form, _, window, cx| {
                            form.change(GroupChange::Remove(to_remove.clone()), window, cx);
                        })),
                )
            })
    }

    /// A friend not in the group yet, with Add.
    fn friend_row(&self, friend: &Friend, cx: &mut Context<Self>) -> Div {
        let id = friend.id.clone();
        let picture = avatar_path(&friend.id, friend.avatar.as_deref());
        person_line((&friend.username, picture), cx).child(
            Button::new(SharedString::from(format!("group-add-{id}")))
                .outline()
                .small()
                .icon(IconName::UserPlus)
                .label(tr!("chat.group.add"))
                .disabled(self.sending)
                .on_click(cx.listener(move |form, _, window, cx| {
                    form.change(GroupChange::Add(id.clone()), window, cx);
                })),
        )
    }

    /// The owner's name field with Save; others see the name as it is.
    fn name_field(&self, cx: &mut Context<Self>) -> Div {
        let new_name = self.new_name(cx);
        h_flex()
            .gap_2()
            .child(div().flex_1().child(Input::new(&self.name)))
            .child(
                Button::new("group-rename")
                    .primary()
                    .label(tr!("chat.group.save"))
                    .disabled(new_name.is_none() || self.sending)
                    .on_click(cx.listener(move |form, _, window, cx| {
                        if let Some(name) = form.new_name(cx) {
                            form.change(GroupChange::Rename(name), window, cx);
                        }
                    })),
            )
    }
}

impl Render for GroupForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.refill_name {
            self.refill_name = false;
            let title = self.group.title.clone();
            self.name
                .update(cx, |input, cx| input.set_value(title, window, cx));
        }
        let muted = cx.theme().muted_foreground;
        let members: Vec<Div> = self
            .group
            .members
            .iter()
            .map(|member| self.member_row(member, cx))
            .collect();
        let addable: Vec<Div> = self
            .friends
            .iter()
            .filter(|friend| {
                !self
                    .group
                    .members
                    .iter()
                    .any(|member| member.id == friend.id)
            })
            .map(|friend| self.friend_row(friend, cx))
            .collect();
        let section = |title: String| div().text_sm().text_color(muted).child(title);
        if self.gone {
            return v_flex()
                .child(PanelHeader::new(MESSAGES_PICTURE, self.group.title.clone()))
                .child(
                    v_flex()
                        .gap_4()
                        .px_5()
                        .py_4()
                        .child(section(tr!("chat.group.gone")))
                        .child(close_button()),
                );
        }
        let danger = cx.theme().danger;
        let body = v_flex()
            .gap_4()
            .px_5()
            .py_4()
            .when(self.owner(), |this| this.child(self.name_field(cx)))
            .when_some(self.problem.clone(), |this, problem| {
                this.child(div().text_sm().text_color(danger).child(problem))
            })
            .child(
                v_flex()
                    .gap_1()
                    .child(section(tr!("chat.group.members", count = members.len())))
                    .child(
                        div()
                            .id("group-members")
                            .max_h(px(220.))
                            .overflow_y_scroll()
                            .child(v_flex().children(members)),
                    ),
            )
            .when(!addable.is_empty(), |this| {
                this.child(
                    v_flex()
                        .gap_1()
                        .child(section(tr!("chat.group.add_friends")))
                        .child(
                            div()
                                .id("group-addable")
                                .max_h(px(160.))
                                .overflow_y_scroll()
                                .child(v_flex().children(addable)),
                        ),
                )
            })
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Button::new("leave-group")
                            .danger()
                            .large()
                            .label(tr!("chat.group.leave"))
                            .disabled(self.sending)
                            .on_click(cx.listener(|form, _, window, cx| {
                                form.change(GroupChange::Leave, window, cx);
                            })),
                    )
                    .child(div().flex_1().child(close_button())),
            );
        v_flex()
            .child(PanelHeader::new(MESSAGES_PICTURE, self.group.title.clone()))
            .child(body)
    }
}

/// Closes the modal.
fn close_button() -> Button {
    Button::new("close-group")
        .ghost()
        .large()
        .w_full()
        .label(tr!("chat.group.close"))
        .on_click(|_, window, cx| window.close_dialog(cx))
}

/// A person's picture and name, filling the row, before what goes at its end.
fn person_line((name, picture): (&str, Option<String>), cx: &App) -> Div {
    h_flex()
        .gap_3()
        .px_2()
        .py_1p5()
        .child(avatar(name, picture.as_deref(), px(32.), cx))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .font_weight(FontWeight::SEMIBOLD)
                .child(name.to_owned()),
        )
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

    use gpui_kit::{TestAppContext, VisualTestContext};

    use super::GroupForm;
    use crate::{
        enums::chat::{ConversationKind, GroupChange},
        models::{
            chat::Conversation,
            social::{Friend, UserRef},
        },
    };

    fn person(name: &str) -> UserRef {
        UserRef {
            id: name.to_owned(),
            username: name.to_owned(),
            avatar: None,
        }
    }

    fn club() -> Conversation {
        Conversation {
            id: "g1".to_owned(),
            kind: ConversationKind::Group,
            title: "club".to_owned(),
            members: vec![person("ann"), person("bob")],
            owner_id: Some("ann".to_owned()),
            last_message: None,
            unread: 0,
        }
    }

    fn form<'a>(
        me: &str,
        cx: &'a mut TestAppContext,
    ) -> (
        gpui_kit::Entity<GroupForm>,
        Rc<RefCell<Vec<GroupChange>>>,
        &'a mut VisualTestContext,
    ) {
        cx.update(gpui_kit::init);
        let changes: Rc<RefCell<Vec<GroupChange>>> = Rc::default();
        let heard = Rc::clone(&changes);
        let cat = Friend {
            id: "cat".to_owned(),
            username: "cat".to_owned(),
            avatar: None,
            online: false,
            away_s: None,
            ratings: BTreeMap::new(),
        };
        let me = me.to_owned();
        let (form, cx) = cx.add_window_view(move |window, cx| {
            GroupForm::new(
                (club(), me, vec![cat]),
                move |change, _, _| heard.borrow_mut().push(change),
                window,
                cx,
            )
        });
        (form, changes, cx)
    }

    #[gpui_kit::test]
    fn changes_are_passed_on_and_the_modal_follows_the_server(cx: &mut TestAppContext) {
        let (form, changes, cx) = form("ann", cx);
        assert!(form.read_with(cx, |form, _| form.owner()));
        form.update_in(cx, |form, window, cx| {
            form.change(GroupChange::Remove("bob".to_owned()), window, cx);
        });
        assert_eq!(*changes.borrow(), [GroupChange::Remove("bob".to_owned())]);
        let mut listed = club();
        listed.members.retain(|member| member.id != "bob");
        form.update(cx, |form, cx| form.follow(Some(listed), cx));
        assert_eq!(form.read_with(cx, |form, _| form.group.members.len()), 1);
        form.update(cx, |form, cx| form.failed("Only friends".to_owned(), cx));
        assert!(form.read_with(cx, |form, _| form.problem.is_some()));
        form.update(cx, |form, cx| form.follow(None, cx));
        assert!(
            form.read_with(cx, |form, _| form.gone),
            "removed, only Close is left"
        );
    }

    #[gpui_kit::test]
    fn only_the_owner_is_offered_renaming_and_removing(cx: &mut TestAppContext) {
        let (form, _, cx) = form("bob", cx);
        assert!(!form.read_with(cx, |form, _| form.owner()));
    }
}
