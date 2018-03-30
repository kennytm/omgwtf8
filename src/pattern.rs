//! Pattern API 1.6
//!
//! This is an extended version of the standard `Pattern` API which allows
//! OMG-WTF-8 strings be used as a needle. This API is used as a demonstration
//! that OMG-WTF-8 can fit into “Pattern 1.x” API.
//!
//! Existing extensions can be found in:
//!
//! * 1.x: <https://github.com/Kimundi/pattern_api_sketch>
//! * 2.0: <https://github.com/Kimundi/rust_pattern_api_v2> ([description])
//!
//! This module extends up on “Pattern 1.5”, except:
//!
//! 1. The trait `SearchPtrs` is renamed to `Haystack` (similar to 2.0’s
//!     `PatternHaystack`)
//! 2. The associated type `Cursor` is split into `StartCursor` and `EndCursor`
//!     for extra type safety when working with OMG-WTF-8 strings.
//! 3. The associated type `Haystack` is removed, assuming it is always
//!     `(Self::StartCursor, Self::EndCursor)`.
//!
//! This module does not provide details like `next_reject` or
//! `ReverseSearcher`. They are expected to be implemented similarly.
//!
//! [description]: https://github.com/rust-lang/rfcs/pull/1309#issuecomment-214030263

use std::mem::size_of;
use std::cmp::max;
use std::fmt::Write;
use std::slice::from_raw_parts;
use OmgWtf8;
use regex::bytes::{Regex, RegexBuilder};

pub trait Pattern<H: Haystack>: Sized {
    type Searcher: Searcher<H>;

    fn into_searcher(self, haystack: H) -> Self::Searcher;

    // fn is_prefix_of(self, haystack: H) -> bool;
    // fn is_suffix_of(self, haystack: H) -> bool;

    fn is_contained_in(self, haystack: H) -> bool {
        self.into_searcher(haystack).next_match().is_some()
    }
}

pub trait Searcher<H: Haystack> {
    fn haystack(&self) -> H;
    fn next_match(&mut self) -> Option<(H::StartCursor, H::EndCursor)>;
    // fn next_reject(&mut self) -> Option<(H::StartCursor, H::EndCursor)>;
}

// Haystack should be implemented for slice references: `&[T]`, `&str`,
// `&mut [T]` etc.
pub trait Haystack: Sized {
    /// If the haystack is treated as an `&[T]` slice, the cursor’s type should
    /// be `*const T`.
    type StartCursor: Copy + PartialOrd<Self::EndCursor>;
    type EndCursor: Copy + PartialOrd<Self::StartCursor>;

    fn cursor_at_front(hs: &Self) -> Self::StartCursor;
    fn cursor_at_back(hs: &Self) -> Self::EndCursor;

    unsafe fn start_to_end_cursor(hs: &Self, cur: Self::StartCursor) -> Self::EndCursor;
    unsafe fn end_to_start_cursor(hs: &Self, cur: Self::EndCursor) -> Self::StartCursor;

    unsafe fn start_cursor_to_offset(hs: &Self, cur: Self::StartCursor) -> usize;
    unsafe fn end_cursor_to_offset(hs: &Self, cur: Self::EndCursor) -> usize;

    unsafe fn range_to_self(hs: Self, start: Self::StartCursor, end: Self::EndCursor) -> Self;
}

//--------------------------------------------------------------------------------------------------

/// Searcher for a single element in a slice.
pub struct SliceElemSearcher<'p, 'h, T: PartialEq + 'p + 'h> {
    haystack: &'h [T],
    elem: &'p T,
    begin: *const T,
    end: *const T,
}

impl<'p, 'h, T: PartialEq + 'p + 'h> Searcher<&'h [T]> for SliceElemSearcher<'p, 'h, T> {
    fn haystack(&self) -> &'h [T] {
        self.haystack
    }

    fn next_match(&mut self) -> Option<(*const T, *const T)> {
        unsafe {
            while self.begin != self.end {
                let cur = self.begin;
                self.begin = cur.offset(1);
                if *cur == *self.elem {
                    return Some((cur, self.begin));
                }
            }
            None
        }
    }

    // fn next_reject(&mut self) -> Option<(*const T, *const T)> {
    //     unsafe {
    //         while self.begin != self.end {
    //             let cur = self.begin;
    //             self.begin = cur.offset(1);
    //             if *cur != *self.elem {
    //                 return Some((cur, self.begin));
    //             }
    //         }
    //         None
    //     }
    // }
}

impl<'h, T> Haystack for &'h [T] {
    type StartCursor = *const T;
    type EndCursor = *const T;

    fn cursor_at_front(hs: &Self) -> Self::StartCursor {
        hs.as_ptr()
    }

    fn cursor_at_back(hs: &Self) -> Self::EndCursor {
        let ptr = hs.as_ptr();
        if size_of::<T>() == 0 {
            (ptr as usize + hs.len()) as *const T
        } else {
            unsafe { ptr.offset(hs.len() as isize) }
        }
    }

    unsafe fn start_to_end_cursor(_: &Self, cur: Self::StartCursor) -> Self::EndCursor {
        cur
    }

    unsafe fn end_to_start_cursor(_: &Self, cur: Self::EndCursor) -> Self::StartCursor {
        cur
    }

    unsafe fn start_cursor_to_offset(hs: &Self, cur: Self::StartCursor) -> usize {
        let size = max(size_of::<T>(), 1);
        let ptr = hs.as_ptr();
        (cur as usize - ptr as usize) / size
    }

    unsafe fn end_cursor_to_offset(hs: &Self, cur: Self::EndCursor) -> usize {
        Self::start_cursor_to_offset(hs, cur)
    }

    unsafe fn range_to_self(hs: Self, start: Self::StartCursor, end: Self::EndCursor) -> Self {
        let start = Self::start_cursor_to_offset(&hs, start);
        let end = Self::end_cursor_to_offset(&hs, end);
        hs.get_unchecked(start..end)
    }
}

impl<'p, 'h, T: PartialEq + 'h + 'p> Pattern<&'h [T]> for &'p T {
    type Searcher = SliceElemSearcher<'p, 'h, T>;

    fn into_searcher(self, haystack: &'h [T]) -> Self::Searcher {
        SliceElemSearcher {
            haystack,
            begin: Haystack::cursor_at_front(&haystack),
            end: Haystack::cursor_at_back(&haystack),
            elem: self,
        }
    }

    // fn is_prefix_of(self, haystack: &'h [T]) -> bool {
    //     haystack.first() == Some(self)
    // }
    // fn is_suffix_of(self, haystack: &'h [T]) -> bool {
    //     haystack.last() == Some(self)
    // }
}

//--------------------------------------------------------------------------------------------------

/// Searcher for an OMG-WTF-8 substring

pub struct OmgWtf8Searcher<'h> {
    haystack: &'h OmgWtf8,
    pattern: Regex,
    begin: *const u8,
    end: *const u8,
    finished: bool,
}

/// Derive the regex pattern from a canonicalized surrogate value
/// (`0xa000 ..= 0xbfff`)
fn append_regex_pattern_from_surrogate(w: &mut String, c: u16) {
    if c >= 0xb000 {
        // low surrogate
        write!(
            w,
            r"(?:\xed\x{0:02x}|[\x80-\xbf][\x8{2:x}\x9{2:x}\xa{2:x}\xb{2:x}])\x{1:02x}",
            c >> 8,
            c & 0xff,
            (c >> 8) & 0xf,
        )
    } else {
        // high surrogate
        let s = (c & 0x3f | (c >> 2) & 0x3c0) + 0x40;
        write!(
            w,
            r"(?:\xed\x{0:02x}\x{1:02x}|\x{2:02x}\x{3:02x}[\x{4:x}0-\x{4:x}f])",
            c >> 8,
            c & 0xff,
            (s >> 8) | 0xf0,
            (s >> 2) & 0x3f | 0x80,
            s & 3 | 8
        )
    }.unwrap();
}

impl<'p, 'h> Pattern<&'h OmgWtf8> for &'p OmgWtf8 {
    type Searcher = OmgWtf8Searcher<'h>;

    fn into_searcher(self, haystack: &'h OmgWtf8) -> OmgWtf8Searcher<'h> {
        let mut pattern = String::with_capacity(self.len() * 4);
        let (begin, middle, end) = self.canonicalize();
        if begin != 0 {
            append_regex_pattern_from_surrogate(&mut pattern, begin);
        }
        for byte in middle {
            write!(&mut pattern, r"\x{:02x}", byte).unwrap();
        }
        if end != 0 {
            append_regex_pattern_from_surrogate(&mut pattern, end);
        }
        OmgWtf8Searcher {
            haystack,
            pattern: RegexBuilder::new(&pattern).unicode(false).build().unwrap(),
            begin: Haystack::cursor_at_front(&haystack),
            end: Haystack::cursor_at_back(&haystack),
            finished: false,
        }
    }
}

impl<'h> Searcher<&'h OmgWtf8> for OmgWtf8Searcher<'h> {
    fn haystack(&self) -> &'h OmgWtf8 {
        self.haystack
    }

    fn next_match(&mut self) -> Option<(*const u8, *const u8)> {
        if self.finished {
            return None;
        }
        unsafe {
            let slice_len = self.end as usize - self.begin as usize;
            let slice = from_raw_parts(self.begin, slice_len);
            match self.pattern.find(slice) {
                None => {
                    self.finished = true;
                    None
                }
                Some(m) => {
                    let mut start = self.begin.offset(m.start() as isize);
                    let mut end = self.begin.offset(m.end() as isize);
                    self.begin = Haystack::end_to_start_cursor(&self.haystack, end);
                    Some((start, end))
                }
            }
        }
    }
}

impl<'h> Haystack for &'h OmgWtf8 {
    type StartCursor = *const u8;
    type EndCursor = *const u8;

    fn cursor_at_front(hs: &Self) -> Self::StartCursor {
        hs.0.as_ptr()
    }
    fn cursor_at_back(hs: &Self) -> Self::EndCursor {
        unsafe { hs.0.as_ptr().offset(hs.0.len() as isize) }
    }

    unsafe fn start_to_end_cursor(hs: &Self, cur: Self::StartCursor) -> Self::EndCursor {
        if cur != Self::cursor_at_front(hs) && 0x80 <= *cur && *cur <= 0xbf {
            cur.offset(2)
        } else {
            cur
        }
    }

    unsafe fn end_to_start_cursor(hs: &Self, cur: Self::EndCursor) -> Self::StartCursor {
        if cur != Self::cursor_at_back(hs) && 0x80 <= *cur && *cur <= 0xbf {
            cur.offset(-2)
        } else {
            cur
        }
    }

    unsafe fn start_cursor_to_offset(hs: &Self, cur: Self::StartCursor) -> usize {
        let ptr = hs.0.as_ptr();
        let mut offset = cur as usize - ptr as usize;
        if offset != 0 && 0x80 <= *cur && *cur <= 0xbf {
            offset += 1;
        }
        offset
    }

    unsafe fn end_cursor_to_offset(hs: &Self, cur: Self::EndCursor) -> usize {
        let ptr = hs.0.as_ptr();
        let mut offset = cur as usize - ptr as usize;
        if offset != hs.len() && 0x80 <= *cur && *cur <= 0xbf {
            offset -= 1;
        }
        offset
    }

    unsafe fn range_to_self(_: Self, start: Self::StartCursor, end: Self::EndCursor) -> Self {
        let len = end as usize - start as usize;
        let slice = from_raw_parts(start, len);
        &*(slice as *const [u8] as *const OmgWtf8)
    }
}

#[test]
fn test_ow8_searcher() {
    // Tests copied from libcore.
    fn some(hs: &OmgWtf8, start: usize, end: usize) -> Option<(*const u8, *const u8)> {
        let ptr = hs.0.as_ptr();
        Some((
            ptr.wrapping_offset(start as isize),
            ptr.wrapping_offset(end as isize),
        ))
    }

    let haystack = OmgWtf8::from_str("abcdeabcd");
    let mut searcher = OmgWtf8::from_str("a").into_searcher(haystack);
    assert_eq!(searcher.next_match(), some(haystack, 0, 1));
    assert_eq!(searcher.next_match(), some(haystack, 5, 6));
    assert_eq!(searcher.next_match(), None);

    let haystack = OmgWtf8::from_str("Áa🁀bÁꁁfg😁각กᘀ각aÁ각ꁁก😁a");
    let mut searcher = OmgWtf8::from_str("x").into_searcher(haystack);
    assert_eq!(searcher.next_match(), None);

    let mut searcher = OmgWtf8::from_str("Á").into_searcher(haystack);
    assert_eq!(searcher.next_match(), some(haystack, 0, 2));
    assert_eq!(searcher.next_match(), some(haystack, 8, 10));
    assert_eq!(searcher.next_match(), some(haystack, 32, 34));
    assert_eq!(searcher.next_match(), None);

    let mut searcher = OmgWtf8::from_str("ก").into_searcher(haystack);
    assert_eq!(searcher.next_match(), some(haystack, 22, 25));
    assert_eq!(searcher.next_match(), some(haystack, 40, 43));
    assert_eq!(searcher.next_match(), None);

    let mut searcher = OmgWtf8::from_str("😁").into_searcher(haystack);
    assert_eq!(searcher.next_match(), some(haystack, 15, 19));
    assert_eq!(searcher.next_match(), some(haystack, 43, 47));
    assert_eq!(searcher.next_match(), None);

    let mut searcher = OmgWtf8::from_str("ꁁ").into_searcher(haystack);
    assert_eq!(searcher.next_match(), some(haystack, 10, 13));
    assert_eq!(searcher.next_match(), some(haystack, 37, 40));
    assert_eq!(searcher.next_match(), None);

    // Now some OMG-WTF-8-specific tests
    let haystack = OmgWtf8::from_str("😱😱😱");

    let pattern = OmgWtf8::from_wide(&[0xd83d]);
    let mut searcher = (&*pattern).into_searcher(haystack);
    assert_eq!(searcher.next_match(), some(haystack, 0, 3));
    assert_eq!(searcher.next_match(), some(haystack, 4, 7));
    assert_eq!(searcher.next_match(), some(haystack, 8, 11));
    assert_eq!(searcher.next_match(), None);

    let pattern = OmgWtf8::from_wide(&[0xde31]);
    let mut searcher = (&*pattern).into_searcher(haystack);
    assert_eq!(searcher.next_match(), some(haystack, 1, 4));
    assert_eq!(searcher.next_match(), some(haystack, 5, 8));
    assert_eq!(searcher.next_match(), some(haystack, 9, 12));
    assert_eq!(searcher.next_match(), None);

    let pattern = OmgWtf8::from_wide(&[0xde31, 0xd83d]);
    let mut searcher = (&*pattern).into_searcher(haystack);
    assert_eq!(searcher.next_match(), some(haystack, 1, 7));
    assert_eq!(searcher.next_match(), some(haystack, 5, 11));
    assert_eq!(searcher.next_match(), None);

    let hs = &haystack[2..];
    let mut searcher = (&*pattern).into_searcher(hs);
    assert_eq!(searcher.next_match(), some(hs, 0, 6));
    assert_eq!(searcher.next_match(), some(hs, 4, 10));
    assert_eq!(searcher.next_match(), None);

    let hs = &haystack[..10];
    let mut searcher = (&*pattern).into_searcher(hs);
    assert_eq!(searcher.next_match(), some(hs, 1, 7));
    assert_eq!(searcher.next_match(), some(hs, 5, 11));
    assert_eq!(searcher.next_match(), None);

    let haystack = OmgWtf8::from_wide(&[0xd83d, 0xd83d, 0xd83d, 0xde31, 0xde31, 0xde31]);

    let pattern = OmgWtf8::from_wide(&[0xd83d]);
    let mut searcher = (&*pattern).into_searcher(&*haystack);
    assert_eq!(searcher.next_match(), some(&haystack, 0, 3));
    assert_eq!(searcher.next_match(), some(&haystack, 3, 6));
    assert_eq!(searcher.next_match(), some(&haystack, 6, 9));
    assert_eq!(searcher.next_match(), None);

    let pattern = OmgWtf8::from_wide(&[0xde31]);
    let mut searcher = (&*pattern).into_searcher(&*haystack);
    assert_eq!(searcher.next_match(), some(&haystack, 7, 10));
    assert_eq!(searcher.next_match(), some(&haystack, 10, 13));
    assert_eq!(searcher.next_match(), some(&haystack, 13, 16));
    assert_eq!(searcher.next_match(), None);
}
