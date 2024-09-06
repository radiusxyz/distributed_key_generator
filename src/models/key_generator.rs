use crate::models::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct KeyGeneratorAddressListModel;

impl KeyGeneratorAddressListModel {
    const ID: &'static str = stringify!(KeyGeneratorAddressListModel);

    pub fn get() -> Result<KeyGeneratorAddressList, KvStoreError> {
        kvstore()?.get(&Self::ID)
    }

    pub fn put(key_generator_address_list: &KeyGeneratorAddressList) -> Result<(), KvStoreError> {
        kvstore()?.put(&Self::ID, key_generator_address_list)
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
}
