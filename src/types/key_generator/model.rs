use radius_sequencer_sdk::kvstore::Lock;

use crate::types::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct KeyGeneratorAddressListModel;

impl KeyGeneratorAddressListModel {
    const ID: &'static str = stringify!(KeyGeneratorAddressListModel);

    pub fn get() -> Result<KeyGeneratorAddressList, KvStoreError> {
        let key = &Self::ID;
        kvstore()?.get(key)
    }

    pub fn get_or_default() -> Result<KeyGeneratorAddressList, KvStoreError> {
        let key = &Self::ID;

        kvstore()?.get_or_default(key)
    }

    pub fn get_mut_or_default() -> Result<Lock<'static, KeyGeneratorAddressList>, KvStoreError> {
        let key = &Self::ID;

        match kvstore()?.get_mut(key) {
            Ok(key_generator_address_list) => Ok(key_generator_address_list),
            Err(error) => {
                if error.is_none_type() {
                    let key_generator_address_list = KeyGeneratorAddressList::default();
                    kvstore()?.put(key, &key_generator_address_list)?;

                    return kvstore()?.get_mut(key);
                }

                Err(error)
            }
        }
    }

    pub fn put(key_generator_address_list: &KeyGeneratorAddressList) -> Result<(), KvStoreError> {
        let key = &Self::ID;
        kvstore()?.put(key, key_generator_address_list)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct KeyGeneratorModel;

impl KeyGeneratorModel {
    const ID: &'static str = stringify!(KeyGeneratorModel);

    pub fn get(address: &Address) -> Result<KeyGenerator, KvStoreError> {
        let key = (Self::ID, address);

        kvstore()?.get(&key)
    }

    pub fn put(key_generator: &KeyGenerator) -> Result<(), KvStoreError> {
        let key = (Self::ID, key_generator.address());
        kvstore()?.put(&key, key_generator)
    }

    pub fn is_exist(address: &Address) -> bool {
        Self::get(address).is_ok()
    }
}
