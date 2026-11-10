use gpui_kit::{
    component::{ActiveTheme as _, WindowExt as _, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use crate::{
    clients::server::ServerClient,
    components::{account::StatsPanel, panel::StartButton, social::avatar},
    enums::social::Relation,
    i18n::tr,
    models::{account::Face, server::ServerError, social::Profile},
    services::social::{face_of, problem_text, relation_text},
    views::live::SessionSource,
};

/// Another player's profile, in a window over the page: who they are, their ratings and record.
pub struct ProfileView {
    client: ServerClient,
    session: SessionSource,
    user_id: String,
    /// Their name and picture, shown while the rest is on its way.
    face: Face,
    profile: Option<Profile>,
    problem: Option<String>,
    /// A friend request or an accept is on its way.
    asking: bool,
    _fetch: Option<Task<()>>,
}

impl ProfileView {
    /// Opens the profile of the player `user_id`, shown as `face` until it has loaded.
    pub fn open(
        (client, session): (ServerClient, SessionSource),
        (user_id, face): (String, Face),
        window: &mut Window,
        cx: &mut App,
    ) {
        let view = cx.new(|cx| Self::new((client, session), (user_id, face), cx));
        window.open_dialog(cx, move |dialog, _, cx| {
            // Themed as the app's panels, like the challenge window.
            dialog
                .w(px(980.))
                .bg(cx.theme().group_box)
                .rounded(cx.theme().radius)
                .child(view.clone())
        });
    }

    /// The profile, asked for at once.
    fn new(
        (client, session): (ServerClient, SessionSource),
        (user_id, face): (String, Face),
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self {
            client,
            session,
            user_id,
            face,
            profile: None,
            problem: None,
            asking: false,
            _fetch: None,
        };
        view.fetch(cx);
        view
    }

    fn token(&self, cx: &App) -> Option<String> {
        (self.session)(cx).map(|session| session.token)
    }

    fn fetch(&mut self, cx: &mut Context<Self>) {
        let Some(token) = self.token(cx) else {
            return;
        };
        let request = self.client.profile(&token, &self.user_id);
        self._fetch = Some(cx.spawn(async move |view, cx| {
            let result = request.await;
            // The window outlives the request unless it was closed.
            let _ = view.update(cx, |view, cx| view.fetched(result, cx));
        }));
    }

    fn fetched(&mut self, result: Result<Profile, ServerError>, cx: &mut Context<Self>) {
        match result {
            Ok(profile) => {
                self.face = face_of(&profile.id, &profile.username, profile.avatar.as_deref());
                self.profile = Some(profile);
            }
            Err(error) => self.problem = Some(problem_text(&error)),
        }
        cx.notify();
    }

    /// Asks them to be friends, or accepts their asking; the profile says where it stands after.
    fn befriend(&mut self, cx: &mut Context<Self>) {
        let (Some(token), Some(relation)) = (self.token(cx), self.relation()) else {
            return;
        };
        let accepting = match relation {
            Relation::RequestReceived => true,
            Relation::None => false,
            Relation::Friends | Relation::RequestSent | Relation::Blocked => return,
        };
        let (client, user_id) = (self.client.clone(), self.user_id.clone());
        self.asking = true;
        self.problem = None;
        cx.spawn(async move |view, cx| {
            let result = if accepting {
                client.accept_friend(&token, &user_id).await
            } else {
                client.request_friend(&token, &user_id).await
            };
            // The window outlives the request unless it was closed.
            let _ = view.update(cx, |view, cx| view.befriended(result, cx));
        })
        .detach();
        cx.notify();
    }

    fn befriended(&mut self, result: Result<(), ServerError>, cx: &mut Context<Self>) {
        self.asking = false;
        match result {
            // Fetched again: the server says whether that made them friends or a request.
            Ok(()) => self.fetch(cx),
            Err(error) => self.problem = Some(problem_text(&error)),
        }
        cx.notify();
    }

    /// Where the player stands with them, once loaded; none on their own profile.
    fn relation(&self) -> Option<Relation> {
        self.profile
            .as_ref()
            .filter(|profile| !profile.mine)
            .map(|profile| profile.relation)
    }

    /// Add friend or Accept, or where the two stand in words.
    fn relation_action(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let relation = self.relation()?;
        if let Some(text) = relation_text(relation) {
            return Some(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(text)
                    .into_any_element(),
            );
        }
        let label = if relation == Relation::RequestReceived {
            tr!("social.profile.accept_request")
        } else {
            tr!("social.actions.add_friend")
        };
        let view = cx.entity().downgrade();
        Some(
            StartButton::new("profile-befriend", label, move |_, cx| {
                // The window outlives its button unless it was closed.
                let _ = view.update(cx, |view, cx| view.befriend(cx));
            })
            .medium()
            .busy(self.asking)
            .into_any_element(),
        )
    }

    /// Their picture and name, when they joined, and what can be done about them.
    fn header(&self, cx: &mut Context<Self>) -> Div {
        let joined = self.profile.as_ref().map(|profile| {
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(tr!("social.profile.joined", date = profile.joined.as_str()))
        });
        let mine = self.profile.as_ref().is_some_and(|profile| profile.mine);
        h_flex()
            .w_full()
            .gap_4()
            .child(avatar(
                &self.face.name,
                self.face.picture.as_deref(),
                px(72.),
                cx,
            ))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .child(self.face.name.clone()),
                    )
                    .children(joined)
                    .when(mine, |this| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().success)
                                .child(tr!("social.profile.you")),
                        )
                    }),
            )
            .children(self.relation_action(cx))
    }
}

impl Render for ProfileView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stats = self.profile.as_ref().map(|profile| profile.stats.clone());
        let loading = stats.is_none() && self.problem.is_none();
        v_flex()
            .id("profile")
            .w_full()
            .max_h(px(760.))
            .overflow_y_scroll()
            .gap_5()
            .p_2()
            .child(self.header(cx))
            .when_some(self.problem.clone(), |this, problem| {
                this.child(div().text_sm().text_color(cx.theme().danger).child(problem))
            })
            .when(loading, |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr!("social.loading")),
                )
            })
            .children(stats.map(StatsPanel::new))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use axum::{
        Json, Router,
        http::StatusCode,
        routing::{get, post},
    };
    use gpui_kit::TestAppContext;
    use serde_json::{Value, json};

    use super::ProfileView;
    use crate::{
        clients::server::ServerClient,
        enums::social::Relation,
        models::account::Face,
        testing::{ann, wait_until},
        views::live::SessionSource,
    };

    /// Bob's profile, a stranger until Ann asks; then the request is waiting.
    fn server(asked: Arc<Mutex<bool>>) -> ServerClient {
        let stats = json!({
            "modes": [],
            "puzzles": {
                "rating": null, "provisional": false, "best": null, "attempted": 0, "solved": 0,
                "best_rush": { "three_minutes": null, "five_minutes": null, "survival": null },
                "battle_rating": null, "battle_provisional": false, "history": []
            }
        });
        let profile_asked = Arc::clone(&asked);
        ServerClient::serving(
            Router::new()
                .route(
                    "/api/users/bob/profile",
                    get(move || async move {
                        let relation = if *profile_asked.lock().expect("the flag") {
                            "request_sent"
                        } else {
                            "none"
                        };
                        Json(json!({
                            "id": "bob", "username": "bob", "joined": "2026-11-09",
                            "relation": relation, "mine": false, "stats": stats,
                        }))
                    }),
                )
                .route(
                    "/api/social/friends/requests",
                    post(move |Json(body): Json<Value>| async move {
                        assert_eq!(body["user_id"], "bob");
                        *asked.lock().expect("the flag") = true;
                        StatusCode::CREATED
                    }),
                ),
        )
    }

    #[gpui_kit::test]
    fn a_profile_loads_and_add_friend_asks_them(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let client = server(Arc::default());
        let session = ann(false);
        let (view, cx) = cx.add_window_view(move |_, cx| {
            let session: SessionSource = Rc::new(move |_| Some(session.clone()));
            let face = Face {
                name: "bob".to_owned(),
                picture: None,
            };
            ProfileView::new((client, session), ("bob".to_owned(), face), cx)
        });
        wait_until(cx, |cx| {
            view.read_with(cx, |view, _| view.profile.is_some())
        });
        assert_eq!(
            view.read_with(cx, |view, _| view.relation()),
            Some(Relation::None)
        );
        view.update(cx, |view, cx| view.befriend(cx));
        wait_until(cx, |cx| {
            view.read_with(cx, |view, _| view.relation() == Some(Relation::RequestSent))
        });
    }
}
