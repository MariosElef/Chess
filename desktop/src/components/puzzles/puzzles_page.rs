use gpui_kit::{
    component::{label::Label, v_flex},
    *,
};

#[derive(IntoElement)]
pub struct PuzzlesPage;

impl RenderOnce for PuzzlesPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_6()
            .child(Label::new("Puzzles").text_2xl())
    }
}
