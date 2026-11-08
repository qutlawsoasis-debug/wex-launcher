use md5::{Digest, Md5};
use uuid::Uuid;

/// Generate Mojang offline UUID from player nickname.
///
/// Mojang implementation:
/// `UUID.nameUUIDFromBytes(("OfflinePlayer:" + nickname).getBytes(StandardCharsets.UTF_8))`
pub fn offline_uuid(nickname: &str) -> Uuid {
    let input = format!("OfflinePlayer:{}", nickname);
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    let mut hash: [u8; 16] = hasher.finalize().into();

    // Set UUID version to 3 (name-based MD5)
    hash[6] = (hash[6] & 0x0f) | 0x30;
    // Set UUID variant to IETF
    hash[8] = (hash[8] & 0x3f) | 0x80;
    Uuid::from_bytes(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offline_uuid_player() {
        let uuid = offline_uuid("Player");
        assert_eq!(uuid.to_string(), "a01e3843-e521-3998-958a-f459800e4d11");
    }

    #[test]
    fn test_offline_uuid_custom() {
        let uuid = offline_uuid("AuraUser");
        assert_eq!(uuid.get_version_num(), 3);
        assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
    }
}
