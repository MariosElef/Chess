use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, Sizable as _, button::Button, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use crate::{models::account::Device, services::time::ago_text};

type OnClick = Box<dyn Fn(&mut Window, &mut App)>;

/// The devices signed in to the account, this one first, with a way to sign out the rest.
#[derive(IntoElement)]
pub struct DevicesPanel {
    devices: Vec<Device>,
    busy: bool,
    problem: Option<String>,
    on_sign_out_others: OnClick,
}

impl DevicesPanel {
    pub fn new(
        devices: Vec<Device>,
        (busy, problem): (bool, Option<String>),
        on_sign_out_others: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            devices,
            busy,
            problem,
            on_sign_out_others: Box::new(on_sign_out_others),
        }
    }
}

impl RenderOnce for DevicesPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let others = self.devices.iter().any(|device| !device.current);
        let mut devices = self.devices;
        // This device first, then the rest as they were last used.
        devices.sort_by_key(|device| !device.current);
        let rows: Vec<Div> = devices
            .iter()
            .map(|device| device_row(device, cx))
            .collect();
        let on_sign_out_others = self.on_sign_out_others;
        v_flex()
            .w_full()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .child("Devices"),
                    )
                    .when(others, |this| {
                        this.child(
                            Button::new("sign-out-others")
                                .outline()
                                .small()
                                .icon(IconName::LogOut)
                                .label("Sign out everywhere else")
                                .loading(self.busy)
                                .on_click(move |_, window, cx| on_sign_out_others(window, cx)),
                        )
                    }),
            )
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .p_2()
                    .rounded(px(8.))
                    .bg(theme.muted)
                    .when_some(self.problem, |this, problem| {
                        this.child(
                            div()
                                .p_2()
                                .text_sm()
                                .text_color(theme.danger)
                                .child(problem),
                        )
                    })
                    .children(rows),
            )
    }
}

/// A device: what it is, when it signed in, and This device or when it was last used.
fn device_row(device: &Device, cx: &App) -> Div {
    let theme = cx.theme();
    let (muted, green) = (theme.muted_foreground, theme.success);
    let used = if device.current {
        "This device".to_owned()
    } else {
        format!("Last used {}", ago_text(device.seen_s))
    };
    h_flex()
        .gap_3()
        .px_3()
        .py_2()
        .child(
            Icon::from(IconName::Monitor)
                .size_5()
                .text_color(muted)
                .flex_shrink_0(),
        )
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(device.device.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child(format!("Signed in {}", ago_text(device.signed_in_s))),
                ),
        )
        .child(
            div()
                .text_sm()
                .when(device.current, |this| this.text_color(green))
                .when(!device.current, |this| this.text_color(muted))
                .child(used),
        )
}
