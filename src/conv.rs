use OmgWtf8;
use std::str::from_utf8;
use std::fmt;

/// Represents a 3-byte sequence as part of a well-formed OMG-WTF-8 sequence.
///
/// Internally, the sequence is encoded as a big-endian integer to simplify
/// computation.
pub(crate) struct ThreeByteSeq(u32);
impl ThreeByteSeq {
    /// Canonicalizes the 3-byte sequence.
    ///
    /// If the input can never be a surrogate, returns 0.
    ///
    /// If the input is a high surrogate, returns a value in the range
    /// `0xa000 ..= 0xafff`.
    ///
    /// If the input is a low surrogate, returns a value in the range
    /// `0xb000 ..= 0xbfff`.
    ///
    /// The nonzero values are the last 2 bytes of the surrogate in its
    /// canonical representation.
    pub(crate) fn canonicalize(self) -> u16 {
        (match self.0 {
            0xeda000...0xedffff => self.0,
            0x800000...0xbfffff => self.0 | 0xb000,
            0xf00000...0xffffffff => {
                ((self.0 >> 4 & 0x303 | self.0 >> 6 & 0x1c3c) - 0x100) | 0xa080
            }
            _ => 0,
        }) as u16
    }

    /// Extracts a WTF-16 code unit from the 3-byte sequence.
    pub(crate) fn as_code_unit(self) -> u16 {
        (match self.0 {
            0xf00000...0xffffffff => {
                (self.0 >> 4 & 3 | self.0 >> 6 & 0xfc | self.0 >> 8 & 0x700) + 0xd7c0
            }
            0x800000...0xbfffff => self.0 & 0x3f | self.0 >> 2 & 0x3c0 | 0xdc00,
            _ => self.0 & 0x3f | self.0 >> 2 & 0xfc0 | self.0 >> 4 & 0xf000,
        }) as u16
    }

    /// Constructs a 3-byte sequence from the bytes.
    pub(crate) fn new(input: &[u8]) -> Self {
        ThreeByteSeq((input[0] as u32) << 16 | (input[1] as u32) << 8 | (input[2] as u32))
    }
}

#[test]
fn test_3bs_canonicalize() {
    fn canonicalize(a: u32) -> u16 {
        ThreeByteSeq(a).canonicalize()
    }

    assert_eq!(canonicalize(0x303030), 0); // '000'
    assert_eq!(canonicalize(0xed9fbf), 0); // U+D7FF
    assert_eq!(canonicalize(0xee8080), 0); // U+E000
    assert_eq!(canonicalize(0xeda080), 0xa080); // U+D800
    assert_eq!(canonicalize(0xedafbf), 0xafbf); // U+DBFF
    assert_eq!(canonicalize(0xedb080), 0xb080); // U+DC00
    assert_eq!(canonicalize(0xedbfbf), 0xbfbf); // U+DFFF
    assert_eq!(canonicalize(0xf09080), 0xa080); // U+10000, high
    assert_eq!(canonicalize(0x908080), 0xb080); // U+10000, low
    assert_eq!(canonicalize(0xf48fbf), 0xafbf); // U+10FFFF, high
    assert_eq!(canonicalize(0x8fbfbf), 0xbfbf); // U+10FFFF, low

    // U+69A03 = D966 DE03 (ed a5 a6 ed b8 83) = f1 a9 a8 83
    assert_eq!(canonicalize(0xeda5a6), 0xa5a6);
    assert_eq!(canonicalize(0xedb883), 0xb883);
    assert_eq!(canonicalize(0xf1a9a8), 0xa5a6);
    assert_eq!(canonicalize(0xa9a883), 0xb883);
}

impl OmgWtf8 {
    /// Creates a new OMG-WTF-8 string from a UTF-8 string.
    pub fn from_str(s: &str) -> &Self {
        unsafe { Self::from_bytes_unchecked(s.as_bytes()) }
    }

    /// Creates a new OMG-WTF-8 string from raw bytes without checking for
    /// well-formed-ness.
    pub(crate) unsafe fn from_bytes_unchecked(s: &[u8]) -> &Self {
        &*(s as *const [u8] as *const Self)
    }

    #[cfg(test)]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// If this string is valid UTF-8, returns this string cast to a `&str`.
    ///
    /// If this string contains unpaired surrogates, returns `None`.
    pub fn to_str(&self) -> Option<&str> {
        from_utf8(&self.0).ok()
    }

    /// Converts from UCS-2 to OMG-WTF-8.
    pub fn from_wide(ucs2: &[u16]) -> Box<Self> {
        let mut buf = Vec::with_capacity(ucs2.len());
        let mut it = ucs2.iter().fuse().cloned();
        'outer: while let Some(mut c1) = it.next() {
            if let 0xd800...0xdbff = c1 {
                // we've got a high surrogate. check if it is followed by a
                // low surrogate.
                while let Some(c2) = it.next() {
                    match c2 {
                        0xd800...0xdbff => {
                            // we've got another high surrogate, keep checking
                            encode_unit(&mut buf, c1);
                            c1 = c2;
                        }
                        0xdc00...0xdfff => {
                            // we've got a low surrogate, write a 4-byte sequence.
                            let c = ((c1 as u32 & 0x3ff) << 10 | (c2 as u32 & 0x3ff)) + 0x1_0000;
                            buf.push((c >> 18 | 0xf0) as u8);
                            buf.push((c >> 12 & 0x3f | 0x80) as u8);
                            buf.push((c >> 6 & 0x3f | 0x80) as u8);
                            buf.push((c & 0x3f | 0x80) as u8);
                            continue 'outer;
                        }
                        _ => {
                            // we've got an unpaired surrogate.
                            encode_unit(&mut buf, c1);
                            encode_unit(&mut buf, c2);
                            continue 'outer;
                        }
                    }
                }
            }
            encode_unit(&mut buf, c1);
        }

        unsafe { Box::from_raw(Box::into_raw(buf.into_boxed_slice()) as *mut Self) }
    }

    pub fn encode_wide(&self) -> EncodeWide {
        EncodeWide {
            src: &self.0,
            low_surrogate: None,
        }
    }
}

impl<'a> From<&'a str> for &'a OmgWtf8 {
    fn from(s: &'a str) -> &'a OmgWtf8 {
        OmgWtf8::from_str(s)
    }
}
impl AsRef<OmgWtf8> for str {
    fn as_ref(&self) -> &OmgWtf8 {
        OmgWtf8::from_str(self)
    }
}

impl fmt::Debug for OmgWtf8 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "OmgWtf8(b\"")?;
        for byte in &self.0 {
            write!(fmt, "\\x{:02x}", byte)?;
        }
        write!(fmt, "\")")?;
        Ok(())
    }
}

impl<'a> From<&'a OmgWtf8> for Box<OmgWtf8> {
    fn from(s: &'a OmgWtf8) -> Box<OmgWtf8> {
        let mut boxed_slice = Box::<[u8]>::from(&s.0);
        let len = boxed_slice.len();
        if len >= 3 {
            if let 0x80...0xbf = boxed_slice[0] {
                let c = ThreeByteSeq::new(&boxed_slice).canonicalize();
                boxed_slice[0] = 0xed;
                boxed_slice[1] = (c >> 8) as u8;
                boxed_slice[2] = c as u8;
            }
            if let 0xf0...0xff = boxed_slice[len - 3] {
                let c = ThreeByteSeq::new(&boxed_slice[(len - 3)..]).canonicalize();
                boxed_slice[len - 3] = 0xed;
                boxed_slice[len - 2] = (c >> 8) as u8;
                boxed_slice[len - 1] = c as u8;
            }
        }
        unsafe { Box::from_raw(Box::into_raw(boxed_slice) as *mut OmgWtf8) }
    }
}

fn encode_unit(buf: &mut Vec<u8>, c: u16) {
    match c {
        0...0x7f => {
            buf.push(c as u8);
        }
        0x80...0x7ff => {
            buf.push((c >> 6 | 0xc0) as u8);
            buf.push((c & 0x3f | 0x80) as u8);
        }
        _ => {
            buf.push((c >> 12 | 0xe0) as u8);
            buf.push((c >> 6 & 0x3f | 0x80) as u8);
            buf.push((c & 0x3f | 0x80) as u8);
        }
    }
}

pub struct EncodeWide<'a> {
    src: &'a [u8],
    low_surrogate: Option<u16>,
}

impl<'a> Iterator for EncodeWide<'a> {
    type Item = u16;
    fn next(&mut self) -> Option<u16> {
        if let Some(c) = self.low_surrogate.take() {
            return Some(c);
        }
        if self.src.is_empty() {
            return None;
        }

        let b1 = self.src[0];
        let (consume_len, code_unit) = match b1 {
            0...0x7f => (1, b1 as u16),
            0xc0...0xdf => {
                // 2-byte sequence
                let b1 = b1 as u16;
                let b2 = self.src[1] as u16;
                (2, (b1 & 0x1f) << 6 | (b2 & 0x3f))
            }
            0xf0...0xff if self.src.len() >= 4 => {
                // 4-byte sequence
                let b1 = b1 as u32;
                let b2 = self.src[1] as u32;
                let b3 = self.src[2] as u32;
                let b4 = self.src[3] as u32;
                let d = (b1 & 7) << 18 | (b2 & 0x3f) << 12 | (b3 & 0x3f) << 6 | (b4 & 0x3f);
                let d = d - 0x1_0000;
                let c1 = ((d >> 10) & 0x3ff | 0xd800) as u16;
                let c2 = (d & 0x3ff | 0xdc00) as u16;
                self.low_surrogate = Some(c2);
                (4, c1)
            }
            _ => (3, ThreeByteSeq::new(self.src).as_code_unit()),
        };
        self.src = &self.src[consume_len..];
        Some(code_unit)
    }
}

#[test]
fn test_to_str() {
    let s = OmgWtf8::from_str("😁😃😅");
    assert_eq!(s.to_str(), Some("😁😃😅"));
    assert_eq!(s[4..].to_str(), Some("😃😅"));
    assert_eq!(s[2..].to_str(), None);
    assert_eq!(s[..10].to_str(), None);
}

#[test]
fn test_from_wide() {
    assert_eq!(OmgWtf8::from_wide(&[0x41]).as_bytes(), b"\x41");
    assert_eq!(OmgWtf8::from_wide(&[0x500]).as_bytes(), b"\xd4\x80");
    assert_eq!(OmgWtf8::from_wide(&[0x91aa]).as_bytes(), b"\xe9\x86\xaa");
    assert_eq!(OmgWtf8::from_wide(&[0xffff]).as_bytes(), b"\xef\xbf\xbf");
    assert_eq!(OmgWtf8::from_wide(&[0xd888]).as_bytes(), b"\xed\xa2\x88");
    assert_eq!(OmgWtf8::from_wide(&[0xdddd]).as_bytes(), b"\xed\xb7\x9d");
    assert_eq!(
        OmgWtf8::from_wide(&[1, 0xd888, 2]).as_bytes(),
        b"\x01\xed\xa2\x88\x02"
    );
    assert_eq!(
        OmgWtf8::from_wide(&[1, 0xdddd, 2]).as_bytes(),
        b"\x01\xed\xb7\x9d\x02"
    );
    assert_eq!(
        OmgWtf8::from_wide(&[0xd888, 0xd888, 0xd888]).as_bytes(),
        b"\xed\xa2\x88\xed\xa2\x88\xed\xa2\x88",
    );
    assert_eq!(
        OmgWtf8::from_wide(&[0xd888, 0xdddd]).as_bytes(), // U+321DD
        b"\xf0\xb2\x87\x9d",
    );
    assert_eq!(
        OmgWtf8::from_wide(&[0xdddd, 0xd888, 0xdddd, 0xd888]).as_bytes(),
        b"\xed\xb7\x9d\xf0\xb2\x87\x9d\xed\xa2\x88",
    );
    assert_eq!(
        OmgWtf8::from_wide(&[0xd888, 0xd888, 0xdddd, 0xdddd]).as_bytes(),
        b"\xed\xa2\x88\xf0\xb2\x87\x9d\xed\xb7\x9d",
    );
}

#[test]
fn test_encode_wide() {
    assert_eq!(
        OmgWtf8::from_str("abc").encode_wide().collect::<Vec<_>>(),
        vec![0x61, 0x62, 0x63],
    );
    assert_eq!(
        OmgWtf8::from_str("測試文字")
            .encode_wide()
            .collect::<Vec<_>>(),
        vec![0x6e2c, 0x8a66, 0x6587, 0x5b57],
    );
    assert_eq!(
        OmgWtf8::from_str("😊😚🙃")
            .encode_wide()
            .collect::<Vec<_>>(),
        vec![0xd83d, 0xde0a, 0xd83d, 0xde1a, 0xd83d, 0xde43],
    );
    assert_eq!(
        unsafe { OmgWtf8::from_bytes_unchecked(b"\xed\xa2\x88\xed\xa2\x88\xed\xa2\x88") }
            .encode_wide()
            .collect::<Vec<_>>(),
        vec![0xd888, 0xd888, 0xd888],
    );
    assert_eq!(
        unsafe { OmgWtf8::from_bytes_unchecked(b"\xed\xb7\x9d\xf0\xb2\x87\x9d\xed\xa2\x88") }
            .encode_wide()
            .collect::<Vec<_>>(),
        vec![0xdddd, 0xd888, 0xdddd, 0xd888],
    );
    assert_eq!(
        unsafe { OmgWtf8::from_bytes_unchecked(b"\xb2\x87\x9d\xf0\xb2\x87\x9d\xf0\xb2\x87") }
            .encode_wide()
            .collect::<Vec<_>>(),
        vec![0xdddd, 0xd888, 0xdddd, 0xd888],
    );
}

#[test]
fn test_boxing_should_canonicalize() {
    assert_eq!(
        Box::<OmgWtf8>::from(OmgWtf8::from_str("abc")).as_bytes(),
        b"abc",
    );
    assert_eq!(
        Box::<OmgWtf8>::from(OmgWtf8::from_str("測試😊")).as_bytes(),
        b"\xe6\xb8\xac\xe8\xa9\xa6\xf0\x9f\x98\x8a",
    );
    assert_eq!(
        Box::<OmgWtf8>::from(unsafe {
            OmgWtf8::from_bytes_unchecked(b"\xb2\x87\x9d\xf0\xb2\x87\x9d\xf0\xb2\x87")
        }).as_bytes(),
        b"\xed\xb7\x9d\xf0\xb2\x87\x9d\xed\xa2\x88",
    );
}
