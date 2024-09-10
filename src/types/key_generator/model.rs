use radius_sequencer_sdk::kvstore::Lock;

use crate::types::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct KeyGeneratorAddressListModel;

impl KeyGeneratorAddressListModel {
    const ID: &'static str = stringify!(KeyGeneratorAddressListModel);

    pub fn initialize() -> Result<(), KvStoreError> {
        if Self::get().is_err() {
            let key = &Self::ID;
            let key_generator_address_list = KeyGeneratorAddressList::default();

            kvstore()?.put(key, &key_generator_address_list)?
        }

        Ok(())
    }

    pub fn put(key_generator_address_list: &KeyGeneratorAddressList) -> Result<(), KvStoreError> {
        let key = &Self::ID;
        kvstore()?.put(key, key_generator_address_list)
    }

    pub fn get() -> Result<KeyGeneratorAddressList, KvStoreError> {
        let key = &Self::ID;

        kvstore()?.get(key)
    }

    pub fn add_key_generator_address(key_generator_address: Address) -> Result<(), KvStoreError> {
        let key = &Self::ID;

        kvstore()?.apply(
            key,
            |locked_key_generator_address_list: &mut Lock<KeyGeneratorAddressList>| {
                locked_key_generator_address_list.insert(key_generator_address)
            },
        )?;

        Ok(())
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
