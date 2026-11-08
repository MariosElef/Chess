use std::rc::Rc;

use gpui_kit::{
    component::{ActiveTheme as _, label::Label, radio::RadioGroup, v_flex},
    *,
};

use crate::{
    enums::settings::Appearance,
    models::engine::{DEPTH_CHOICES, EngineSettings, HASH_CHOICES, depth_label, thread_choices},
};

type OnAppearance = Rc<dyn Fn(Appearance, &mut Window, &mut App)>;
type OnEngine = Rc<dyn Fn(EngineSettings, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SettingsPage {
    appearance: Appearance,
    engine: EngineSettings,
    /// This computer's cores, which decide the thread choices.
    cores: usize,
    on_appearance: OnAppearance,
    on_engine: OnEngine,
}

impl SettingsPage {
    pub fn new(
        appearance: Appearance,
        engine: EngineSettings,
        cores: usize,
        on_appearance: impl Fn(Appearance, &mut Window, &mut App) + 'static,
        on_engine: impl Fn(EngineSettings, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            appearance,
            engine,
            cores,
            on_appearance: Rc::new(on_appearance),
            on_engine: Rc::new(on_engine),
        }
    }
}

impl RenderOnce for SettingsPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_6()
            .gap_6()
            .child(Label::new("Settings").text_2xl())
            .child(self.appearance_section())
            .child(self.engine_section(cx))
    }
}

impl SettingsPage {
    fn appearance_section(&self) -> Div {
        let on_appearance = Rc::clone(&self.on_appearance);
        section("Appearance").child(
            RadioGroup::horizontal("appearance")
                .children(Appearance::ALL.map(Appearance::label))
                .selected_index(Some(self.appearance.index()))
                .on_click(move |index, window, cx| {
                    on_appearance(Appearance::ALL[*index], window, cx);
                }),
        )
    }

    /// Stockfish's threads, memory and depth; the thread choices come from this device's cores.
    fn engine_section(&self, cx: &App) -> Div {
        section("Stockfish")
            .child(hint(
                "Fewer threads and a depth limit keep the computer quiet; more search deeper.",
                cx,
            ))
            .child(Label::new("Threads").text_sm())
            .child(self.threads_choice())
            .child(Label::new("Memory (MB)").text_sm())
            .child(self.hash_choice())
            .child(Label::new("Search depth").text_sm())
            .child(self.depth_choice())
    }

    fn threads_choice(&self) -> RadioGroup {
        let choices = thread_choices(self.cores);
        let selected = choices
            .iter()
            .position(|&threads| threads == self.engine.threads);
        let labels: Vec<String> = choices.iter().map(ToString::to_string).collect();
        let (engine, on_engine) = (self.engine, Rc::clone(&self.on_engine));
        RadioGroup::horizontal("engine-threads")
            .children(labels)
            .selected_index(selected)
            .on_click(move |index, window, cx| {
                on_engine(
                    EngineSettings {
                        threads: choices[*index],
                        ..engine
                    },
                    window,
                    cx,
                );
            })
    }

    fn hash_choice(&self) -> RadioGroup {
        let selected = HASH_CHOICES
            .iter()
            .position(|&mb| mb == self.engine.hash_mb);
        let (engine, on_engine) = (self.engine, Rc::clone(&self.on_engine));
        RadioGroup::horizontal("engine-hash")
            .children(HASH_CHOICES.map(|mb| mb.to_string()))
            .selected_index(selected)
            .on_click(move |index, window, cx| {
                on_engine(
                    EngineSettings {
                        hash_mb: HASH_CHOICES[*index],
                        ..engine
                    },
                    window,
                    cx,
                );
            })
    }

    fn depth_choice(&self) -> RadioGroup {
        let selected = DEPTH_CHOICES
            .iter()
            .position(|&depth| depth == self.engine.depth);
        let (engine, on_engine) = (self.engine, Rc::clone(&self.on_engine));
        RadioGroup::horizontal("engine-depth")
            .children(DEPTH_CHOICES.map(depth_label))
            .selected_index(selected)
            .on_click(move |index, window, cx| {
                on_engine(
                    EngineSettings {
                        depth: DEPTH_CHOICES[*index],
                        ..engine
                    },
                    window,
                    cx,
                );
            })
    }
}

fn section(title: &'static str) -> Div {
    v_flex().gap_2().child(Label::new(title).text_lg())
}

fn hint(text: &'static str, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text)
}
