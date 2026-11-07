/// One of the 64 squares: a1 is 0, h1 is 7, a8 is 56, h8 is 63.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Square(u8);

impl Square {
    /// `file` 0–7 is a–h, `rank` 0–7 is 1–8; anything else is off the board.
    pub const fn new(file: u8, rank: u8) -> Option<Self> {
        if file < 8 && rank < 8 {
            Some(Self(rank * 8 + file))
        } else {
            None
        }
    }

    pub const fn file(self) -> u8 {
        self.0 % 8
    }

    pub const fn rank(self) -> u8 {
        self.0 / 8
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// a1 is dark, so a square is light when its file and rank differ in parity.
    pub const fn is_light(self) -> bool {
        (self.file() + self.rank()) % 2 == 1
    }

    pub fn all() -> impl Iterator<Item = Self> {
        (0..64).map(Self)
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", char::from(b'a' + self.file()), self.rank() + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::Square;

    #[test]
    fn corners_have_their_names() {
        let names: Vec<String> = [(0, 0), (7, 0), (0, 7), (7, 7)]
            .into_iter()
            .map(|(file, rank)| Square::new(file, rank).expect("on the board").to_string())
            .collect();
        assert_eq!(names, ["a1", "h1", "a8", "h8"]);
    }

    #[test]
    fn off_the_board_is_none() {
        assert_eq!(Square::new(8, 0), None);
        assert_eq!(Square::new(0, 8), None);
    }

    #[test]
    fn a1_is_dark_and_h1_is_light() {
        assert!(!Square::new(0, 0).expect("a1").is_light());
        assert!(Square::new(7, 0).expect("h1").is_light());
    }

    #[test]
    fn all_lists_64_squares_in_order() {
        let squares: Vec<Square> = Square::all().collect();
        assert_eq!(squares.len(), 64);
        assert_eq!(squares[0].to_string(), "a1");
        assert_eq!(squares[63].to_string(), "h8");
    }
}
