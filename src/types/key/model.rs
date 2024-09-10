use radius_sequencer_sdk::kvstore::Lock;
use skde::{delay_encryption::SecretKey, key_aggregation::AggregatedKey};

use crate::types::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PartialKeyListModel;

impl PartialKeyListModel {
    const ID: &'static str = stringify!(PartialKeyListModel);

    pub fn put(key_id: u64, sequencing_info_list: &PartialKeyList) -> Result<(), KvStoreError> {
        let key = &(Self::ID, key_id);

        kvstore()?.put(key, sequencing_info_list)
    }

    pub fn get(key_id: u64) -> Result<PartialKeyList, KvStoreError> {
        let key = &(Self::ID, key_id);

        kvstore()?.get(key)
    }

    pub fn get_mut_or_default(key_id: u64) -> Result<Lock<'static, PartialKeyList>, KvStoreError> {
        let key = &(Self::ID, key_id);

        match kvstore()?.get_mut(key) {
            Ok(partial_key_list) => Ok(partial_key_list),
            Err(error) => {
                if error.is_none_type() {
                    let partial_key_list = PartialKeyList::default();
                    kvstore()?.put(key, &partial_key_list)?;

                    return kvstore()?.get_mut(key);
                }

                Err(error)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AggregatedKeyModel;

impl AggregatedKeyModel {
    const ID: &'static str = stringify!(AggregatedKeyModel);

    pub fn put(key_id: u64, decryption_key: &AggregatedKey) -> Result<(), KvStoreError> {
        let key = &(Self::ID, key_id);

        kvstore()?.put(key, decryption_key)
    }

    pub fn get(key_id: u64) -> Result<AggregatedKey, KvStoreError> {
        let key = &(Self::ID, key_id);

        kvstore()?.get(key)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DecryptionKeyModel;

impl DecryptionKeyModel {
    const ID: &'static str = stringify!(DecryptionKeyModel);

    pub fn put(key_id: u64, decryption_key: &SecretKey) -> Result<(), KvStoreError> {
        let key = &(Self::ID, key_id);

        kvstore()?.put(key, decryption_key)
    }

    pub fn get(key_id: u64) -> Result<SecretKey, KvStoreError> {
        let key = &(Self::ID, key_id);

        kvstore()?.get(key)
    }
}
