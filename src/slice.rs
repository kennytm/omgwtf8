use OmgWtf8;
use std::ops::{Index, Range, RangeFrom, RangeFull, RangeTo};

/// Type of an index in an OMG-WTF-8 string.
pub(crate) enum IndexType {
    /// Boundary of a WTF-8 character sequence.
    CharBoundary,
    /// Byte 1 in a 4-byte sequence.
    FourByteSeq1,
    /// Byte 2 in a 4-byte sequence.
    FourByteSeq2,
    /// Byte 3 in a 4-byte sequence.
    FourByteSeq3,
    /// Pointing inside a 2- or 3-byte sequence.
    Interior,
    /// Out of bounds.
    OutOfBounds,
}

impl OmgWtf8 {
    /// Obtains the length of this string.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Classifies the kind of index in this string.
    pub(crate) fn classify_index(&self, index: usize) -> IndexType {
        let len = self.0.len();
        if index == 0 || index == len {
            return IndexType::CharBoundary;
        }
        match self.0.get(index) {
            Some(&0x80...0xbf) => {
                if 1 <= index && index <= len - 3 && self.0[index - 1] >= 0xf0 {
                    IndexType::FourByteSeq1
                } else if 2 <= index && index <= len - 2 && self.0[index - 2] >= 0xf0 {
                    IndexType::FourByteSeq2
                } else if 3 <= index && index <= len - 1 && self.0[index - 3] >= 0xf0 {
                    IndexType::FourByteSeq3
                } else {
                    IndexType::Interior
                }
            }
            Some(_) => IndexType::CharBoundary,
            None => IndexType::OutOfBounds,
        }
    }
}

/// Allows OMG-WTF-8 strings be sliced using `s[..]`.
impl Index<RangeFull> for OmgWtf8 {
    type Output = Self;
    fn index(&self, _: RangeFull) -> &Self {
        self
    }
}

/// Allows OMG-WTF-8 strings be sliced using `s[..j]`.
impl Index<RangeTo<usize>> for OmgWtf8 {
    type Output = Self;
    fn index(&self, mut range: RangeTo<usize>) -> &Self {
        match self.classify_index(range.end) {
            IndexType::FourByteSeq2 => range.end += 1,
            IndexType::CharBoundary => {}
            _ => panic!("Invalid end index {}", range.end),
        };
        unsafe { Self::from_bytes_unchecked(&self.0[range]) }
    }
}

/// Allows OMG-WTF-8 strings be sliced using `s[i..]`.
impl Index<RangeFrom<usize>> for OmgWtf8 {
    type Output = Self;
    fn index(&self, mut range: RangeFrom<usize>) -> &Self {
        match self.classify_index(range.start) {
            IndexType::FourByteSeq2 => range.start -= 1,
            IndexType::CharBoundary => {}
            _ => panic!("Invalid start index {}", range.start),
        };
        unsafe { Self::from_bytes_unchecked(&self.0[range]) }
    }
}

/// Allows OMG-WTF-8 strings be sliced using `s[i..j]`.
impl Index<Range<usize>> for OmgWtf8 {
    type Output = Self;
    fn index(&self, mut range: Range<usize>) -> &Self {
        if range.start == range.end {
            return Self::from_str("");
        }
        match self.classify_index(range.start) {
            IndexType::FourByteSeq2 => range.start -= 1,
            IndexType::CharBoundary => {}
            _ => panic!("Invalid start index {}", range.start),
        };
        match self.classify_index(range.end) {
            IndexType::FourByteSeq2 => range.end += 1,
            IndexType::CharBoundary => {}
            _ => panic!("Invalid end index {}", range.end),
        };
        unsafe { Self::from_bytes_unchecked(&self.0[range]) }
    }
}

#[test]
fn test_ow8_len() {
    let s = OmgWtf8::from_str("foo");
    assert_eq!(s.len(), 3);
    assert_eq!(s.as_bytes(), b"foo");
}
#[test]
fn test_ow8_slices_str() {
    let s = OmgWtf8::from_str("foo");
    assert_eq!(s[..].as_bytes(), b"foo");
    assert_eq!(s[1..].as_bytes(), b"oo");
    assert_eq!(s[..2].as_bytes(), b"fo");
    assert_eq!(s[1..2].as_bytes(), b"o");
}
#[test]
fn test_ow8_slices_utf8() {
    let s = OmgWtf8::from_str("測試文字");
    assert_eq!(
        s[..].as_bytes(),
        b"\xe6\xb8\xac\xe8\xa9\xa6\xe6\x96\x87\xe5\xad\x97"
    );
    assert_eq!(s[3..].as_bytes(), b"\xe8\xa9\xa6\xe6\x96\x87\xe5\xad\x97");
    assert_eq!(s[..6].as_bytes(), b"\xe6\xb8\xac\xe8\xa9\xa6");
    assert_eq!(s[3..9].as_bytes(), b"\xe8\xa9\xa6\xe6\x96\x87");
}
#[test]
fn test_ow8_slices_valid() {
    let s = unsafe {
        OmgWtf8::from_bytes_unchecked(b"\x90\x81\x81\xed\xb1\x81\xed\xa0\x80\xf0\x90\x81")
    };
    assert_eq!(
        s[..].as_bytes(),
        b"\x90\x81\x81\xed\xb1\x81\xed\xa0\x80\xf0\x90\x81"
    );
    assert_eq!(s[3..].as_bytes(), b"\xed\xb1\x81\xed\xa0\x80\xf0\x90\x81");
    assert_eq!(s[..6].as_bytes(), b"\x90\x81\x81\xed\xb1\x81");
    assert_eq!(s[3..9].as_bytes(), b"\xed\xb1\x81\xed\xa0\x80");
}
#[test]
fn test_ow8_slices_split() {
    let s = OmgWtf8::from_str("😀😂😄");
    assert_eq!(
        s[..].as_bytes(),
        b"\xf0\x9f\x98\x80\xf0\x9f\x98\x82\xf0\x9f\x98\x84"
    );
    assert_eq!(
        s[2..].as_bytes(),
        b"\x9f\x98\x80\xf0\x9f\x98\x82\xf0\x9f\x98\x84"
    );
    assert_eq!(s[4..].as_bytes(), b"\xf0\x9f\x98\x82\xf0\x9f\x98\x84");
    assert_eq!(
        s[..10].as_bytes(),
        b"\xf0\x9f\x98\x80\xf0\x9f\x98\x82\xf0\x9f\x98"
    );
    assert_eq!(s[..8].as_bytes(), b"\xf0\x9f\x98\x80\xf0\x9f\x98\x82");
    assert_eq!(
        s[2..10].as_bytes(),
        b"\x9f\x98\x80\xf0\x9f\x98\x82\xf0\x9f\x98"
    );
    assert_eq!(s[4..8].as_bytes(), b"\xf0\x9f\x98\x82");
    assert_eq!(s[2..4].as_bytes(), b"\x9f\x98\x80");
    assert_eq!(s[2..2].as_bytes(), b"");
    assert_eq!(s[0..2].as_bytes(), b"\xf0\x9f\x98");
    assert_eq!(s[4..4].as_bytes(), b"");
}
#[test]
#[should_panic]
fn test_slice_into_invalid_index_split_begin_1() {
    let s = unsafe { OmgWtf8::from_bytes_unchecked(b"\x90\x80\x80\x7e") };
    let _ = s[..1];
}
#[test]
#[should_panic]
fn test_slice_into_invalid_index_split_begin_2() {
    let s = unsafe { OmgWtf8::from_bytes_unchecked(b"\x90\x80\x80\x7e") };
    let _ = s[..2];
}
#[test]
#[should_panic]
fn test_slice_into_invalid_index_split_end_1() {
    let s = unsafe { OmgWtf8::from_bytes_unchecked(b"\x7e\xf0\x90\x80") };
    let _ = s[2..];
}
#[test]
#[should_panic]
fn test_slice_into_invalid_index_split_end_2() {
    let s = unsafe { OmgWtf8::from_bytes_unchecked(b"\x7e\xf0\x90\x80") };
    let _ = s[3..];
}
#[test]
#[should_panic]
fn test_slice_into_invalid_index_canonical_1() {
    let s = unsafe { OmgWtf8::from_bytes_unchecked(b"\xed\xaf\xbf") };
    let _ = s[1..];
}
#[test]
#[should_panic]
fn test_slice_into_invalid_index_canonical_2() {
    let s = unsafe { OmgWtf8::from_bytes_unchecked(b"\xed\xaf\xbf") };
    let _ = s[2..];
}
#[test]
#[should_panic]
fn test_slice_into_invalid_index_wrong_order() {
    let s = OmgWtf8::from_str("12345");
    let _ = s[3..1];
}
