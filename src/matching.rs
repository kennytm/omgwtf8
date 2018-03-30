use pattern::{Haystack, Pattern, Searcher};

/// Extension for matching
pub trait MatchExt: Haystack {
    fn contains<P: Pattern<Self>>(self, pat: P) -> bool {
        pat.is_contained_in(self)
    }

    fn split<P: Pattern<Self>>(self, pat: P) -> Split<Self, P> {
        let start = Self::cursor_at_front(&self);
        let end = Self::cursor_at_back(&self);
        let matcher = pat.into_searcher(self);
        Split {
            start,
            end,
            matcher,
            allow_trailing_empty: true,
            finished: false,
        }
    }

    fn find<P: Pattern<Self>>(self, pat: P) -> Option<usize> {
        let mut searcher = pat.into_searcher(self);
        let cursor = searcher.next_match()?.0;
        unsafe { Some(Self::start_cursor_to_offset(&searcher.haystack(), cursor)) }
    }
}

impl<H: Haystack> MatchExt for H {}

pub struct Split<H: Haystack, P: Pattern<H>> {
    start: H::StartCursor,
    end: H::EndCursor,
    matcher: P::Searcher,
    allow_trailing_empty: bool,
    finished: bool,
}

impl<H: Haystack, P: Pattern<H>> Split<H, P> {
    fn get_end(&mut self) -> Option<H> {
        if !self.finished && (self.allow_trailing_empty || self.start < self.end) {
            self.finished = true;
            unsafe {
                Some(H::range_to_self(
                    self.matcher.haystack(),
                    self.start,
                    self.end,
                ))
            }
        } else {
            None
        }
    }
}

impl<H: Haystack, P: Pattern<H>> Iterator for Split<H, P> {
    type Item = H;
    fn next(&mut self) -> Option<H> {
        if self.finished {
            return None;
        }
        match self.matcher.next_match() {
            Some((a, b)) => unsafe {
                let haystack = self.matcher.haystack();
                let a = H::start_to_end_cursor(&haystack, a);
                let b = H::end_to_start_cursor(&haystack, b);
                let elt = H::range_to_self(haystack, self.start, a);
                self.start = b;
                Some(elt)
            },
            None => self.get_end(),
        }
    }
}

#[test]
fn test_slice_pattern_api() {
    let p = &[1, 2, 3, 4, 5, 6][..];
    assert!(p.contains(&1));
    assert!(p.contains(&3));
    assert!(p.contains(&6));
    assert!(!p.contains(&10));

    assert_eq!(p.find(&1), Some(0));
    assert_eq!(p.find(&3), Some(2));
    assert_eq!(p.find(&6), Some(5));
    assert_eq!(p.find(&10), None);

    let q = &[1, 2, 3, 4, 1, 2, 4, 1, 5, 4, 4, 4, 7][..];
    assert_eq!(
        MatchExt::split(q, &4).collect::<Vec<_>>(),
        vec![&[1, 2, 3][..], &[1, 2][..], &[1, 5][..], &[], &[], &[7][..]]
    );
}

#[test]
fn test_ow8_pattern_api() {
    use OmgWtf8;

    let x = OmgWtf8::from_str("😀A😑B😢😳🙄");
    let y = OmgWtf8::from_wide(&[0xd83d]);
    assert_eq!(
        x.split(&*y).collect::<Vec<_>>(),
        &[
            OmgWtf8::from_str(""),
            &*OmgWtf8::from_wide(&[0xde00, 0x41]),
            &*OmgWtf8::from_wide(&[0xde11, 0x42]),
            &*OmgWtf8::from_wide(&[0xde22]),
            &*OmgWtf8::from_wide(&[0xde33]),
            &*OmgWtf8::from_wide(&[0xde44]),
        ]
    );

    assert_eq!(x.find(&*OmgWtf8::from_wide(&[0xde00])), Some(2));
    assert_eq!(x.find(OmgWtf8::from_str("B")), Some(9));
    assert_eq!(x.find(&*OmgWtf8::from_wide(&[0xde55])), None);
}
