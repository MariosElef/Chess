use std::time::Duration;

use gpui_kit::{component::ActiveTheme as _, prelude::FluentBuilder as _, *};

/// Below this, the clock shows tenths of a second and turns red.
const LOW_TIME: Duration = Duration::from_secs(10);

/// One player's time left; lit while it is running.
#[derive(IntoElement)]
pub struct ClockFace {
    remaining: Duration,
    running: bool,
}

impl ClockFace {
    pub fn new(remaining: Duration, running: bool) -> Self {
        Self { remaining, running }
    }
}

impl RenderOnce for ClockFace {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let low = self.remaining < LOW_TIME;
        div()
            .min_w_24()
            .px_3()
            .py_1()
            .rounded(theme.radius)
            .text_right()
            .text_xl()
            .font_family(theme.mono_font_family.clone())
            .bg(theme.muted)
            .text_color(theme.muted_foreground)
            .when(self.running, |this| {
                this.bg(theme.primary).text_color(theme.primary_foreground)
            })
            .when(low, |this| {
                this.bg(theme.danger).text_color(theme.danger_foreground)
            })
            .child(format_clock(self.remaining))
    }
}

/// `1:05:00` over an hour, `4:59` normally, `0:09.4` in the last ten seconds.
pub fn format_clock(time: Duration) -> String {
    let total_seconds = time.as_secs();
    let (hours, minutes, seconds) = (
        total_seconds / 3600,
        total_seconds / 60 % 60,
        total_seconds % 60,
    );
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else if time < LOW_TIME {
        format!("0:{seconds:02}.{}", time.subsec_millis() / 100)
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::format_clock;

    #[test]
    fn minutes_and_seconds() {
        assert_eq!(format_clock(Duration::from_secs(299)), "4:59");
        assert_eq!(format_clock(Duration::from_secs(600)), "10:00");
    }

    #[test]
    fn hours_when_needed() {
        assert_eq!(format_clock(Duration::from_secs(3900)), "1:05:00");
    }

    #[test]
    fn tenths_in_the_last_ten_seconds() {
        assert_eq!(format_clock(Duration::from_millis(9_450)), "0:09.4");
        assert_eq!(format_clock(Duration::ZERO), "0:00.0");
    }
}
