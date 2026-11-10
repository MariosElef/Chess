use std::rc::Rc;

use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    *,
};

use super::{
    board_previews::{piece_set_preview, theme_preview},
    option_tile::OptionTile,
    settings_pictures::{
        APPEARANCE_PICTURE, BOARD_PICTURE, DARK_PICTURE, LANGUAGE_PICTURE, LIGHT_PICTURE,
        SOUNDS_PICTURE, STOCKFISH_PICTURE, SYSTEM_PICTURE,
    },
};
use crate::{
    components::{board::BoardStyle, panel::ChoiceTile, shell::SETTINGS_PICTURE},
    enums::settings::{Appearance, BoardTheme, Language, PieceSet, SoundSet},
    i18n::tr,
    models::engine::{DEPTH_CHOICES, EngineSettings, HASH_CHOICES, depth_label, thread_choices},
};

type OnLanguage = Rc<dyn Fn(Language, &mut Window, &mut App)>;
type OnAppearance = Rc<dyn Fn(Appearance, &mut Window, &mut App)>;
type OnEngine = Rc<dyn Fn(EngineSettings, &mut Window, &mut App)>;
type OnBoardStyle = Rc<dyn Fn(BoardStyle, &mut Window, &mut App)>;
type OnSoundSet = Rc<dyn Fn(SoundSet, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SettingsPage {
    language: Language,
    appearance: Appearance,
    board_style: BoardStyle,
    sound_set: SoundSet,
    engine: EngineSettings,
    /// This computer's cores, which decide the thread choices.
    cores: usize,
    on_language: OnLanguage,
    on_appearance: OnAppearance,
    on_board_style: OnBoardStyle,
    on_sound_set: OnSoundSet,
    on_engine: OnEngine,
}

/// What the settings page shows: the current choices and where each change goes.
pub struct SettingsChoices {
    pub language: Language,
    pub appearance: Appearance,
    pub board_style: BoardStyle,
    pub sound_set: SoundSet,
    pub engine: EngineSettings,
    pub cores: usize,
}

impl SettingsPage {
    pub fn new(
        choices: SettingsChoices,
        on_language: impl Fn(Language, &mut Window, &mut App) + 'static,
        on_appearance: impl Fn(Appearance, &mut Window, &mut App) + 'static,
        on_board_style: impl Fn(BoardStyle, &mut Window, &mut App) + 'static,
        on_sound_set: impl Fn(SoundSet, &mut Window, &mut App) + 'static,
        on_engine: impl Fn(EngineSettings, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            language: choices.language,
            appearance: choices.appearance,
            board_style: choices.board_style,
            sound_set: choices.sound_set,
            engine: choices.engine,
            cores: choices.cores,
            on_language: Rc::new(on_language),
            on_appearance: Rc::new(on_appearance),
            on_board_style: Rc::new(on_board_style),
            on_sound_set: Rc::new(on_sound_set),
            on_engine: Rc::new(on_engine),
        }
    }
}

impl RenderOnce for SettingsPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let column = v_flex()
            .w_full()
            .max_w(px(760.))
            .gap_5()
            .child(
                h_flex()
                    .gap_3()
                    .child(img(SETTINGS_PICTURE).size_10())
                    .child(
                        div()
                            .text_3xl()
                            .font_weight(FontWeight::BOLD)
                            .child(tr!("settings.title")),
                    ),
            )
            .child(self.language_section(cx))
            .child(self.appearance_section(cx))
            .child(self.board_section(cx))
            .child(self.sound_section(cx))
            .child(self.engine_section(cx));
        div()
            .id("settings-page")
            .size_full()
            .overflow_y_scroll()
            .child(h_flex().w_full().justify_center().p_6().child(column))
    }
}

impl SettingsPage {
    /// English and Greek, each named in its own language.
    fn language_section(&self, cx: &App) -> Div {
        let tiles: Vec<Div> = Language::ALL
            .into_iter()
            .map(|language| {
                let on_language = Rc::clone(&self.on_language);
                div().flex_1().child(
                    ChoiceTile::new(
                        format!("language-{}", language.index()),
                        language.label(),
                        language == self.language,
                        move |window, cx| on_language(language, window, cx),
                    )
                    .picture(LANGUAGE_PICTURE)
                    .height(px(48.)),
                )
            })
            .collect();
        card(
            (LANGUAGE_PICTURE, tr!("settings.language.title")),
            tr!("settings.language.about"),
            h_flex().gap_3().children(tiles),
            cx,
        )
    }

    /// System, Light and Dark, as tiles with their icons.
    fn appearance_section(&self, cx: &App) -> Div {
        let tiles: Vec<Div> = Appearance::ALL
            .into_iter()
            .map(|appearance| {
                let on_appearance = Rc::clone(&self.on_appearance);
                let tile = ChoiceTile::new(
                    format!("appearance-{}", appearance.index()),
                    appearance.label(),
                    appearance == self.appearance,
                    move |window, cx| on_appearance(appearance, window, cx),
                )
                .picture(appearance_picture(appearance))
                .height(px(48.));
                // An equal share of the row each, not the whole row each.
                div().flex_1().child(tile)
            })
            .collect();
        card(
            (APPEARANCE_PICTURE, tr!("settings.appearance.title")),
            tr!("settings.appearance.about"),
            h_flex().gap_3().children(tiles),
            cx,
        )
    }

    /// The squares' colours and the pieces for every board, each choice shown as it looks.
    fn board_section(&self, cx: &App) -> Div {
        card(
            (BOARD_PICTURE, tr!("settings.board.title")),
            tr!("settings.board.about"),
            v_flex()
                .gap_4()
                .child(labelled(
                    tr!("settings.board.theme"),
                    self.theme_tiles(cx),
                    cx,
                ))
                .child(labelled(
                    tr!("settings.board.pieces"),
                    self.piece_set_tiles(cx),
                    cx,
                )),
            cx,
        )
    }

    fn theme_tiles(&self, cx: &App) -> Div {
        let style = self.board_style;
        h_flex()
            .flex_wrap()
            .justify_center()
            .gap_3()
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
            .justify_center()
            .gap_3()
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

    /// The app's sounds: the standard set or Lichess's lisp; picking one plays it.
    fn sound_section(&self, cx: &App) -> Div {
        let tiles: Vec<Div> = SoundSet::ALL
            .into_iter()
            .map(|set| {
                let on_sound_set = Rc::clone(&self.on_sound_set);
                div().flex_1().child(
                    ChoiceTile::new(
                        format!("sound-set-{}", set.index()),
                        set.label(),
                        set == self.sound_set,
                        move |window, cx| on_sound_set(set, window, cx),
                    )
                    .picture(SOUNDS_PICTURE)
                    .height(px(48.)),
                )
            })
            .collect();
        card(
            (SOUNDS_PICTURE, tr!("settings.sounds.title")),
            tr!("settings.sounds.about"),
            h_flex().gap_3().children(tiles),
            cx,
        )
    }

    /// Stockfish's threads, memory and depth; the thread choices come from this device's cores.
    fn engine_section(&self, cx: &App) -> Div {
        let threads = thread_choices(self.cores);
        let engine = self.engine;
        card(
            (STOCKFISH_PICTURE, "Stockfish".to_owned()),
            tr!("settings.stockfish.about"),
            v_flex()
                .gap_4()
                .child(labelled(
                    tr!("settings.stockfish.threads"),
                    self.engine_tiles(
                        ("engine-threads", &threads, engine.threads),
                        |threads| threads.to_string(),
                        move |threads| EngineSettings { threads, ..engine },
                    ),
                    cx,
                ))
                .child(labelled(
                    tr!("settings.stockfish.memory"),
                    self.engine_tiles(
                        ("engine-hash", &HASH_CHOICES, engine.hash_mb),
                        |hash_mb| hash_mb.to_string(),
                        move |hash_mb| EngineSettings { hash_mb, ..engine },
                    ),
                    cx,
                ))
                .child(labelled(
                    tr!("settings.stockfish.depth"),
                    self.engine_tiles(
                        ("engine-depth", &DEPTH_CHOICES, engine.depth),
                        depth_label,
                        move |depth| EngineSettings { depth, ..engine },
                    ),
                    cx,
                )),
            cx,
        )
    }

    /// A row of tiles, one per choice, named by `label`, the current one ringed.
    fn engine_tiles<T: Copy + PartialEq + 'static>(
        &self,
        (group, choices, current): (&'static str, &[T], T),
        label: impl Fn(T) -> String,
        settings: impl Fn(T) -> EngineSettings + 'static,
    ) -> Div {
        let settings = Rc::new(settings);
        let tiles: Vec<Div> = choices
            .iter()
            .enumerate()
            .map(|(index, &choice)| {
                let (on_engine, settings) = (Rc::clone(&self.on_engine), Rc::clone(&settings));
                div().flex_1().child(
                    ChoiceTile::new(
                        format!("{group}-{index}"),
                        label(choice),
                        choice == current,
                        move |window, cx| on_engine(settings(choice), window, cx),
                    )
                    .height(px(36.)),
                )
            })
            .collect();
        h_flex().gap_2().children(tiles)
    }
}

/// A settings card, as the app's panels: a band with its icon and title, a line about it, and
/// its choices under them.
fn card(
    (picture, title): (&'static str, String),
    about: String,
    body: impl IntoElement,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    v_flex()
        .w_full()
        .rounded(px(8.))
        .overflow_hidden()
        .bg(theme.group_box)
        .child(
            h_flex()
                .gap_2()
                .px_5()
                .py_3()
                .bg(theme.muted)
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .child(img(picture).size_5())
                .child(title),
        )
        .child(
            v_flex()
                .gap_4()
                .px_5()
                .py_4()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(about),
                )
                .child(body),
        )
}

/// A small heading over a row of choices.
fn labelled(label: String, choices: impl IntoElement, cx: &App) -> Div {
    v_flex()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
        .child(choices)
}

fn appearance_picture(appearance: Appearance) -> &'static str {
    match appearance {
        Appearance::System => SYSTEM_PICTURE,
        Appearance::Light => LIGHT_PICTURE,
        Appearance::Dark => DARK_PICTURE,
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui_kit::{Context, IntoElement, Render, TestAppContext, Window};

    use super::{SettingsChoices, SettingsPage};
    use crate::{
        components::board::BoardStyle,
        enums::settings::{Appearance, BoardTheme, Language, PieceSet, SoundSet},
        models::engine::EngineSettings,
        testing::click,
    };

    /// Draws the settings page and keeps the last board style it was asked for.
    struct Harness {
        style: Rc<Cell<BoardStyle>>,
        appearance: Rc<Cell<Appearance>>,
        engine: Rc<Cell<EngineSettings>>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let chosen = Rc::clone(&self.style);
            let (appearance, engine) = (Rc::clone(&self.appearance), Rc::clone(&self.engine));
            SettingsPage::new(
                SettingsChoices {
                    language: Language::English,
                    appearance: self.appearance.get(),
                    board_style: self.style.get(),
                    sound_set: SoundSet::Standard,
                    engine: self.engine.get(),
                    cores: 4,
                },
                |_, _, _| {},
                move |picked, _, _| appearance.set(picked),
                move |style, _, _| chosen.set(style),
                |_, _, _| {},
                move |settings, _, _| engine.set(settings),
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
            appearance: Rc::new(Cell::new(Appearance::System)),
            engine: Rc::new(Cell::new(EngineSettings::for_cores(4))),
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

    #[gpui_kit::test]
    fn appearance_and_stockfish_are_chosen_by_their_tiles(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let appearance = Rc::new(Cell::new(Appearance::System));
        let engine = Rc::new(Cell::new(EngineSettings::for_cores(4)));
        let (picked, settings) = (Rc::clone(&appearance), Rc::clone(&engine));
        let (_, cx) = cx.add_window_view(|_, _| Harness {
            style: Rc::new(Cell::new(BoardStyle::default())),
            appearance: picked,
            engine: settings,
        });
        click(cx, "appearance-2");
        assert_eq!(appearance.get(), Appearance::Dark);
        let before = engine.get();
        click(cx, "engine-hash-2");
        assert_eq!(engine.get().hash_mb, 256);
        assert_eq!(
            engine.get().threads,
            before.threads,
            "the other settings stay"
        );
    }
}
