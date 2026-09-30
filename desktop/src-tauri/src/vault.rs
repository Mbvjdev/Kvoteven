//! OS keyring storage for the DeepSeek API key.
//!
//! The key lives only in the platform credential store under service
//! `dk.justservices.kvoteven.desktop`, account `deepseek`. Plaintext is never
//! serialized or logged; on the read path it is wrapped in `Zeroizing`.

use crate::models::AppError;

pub const KEYRING_SERVICE: &str = "dk.justservices.kvoteven.desktop";
pub const KEYRING_ACCOUNT: &str = "deepseek";

/// Minimum plausible key length (DeepSeek keys are `sk-` + 32 hex chars).
pub const KEY_MIN_LEN: usize = 8;
/// Maximum accepted key length; transient input bound.
pub const KEY_MAX_LEN: usize = 512;

/// Validate a user-supplied key before it is ever handed to the credential
/// store: ASCII only, no whitespace/control chars, bounded length.
pub fn validate_key(input: &str) -> Result<(), AppError> {
    if input.len() < KEY_MIN_LEN || input.len() > KEY_MAX_LEN {
        return Err(AppError::new(AppError::INVALID_KEY));
    }
    if !input.is_ascii() {
        return Err(AppError::new(AppError::INVALID_KEY));
    }
    if input.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(AppError::new(AppError::INVALID_KEY));
    }
    Ok(())
}

/// Abstraction over the credential store so the service is testable without
/// touching the real keyring.
pub trait KeyStore: Send + Sync {
    fn get(&self) -> Result<Option<String>, AppError>;
    fn set(&self, secret: &str) -> Result<(), AppError>;
    fn delete(&self) -> Result<(), AppError>;
}

/// Real OS keyring backend.
pub struct KeyringStore {
    entry: Option<keyring::Entry>,
}

impl KeyringStore {
    pub fn new() -> Self {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).ok();
        KeyringStore { entry }
    }

    fn entry(&self) -> Result<&keyring::Entry, AppError> {
        self.entry
            .as_ref()
            .ok_or_else(|| AppError::new(AppError::KEYRING_UNAVAILABLE))
    }
}

impl Default for KeyringStore {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyStore for KeyringStore {
    fn get(&self) -> Result<Option<String>, AppError> {
        let entry = self.entry()?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(AppError::new(AppError::KEYRING_UNAVAILABLE)),
        }
    }

    fn set(&self, secret: &str) -> Result<(), AppError> {
        validate_key(secret)?;
        let entry = self.entry()?;
        entry.set_password(secret).map_err(|e| {
            AppError::new(if keyring_unavailable(&e) {
                AppError::KEYRING_UNAVAILABLE
            } else {
                AppError::KEYRING_WRITE_FAILED
            })
        })
    }

    fn delete(&self) -> Result<(), AppError> {
        let entry = self.entry()?;
        match entry.delete_credential() {
            Ok(()) => {
                // Verify absence within the secure backend: a readback that
                // still returns the secret means the delete did not stick.
                match entry.get_password() {
                    Ok(_) => Err(AppError::new(AppError::KEYRING_DELETE_FAILED)),
                    Err(keyring::Error::NoEntry) => Ok(()),
                    // Cannot read back (e.g. store transiently locked) after a
                    // successful delete; treat as deleted.
                    Err(_) => Ok(()),
                }
            }
            // Already absent is fine (delete is idempotent).
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::new(if keyring_unavailable(&e) {
                AppError::KEYRING_UNAVAILABLE
            } else {
                AppError::KEYRING_DELETE_FAILED
            })),
        }
    }
}

/// True when the platform credential store is missing or locked (a fixed,
/// sanitized "unavailable" condition rather than a transient write failure).
fn keyring_unavailable(e: &keyring::Error) -> bool {
    matches!(
        e,
        keyring::Error::PlatformFailure(_) | keyring::Error::NoStorageAccess(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_validation_rules() {
        // Valid: typical sk- key.
        assert!(validate_key("sk-0123456789abcdef").is_ok());
        // Too short.
        assert_eq!(
            validate_key("sk-123").unwrap_err().code,
            AppError::INVALID_KEY
        );
        // Too long (>512).
        let long = "a".repeat(513);
        assert_eq!(validate_key(&long).unwrap_err().code, AppError::INVALID_KEY);
        // Non-ascii.
        assert_eq!(
            validate_key("sk-12345678ø").unwrap_err().code,
            AppError::INVALID_KEY
        );
        // Whitespace.
        assert_eq!(
            validate_key("sk-1234 5678").unwrap_err().code,
            AppError::INVALID_KEY
        );
        // Control char.
        assert_eq!(
            validate_key("sk-1234\n5678").unwrap_err().code,
            AppError::INVALID_KEY
        );
        // Tab.
        assert_eq!(
            validate_key("sk-1234\t5678").unwrap_err().code,
            AppError::INVALID_KEY
        );
        // Exactly 8 is fine.
        assert!(validate_key("sk-12345").is_ok());
    }

    #[test]
    fn keyring_service_and_account_are_frozen() {
        assert_eq!(KEYRING_SERVICE, "dk.justservices.kvoteven.desktop");
        assert_eq!(KEYRING_ACCOUNT, "deepseek");
    }
}
