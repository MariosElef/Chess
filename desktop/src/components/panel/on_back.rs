use std::rc::Rc;

use gpui_kit::*;

/// What a panel's back arrow does: back to the menu it was opened from.
pub type OnBack = Rc<dyn Fn(&mut Window, &mut App)>;
