use std::{borrow::Borrow, ops::Deref, sync::Arc};

/// Nominal type for [`Arc<String>`].
///
/// Needed because [`Arc<String>`] doesn't implement [`AsRef<str>`]
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SyncString(Arc<String>);

impl Deref for SyncString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<SyncString> for Arc<String> {
    fn from(s: SyncString) -> Self {
        s.0
    }
}

impl From<String> for SyncString {
    fn from(s: String) -> Self {
        Self(Arc::new(s))
    }
}

impl Clone for SyncString {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl AsRef<String> for SyncString {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl AsRef<str> for SyncString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for SyncString {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Borrow<String> for SyncString {
    fn borrow(&self) -> &String {
        &self.0
    }
}

impl PartialEq<str> for SyncString {
    fn eq(&self, other: &str) -> bool {
        self.0.as_ref() == other
    }
}

impl serde::Serialize for SyncString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}
