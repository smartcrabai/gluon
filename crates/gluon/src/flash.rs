use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use tower_sessions::session::Error as SessionError;

const FLASH_KEY: &str = "_gluon_flash";

/// Flash messages persisted in the session for exactly one subsequent request.
///
/// Use [`Flash::set`] to enqueue a message before issuing a redirect, and
/// [`Flash::take`] (or the [`Flash`] extractor pattern) to read and drain the
/// messages on the next request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Flash(pub HashMap<String, String>);

impl Flash {
    /// Sets a flash entry. The entry is consumed on the next request via [`Flash::take`].
    ///
    /// # Errors
    ///
    /// Returns a [`SessionError`] when reading from or writing to the underlying
    /// session store fails.
    pub async fn set(
        session: &Session,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), SessionError> {
        let mut flash: Self = session.get(FLASH_KEY).await?.unwrap_or_default();
        flash.0.insert(key.into(), value.into());
        session.insert(FLASH_KEY, flash).await
    }

    /// Reads and removes the entire flash map from the session.
    ///
    /// # Errors
    ///
    /// Returns a [`SessionError`] when removing the entry from the underlying
    /// session store fails.
    pub async fn take(session: &Session) -> Result<Self, SessionError> {
        let flash: Option<Self> = session.remove(FLASH_KEY).await?;
        Ok(flash.unwrap_or_default())
    }

    /// Returns the value associated with `key`, if any.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tower_sessions::MemoryStore;

    fn new_session() -> Session {
        Session::new(None, Arc::new(MemoryStore::default()), None)
    }

    #[tokio::test]
    async fn set_and_take_round_trip() {
        let session = new_session();
        Flash::set(&session, "notice", "hi").await.unwrap();
        let taken = Flash::take(&session).await.unwrap();
        assert_eq!(taken.0.len(), 1);
        assert_eq!(taken.get("notice"), Some("hi"));
    }

    #[tokio::test]
    async fn take_clears_state() {
        let session = new_session();
        Flash::set(&session, "notice", "hi").await.unwrap();
        let _ = Flash::take(&session).await.unwrap();
        let again = Flash::take(&session).await.unwrap();
        assert!(again.0.is_empty());
    }

    #[tokio::test]
    async fn set_multiple_keys_round_trip() {
        let session = new_session();
        Flash::set(&session, "notice", "a").await.unwrap();
        Flash::set(&session, "alert", "b").await.unwrap();
        let taken = Flash::take(&session).await.unwrap();
        assert_eq!(taken.0.len(), 2);
        assert_eq!(taken.get("notice"), Some("a"));
        assert_eq!(taken.get("alert"), Some("b"));
    }

    #[tokio::test]
    async fn take_when_unset_returns_empty() {
        let session = new_session();
        let taken = Flash::take(&session).await.unwrap();
        assert!(taken.0.is_empty());
    }
}
