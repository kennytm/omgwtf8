use OmgWtf8;
use conv::ThreeByteSeq;
use std::hash::{Hash, Hasher};
use std::cmp::Ordering;

impl OmgWtf8 {
    /// Split the string into three parts: the beginning low surrogate, the
    /// well-formed WTF-8 string in the middle, and the ending high surrogate.
    pub(crate) fn canonicalize(&self) -> (u16, &[u8], u16) {
        let len = self.0.len();
        match len {
            0...2 => (0, &self.0, 0),
            3 => match ThreeByteSeq::new(&self.0).canonicalize() {
                c @ 0xa000...0xafff => (0, &[], c),
                c @ 0xb000...0xbfff => (c, &[], 0),
                _ => (0, &self.0, 0),
            },
            4...5 => match ThreeByteSeq::new(&self.0).canonicalize() {
                c @ 0xb000...0xbfff => (c, &self.0[3..], 0),
                _ => match ThreeByteSeq::new(&self.0[len - 3..]).canonicalize() {
                    c @ 0xa000...0xafff => (0, &self.0[..len - 3], c),
                    _ => (0, &self.0, 0),
                },
            },
            _ => {
                let beg = ThreeByteSeq::new(&self.0).canonicalize();
                let end = ThreeByteSeq::new(&self.0[len - 3..]).canonicalize();
                match (beg, end) {
                    (0xb000...0xbfff, 0xa000...0xafff) => (beg, &self.0[3..len - 3], end),
                    (0xb000...0xbfff, _) => (beg, &self.0[3..], 0),
                    (_, 0xa000...0xafff) => (0, &self.0[..len - 3], end),
                    _ => (0, &self.0, 0),
                }
            }
        }
    }
}

/// Two OMG-WTF-8 strings can be compared for equality.
impl Eq for OmgWtf8 {}

/// Two OMG-WTF-8 strings can be compared for partial equality.
impl PartialEq for OmgWtf8 {
    fn eq(&self, other: &Self) -> bool {
        self.canonicalize() == other.canonicalize()
    }
}

/// Two OMG-WTF-8 strings can be totally ordered.
///
/// Note that the exact ordering is unspecified when unpaired surrogate exists.
impl Ord for OmgWtf8 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.canonicalize().cmp(&other.canonicalize())
    }
}

/// Two OMG-WTF-8 strings can be partially ordered.
impl PartialOrd for OmgWtf8 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// An OMG-WTF-8 string can be hashed for use in `HashMap` and `HashSet`.
impl Hash for OmgWtf8 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonicalize().hash(state)
    }
}

#[test]
fn test_ow8_canonicalized_equality() {
    unsafe {
        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83\xed\xa5\xa6"),
        );
        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83\xf1\xa9\xa8"),
        );
        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"\xa9\xa8\x83\xed\xa5\xa6"),
        );
        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83\xed\xa5\xa6\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"\xa9\xa8\x83\xed\xa5\xa6\xed\xa5\xa6"),
        );
        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"\xa9\xa8\x83\xf1\xa9\xa8"),
        );
        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"\x93\xa8\x83\xf1\xa9\xa3"),
        );
        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"\xa9\xa8\x83\xf1\xa9\xa8"),
            OmgWtf8::from_bytes_unchecked(b"\x93\xa8\x83\xf1\xa9\xa3"),
        );
        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"\xa9\xa8\x83a\xf1\xa9\xa8"),
            OmgWtf8::from_bytes_unchecked(b"\x93\xa8\x83a\xf1\xa9\xa3"),
        );
        assert_ne!(
            OmgWtf8::from_bytes_unchecked(b"\xa9\xa8\x83a\xf1\xa9\xa8"),
            OmgWtf8::from_bytes_unchecked(b"\x93\xa8\x83A\xf1\xa9\xa3"),
        );
        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83"),
            OmgWtf8::from_bytes_unchecked(b"\x93\xa8\x83"),
        );
        assert_ne!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83"),
            OmgWtf8::from_bytes_unchecked(b"\x93\xa9\x83"),
        );
        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83a"),
            OmgWtf8::from_bytes_unchecked(b"\x93\xa8\x83a"),
        );
        assert_ne!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83a"),
            OmgWtf8::from_bytes_unchecked(b"\x93\xa9\x83a"),
        );
        assert_ne!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83a"),
            OmgWtf8::from_bytes_unchecked(b"\x93\xa8\x83A"),
        );
        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83ab"),
            OmgWtf8::from_bytes_unchecked(b"\x93\xa8\x83ab"),
        );
        assert_ne!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83ab"),
            OmgWtf8::from_bytes_unchecked(b"\x93\xa9\x83ab"),
        );
        assert_ne!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xb8\x83ab"),
            OmgWtf8::from_bytes_unchecked(b"\x93\xa8\x83AB"),
        );

        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"\xf1\xa9\xa3"),
        );
        assert_ne!(
            OmgWtf8::from_bytes_unchecked(b"\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"\xf2\xa9\xa3"),
        );
        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"a\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"a\xf1\xa9\xa3"),
        );
        assert_ne!(
            OmgWtf8::from_bytes_unchecked(b"a\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"a\xf2\xa9\xa3"),
        );
        assert_ne!(
            OmgWtf8::from_bytes_unchecked(b"a\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"A\xf1\xa9\xa3"),
        );
        assert_eq!(
            OmgWtf8::from_bytes_unchecked(b"ab\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"ab\xf1\xa9\xa3"),
        );
        assert_ne!(
            OmgWtf8::from_bytes_unchecked(b"ab\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"ab\xf2\xa9\xa3"),
        );
        assert_ne!(
            OmgWtf8::from_bytes_unchecked(b"ab\xed\xa5\xa6"),
            OmgWtf8::from_bytes_unchecked(b"AB\xf1\xa9\xa3"),
        );
    }
}
