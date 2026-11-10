mod bot_characters;
mod bots;
mod clock_warning;
mod countdown;
mod game_history;
mod game_over;
mod online_game;
mod side_label;
mod speed_words;
mod status;
mod time_controls;
mod time_left;
mod whole_seconds;

pub use bot_characters::{
    deal_answer, deal_offer, greeting, moment_of, move_line, quit_line, rage_quits, think_time_for,
};
pub use bots::{
    BOTS, BotStyle, CUSTOM_STRENGTH, bot_options, bot_style, custom_bot, pick_bot_move,
    player_side, think_time,
};
pub use clock_warning::ClockWarning;
pub use countdown::{COUNTDOWN_TICK, Countdown};
pub use game_history::{game_line, opened_game, result_word};
pub use game_over::{outcome_reason, result_title};
pub use online_game::{
    can_abort, challenge_terms, clock_left, friend_challenges, has_ended, my_side, outcome_of,
    rating_line, replay, result_line,
};
pub use side_label::side_label;
pub use speed_words::{category_name, control_label};
pub use status::status_text;
pub use time_controls::{DEFAULT as DEFAULT_TIME_CONTROL, presets_in};
pub use time_left::time_left;
pub use whole_seconds::whole_seconds;
