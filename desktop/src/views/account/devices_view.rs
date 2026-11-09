use gpui_kit::*;

use crate::{
    clients::server::ServerClient,
    components::account::DevicesPanel,
    models::{account::Device, server::ServerError},
    services::server::error_message,
};

/// The Account page's Devices: what is signed in to the account, and signing out the rest.
pub struct DevicesView {
    client: ServerClient,
    /// The signed-in token the devices were fetched with; none once signed out.
    token: Option<String>,
    devices: Vec<Device>,
    /// The others are being signed out.
    busy: bool,
    problem: Option<String>,
    _fetch: Option<Task<()>>,
    /// Signing out the others, on its way; signing out of this one drops it.
    _signing_out: Option<Task<()>>,
}

impl DevicesView {
    pub fn new(client: ServerClient) -> Self {
        Self {
            client,
            token: None,
            devices: Vec::new(),
            busy: false,
            problem: None,
            _fetch: None,
            _signing_out: None,
        }
    }

    /// The page came on screen: the devices are fetched again.
    pub fn open(&mut self, token: String, cx: &mut Context<Self>) {
        self.token = Some(token);
        self.fetch(cx);
    }

    /// Signed out: nothing of the last account is kept.
    pub fn clear(&mut self) {
        *self = Self::new(self.client.clone());
    }

    fn fetch(&mut self, cx: &mut Context<Self>) {
        let Some(token) = self.token.clone() else {
            return;
        };
        let request = self.client.devices(&token);
        self._fetch = Some(cx.spawn(async move |view, cx| {
            let result = request.await;
            // The view outlives the request unless the window is closing.
            let _ = view.update(cx, |view, cx| view.fetched(result, cx));
        }));
    }

    fn fetched(&mut self, result: Result<Vec<Device>, ServerError>, cx: &mut Context<Self>) {
        match result {
            Ok(devices) => {
                self.devices = devices;
                self.problem = None;
            }
            Err(error) => self.problem = Some(error_message(&error)),
        }
        cx.notify();
    }

    fn sign_out_others(&mut self, cx: &mut Context<Self>) {
        let Some(token) = self.token.clone().filter(|_| !self.busy) else {
            return;
        };
        self.busy = true;
        let request = self.client.sign_out_others(&token);
        self._signing_out = Some(cx.spawn(async move |view, cx| {
            let result = request.await;
            // The view outlives the request unless the window is closing.
            let _ = view.update(cx, |view, cx| view.signed_out_others(result, cx));
        }));
        cx.notify();
    }

    fn signed_out_others(&mut self, result: Result<(), ServerError>, cx: &mut Context<Self>) {
        self.busy = false;
        match result {
            Ok(()) => self.fetch(cx),
            Err(error) => self.problem = Some(error_message(&error)),
        }
        cx.notify();
    }
}

impl Render for DevicesView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().downgrade();
        DevicesPanel::new(
            self.devices.clone(),
            (self.busy, self.problem.clone()),
            move |_, cx| {
                // The view outlives its buttons unless the window is closing.
                let _ = view.update(cx, |view, cx| view.sign_out_others(cx));
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use axum::{Json, Router, http::StatusCode, routing::get};
    use gpui_kit::TestAppContext;
    use serde_json::json;

    use super::DevicesView;
    use crate::{clients::server::ServerClient, testing::wait_until};

    #[gpui_kit::test]
    fn the_other_devices_are_signed_out_and_the_list_fetched_again(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let signed_out = Arc::new(AtomicBool::new(false));
        let (asked, listed) = (Arc::clone(&signed_out), Arc::clone(&signed_out));
        let client = ServerClient::serving(
            Router::new()
                .route(
                    "/api/sessions",
                    get(move || async move {
                        let mut devices = vec![json!({
                            "id": "s1", "device": "Chess app on macOS", "current": true,
                            "signed_in_s": 60, "seen_s": 0
                        })];
                        if !listed.load(Ordering::SeqCst) {
                            devices.push(json!({
                                "id": "s2", "device": "Chess app on Linux", "current": false,
                                "signed_in_s": 86_400, "seen_s": 3600
                            }));
                        }
                        Json(devices)
                    }),
                )
                .route(
                    "/api/sessions/others",
                    axum::routing::delete(move || async move {
                        asked.store(true, Ordering::SeqCst);
                        StatusCode::NO_CONTENT
                    }),
                ),
        );
        let (view, cx) = cx.add_window_view(|_, _| DevicesView::new(client));
        view.update(cx, |view, cx| view.open("abc".to_owned(), cx));
        wait_until(cx, |cx| {
            view.read_with(cx, |view, _| view.devices.len() == 2)
        });
        view.update(cx, |view, cx| view.sign_out_others(cx));
        wait_until(cx, |cx| {
            view.read_with(cx, |view, _| view.devices.len() == 1)
        });
        assert!(signed_out.load(Ordering::SeqCst));
    }
}
