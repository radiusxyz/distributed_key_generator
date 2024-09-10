use radius_sequencer_sdk::kvstore::Lock;
use skde::{
    delay_encryption::SecretKey, key_aggregation::AggregatedKey, key_generation::PartialKey,
};

use crate::types::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct KeyIdModel;

impl KeyIdModel {
    const ID: &'static str = stringify!(KeyIdModel);

    pub fn put(key_id: u64) -> Result<(), KvStoreError> {
        let key = &Self::ID;

        kvstore()?.put(key, &key_id)
    }

    pub fn get() -> Result<u64, KvStoreError> {
        let key = &Self::ID;

        kvstore()?.get(key)
    }

    pub fn get_or_default() -> Result<u64, KvStoreError> {
        let key = &Self::ID;

        kvstore()?.get_or_default(key)
    }

    pub fn get_mut_or_default() -> Result<Lock<'static, u64>, KvStoreError> {
        let key = &Self::ID;

        match kvstore()?.get_mut(key) {
            Ok(key_id) => Ok(key_id),
            Err(error) => {
                if error.is_none_type() {
                    let key_id = 0;
                    kvstore()?.put(key, &key_id)?;

                    return kvstore()?.get_mut(key);
                }

                Err(error)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PartialKeyListModel;

impl PartialKeyListModel {
    const ID: &'static str = stringify!(PartialKeyListModel);

    pub fn initialize(key_id: u64) -> Result<(), KvStoreError> {
        if Self::get(key_id).is_err() {
            let key = &(Self::ID, key_id);
            let partial_key_list = PartialKeyList::default();

            kvstore()?.put(key, &partial_key_list)?
        }

        Ok(())
    }

    pub fn put(key_id: u64, sequencing_info_list: &PartialKeyList) -> Result<(), KvStoreError> {
        let key = &(Self::ID, key_id);

        kvstore()?.put(key, sequencing_info_list)
    }

    pub fn get(key_id: u64) -> Result<PartialKeyList, KvStoreError> {
        let key = &(Self::ID, key_id);

        kvstore()?.get(key)
    }

    pub fn get_or_default(key_id: u64) -> Result<PartialKeyList, KvStoreError> {
        let key = &(Self::ID, key_id);

        kvstore()?.get_or_default(key)
    }

    pub fn add_key_generator_address(
        key_id: u64,
        address: Address,
        partial_key: PartialKey,
    ) -> Result<(), KvStoreError> {
        let key = &(Self::ID, key_id);

        kvstore()?.apply(key, |locked_partial_key_list: &mut Lock<PartialKeyList>| {
            locked_partial_key_list.insert(address, partial_key)
        })?;

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AggregatedKeyModel;

impl AggregatedKeyModel {
    const ID: &'static str = stringify!(AggregatedKeyModel);

    pub fn put(key_id: u64, aggregated_key: &AggregatedKey) -> Result<(), KvStoreError> {
        let key = &(Self::ID, key_id);

        kvstore()?.put(key, aggregated_key)
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
