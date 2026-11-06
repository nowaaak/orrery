const MAX_DEPTH: u32 = 512;

#[derive(Debug, Clone, PartialEq)]
pub enum Tag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(u8, Vec<Tag>),
    Compound(Vec<(String, Tag)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Tag {
    pub fn id(&self) -> u8 {
        match self {
            Self::Byte(_) => 1,
            Self::Short(_) => 2,
            Self::Int(_) => 3,
            Self::Long(_) => 4,
            Self::Float(_) => 5,
            Self::Double(_) => 6,
            Self::ByteArray(_) => 7,
            Self::String(_) => 8,
            Self::List(..) => 9,
            Self::Compound(_) => 10,
            Self::IntArray(_) => 11,
            Self::LongArray(_) => 12,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Tag> {
        match self {
            Self::Compound(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, value: Tag) {
        if let Self::Compound(entries) = self {
            match entries.iter_mut().find(|(k, _)| k == key) {
                Some(entry) => entry.1 = value,
                None => entries.push((key.to_string(), value)),
            }
        }
    }

    pub fn remove(&mut self, key: &str) {
        if let Self::Compound(entries) = self {
            entries.retain(|(k, _)| k != key);
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Byte(v) => Some(i64::from(*v)),
            Self::Short(v) => Some(i64::from(*v)),
            Self::Int(v) => Some(i64::from(*v)),
            Self::Long(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Tag]> {
        match self {
            Self::List(_, items) => Some(items),
            _ => None,
        }
    }
}

pub fn read(data: &[u8]) -> Option<(String, Tag)> {
    let mut reader = Reader { data, pos: 0 };
    let tag = reader.byte()?;
    if tag != 10 {
        return None;
    }
    let name = reader.string()?;
    let root = reader.payload(tag, 0)?;
    Some((name, root))
}

pub fn write(name: &str, root: &Tag) -> Vec<u8> {
    let mut out = vec![root.id()];
    write_string(&mut out, name);
    write_payload(&mut out, root);
    out
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(len)?;
        let slice = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    fn array<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.take(N)?.try_into().ok()
    }

    fn byte(&mut self) -> Option<u8> {
        Some(self.array::<1>()?[0])
    }

    fn length(&mut self) -> Option<usize> {
        let len = usize::try_from(i32::from_be_bytes(self.array()?)).ok()?;
        (len <= self.data.len() - self.pos).then_some(len)
    }

    fn string(&mut self) -> Option<String> {
        let len = usize::from(u16::from_be_bytes(self.array()?));
        Some(decode_mutf8(self.take(len)?))
    }

    fn payload(&mut self, tag: u8, depth: u32) -> Option<Tag> {
        if depth > MAX_DEPTH {
            return None;
        }
        Some(match tag {
            1 => Tag::Byte(i8::from_be_bytes(self.array()?)),
            2 => Tag::Short(i16::from_be_bytes(self.array()?)),
            3 => Tag::Int(i32::from_be_bytes(self.array()?)),
            4 => Tag::Long(i64::from_be_bytes(self.array()?)),
            5 => Tag::Float(f32::from_be_bytes(self.array()?)),
            6 => Tag::Double(f64::from_be_bytes(self.array()?)),
            7 => {
                let len = self.length()?;
                Tag::ByteArray(self.take(len)?.iter().map(|b| *b as i8).collect())
            }
            8 => Tag::String(self.string()?),
            9 => {
                let element = self.byte()?;
                let len = self.length()?;
                if element == 0 && len > 0 {
                    return None;
                }
                let mut items = Vec::with_capacity(len.min(1024));
                for _ in 0..len {
                    items.push(self.payload(element, depth + 1)?);
                }
                Tag::List(element, items)
            }
            10 => {
                let mut entries = Vec::new();
                loop {
                    let child = self.byte()?;
                    if child == 0 {
                        break;
                    }
                    let name = self.string()?;
                    entries.push((name, self.payload(child, depth + 1)?));
                }
                Tag::Compound(entries)
            }
            11 => {
                let len = self.length()?;
                let bytes = self.take(len.checked_mul(4)?)?;
                Tag::IntArray(
                    bytes
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|c| i32::from_be_bytes(*c))
                        .collect(),
                )
            }
            12 => {
                let len = self.length()?;
                let bytes = self.take(len.checked_mul(8)?)?;
                Tag::LongArray(
                    bytes
                        .as_chunks::<8>()
                        .0
                        .iter()
                        .map(|c| i64::from_be_bytes(*c))
                        .collect(),
                )
            }
            _ => return None,
        })
    }
}

fn write_len(out: &mut Vec<u8>, len: usize) {
    out.extend(i32::try_from(len).unwrap_or(i32::MAX).to_be_bytes());
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    let mut encoded = encode_mutf8(value);
    if encoded.len() > usize::from(u16::MAX) {
        let mut cut = usize::from(u16::MAX);
        while cut > 0 && encoded[cut] & 0xC0 == 0x80 {
            cut -= 1;
        }
        encoded.truncate(cut);
    }
    out.extend((encoded.len() as u16).to_be_bytes());
    out.extend(encoded);
}

fn write_payload(out: &mut Vec<u8>, tag: &Tag) {
    match tag {
        Tag::Byte(v) => out.extend(v.to_be_bytes()),
        Tag::Short(v) => out.extend(v.to_be_bytes()),
        Tag::Int(v) => out.extend(v.to_be_bytes()),
        Tag::Long(v) => out.extend(v.to_be_bytes()),
        Tag::Float(v) => out.extend(v.to_be_bytes()),
        Tag::Double(v) => out.extend(v.to_be_bytes()),
        Tag::ByteArray(values) => {
            write_len(out, values.len());
            out.extend(values.iter().map(|v| *v as u8));
        }
        Tag::String(value) => write_string(out, value),
        Tag::List(element, items) => {
            let element = items.first().map_or(*element, Tag::id);
            out.push(if items.is_empty() { 0 } else { element });
            write_len(out, items.len());
            for item in items {
                write_payload(out, item);
            }
        }
        Tag::Compound(entries) => {
            for (name, value) in entries {
                out.push(value.id());
                write_string(out, name);
                write_payload(out, value);
            }
            out.push(0);
        }
        Tag::IntArray(values) => {
            write_len(out, values.len());
            for v in values {
                out.extend(v.to_be_bytes());
            }
        }
        Tag::LongArray(values) => {
            write_len(out, values.len());
            for v in values {
                out.extend(v.to_be_bytes());
            }
        }
    }
}

fn decode_mutf8(bytes: &[u8]) -> String {
    let mut units: Vec<u16> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let continuation = |offset: usize| {
            bytes
                .get(i + offset)
                .filter(|c| *c & 0xC0 == 0x80)
                .map(|c| u32::from(c & 0x3F))
        };
        let (code, width) = if b < 0x80 {
            (u32::from(b), 1)
        } else if b & 0xE0 == 0xC0 {
            match continuation(1) {
                Some(c1) => ((u32::from(b & 0x1F) << 6) | c1, 2),
                None => (0xFFFD, 1),
            }
        } else if b & 0xF0 == 0xE0 {
            match (continuation(1), continuation(2)) {
                (Some(c1), Some(c2)) => ((u32::from(b & 0x0F) << 12) | (c1 << 6) | c2, 3),
                _ => (0xFFFD, 1),
            }
        } else if b & 0xF8 == 0xF0 {
            match (continuation(1), continuation(2), continuation(3)) {
                (Some(c1), Some(c2), Some(c3)) => {
                    ((u32::from(b & 0x07) << 18) | (c1 << 12) | (c2 << 6) | c3, 4)
                }
                _ => (0xFFFD, 1),
            }
        } else {
            (0xFFFD, 1)
        };
        match char::from_u32(code) {
            Some(c) if code > 0xFFFF => {
                let mut buf = [0u16; 2];
                units.extend_from_slice(c.encode_utf16(&mut buf));
            }
            _ => units.push(u16::try_from(code).unwrap_or(0xFFFD)),
        }
        i += width;
    }
    String::from_utf16_lossy(&units)
}

fn encode_mutf8(value: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(value.len());
    for unit in value.encode_utf16() {
        let unit = u32::from(unit);
        match unit {
            0x01..=0x7F => out.push(unit as u8),
            0x00 | 0x80..=0x7FF => {
                out.push(0xC0 | (unit >> 6) as u8);
                out.push(0x80 | (unit & 0x3F) as u8);
            }
            _ => {
                out.push(0xE0 | (unit >> 12) as u8);
                out.push(0x80 | ((unit >> 6) & 0x3F) as u8);
                out.push(0x80 | (unit & 0x3F) as u8);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Tag {
        Tag::Compound(vec![
            ("byte".into(), Tag::Byte(-1)),
            ("short".into(), Tag::Short(300)),
            ("int".into(), Tag::Int(-70000)),
            ("long".into(), Tag::Long(1 << 40)),
            ("float".into(), Tag::Float(1.5)),
            ("double".into(), Tag::Double(-2.25)),
            ("bytes".into(), Tag::ByteArray(vec![1, -2, 3])),
            ("text".into(), Tag::String("Grüße 🚀\0 end".into())),
            (
                "list".into(),
                Tag::List(10, vec![Tag::Compound(vec![("a".into(), Tag::Int(1))])]),
            ),
            ("empty".into(), Tag::List(0, Vec::new())),
            ("ints".into(), Tag::IntArray(vec![1, -1])),
            ("longs".into(), Tag::LongArray(vec![i64::MIN, 7])),
        ])
    }

    #[test]
    fn roundtrips_every_tag() {
        let bytes = write("root", &sample());
        let (name, tag) = read(&bytes).unwrap();
        assert_eq!(name, "root");
        assert_eq!(tag, sample());
        assert_eq!(write("root", &tag), bytes);
    }

    #[test]
    fn uses_java_modified_utf8() {
        assert_eq!(encode_mutf8("\0"), [0xC0, 0x80]);
        let rocket = encode_mutf8("🚀");
        assert_eq!(rocket.len(), 6);
        assert_eq!(decode_mutf8(&rocket), "🚀");
        assert_eq!(decode_mutf8("🚀".as_bytes()), "🚀");
        assert_eq!(decode_mutf8(&[0xFF, b'a']), "\u{FFFD}a");
    }

    #[test]
    fn edits_compounds() {
        let mut tag = sample();
        tag.set("int", Tag::Int(5));
        tag.set("new", Tag::String("x".into()));
        tag.remove("byte");
        assert_eq!(tag.get("int").and_then(Tag::as_i64), Some(5));
        assert_eq!(tag.get("new").and_then(Tag::as_str), Some("x"));
        assert!(tag.get("byte").is_none());
    }

    #[test]
    fn rejects_truncated_and_hostile_input() {
        let bytes = write("", &sample());
        for len in [0, 1, 3, bytes.len() / 2, bytes.len() - 1] {
            assert!(read(&bytes[..len]).is_none(), "{len}");
        }
        assert!(read(&[10, 0, 0, 7, 0, 1, b'a', 0x7F, 0xFF, 0xFF, 0xFF]).is_none());
        assert!(read(&[8, 0, 0, 0, 0]).is_none());
        let mut deep = vec![10, 0, 0];
        for _ in 0..600 {
            deep.extend([10, 0, 1, b'a']);
        }
        assert!(read(&deep).is_none());
    }
}
