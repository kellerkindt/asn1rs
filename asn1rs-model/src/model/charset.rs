use crate::model::Tag;

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum Charset {
    Utf8,
    /// ITU-T X.680 | ISO/IEC 8824-1, 43.3
    Numeric,
    /// ITU-T X.680 | ISO/IEC 8824-1, 43.3
    Printable,

    // /// (Also T61String)
    // Teletext,
    // Videotext,
    /// Encoding as in ISO/IEC 646 (??)
    Ia5,

    // GraphicsString,
    /// ITU-T X.680 | ISO/IEC 8824-1, 43.3
    /// (Also ISO646String)
    Visible,
}

impl Charset {
    /// Sorted according to ITU-T X.680, 43.5
    /// ```rust
    /// use asn1rs_model::model::Charset;
    /// assert!(Charset::NUMERIC_STRING_CHARACTERS.chars().all(|c| Charset::Numeric.is_valid(c)));
    /// assert!(Charset::NUMERIC_STRING_CHARACTERS.chars().all(|c| Charset::Utf8.is_valid(c)));
    /// assert!(Charset::NUMERIC_STRING_CHARACTERS.chars().all(|c| Charset::Printable.is_valid(c)));
    /// assert!(Charset::NUMERIC_STRING_CHARACTERS.chars().all(|c| Charset::Ia5.is_valid(c)));
    /// assert!(Charset::NUMERIC_STRING_CHARACTERS.chars().all(|c| Charset::Visible.is_valid(c)));
    /// assert_eq!(11, Charset::NUMERIC_STRING_CHARACTERS.chars().count());
    /// ```
    pub const NUMERIC_STRING_CHARACTERS: &'static str = " 0123456789";

    /// Sorted according to ITU-T X.680, 43.6
    /// ```rust
    /// use asn1rs_model::model::Charset;
    /// assert!(Charset::PRINTABLE_STRING_CHARACTERS.chars().all(|c| Charset::Printable.is_valid(c)));
    /// assert!(Charset::PRINTABLE_STRING_CHARACTERS.chars().all(|c| Charset::Utf8.is_valid(c)));
    /// assert!(Charset::PRINTABLE_STRING_CHARACTERS.chars().all(|c| Charset::Ia5.is_valid(c)));
    /// assert!(Charset::PRINTABLE_STRING_CHARACTERS.chars().all(|c| Charset::Visible.is_valid(c)));
    /// assert_eq!(74, Charset::PRINTABLE_STRING_CHARACTERS.chars().count());
    /// ```
    pub const PRINTABLE_STRING_CHARACTERS: &'static str =
        " '()+,-./0123456789:=?ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    /// Sorted according to ITU-T X.680, 43.8
    /// ```rust
    /// use asn1rs_model::model::Charset;
    /// assert!(Charset::IA5_STRING_CHARACTERS.chars().all(|c| Charset::Ia5.is_valid(c)));
    /// assert!(Charset::IA5_STRING_CHARACTERS.chars().all(|c| Charset::Utf8.is_valid(c)));
    /// assert_eq!(128, Charset::IA5_STRING_CHARACTERS.chars().count());
    /// ```
    pub const IA5_STRING_CHARACTERS: &'static str =
        "\u{00}\u{01}\u{02}\u{03}\u{04}\u{05}\u{06}\u{07}\u{08}\u{09}\u{0A}\u{0B}\u{0C}\u{0D}\u{0E}\u{0F}\u{10}\u{11}\u{12}\u{13}\u{14}\u{15}\u{16}\u{17}\u{18}\u{19}\u{1A}\u{1B}\u{1C}\u{1D}\u{1E}\u{1F} !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\u{7F}";

    /// Sorted according to ITU-T X.680, 43.7
    /// ```rust
    /// use asn1rs_model::model::Charset;
    /// assert!(Charset::VISIBLE_STRING_CHARACTERS.chars().all(|c| Charset::Visible.is_valid(c)));
    /// assert!(Charset::VISIBLE_STRING_CHARACTERS.chars().all(|c| Charset::Ia5.is_valid(c)));
    /// assert!(Charset::VISIBLE_STRING_CHARACTERS.chars().all(|c| Charset::Utf8.is_valid(c)));
    /// assert_eq!(95, Charset::VISIBLE_STRING_CHARACTERS.chars().count());
    /// ```
    pub const VISIBLE_STRING_CHARACTERS: &'static str =
        " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";

    pub fn default_tag(self) -> Tag {
        match self {
            Charset::Utf8 => Tag::DEFAULT_UTF8_STRING,
            Charset::Numeric => Tag::DEFAULT_NUMERIC_STRING,
            Charset::Printable => Tag::DEFAULT_PRINTABLE_STRING,
            Charset::Ia5 => Tag::DEFAULT_IA5_STRING,
            Charset::Visible => Tag::DEFAULT_VISIBLE_STRING,
        }
    }

    pub fn find_invalid(self, str: &str) -> Option<(usize, char)> {
        str.chars()
            .enumerate()
            .find(|(_index, char)| !self.is_valid(*char))
    }

    pub const fn is_valid(self, char: char) -> bool {
        match self {
            Charset::Utf8 => true,
            Charset::Numeric => matches!(char, ' ' | '0'..='9'),
            Charset::Printable => {
                matches!(char, ' ' | '\'' ..= ')' | '+' ..= ':' | '=' | '?' | 'A'..='Z' | 'a'..='z'  )
            }
            Charset::Ia5 => matches!(char as u32, 0_u32..=127),
            Charset::Visible => matches!(char as u32, 32_u32..=126),
        }
    }
}
