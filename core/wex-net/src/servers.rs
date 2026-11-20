use std::fs;
use std::io::{self, Cursor, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerListError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Corrupted NBT data: {0}")]
    Nbt(String),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
}

/// A server entry loaded from pack config (such as servers.json.example or manifest.json)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackServerEntry {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(alias = "address", alias = "ip")]
    pub ip: String,
    #[serde(default)]
    pub icon: Option<String>,
}

// ---------------------------------------------------------------------------
// NBT Representation & Serialization (Uncompressed Big-Endian)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum NbtTag {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    List { tag_id: u8, elements: Vec<NbtTag> },
    Compound(Vec<(String, NbtTag)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl NbtTag {
    pub fn type_id(&self) -> u8 {
        match self {
            NbtTag::End => 0,
            NbtTag::Byte(_) => 1,
            NbtTag::Short(_) => 2,
            NbtTag::Int(_) => 3,
            NbtTag::Long(_) => 4,
            NbtTag::Float(_) => 5,
            NbtTag::Double(_) => 6,
            NbtTag::ByteArray(_) => 7,
            NbtTag::String(_) => 8,
            NbtTag::List { .. } => 9,
            NbtTag::Compound(_) => 10,
            NbtTag::IntArray(_) => 11,
            NbtTag::LongArray(_) => 12,
        }
    }

    pub fn get(&self, key: &str) -> Option<&NbtTag> {
        match self {
            NbtTag::Compound(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, tag)| tag),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut NbtTag> {
        match self {
            NbtTag::Compound(entries) => entries
                .iter_mut()
                .find(|(k, _)| k == key)
                .map(|(_, tag)| tag),
            _ => None,
        }
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(NbtTag::String(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, new_tag: NbtTag) {
        if let NbtTag::Compound(entries) = self {
            if let Some(pos) = entries.iter().position(|(k, _)| k == key) {
                entries[pos].1 = new_tag;
            } else {
                entries.push((key.to_string(), new_tag));
            }
        }
    }
}

/// Maximum nesting of lists/compounds accepted while reading (Minecraft itself allows 512).
const MAX_NBT_DEPTH: usize = 512;
/// Upper bound for up-front allocation; real data grows the vector as it is read.
const MAX_PREALLOC: usize = 4096;

pub struct NbtReader<R> {
    reader: R,
    depth: usize,
}

impl<R: Read> NbtReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader, depth: 0 }
    }

    pub fn read_root(&mut self) -> Result<(String, NbtTag), ServerListError> {
        let mut type_byte = [0u8; 1];
        if self.reader.read_exact(&mut type_byte).is_err() {
            return Err(ServerListError::Nbt("Empty NBT stream".to_string()));
        }

        if type_byte[0] != 10 {
            return Err(ServerListError::Nbt(format!(
                "Expected root compound (10), got {}",
                type_byte[0]
            )));
        }

        let name = self.read_string()?;
        let tag = self.read_compound_payload()?;
        Ok((name, tag))
    }

    fn read_tag_payload(&mut self, tag_id: u8) -> Result<NbtTag, ServerListError> {
        match tag_id {
            0 => Ok(NbtTag::End),
            1 => {
                let mut b = [0u8; 1];
                self.reader.read_exact(&mut b)?;
                Ok(NbtTag::Byte(b[0] as i8))
            }
            2 => {
                let mut b = [0u8; 2];
                self.reader.read_exact(&mut b)?;
                Ok(NbtTag::Short(i16::from_be_bytes(b)))
            }
            3 => {
                let mut b = [0u8; 4];
                self.reader.read_exact(&mut b)?;
                Ok(NbtTag::Int(i32::from_be_bytes(b)))
            }
            4 => {
                let mut b = [0u8; 8];
                self.reader.read_exact(&mut b)?;
                Ok(NbtTag::Long(i64::from_be_bytes(b)))
            }
            5 => {
                let mut b = [0u8; 4];
                self.reader.read_exact(&mut b)?;
                Ok(NbtTag::Float(f32::from_be_bytes(b)))
            }
            6 => {
                let mut b = [0u8; 8];
                self.reader.read_exact(&mut b)?;
                Ok(NbtTag::Double(f64::from_be_bytes(b)))
            }
            7 => {
                let mut len_b = [0u8; 4];
                self.reader.read_exact(&mut len_b)?;
                let len = i32::from_be_bytes(len_b);
                if len < 0 {
                    return Err(ServerListError::Nbt(format!(
                        "Negative ByteArray len: {len}"
                    )));
                }
                // Do not trust the header: read at most `len` bytes and verify they exist.
                let mut bytes = Vec::new();
                (&mut self.reader)
                    .take(len as u64)
                    .read_to_end(&mut bytes)?;
                if bytes.len() != len as usize {
                    return Err(ServerListError::Nbt("Truncated ByteArray".to_string()));
                }
                Ok(NbtTag::ByteArray(bytes))
            }
            8 => Ok(NbtTag::String(self.read_string()?)),
            9 => {
                let mut item_type_b = [0u8; 1];
                self.reader.read_exact(&mut item_type_b)?;
                let item_type = item_type_b[0];

                let mut len_b = [0u8; 4];
                self.reader.read_exact(&mut len_b)?;
                let len = i32::from_be_bytes(len_b);

                if len <= 0 {
                    return Ok(NbtTag::List {
                        tag_id: item_type,
                        elements: Vec::new(),
                    });
                }

                if self.depth >= MAX_NBT_DEPTH {
                    return Err(ServerListError::Nbt("NBT nesting too deep".to_string()));
                }
                self.depth += 1;
                let mut elements = Vec::with_capacity((len as usize).min(MAX_PREALLOC));
                for _ in 0..len {
                    match self.read_tag_payload(item_type) {
                        Ok(tag) => elements.push(tag),
                        Err(err) => {
                            self.depth -= 1;
                            return Err(err);
                        }
                    }
                }
                self.depth -= 1;
                Ok(NbtTag::List {
                    tag_id: item_type,
                    elements,
                })
            }
            10 => self.read_compound_payload(),
            11 => {
                let mut len_b = [0u8; 4];
                self.reader.read_exact(&mut len_b)?;
                let len = i32::from_be_bytes(len_b);
                if len < 0 {
                    return Err(ServerListError::Nbt(format!(
                        "Negative IntArray len: {len}"
                    )));
                }
                let mut elements = Vec::with_capacity((len as usize).min(MAX_PREALLOC));
                for _ in 0..len {
                    let mut b = [0u8; 4];
                    self.reader.read_exact(&mut b)?;
                    elements.push(i32::from_be_bytes(b));
                }
                Ok(NbtTag::IntArray(elements))
            }
            12 => {
                let mut len_b = [0u8; 4];
                self.reader.read_exact(&mut len_b)?;
                let len = i32::from_be_bytes(len_b);
                if len < 0 {
                    return Err(ServerListError::Nbt(format!(
                        "Negative LongArray len: {len}"
                    )));
                }
                let mut elements = Vec::with_capacity((len as usize).min(MAX_PREALLOC));
                for _ in 0..len {
                    let mut b = [0u8; 8];
                    self.reader.read_exact(&mut b)?;
                    elements.push(i64::from_be_bytes(b));
                }
                Ok(NbtTag::LongArray(elements))
            }
            other => Err(ServerListError::Nbt(format!(
                "Unknown NBT tag type: {other}"
            ))),
        }
    }

    fn read_compound_payload(&mut self) -> Result<NbtTag, ServerListError> {
        if self.depth >= MAX_NBT_DEPTH {
            return Err(ServerListError::Nbt("NBT nesting too deep".to_string()));
        }
        self.depth += 1;
        let result = self.read_compound_entries();
        self.depth -= 1;
        result
    }

    fn read_compound_entries(&mut self) -> Result<NbtTag, ServerListError> {
        let mut entries = Vec::new();
        loop {
            let mut tag_byte = [0u8; 1];
            self.reader.read_exact(&mut tag_byte)?;
            let tag_id = tag_byte[0];
            if tag_id == 0 {
                break; // TAG_End
            }
            let name = self.read_string()?;
            let tag = self.read_tag_payload(tag_id)?;
            entries.push((name, tag));
        }
        Ok(NbtTag::Compound(entries))
    }

    fn read_string(&mut self) -> Result<String, ServerListError> {
        let mut len_b = [0u8; 2];
        self.reader.read_exact(&mut len_b)?;
        let len = u16::from_be_bytes(len_b) as usize;
        let mut bytes = vec![0u8; len];
        self.reader.read_exact(&mut bytes)?;
        Ok(decode_modified_utf8(&bytes))
    }
}

/// Decodes Java "modified UTF-8" (NUL as C0 80, supplementary characters as
/// surrogate pairs of two 3-byte sequences). Falls back to lossy standard UTF-8.
fn decode_modified_utf8(bytes: &[u8]) -> String {
    fn decode(bytes: &[u8]) -> Option<String> {
        let mut units: Vec<u16> = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b < 0x80 {
                units.push(b as u16);
                i += 1;
            } else if b & 0xE0 == 0xC0 {
                let c = *bytes.get(i + 1)?;
                if c & 0xC0 != 0x80 {
                    return None;
                }
                units.push((((b & 0x1F) as u16) << 6) | (c & 0x3F) as u16);
                i += 2;
            } else if b & 0xF0 == 0xE0 {
                let c = *bytes.get(i + 1)?;
                let d = *bytes.get(i + 2)?;
                if c & 0xC0 != 0x80 || d & 0xC0 != 0x80 {
                    return None;
                }
                units.push(
                    (((b & 0x0F) as u16) << 12) | (((c & 0x3F) as u16) << 6) | (d & 0x3F) as u16,
                );
                i += 3;
            } else {
                return None;
            }
        }
        String::from_utf16(&units).ok()
    }
    decode(bytes).unwrap_or_else(|| String::from_utf8_lossy(bytes).into_owned())
}

/// Encodes a string as Java "modified UTF-8".
fn encode_modified_utf8(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut buf = [0u16; 2];
    for ch in s.chars() {
        if ch == '\0' {
            out.extend_from_slice(&[0xC0, 0x80]);
            continue;
        }
        for unit in ch.encode_utf16(&mut buf) {
            let u = *unit;
            if u < 0x80 {
                out.push(u as u8);
            } else if u < 0x800 {
                out.push(0xC0 | (u >> 6) as u8);
                out.push(0x80 | (u & 0x3F) as u8);
            } else {
                out.push(0xE0 | (u >> 12) as u8);
                out.push(0x80 | ((u >> 6) & 0x3F) as u8);
                out.push(0x80 | (u & 0x3F) as u8);
            }
        }
    }
    out
}

pub struct NbtWriter<W> {
    writer: W,
}

impl<W: Write> NbtWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn write_root(&mut self, root_name: &str, tag: &NbtTag) -> Result<(), ServerListError> {
        if tag.type_id() != 10 {
            return Err(ServerListError::Nbt(
                "Root tag must be Compound (10)".to_string(),
            ));
        }
        self.writer.write_all(&[10])?;
        self.write_string(root_name)?;
        self.write_compound_payload(tag)?;
        self.writer.flush()?;
        Ok(())
    }

    fn write_tag_payload(&mut self, tag: &NbtTag) -> Result<(), ServerListError> {
        match tag {
            NbtTag::End => Ok(()),
            NbtTag::Byte(v) => Ok(self.writer.write_all(&[*v as u8])?),
            NbtTag::Short(v) => Ok(self.writer.write_all(&v.to_be_bytes())?),
            NbtTag::Int(v) => Ok(self.writer.write_all(&v.to_be_bytes())?),
            NbtTag::Long(v) => Ok(self.writer.write_all(&v.to_be_bytes())?),
            NbtTag::Float(v) => Ok(self.writer.write_all(&v.to_be_bytes())?),
            NbtTag::Double(v) => Ok(self.writer.write_all(&v.to_be_bytes())?),
            NbtTag::ByteArray(bytes) => {
                self.writer.write_all(&(bytes.len() as i32).to_be_bytes())?;
                Ok(self.writer.write_all(bytes)?)
            }
            NbtTag::String(s) => self.write_string(s),
            NbtTag::List { tag_id, elements } => {
                self.writer.write_all(&[*tag_id])?;
                self.writer
                    .write_all(&(elements.len() as i32).to_be_bytes())?;
                for el in elements {
                    self.write_tag_payload(el)?;
                }
                Ok(())
            }
            NbtTag::Compound(_) => self.write_compound_payload(tag),
            NbtTag::IntArray(ints) => {
                self.writer.write_all(&(ints.len() as i32).to_be_bytes())?;
                for i in ints {
                    self.writer.write_all(&i.to_be_bytes())?;
                }
                Ok(())
            }
            NbtTag::LongArray(longs) => {
                self.writer.write_all(&(longs.len() as i32).to_be_bytes())?;
                for l in longs {
                    self.writer.write_all(&l.to_be_bytes())?;
                }
                Ok(())
            }
        }
    }

    fn write_compound_payload(&mut self, compound: &NbtTag) -> Result<(), ServerListError> {
        if let NbtTag::Compound(entries) = compound {
            for (key, tag) in entries {
                self.writer.write_all(&[tag.type_id()])?;
                self.write_string(key)?;
                self.write_tag_payload(tag)?;
            }
            self.writer.write_all(&[0])?; // TAG_End
        }
        Ok(())
    }

    fn write_string(&mut self, s: &str) -> Result<(), ServerListError> {
        let bytes = encode_modified_utf8(s);
        let len = u16::try_from(bytes.len())
            .map_err(|_| ServerListError::Nbt("String too long for NBT".to_string()))?;
        self.writer.write_all(&len.to_be_bytes())?;
        self.writer.write_all(&bytes)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Server List Operations
// ---------------------------------------------------------------------------

/// Normalizes server address for matching (e.g. lowercase, trimmed)
pub fn normalize_address(addr: &str) -> String {
    let trimmed = addr.trim().to_lowercase();
    if !trimmed.contains(':') {
        format!("{trimmed}:25565")
    } else {
        trimmed
    }
}

/// Reads uncompressed `servers.dat` or returns a default empty compound if file does not exist.
pub fn load_servers_dat<P: AsRef<Path>>(path: P) -> Result<NbtTag, ServerListError> {
    let p = path.as_ref();
    if !p.exists() {
        return Ok(NbtTag::Compound(vec![(
            "servers".to_string(),
            NbtTag::List {
                tag_id: 10,
                elements: Vec::new(),
            },
        )]));
    }

    let bytes = fs::read(p)?;
    let mut reader = NbtReader::new(Cursor::new(bytes));
    let (_root_name, tag) = reader.read_root()?;
    Ok(tag)
}

/// Saves NBT compound to `servers.dat` atomically with backup.
pub fn save_servers_dat<P: AsRef<Path>>(
    path: P,
    root_compound: &NbtTag,
) -> Result<(), ServerListError> {
    let target_path = path.as_ref();
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write to memory first
    let mut buffer = Vec::new();
    let mut writer = NbtWriter::new(&mut buffer);
    writer.write_root("", root_compound)?;

    // Backup existing servers.dat if present
    if target_path.exists() {
        let backup_path = target_path.with_extension("dat.bak");
        let _ = fs::copy(target_path, backup_path);
    }

    let temp_path = target_path.with_extension("dat.tmp");
    fs::write(&temp_path, buffer)?;
    fs::rename(temp_path, target_path)?;

    Ok(())
}

/// Merges pack servers into existing `servers.dat` root compound.
/// Preserves all custom user servers and any unknown NBT tags.
pub fn merge_pack_servers(root_compound: &mut NbtTag, pack_servers: &[PackServerEntry]) {
    let servers_list_tag = match root_compound.get_mut("servers") {
        Some(tag) => tag,
        None => {
            root_compound.set(
                "servers",
                NbtTag::List {
                    tag_id: 10,
                    elements: Vec::new(),
                },
            );
            root_compound.get_mut("servers").unwrap()
        }
    };

    let existing_elements = match servers_list_tag {
        NbtTag::List { elements, tag_id } => {
            *tag_id = 10;
            elements
        }
        _ => return,
    };

    // Iterate through pack servers to merge
    for pack_srv in pack_servers {
        let norm_pack_addr = normalize_address(&pack_srv.ip);

        // Check if server already exists in user's list
        let mut found = false;
        for srv_tag in existing_elements.iter_mut() {
            if let Some(ip) = srv_tag.get_string("ip") {
                if normalize_address(ip) == norm_pack_addr {
                    found = true;
                    // Update name or icon if pack provides it, but preserve all other fields
                    if let Some(icon) = &pack_srv.icon {
                        srv_tag.set("icon", NbtTag::String(icon.clone()));
                    }
                    break;
                }
            }
        }

        // If not found, add as a new server entry
        if !found {
            let mut new_entry = Vec::new();
            new_entry.push(("name".to_string(), NbtTag::String(pack_srv.name.clone())));
            new_entry.push(("ip".to_string(), NbtTag::String(pack_srv.ip.clone())));
            if let Some(icon) = &pack_srv.icon {
                new_entry.push(("icon".to_string(), NbtTag::String(icon.clone())));
            }
            existing_elements.push(NbtTag::Compound(new_entry));
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nbt_roundtrip_preserves_unknown_tags() {
        let original_compound = NbtTag::Compound(vec![
            (
                "servers".to_string(),
                NbtTag::List {
                    tag_id: 10,
                    elements: vec![NbtTag::Compound(vec![
                        (
                            "name".to_string(),
                            NbtTag::String("My Vanilla Server".to_string()),
                        ),
                        (
                            "ip".to_string(),
                            NbtTag::String("vanilla.example.com".to_string()),
                        ),
                        ("acceptTextures".to_string(), NbtTag::Byte(1)),
                        ("customModdedTag".to_string(), NbtTag::Int(42)),
                        (
                            "nestedTag".to_string(),
                            NbtTag::Compound(vec![(
                                "foo".to_string(),
                                NbtTag::String("bar".to_string()),
                            )]),
                        ),
                    ])],
                },
            ),
            ("unknownRootTag".to_string(), NbtTag::Long(123456789)),
        ]);

        let mut buffer = Vec::new();
        let mut writer = NbtWriter::new(&mut buffer);
        writer.write_root("", &original_compound).unwrap();

        let mut reader = NbtReader::new(Cursor::new(buffer));
        let (_root_name, decoded) = reader.read_root().unwrap();

        assert_eq!(original_compound, decoded);
    }

    #[test]
    fn test_merge_pack_servers_no_duplicates() {
        let mut root = NbtTag::Compound(vec![(
            "servers".to_string(),
            NbtTag::List {
                tag_id: 10,
                elements: vec![
                    NbtTag::Compound(vec![
                        (
                            "name".to_string(),
                            NbtTag::String("User Existing Server".to_string()),
                        ),
                        (
                            "ip".to_string(),
                            NbtTag::String("play.auraclient.ru:25565".to_string()),
                        ),
                        ("acceptTextures".to_string(), NbtTag::Byte(1)),
                    ]),
                    NbtTag::Compound(vec![
                        ("name".to_string(), NbtTag::String("Hypixel".to_string())),
                        (
                            "ip".to_string(),
                            NbtTag::String("mc.hypixel.net".to_string()),
                        ),
                    ]),
                ],
            },
        )]);

        let pack_servers = vec![
            PackServerEntry {
                id: Some("aura-main".to_string()),
                name: "Aura Official".to_string(),
                ip: "play.auraclient.ru".to_string(),
                icon: Some("icon_base64".to_string()),
            },
            PackServerEntry {
                id: Some("aura-event".to_string()),
                name: "Aura Event".to_string(),
                ip: "event.auraclient.ru:25565".to_string(),
                icon: None,
            },
        ];

        merge_pack_servers(&mut root, &pack_servers);

        let servers = match root.get("servers").unwrap() {
            NbtTag::List { elements, .. } => elements,
            _ => panic!("Expected list"),
        };

        // Total should be 3: Existing Aura (updated), Hypixel (preserved), and Event (added)
        assert_eq!(servers.len(), 3);

        // First server kept user custom name and acceptTextures tag, but got icon updated
        let first = &servers[0];
        assert_eq!(first.get_string("name").unwrap(), "User Existing Server");
        assert_eq!(first.get("acceptTextures").unwrap(), &NbtTag::Byte(1));
        assert_eq!(first.get_string("icon").unwrap(), "icon_base64");

        // Hypixel is intact
        let second = &servers[1];
        assert_eq!(second.get_string("name").unwrap(), "Hypixel");

        // Event server added
        let third = &servers[2];
        assert_eq!(third.get_string("name").unwrap(), "Aura Event");
        assert_eq!(third.get_string("ip").unwrap(), "event.auraclient.ru:25565");
    }

    #[test]
    fn test_save_and_load_servers_dat() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("servers.dat");

        let mut root = NbtTag::Compound(vec![(
            "servers".to_string(),
            NbtTag::List {
                tag_id: 10,
                elements: vec![],
            },
        )]);

        let pack = vec![PackServerEntry {
            id: None,
            name: "Test Server".to_string(),
            ip: "127.0.0.1:25565".to_string(),
            icon: None,
        }];

        merge_pack_servers(&mut root, &pack);
        save_servers_dat(&file_path, &root).unwrap();

        let loaded = load_servers_dat(&file_path).unwrap();
        assert_eq!(root, loaded);
    }
}

#[cfg(test)]
mod robustness_tests {
    use super::*;

    fn read(bytes: &[u8]) -> Result<(String, NbtTag), ServerListError> {
        NbtReader::new(Cursor::new(bytes.to_vec())).read_root()
    }

    #[test]
    fn truncated_file_is_an_error() {
        let root = NbtTag::Compound(vec![(
            "servers".into(),
            NbtTag::List {
                tag_id: 10,
                elements: vec![NbtTag::Compound(vec![(
                    "ip".into(),
                    NbtTag::String("example.org".into()),
                )])],
            },
        )]);
        let mut buf = Vec::new();
        NbtWriter::new(&mut buf).write_root("", &root).unwrap();
        for cut in [1, 5, buf.len() / 2, buf.len() - 1] {
            assert!(read(&buf[..cut]).is_err(), "cut at {cut}");
        }
    }

    #[test]
    fn huge_length_headers_do_not_allocate_or_panic() {
        // compound root, list "" of compound with i32::MAX elements, then EOF
        let list = [
            0x0A, 0x00, 0x00, 0x09, 0x00, 0x00, 0x0A, 0x7F, 0xFF, 0xFF, 0xFF,
        ];
        assert!(read(&list).is_err());
        // byte array / int array / long array with i32::MAX length
        for tag in [7u8, 11, 12] {
            let bytes = [
                0x0A, 0x00, 0x00, tag, 0x00, 0x01, b'a', 0x7F, 0xFF, 0xFF, 0xFF,
            ];
            assert!(read(&bytes).is_err(), "tag {tag}");
        }
    }

    #[test]
    fn deep_nesting_is_rejected_without_stack_overflow() {
        let mut bytes = vec![0x0A, 0x00, 0x00];
        for _ in 0..100_000 {
            bytes.extend_from_slice(&[0x09, 0x00, 0x01, b'a', 0x09, 0x00, 0x00, 0x00, 0x01]);
        }
        let handle = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || read(&bytes).is_err())
            .unwrap();
        assert!(handle.join().unwrap());
    }

    #[test]
    fn modified_utf8_emoji_names_are_decoded() {
        // U+1F600 as a CESU-8 surrogate pair plus an encoded NUL (C0 80)
        let mut bytes = vec![0x0A, 0x00, 0x00, 0x08, 0x00, 0x04, b'n', b'a', b'm', b'e'];
        let s: &[u8] = &[b'A', 0xED, 0xA0, 0xBD, 0xED, 0xB8, 0x80, 0xC0, 0x80, b'z'];
        bytes.extend_from_slice(&(s.len() as u16).to_be_bytes());
        bytes.extend_from_slice(s);
        bytes.push(0);
        let (_, root) = read(&bytes).unwrap();
        assert_eq!(root.get_string("name"), Some("A\u{1F600}\0z"));
    }

    #[test]
    fn emoji_roundtrips_through_writer_as_modified_utf8() {
        let root = NbtTag::Compound(vec![("n".into(), NbtTag::String("hi \u{1F600}".into()))]);
        let mut buf = Vec::new();
        NbtWriter::new(&mut buf).write_root("", &root).unwrap();
        assert!(!buf.iter().any(|b| *b >= 0xF0));
        assert_eq!(read(&buf).unwrap().1, root);
    }
}
