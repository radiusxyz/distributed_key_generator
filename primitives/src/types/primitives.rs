use radius_sdk::kvstore::{KvStoreError, Model};
use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Model, Default, Hash, Serialize, Deserialize)]
#[kvstore(key())]
pub struct SessionId(u64);

impl From<u64> for SessionId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl Into<u64> for SessionId {
    fn into(self) -> u64 {
        self.0
    }
}

impl SessionId {
    pub fn initialize() -> Result<(), KvStoreError> {
        Self(0).put()
    }

    pub fn is_initial(&self) -> bool {
        self.0 == 0
    }

    pub fn prev(self) -> Option<Self> {
        self.0.checked_sub(1).map(Self)
    }

    pub fn next(&self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }

    pub fn next_mut(&mut self) -> Result<(), Error> {
        self.0 = self.next().ok_or(Error::Arithmetic)?.into();
        Ok(())
    }

    pub fn prev_mut(&mut self) -> Result<(), Error> {
        self.0 = self.prev().ok_or(Error::Arithmetic)?.into();
        Ok(())
    }
}
