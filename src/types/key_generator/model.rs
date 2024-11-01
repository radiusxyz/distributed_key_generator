use radius_sdk::kvstore::Lock;

use crate::types::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DistributedKeyGenerationAddressListModel;

impl DistributedKeyGenerationAddressListModel {
    const ID: &'static str = stringify!(DistributedKeyGenerationAddressListModel);

    pub fn initialize() -> Result<(), KvStoreError> {
        if Self::get().is_err() {
            let key = &Self::ID;
            let distributed_key_generation_address_list =
                DistributedKeyGenerationAddressList::default();

            kvstore()?.put(key, &distributed_key_generation_address_list)?
        }

        Ok(())
    }

    pub fn put(
        distributed_key_generation_address_list: &DistributedKeyGenerationAddressList,
    ) -> Result<(), KvStoreError> {
        let key = &Self::ID;
        kvstore()?.put(key, distributed_key_generation_address_list)
    }

    pub fn get() -> Result<DistributedKeyGenerationAddressList, KvStoreError> {
        let key = &Self::ID;

        kvstore()?.get(key)
    }

    pub fn add_distributed_key_generation_address(
        distributed_key_generation_address: Address,
    ) -> Result<(), KvStoreError> {
        let key = &Self::ID;

        kvstore()?.apply(
            key,
            |locked_distributed_key_generation_address_list: &mut Lock<
                DistributedKeyGenerationAddressList,
            >| {
                locked_distributed_key_generation_address_list
                    .insert(distributed_key_generation_address)
            },
        )?;

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DistributedKeyGenerationModel;

impl DistributedKeyGenerationModel {
    const ID: &'static str = stringify!(DistributedKeyGenerationModel);

    pub fn get(address: &Address) -> Result<DistributedKeyGeneration, KvStoreError> {
        let key = (Self::ID, address);

        kvstore()?.get(&key)
    }

    pub fn put(key_generator: &DistributedKeyGeneration) -> Result<(), KvStoreError> {
        let key = (Self::ID, key_generator.address());
        kvstore()?.put(&key, key_generator)
    }

    pub fn is_exist(address: &Address) -> bool {
        Self::get(address).is_ok()
    }
}
