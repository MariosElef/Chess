use gpui_kit::{
    component::{label::Label, radio::RadioGroup, v_flex},
    *,
};

use super::appearance::Appearance;

type OnAppearance = Box<dyn Fn(Appearance, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SettingsPage {
    appearance: Appearance,
    on_appearance: OnAppearance,
}

impl SettingsPage {
    pub fn new(
        appearance: Appearance,
        on_appearance: impl Fn(Appearance, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            appearance,
            on_appearance: Box::new(on_appearance),
        }
    }
}

impl RenderOnce for SettingsPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let selected = Some(self.appearance.index());
        let on_appearance = self.on_appearance;

        v_flex()
            .size_full()
            .p_6()
            .gap_6()
            .child(Label::new("Settings").text_2xl())
            .child(
                v_flex()
                    .gap_2()
                    .child(Label::new("Appearance").text_lg())
                    .child(
                        RadioGroup::horizontal("appearance")
                            .children(Appearance::ALL.map(Appearance::label))
                            .selected_index(selected)
                            .on_click(move |index, window, cx| {
                                on_appearance(Appearance::ALL[*index], window, cx);
                            }),
                    ),
            )
    }
}
