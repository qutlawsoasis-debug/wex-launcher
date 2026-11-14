//! Sealing of account secrets (refresh and access tokens) before they touch the disk.
//!
//! Windows: DPAPI (`CryptProtectData`) in the current-user scope with Wex entropy.
//! The sealed blob can only be opened by the same Windows user, it survives
//! reinstalling or moving the launcher, and it is never readable as plain text.
//!
//! Other systems: there is no system keystore binding here yet. The blob is
//! stored unsealed in a file only the owner may read (mode 0600), and the
//! cipher reports itself as `file-0600` so the UI and reports can say so.

use crate::error::{Result, WexError};

/// Header of every sealed secrets file: magic, format version, cipher id.
const MAGIC: &[u8; 4] = b"WEXS";
const FORMAT: u8 = 1;

pub trait SecretCipher: Send + Sync {
    /// Stable one-byte id written into the file header.
    fn id(&self) -> u8;
    /// Human-readable name for logs and reports.
    fn name(&self) -> &'static str;
    fn seal(&self, plain: &[u8]) -> Result<Vec<u8>>;
    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>>;
}

/// The cipher Wex uses on this system.
pub fn system_cipher() -> Box<dyn SecretCipher> {
    #[cfg(windows)]
    {
        Box::new(DpapiCipher)
    }
    #[cfg(not(windows))]
    {
        Box::new(OwnerOnlyFile)
    }
}

/// Wrap sealed bytes with the file header.
pub(crate) fn encode(cipher: &dyn SecretCipher, plain: &[u8]) -> Result<Vec<u8>> {
    let sealed = cipher.seal(plain)?;
    let mut out = Vec::with_capacity(sealed.len() + 6);
    out.extend_from_slice(MAGIC);
    out.push(FORMAT);
    out.push(cipher.id());
    out.extend_from_slice(&sealed);
    Ok(out)
}

/// Check the header and open the payload.
pub(crate) fn decode(cipher: &dyn SecretCipher, data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 6 || &data[..4] != MAGIC || data[4] != FORMAT {
        return Err(corrupt("unknown secrets file format"));
    }
    if data[5] != cipher.id() {
        return Err(corrupt(&format!(
            "secrets were sealed with cipher {}, this system uses {}",
            data[5],
            cipher.id()
        )));
    }
    cipher.open(&data[6..])
}

pub(crate) fn corrupt(details: &str) -> WexError {
    WexError::Auth {
        code: "AUTH_SECRETS_UNREADABLE",
        message: "Не удалось прочитать сохранённый вход. Войдите в аккаунт ещё раз.".to_string(),
        details: Some(details.to_string()),
    }
}

#[cfg(windows)]
pub struct DpapiCipher;

#[cfg(windows)]
const DPAPI_ENTROPY: &[u8] = b"Wex launcher accounts v1";

#[cfg(windows)]
impl SecretCipher for DpapiCipher {
    fn id(&self) -> u8 {
        1
    }

    fn name(&self) -> &'static str {
        "dpapi-user"
    }

    fn seal(&self, plain: &[u8]) -> Result<Vec<u8>> {
        dpapi::protect(plain, DPAPI_ENTROPY)
    }

    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>> {
        dpapi::unprotect(sealed, DPAPI_ENTROPY)
    }
}

#[cfg(windows)]
mod dpapi {
    use super::corrupt;
    use crate::error::{Result, WexError};
    use std::ptr;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        }
    }

    /// Copy the DPAPI output into a Vec and free the buffer DPAPI allocated.
    unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let bytes = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        LocalFree(out.pbData as _);
        bytes
    }

    pub fn protect(plain: &[u8], entropy: &[u8]) -> Result<Vec<u8>> {
        let input = blob(plain);
        let entropy = blob(entropy);
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };
        // SAFETY: input and entropy point to live slices for the duration of the call;
        // DPAPI allocates `out` with LocalAlloc, which `take` frees.
        let ok = unsafe {
            CryptProtectData(
                &input,
                ptr::null(),
                &entropy,
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        };
        if ok == 0 {
            return Err(WexError::Auth {
                code: "AUTH_SECRETS_UNWRITABLE",
                message: "Windows не дала зашифровать данные входа. Повторите вход.".to_string(),
                details: Some(format!(
                    "CryptProtectData failed: {}",
                    std::io::Error::last_os_error()
                )),
            });
        }
        Ok(unsafe { take(out) })
    }

    pub fn unprotect(sealed: &[u8], entropy: &[u8]) -> Result<Vec<u8>> {
        let input = blob(sealed);
        let entropy = blob(entropy);
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };
        // SAFETY: as in `protect`.
        let ok = unsafe {
            CryptUnprotectData(
                &input,
                ptr::null_mut(),
                &entropy,
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        };
        if ok == 0 {
            return Err(corrupt(&format!(
                "CryptUnprotectData failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(unsafe { take(out) })
    }
}

#[cfg(not(windows))]
pub struct OwnerOnlyFile;

#[cfg(not(windows))]
impl SecretCipher for OwnerOnlyFile {
    fn id(&self) -> u8 {
        2
    }

    fn name(&self) -> &'static str {
        "file-0600"
    }

    fn seal(&self, plain: &[u8]) -> Result<Vec<u8>> {
        Ok(plain.to_vec())
    }

    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>> {
        Ok(sealed.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trip_and_rejects_foreign_data() {
        let c = system_cipher();
        let data = encode(c.as_ref(), b"secret-refresh-token").unwrap();
        assert_eq!(&data[..4], MAGIC);
        assert_eq!(decode(c.as_ref(), &data).unwrap(), b"secret-refresh-token");
        assert!(decode(c.as_ref(), b"garbage").is_err());
        let mut wrong = data.clone();
        wrong[5] = 99;
        assert!(decode(c.as_ref(), &wrong).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn dpapi_output_does_not_contain_plain_text() {
        let plain = b"M.C512_BL2.0.U.-refresh-token-value";
        let sealed = DpapiCipher.seal(plain).unwrap();
        assert!(!sealed.windows(plain.len()).any(|w| w == plain));
        assert_eq!(DpapiCipher.open(&sealed).unwrap(), plain);
    }

    #[cfg(windows)]
    #[test]
    fn dpapi_rejects_tampered_blob() {
        let mut sealed = DpapiCipher.seal(b"token").unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 0xff;
        assert!(DpapiCipher.open(&sealed).is_err());
    }
}
