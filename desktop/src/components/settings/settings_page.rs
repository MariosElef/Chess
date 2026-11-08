use std::rc::Rc;

use gpui_kit::{
    component::{ActiveTheme as _, h_flex, label::Label, radio::RadioGroup, v_flex},
    *,
};

use super::{
    board_previews::{piece_set_preview, theme_preview},
    option_tile::OptionTile,
};
use crate::{
    components::board::BoardStyle,
    enums::settings::{Appearance, BoardTheme, PieceSet},
    models::engine::{DEPTH_CHOICES, EngineSettings, HASH_CHOICES, depth_label, thread_choices},
};

type OnAppearance = Rc<dyn Fn(Appearance, &mut Window, &mut App)>;
type OnEngine = Rc<dyn Fn(EngineSettings, &mut Window, &mut App)>;
type OnBoardStyle = Rc<dyn Fn(BoardStyle, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SettingsPage {
    appearance: Appearance,
    board_style: BoardStyle,
    engine: EngineSettings,
    /// This computer's cores, which decide the thread choices.
    cores: usize,
    on_appearance: OnAppearance,
    on_board_style: OnBoardStyle,
    on_engine: OnEngine,
}

/// What the settings page shows: the current choices and where each change goes.
pub struct SettingsChoices {
    pub appearance: Appearance,
    pub board_style: BoardStyle,
    pub engine: EngineSettings,
    pub cores: usize,
}

impl SettingsPage {
    pub fn new(
        choices: SettingsChoices,
        on_appearance: impl Fn(Appearance, &mut Window, &mut App) + 'static,
        on_board_style: impl Fn(BoardStyle, &mut Window, &mut App) + 'static,
        on_engine: impl Fn(EngineSettings, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            appearance: choices.appearance,
            board_style: choices.board_style,
            engine: choices.engine,
            cores: choices.cores,
            on_appearance: Rc::new(on_appearance),
            on_board_style: Rc::new(on_board_style),
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
            .child(self.board_section(cx))
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

    /// The squares' colours and the pieces, for every board in the app, each choice shown as
    /// it looks.
    fn board_section(&self, cx: &App) -> Div {
        section("Board")
            .child(Label::new("Theme").text_sm())
            .child(self.theme_tiles(cx))
            .child(Label::new("Pieces").text_sm())
            .child(self.piece_set_tiles(cx))
    }

    fn theme_tiles(&self, cx: &App) -> Div {
        let style = self.board_style;
        h_flex()
            .flex_wrap()
            .gap_2()
            .children(BoardTheme::ALL.map(|theme| {
                let on_board_style = Rc::clone(&self.on_board_style);
                OptionTile::new(
                    "board-theme",
                    theme.index(),
                    theme.label(),
                    theme_preview(theme, cx),
                    Rc::new(move |window, cx| {
                        on_board_style(BoardStyle { theme, ..style }, window, cx);
                    }),
                )
                .selected(theme == style.theme)
            }))
    }

    fn piece_set_tiles(&self, cx: &App) -> Div {
        let style = self.board_style;
        h_flex()
            .flex_wrap()
            .gap_2()
            .children(PieceSet::ALL.map(|pieces| {
                let on_board_style = Rc::clone(&self.on_board_style);
                OptionTile::new(
                    "board-pieces",
                    pieces.index(),
                    pieces.label(),
                    piece_set_preview(pieces, style.theme, cx),
                    Rc::new(move |window, cx| {
                        on_board_style(BoardStyle { pieces, ..style }, window, cx);
                    }),
                )
                .selected(pieces == style.pieces)
            }))
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

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui_kit::{Context, IntoElement, Render, TestAppContext, Window};

    use super::{SettingsChoices, SettingsPage};
    use crate::{
        components::board::BoardStyle,
        enums::settings::{Appearance, BoardTheme, PieceSet},
        models::engine::EngineSettings,
        testing::click,
    };

    /// Draws the settings page and keeps the last board style it was asked for.
    struct Harness {
        style: Rc<Cell<BoardStyle>>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let chosen = Rc::clone(&self.style);
            SettingsPage::new(
                SettingsChoices {
                    appearance: Appearance::System,
                    board_style: self.style.get(),
                    engine: EngineSettings::for_cores(4),
                    cores: 4,
                },
                |_, _, _| {},
                move |style, _, _| chosen.set(style),
                |_, _, _| {},
            )
        }
    }

    #[gpui_kit::test]
    fn clicking_a_tile_chooses_its_theme_or_pieces(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let style = Rc::new(Cell::new(BoardStyle::default()));
        let harness_style = Rc::clone(&style);
        let (_, cx) = cx.add_window_view(|_, _| Harness {
            style: harness_style,
        });
        click(cx, "board-theme-3");
        assert_eq!(style.get().theme, BoardTheme::Green);
        click(cx, "board-pieces-1");
        assert_eq!(
            style.get(),
            BoardStyle {
                theme: BoardTheme::Green,
                pieces: PieceSet::Modern,
            },
            "choosing pieces keeps the theme"
        );
    }
}
