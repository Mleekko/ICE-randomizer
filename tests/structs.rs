use radix_engine::prelude::{ComponentAddress, NonFungibleVault, ResourceAddress, ScryptoSbor, Secp256k1PublicKey, Vault};
use scrypto::component::KeyValueStore;
use scrypto::prelude::ResourceManager;
use transaction::prelude::*;

#[derive(Copy, Clone)]
pub struct Account {
    pub key: Secp256k1PublicKey,
    pub address: ComponentAddress,
}

#[derive(Copy, Clone)]
pub struct TestEnv {
    pub owner: Account,
    pub users: [Account; 5],
}

#[derive(Copy, Clone)]
pub struct DeployedEnv {
    pub env: TestEnv,

    pub rrc404_component: ComponentAddress,
    pub ice_randomizer: ComponentAddress,
    pub randomizer_owner: ResourceAddress,
    pub ticket_address: ResourceAddress,
}

#[derive(ScryptoSbor)]
pub struct IceRandomizerState {
    pub ticket_manager: ResourceManager,

    pub ticket_seq: u32,
    pub tickets_by_idx: KeyValueStore<u16, u32>,
    pub tickets_id_to_idx: KeyValueStore<u32, u16>,
    pub tickets_count: u16,

    pub melt_list: Vec<u32>,

    pub water: Vault,
    pub ice: NonFungibleVault,
}
