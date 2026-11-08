use chess_domain::Square;

/// The player's own drawings on the board: red squares and arrows, made with the right button.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Annotations {
    marks: Vec<Square>,
    arrows: Vec<(Square, Square)>,
}

impl Annotations {
    pub fn marks(&self) -> &[Square] {
        &self.marks
    }

    pub fn arrows(&self) -> &[(Square, Square)] {
        &self.arrows
    }

    /// A right-drag from `start` to `end`: the same square toggles a mark, two toggle an arrow.
    pub fn release(&mut self, start: Square, end: Square) {
        if start == end {
            toggle(&mut self.marks, start);
        } else {
            toggle(&mut self.arrows, (start, end));
        }
    }

    pub fn clear(&mut self) {
        self.marks.clear();
        self.arrows.clear();
    }
}

/// Adds `item`, or removes it if it is already there.
fn toggle<T: PartialEq>(items: &mut Vec<T>, item: T) {
    if let Some(index) = items.iter().position(|existing| *existing == item) {
        items.remove(index);
    } else {
        items.push(item);
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::Square;

    use super::Annotations;

    fn sq(name: &str) -> Square {
        name.parse().expect("a square")
    }

    #[test]
    fn a_right_click_marks_a_square_and_a_second_unmarks_it() {
        let mut annotations = Annotations::default();
        annotations.release(sq("e4"), sq("e4"));
        assert_eq!(annotations.marks(), [sq("e4")]);
        annotations.release(sq("e4"), sq("e4"));
        assert!(annotations.marks().is_empty());
    }

    #[test]
    fn a_right_drag_draws_an_arrow_and_redrawing_it_removes_it() {
        let mut annotations = Annotations::default();
        annotations.release(sq("g1"), sq("f3"));
        assert_eq!(annotations.arrows(), [(sq("g1"), sq("f3"))]);
        annotations.release(sq("g1"), sq("f3"));
        assert!(annotations.arrows().is_empty());
    }

    #[test]
    fn opposite_arrows_are_different_arrows() {
        let mut annotations = Annotations::default();
        annotations.release(sq("e2"), sq("e4"));
        annotations.release(sq("e4"), sq("e2"));
        assert_eq!(annotations.arrows().len(), 2);
    }

    #[test]
    fn clearing_removes_everything() {
        let mut annotations = Annotations::default();
        annotations.release(sq("e4"), sq("e4"));
        annotations.release(sq("e2"), sq("e4"));
        annotations.clear();
        assert_eq!(annotations, Annotations::default());
    }
}
