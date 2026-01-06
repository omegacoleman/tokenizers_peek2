use serde::{Deserialize, Deserializer, Serialize};

use unicode_categories::UnicodeCategories;

use crate::tokenizer::{
    PreTokenizedString, PreTokenizer, Result,
};

use crate::normalizer::Range;

fn do_branch0(chars: &mut std::str::CharIndices) -> usize
{
    let mut backup = chars.clone();
    let _ = chars.next();
    let ch0 = chars.next().unwrap_or((0, '\0')).1;
    let ch1 = chars.next().unwrap_or((0, '\0')).1;
    match ch0 {
        'S' | 'D' | 'M' | 'T' | 's' | 'd' | 'm' | 't' => return 2,
        'R' | 'V' | 'r' | 'v' => match ch1 {
            'E' | 'e' => return 3,
            _ => (),
        },
        'L' | 'l' => match ch1 {
            'L' | 'l' => return 3,
            _ => (),
        },
        _ => (),
    }
    do_branch1(&mut backup)
}

fn do_branch1(chars: &mut std::str::CharIndices) -> usize
{
    for (pos, ch) in chars.skip(1) {
        if !ch.is_letter() {
            return pos;
        }
    }
    0
}

fn do_branch2(chars: &mut std::str::CharIndices) -> usize
{
    let mut count = 1;
    for (pos, ch) in chars.skip(1) {
        if !ch.is_number() || count >= 3 {
            return pos;
        }
        count += 1;
    }
    0
}

fn do_branch3(chars: &mut std::str::CharIndices) -> usize
{
    let mut rsnap = false;
    for (pos, ch) in chars.skip(1) {
        if !rsnap && (ch.is_number() || ch.is_letter() || ch.is_whitespace()) {
            rsnap = true;
        }
        if rsnap && ch != '\r' && ch != '\n' {
            return pos;
        }
    }
    0
}

fn do_branch4(chars: &mut std::str::CharIndices) -> usize
{
    let mut after_last_lf: Option<usize> = None;
    let mut before_last_ws: usize = 0;
    let mut last_ch_is_lf = false;
    for (pos, ch) in chars {
        if last_ch_is_lf {
            after_last_lf = Some(pos);
        }
        last_ch_is_lf = ch == '\r' || ch == '\n';
        if ch.is_whitespace() {
            before_last_ws = pos;
        } else {
            return match after_last_lf {
                Some(rpos) => rpos,
                _ => match before_last_ws {
                    0 => pos,
                    n => n,
                },
            }
        }
    }
    0
}

enum Peek2Category {
    Other,
    Space,
    Squote,
    LineFold,
    OtherLetter,
    OtherWhitespace,
    OtherNumber,
}

impl From<Peek2Category> for usize {
    fn from(c: Peek2Category) -> Self {
        match c {
            Peek2Category::Other => 0,
            Peek2Category::Space => 1,
            Peek2Category::Squote => 2,
            Peek2Category::LineFold => 3,
            Peek2Category::OtherLetter => 4,
            Peek2Category::OtherWhitespace => 5,
            Peek2Category::OtherNumber => 6,
        }
    }
}

impl From<char> for Peek2Category {
    fn from(ch: char) -> Self {
        match ch {
            ' ' => Peek2Category::Space,
            '\'' => Peek2Category::Squote,
            '\r' | '\n' => Peek2Category::LineFold,
            _ => {
                if ch.is_letter() {
                    Peek2Category::OtherLetter
                } else if ch.is_whitespace() {
                    Peek2Category::OtherWhitespace
                } else if ch.is_number() {
                    Peek2Category::OtherNumber
                } else {
                    Peek2Category::Other
                }
            },
        }
    }
}

type DoBranch = fn (&mut std::str::CharIndices) -> usize;

static PEEK2TBL: [[DoBranch; 7]; 7] = [
    // other
    [
      do_branch3, // other
      do_branch3, // space
      do_branch3, // squote
      do_branch3, // linefold
      do_branch1, // other_letter
      do_branch3, // other_whitespace
      do_branch3, // other_number
    ],
    // space
    [
      do_branch3, // other
      do_branch4, // space
      do_branch3, // squote
      do_branch4, // linefold
      do_branch1, // other_letter
      do_branch4, // other_whitespace
      do_branch4, // other_number
    ],
    // squote
    [
      do_branch3, // other
      do_branch3, // space
      do_branch3, // squote
      do_branch3, // linefold
      do_branch0, // other_letter
      do_branch3, // other_whitespace
      do_branch3, // other_number
    ],
    // linefold
    [
      do_branch4, // other
      do_branch4, // space
      do_branch4, // squote
      do_branch4, // linefold
      do_branch4, // other_letter
      do_branch4, // other_whitespace
      do_branch4, // other_number
    ],
    // other_letter
    [
      do_branch1, // other
      do_branch1, // space
      do_branch1, // squote
      do_branch1, // linefold
      do_branch1, // other_letter
      do_branch1, // other_whitespace
      do_branch1, // other_number
    ],
    // other_whitespace
    [
      do_branch4, // other
      do_branch4, // space
      do_branch4, // squote
      do_branch4, // linefold
      do_branch1, // other_letter
      do_branch4, // other_whitespace
      do_branch4, // other_number
    ],
    // other_number
    [
      do_branch2, // other
      do_branch2, // space
      do_branch2, // squote
      do_branch2, // linefold
      do_branch2, // other_letter
      do_branch2, // other_whitespace
      do_branch2, // other_number
    ],
];

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub struct PeekTwo {
}

impl<'de> Deserialize<'de> for PeekTwo {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        enum Type {
            PeekTwo,
        }

        #[derive(Deserialize)]
        pub struct PeekTwoHelper {
            #[serde(rename = "type")]
            _type: Type,
        }

        let _helper = PeekTwoHelper::deserialize(deserializer)?;
        Self::new().map_err(serde::de::Error::custom)
    }
}

impl Clone for PeekTwo {
    fn clone(&self) -> Self {
        Self::new().unwrap()
    }
}

impl PartialEq for PeekTwo {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl PeekTwo {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }
}

impl PreTokenizer for PeekTwo {
    fn pre_tokenize(&self, pretokenized: &mut PreTokenizedString) -> Result<()> {
        pretokenized.split(|_, normalized| {
            let mut vec = Vec::new();
            let mut pos: usize = 0;
            while pos < normalized.len() {
                let remain = &normalized.get()[pos..];
                let mut char_indicies = remain.char_indices();

                let mut peek2 = char_indicies.clone();
                let c0 = match peek2.next() {
                    Some((_, ch)) => ch,
                    _ => break,
                };
                let c1 = match peek2.next() {
                    Some((_, ch)) => ch,
                    _ => '\0',
                };
                let mut incr = PEEK2TBL
                    [usize::from(Peek2Category::from(c0))]
                    [usize::from(Peek2Category::from(c1))]
                    (&mut char_indicies);
                if incr == 0 {
                    incr = normalized.len() - pos;
                }
                vec.push(normalized.slice(Range::Normalized(pos..(pos + incr)))
                    .ok_or("branch didn't break on char bounds")?);
                pos += incr;
            }

            Ok(vec)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OffsetReferential, OffsetType, PreTokenizer};

    #[test]
    fn basic() {
        let tests = vec![
            (
                "How are you doing?",
                vec![
                    ("How", (0, 3)),
                    (" are", (3, 7)),
                    (" you", (7, 11)),
                    (" doing", (11, 17)),
                    ("?", (17, 18)),
                ],
            ),
        ];

        for (s, res) in tests {
            let mut pretokenized = PreTokenizedString::from(s);
            let pretok = PeekTwo::new().unwrap();
            pretok.pre_tokenize(&mut pretokenized).unwrap();
            assert_eq!(
                pretokenized
                    .get_splits(OffsetReferential::Original, OffsetType::Byte)
                    .into_iter()
                    .map(|(s, o, _)| (s, o))
                    .collect::<Vec<_>>(),
                res
            );
        }
    }

    #[test]
    fn serialization() {
        let tok = PeekTwo::new().unwrap();
        let tok_s =
            r#"{"type":"PeekTwo"}"#;
        assert_eq!(serde_json::to_string(&tok).unwrap(), tok_s);
        assert_eq!(serde_json::from_str::<PeekTwo>(tok_s).unwrap(), tok);
    }
}
