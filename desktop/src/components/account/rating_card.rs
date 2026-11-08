use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use crate::{
    components::panel::START_GREEN,
    services::account::{graph_steps, rating_change},
};

/// The graph's height, its line's light blue, and the red of a fall; a rise is the app's green.
const GRAPH_HEIGHT: Pixels = px(64.);
const LINE: u32 = 0x0090_d0f8;
const RISE: u32 = START_GREEN;
const FALL: u32 = 0x00e0_4f3c;

/// A rating as chess.com's stats show it: the mode's picture, its name and rating, how the
/// last result moved it, the rating over time under them, and a few lines about it.
#[derive(IntoElement)]
pub struct RatingCard {
    picture: Option<&'static str>,
    name: SharedString,
    rating: SharedString,
    history: Vec<i32>,
    lines: Vec<String>,
    /// Across the whole row: the lines beside the graph instead of under it.
    wide: bool,
}

impl RatingCard {
    pub fn new(
        (picture, name): (Option<&'static str>, impl Into<SharedString>),
        rating: impl Into<SharedString>,
        history: Vec<i32>,
    ) -> Self {
        Self {
            picture,
            name: name.into(),
            rating: rating.into(),
            history,
            lines: Vec::new(),
            wide: false,
        }
    }

    /// Across the whole row, its lines beside the graph.
    pub fn wide(mut self) -> Self {
        self.wide = true;
        self
    }

    /// A small line under the graph, or beside it on a wide card: the best or the record.
    pub fn line(mut self, line: impl Into<String>) -> Self {
        self.lines.push(line.into());
        self
    }
}

impl RenderOnce for RatingCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let head = h_flex()
            .gap_3()
            .when_some(self.picture, |this, path| {
                this.child(img(path).size_12().flex_shrink_0())
            })
            .child(
                v_flex()
                    .flex_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child(self.name),
                    )
                    .child(
                        div()
                            .text_3xl()
                            .font_weight(FontWeight::EXTRA_BOLD)
                            .child(self.rating),
                    ),
            )
            .children(change_badge(rating_change(&self.history)));
        let lines: Vec<Div> = self
            .lines
            .into_iter()
            .map(|line| div().text_xs().text_color(muted).child(line))
            .collect();
        let graph = rating_graph(&self.history, theme.secondary);
        let lines = v_flex().gap_0p5().children(lines);
        let card = v_flex()
            .flex_1()
            .min_w_0()
            .gap_3()
            .p_4()
            .rounded(px(8.))
            .bg(theme.muted)
            .child(head);
        if self.wide {
            return card.child(
                h_flex()
                    .items_start()
                    .gap_5()
                    .child(div().flex_1().min_w_0().child(graph))
                    .child(lines.w(px(280.)).flex_shrink_0()),
            );
        }
        card.child(graph).child(lines)
    }
}

/// "↑ 43" in green or "↓ 59" in red; nothing before a change, or for none.
fn change_badge(change: Option<i32>) -> Option<Div> {
    let change = change.filter(|change| *change != 0)?;
    let (icon, colour) = if change > 0 {
        (IconName::ArrowUp, RISE)
    } else {
        (IconName::ArrowDown, FALL)
    };
    Some(
        h_flex()
            .self_end()
            .gap_0p5()
            .text_color(rgb(colour))
            .font_weight(FontWeight::BOLD)
            .child(Icon::from(icon).size_4())
            .child(change.abs().to_string()),
    )
}

/// The rating over time: a light blue line in steps over a darker area.
fn rating_graph(history: &[i32], ground: Hsla) -> Canvas<()> {
    let steps = graph_steps(history);
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            window.paint_quad(fill(bounds, ground));
            paint_area(&steps, bounds, window);
            paint_line(&steps, bounds, window);
        },
    )
    .w_full()
    .h(GRAPH_HEIGHT)
}

fn at(bounds: Bounds<Pixels>, (across, down): (f32, f32)) -> Point<Pixels> {
    point(
        bounds.origin.x + bounds.size.width * across,
        bounds.origin.y + bounds.size.height * down,
    )
}

/// Under the line, a faint wash of its colour down to the bottom.
fn paint_area(steps: &[(f32, f32)], bounds: Bounds<Pixels>, window: &mut Window) {
    let mut path = PathBuilder::fill();
    path.move_to(at(bounds, (0., 1.)));
    for &spot in steps {
        path.line_to(at(bounds, spot));
    }
    path.line_to(at(bounds, (1., 1.)));
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, rgb(LINE).opacity(0.12));
    }
}

fn paint_line(steps: &[(f32, f32)], bounds: Bounds<Pixels>, window: &mut Window) {
    let mut path = PathBuilder::stroke(px(2.));
    let mut spots = steps.iter();
    let Some(&first) = spots.next() else {
        return;
    };
    path.move_to(at(bounds, first));
    for &spot in spots {
        path.line_to(at(bounds, spot));
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, rgb(LINE));
    }
}
