use gpui_kit::{
    App, Hsla, Window,
    component::{Theme, ThemeColor, ThemeMode},
    rgb,
};

use crate::enums::settings::Appearance;

/// Puts the window in the chosen appearance; System follows the Mac's own setting.
pub fn apply_appearance(appearance: Appearance, window: &mut Window, cx: &mut App) {
    match appearance {
        Appearance::System => Theme::sync_system_appearance(Some(window), cx),
        Appearance::Light => Theme::change(ThemeMode::Light, Some(window), cx),
        Appearance::Dark => Theme::change(ThemeMode::Dark, Some(window), cx),
    }
    let theme = Theme::global_mut(cx);
    if theme.is_dark() {
        paint_dark(&mut theme.colors);
    } else {
        paint_light(&mut theme.colors);
    }
    window.refresh();
}

/// mc2's ink scale: every grey in the product.
const INK_50: u32 = 0x00fa_fafa;
const INK_100: u32 = 0x00f5_f5f5;
const INK_200: u32 = 0x00ed_eded;
const INK_300: u32 = 0x00d1_d1d1;
const INK_400: u32 = 0x009a_9a9a;
const INK_500: u32 = 0x005a_5a5a;
const INK_700: u32 = 0x0030_3030;
const INK_800: u32 = 0x0022_2222;
const INK_900: u32 = 0x001b_1b1b;
const INK_950: u32 = 0x0011_1111;
const WHITE: u32 = 0x00ff_ffff;

fn ink(hex: u32) -> Hsla {
    rgb(hex).into()
}

/// Dark: the page is 950, a card 900, the chrome around them 800, a border 700.
fn paint_dark(colors: &mut ThemeColor) {
    colors.background = ink(INK_950);
    colors.foreground = ink(INK_200);
    colors.group_box = ink(INK_900);
    colors.popover = ink(INK_800);
    colors.popover_foreground = ink(INK_200);
    colors.muted = ink(INK_800);
    colors.muted_foreground = ink(INK_400);
    colors.accent = ink(INK_700);
    colors.accent_foreground = ink(WHITE);
    colors.secondary = ink(INK_800);
    colors.secondary_hover = ink(INK_700);
    colors.secondary_active = ink(INK_700);
    colors.border = ink(INK_700);
    colors.input = ink(INK_500);
    colors.list = ink(INK_950);
    colors.list_hover = ink(INK_800);
    colors.sidebar = ink(INK_800);
    colors.sidebar_foreground = ink(INK_50);
    colors.sidebar_accent = ink(INK_700);
    colors.sidebar_accent_foreground = ink(WHITE);
    colors.sidebar_border = ink(INK_800);
    colors.title_bar = ink(INK_800);
    colors.title_bar_border = ink(INK_800);
    colors.window_border = ink(INK_700);
}

/// Light: the same ramps read from the other end; the chrome is darker than the page.
fn paint_light(colors: &mut ThemeColor) {
    colors.background = ink(INK_50);
    colors.foreground = ink(INK_950);
    colors.group_box = ink(WHITE);
    colors.popover = ink(WHITE);
    colors.popover_foreground = ink(INK_950);
    colors.muted = ink(INK_100);
    colors.muted_foreground = ink(INK_500);
    colors.accent = ink(INK_100);
    colors.accent_foreground = ink(INK_950);
    colors.secondary = ink(INK_100);
    colors.secondary_hover = ink(INK_200);
    colors.secondary_active = ink(INK_200);
    colors.border = ink(INK_200);
    colors.input = ink(INK_300);
    colors.list = ink(INK_50);
    colors.list_hover = ink(INK_100);
    colors.sidebar = ink(INK_200);
    colors.sidebar_foreground = ink(INK_950);
    colors.sidebar_accent = ink(WHITE);
    colors.sidebar_accent_foreground = ink(INK_950);
    colors.sidebar_border = ink(INK_200);
    colors.title_bar = ink(INK_200);
    colors.title_bar_border = ink(INK_200);
    colors.window_border = ink(INK_300);
}
