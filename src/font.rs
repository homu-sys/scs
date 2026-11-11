#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontSize {
    Large,
    Small,
}

impl FontSize {
    pub fn glyph(self, ch: char) -> Option<&'static [&'static str]> {
        match self {
            Self::Large => match ch {
                '0'..='9' => Some(LARGE_DIGITS[(ch as u8 - b'0') as usize]),
                ':' => Some(LARGE_COLON),
                _ => None,
            },
            Self::Small => match ch {
                '0'..='9' => Some(SMALL_DIGITS[(ch as u8 - b'0') as usize]),
                ':' => Some(SMALL_COLON),
                _ => None,
            },
        }
    }

    pub fn width(self, text: &str) -> i32 {
        let mut width = 0;
        let mut glyphs = 0;
        for ch in text.chars() {
            if let Some(glyph) = self.glyph(ch) {
                if glyphs > 0 {
                    width += self.gap();
                }
                width += glyph_width(glyph);
                glyphs += 1;
            }
        }
        width
    }

    pub fn height(self) -> i32 {
        match self {
            Self::Large => 9,
            Self::Small => 7,
        }
    }

    pub fn gap(self) -> i32 {
        1
    }
}

pub fn glyph_width(glyph: &[&str]) -> i32 {
    glyph
        .first()
        .map(|row| row.chars().count() as i32)
        .unwrap_or(0)
}

const LARGE_0: &[&str] = &[
    " ##### ", "##   ##", "##   ##", "##  ###", "## # ##", "###  ##", "##   ##", "##   ##",
    " ##### ",
];

const LARGE_1: &[&str] = &[
    "   ##  ", "  ###  ", "   ##  ", "   ##  ", "   ##  ", "   ##  ", "   ##  ", "   ##  ",
    " ######",
];

const LARGE_2: &[&str] = &[
    " ##### ", "##   ##", "     ##", "    ## ", "   ##  ", "  ##   ", " ##    ", "##     ",
    "#######",
];

const LARGE_3: &[&str] = &[
    "###### ", "     ##", "     ##", " ##### ", "     ##", "     ##", "     ##", "##   ##",
    " ##### ",
];

const LARGE_4: &[&str] = &[
    "    ## ", "   ### ", "  # ## ", " ## ## ", "##  ## ", "#######", "    ## ", "    ## ",
    "    ## ",
];

const LARGE_5: &[&str] = &[
    "#######", "##     ", "##     ", "###### ", "     ##", "     ##", "     ##", "##   ##",
    " ##### ",
];

const LARGE_6: &[&str] = &[
    "  #### ", " ##    ", "##     ", "###### ", "##   ##", "##   ##", "##   ##", "##   ##",
    " ##### ",
];

const LARGE_7: &[&str] = &[
    "#######", "     ##", "    ## ", "   ##  ", "  ##   ", "  ##   ", " ##    ", " ##    ",
    " ##    ",
];

const LARGE_8: &[&str] = &[
    " ##### ", "##   ##", "##   ##", "##   ##", " ##### ", "##   ##", "##   ##", "##   ##",
    " ##### ",
];

const LARGE_9: &[&str] = &[
    " ##### ", "##   ##", "##   ##", "##   ##", " ######", "     ##", "     ##", "    ## ",
    " ####  ",
];

const LARGE_COLON: &[&str] = &["  ", "##", "##", "  ", "  ", "  ", "##", "##", "  "];

const LARGE_DIGITS: [&[&str]; 10] = [
    LARGE_0, LARGE_1, LARGE_2, LARGE_3, LARGE_4, LARGE_5, LARGE_6, LARGE_7, LARGE_8, LARGE_9,
];

const SMALL_0: &[&str] = &[
    " ### ", "#   #", "#  ##", "# # #", "##  #", "#   #", " ### ",
];

const SMALL_1: &[&str] = &[
    "  #  ", " ##  ", "  #  ", "  #  ", "  #  ", "  #  ", " ### ",
];

const SMALL_2: &[&str] = &[
    " ### ", "#   #", "    #", "   # ", "  #  ", " #   ", "#####",
];

const SMALL_3: &[&str] = &[
    "#### ", "    #", "    #", " ### ", "    #", "    #", "#### ",
];

const SMALL_4: &[&str] = &[
    "   # ", "  ## ", " # # ", "#  # ", "#####", "   # ", "   # ",
];

const SMALL_5: &[&str] = &[
    "#####", "#    ", "#### ", "    #", "    #", "#   #", " ### ",
];

const SMALL_6: &[&str] = &[
    " ### ", "#    ", "#    ", "#### ", "#   #", "#   #", " ### ",
];

const SMALL_7: &[&str] = &[
    "#####", "    #", "   # ", "  #  ", " #   ", " #   ", " #   ",
];

const SMALL_8: &[&str] = &[
    " ### ", "#   #", "#   #", " ### ", "#   #", "#   #", " ### ",
];

const SMALL_9: &[&str] = &[
    " ### ", "#   #", "#   #", " ####", "    #", "    #", " ### ",
];

const SMALL_COLON: &[&str] = &["  ", "##", "  ", "  ", "##", "  ", "  "];

const SMALL_DIGITS: [&[&str]; 10] = [
    SMALL_0, SMALL_1, SMALL_2, SMALL_3, SMALL_4, SMALL_5, SMALL_6, SMALL_7, SMALL_8, SMALL_9,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_clock_widths() {
        assert_eq!(FontSize::Large.width("12:34:56"), 53);
        assert_eq!(FontSize::Small.width("12:34:56"), 41);
        assert_eq!(FontSize::Large.height(), 9);
    }

    #[test]
    fn every_glyph_has_expected_height() {
        for ch in "0123456789:".chars() {
            let large = FontSize::Large.glyph(ch).unwrap();
            let small = FontSize::Small.glyph(ch).unwrap();
            assert_eq!(large.len(), 9);
            assert_eq!(small.len(), 7);
        }
    }
}
