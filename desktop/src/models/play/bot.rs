use crate::{enums::play::BotCharacter, i18n::tr};

/// A computer opponent: who it is, how strong it plays, and its picture, if it has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bot {
    pub name: &'static str,
    pub rating: u32,
    /// The picture's asset path.
    pub picture: Option<&'static str>,
    /// How it talks and acts; the custom bot has no character.
    pub character: Option<BotCharacter>,
}

impl Bot {
    /// Its name as the player reads it: the named bots keep theirs, the custom one is called
    /// "Custom" in the language being shown.
    pub fn shown_name(&self) -> String {
        if self.character.is_some() {
            self.name.to_owned()
        } else {
            tr!("play.bot.custom")
        }
    }
}
