extern crate regex;

mod slice;
mod conv;
mod cmp;
pub mod pattern;
mod matching;

/// An OMG-WTF-8 string.
pub struct OmgWtf8([u8]);

pub use matching::MatchExt;
