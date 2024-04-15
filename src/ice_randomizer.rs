use random::Random;
use scrypto::prelude::*;

#[derive(NonFungibleData, ScryptoSbor, Debug)]
struct RandomIceTicket {
    #[mutable]
    result: Option<NonFungibleLocalId>,
}

#[blueprint]
#[types(u16, u32)]
mod ice {
    /* Rrc404 Component */
    extern_blueprint!(
        "package_rdx1phtl6486q77dvsusa4pkft4dt5yzgfyt3p5grmynm76ue79epwz9k8",
        // "package_tdx_2_1p4nswlz52epvzayucenlch40sujdv22scuqy28zc7we5w0ly82mrat",
        // "package_sim1p5qqqqqqqqqqqqqqqqqpecwwrnsqqqqqqqqqqqqqqqqqqqqqj5zvnh",
        Rrc404NFT {
            fn freeze(&self, deposit: Bucket) -> (Bucket, Bucket);
            fn melt(&self, nft_bucket: Bucket) -> Bucket;
        }
    );
    const RRC404: Global<Rrc404NFT> = global_component!(
        Rrc404NFT,
        "component_rdx1czscv9f2mv034hewjplej5ef4f2ecug2fxxelfpgxrsrhw4mglq2yp"
        // "component_tdx_2_1czuyqr546ptgwn40gtearfe39jfp4w55jx8fsfyanna896l7s4sc8a"
        // "component_sim1cqqqqqqqqqqqqqqqqqqpecwwrnsqqqqqqqqqqqqqqqqqqqqqgguvvr"
    );
    const WATER_RESOURCE: ResourceManager = resource_manager!(
        "resource_rdx1t4h4396mukhpzdrr5sfvegjsxl8q7a34q2vkt4quxcxahna8fucuz4"
        // "resource_tdx_2_1thpd5wxvj7pz4u67z39l424vd4ajnnnx2sjff8wktq6cnlwkenwe0e"
        // "resource_sim1t5qqqqqqqqqqqqqqqqqpecwwrnsqqqqqqqqqqqqqqqqqqqqqs3ask4"
    );
    const ICE_RESOURCE: ResourceManager = resource_manager!(
        "resource_rdx1n2y299ekzx4au2v9yjmxzu650ulvk5ndx3u5tlevfclk0uvdgs30px"
        // "resource_tdx_2_1n2aclv9vx3z2hxxxafswfqlpt3cvqfkw7dc4eqrp8n4yan6s47ad0n"
        // "resource_sim1ngqqqqqqqqqqqqqqqqqpecwwrnsqqqqqqqqqqqqqqqqqqqqq6lw2hr"
    );

    /* .Random */
    extern_blueprint!(
        "package_rdx1p55tuj30yf842s6cjraqz5arhtf98jcjtmkjcmxrn6efvvc829g2jf",
        // "package_tdx_2_1p527rqesssgtadvr23elxrnrt6rw2jnfa5ke8n85ykcxmvjt06cvv6",
        // "package_sim1p5qqqqqqqyqszqgqqqqqqqgpqyqsqqqqxumnwqgqqqqqqycnnzj0hj",
        RandomComponent {
            fn request_random(&self, address: ComponentAddress, method_name: String, on_error: String,
                key: u32, badge_opt: Option<FungibleBucket>, expected_fee: u8) -> u32;
        }
    );
    const RNG: Global<RandomComponent> = global_component!(
        RandomComponent,
        "component_rdx1cqz6m403yq9xzqj7g5ujq3yd6w0ge8shur53z8754gj8rxde8xd0sr"
        // "component_tdx_2_1czzxynn4m4snhattvdf6knlyfs3ss70yufj975uh2mdhp8jes938sd"
        // "component_sim1cqqqqqqqqyqszqgqqqqqqqgpqyqsqqqqxumnwqgqqqqqqycnf7v0gx"
    );
    const RANDOM_BADGE: ResourceManager = resource_manager!(
        "resource_rdx1thufp23mqn3hefdza383tk2fxs3rvwv97djzq8x5czzdqrgkc807wj"
        // "resource_tdx_2_1t59tdtsvv7sc0nej3z585w5nmqpq3z5cms7xdwvkyqaqreu9j3rvyu"
        // "resource_sim1t5qqqqqqqyqszqgqqqqqqqgpqyqsqqqqxumnwqgqqqqqqycn38dnjs"
    );


    enable_method_auth! {
        roles {
            random_provider => updatable_by: [];
        },
        methods {
            deposit => PUBLIC;
            withdraw => PUBLIC;
            mint => restrict_to: [OWNER];
            melt => restrict_to: [OWNER];
            do_mint => restrict_to: [random_provider];
        }
    }

    struct IceRandomizer {
        ticket_manager: ResourceManager,

        /// Ticket ID auto-increment.
        ticket_seq: u32,
        /// Stores the tickets that have not participated in the draw yet.
        /// The key is ordinal - in range [0, tickets_count).
        tickets_by_idx: KeyValueStore<u16, u32>,
        /// Reversed map - the key is ticket NFT id, the value is index bound by `tickets_count`.
        tickets_id_to_idx: KeyValueStore<u32, u16>,
        /// The number of tickets still pending draw.
        tickets_count: u16,

        melt_list: Vec<u32>,

        water: Vault,
        ice: NonFungibleVault,
    }

    impl IceRandomizer {
        pub fn instantiate() -> (Global<IceRandomizer>, Bucket) {
            debug!("LOG:IceRandomizer::instantiate()\n");

            let (address_reservation, component_address) =
                Runtime::allocate_component_address(<IceRandomizer>::blueprint_id());

            let owner_badge = Self::create_owner_badge();
            let ticket_manager = Self::create_ticket_manager(component_address);

            let randomizer = Self {
                ticket_manager,
                ticket_seq: 1,
                tickets_by_idx: KeyValueStore::new_with_registered_type(),
                tickets_id_to_idx: KeyValueStore::new_with_registered_type(),
                tickets_count: 0,
                melt_list: Vec::new(),
                water: Vault::new(WATER_RESOURCE.address()),
                ice: Vault::new(ICE_RESOURCE.address()).as_non_fungible(),
            }
                .instantiate()
                .prepare_to_globalize(
                    OwnerRole::Fixed(
                        rule!(require(owner_badge.resource_address()))
                    )
                )
                .with_address(address_reservation)
                .roles(roles!(
                    random_provider => rule!(require(RANDOM_BADGE.address()));
                ))
                .globalize();
            return (randomizer, owner_badge);
        }

        fn create_owner_badge() -> Bucket {
            return ResourceBuilder::new_fungible(OwnerRole::None)
                .divisibility(DIVISIBILITY_NONE)
                .metadata(metadata!(
                    init {
                        "name" => "IceRandomizer Owner", locked;
                        "symbol" => "IRANDOWNER", locked;
                        "description" => "IceRandomizer Owner", locked;
                        "icon_url" => Url::of("https://ice-rng.radix.live/image/ice-randomizer-icon.png"), locked;
                        "tags" => vec!("badge", "rng", "ice"), locked;
                    }
                ))
                .mint_initial_supply(1)
                .into();
        }

        fn create_ticket_manager(component_address: ComponentAddress) -> ResourceManager {
            return ResourceBuilder::new_integer_non_fungible::<RandomIceTicket>(OwnerRole::Fixed(
                rule!(require(global_caller(component_address)))))
                .metadata(metadata!(
                    init {
                        "name" => "IRAND", locked;
                        "description" => "Ice Randomizer Ticket", locked;
                        "icon_url" => Url::of("https://ice-rng.radix.live/image/ice-randomizer-icon.png"), locked;
                    }
                ))
                .mint_roles(mint_roles! {
                    minter => rule!(require(global_caller(component_address)));
                    minter_updater => rule!(deny_all);
                })
                .burn_roles(burn_roles! {
                    burner => rule!(require(global_caller(component_address)));
                    burner_updater => rule!(deny_all);
                })
                .non_fungible_data_update_roles(non_fungible_data_update_roles! {
                    non_fungible_data_updater => rule!(require(global_caller(component_address)));
                    non_fungible_data_updater_updater => rule!(deny_all);
                })
                .create_with_no_initial_supply();
        }

        /** assumes positive dec */
        fn split_int_and_fraction(d: Decimal) -> (I192, I192) {
            let raw_num = d.0;
            let divisor = I192::TEN.pow(Decimal::SCALE);
            return (raw_num / divisor, raw_num % divisor);
        }

        pub fn deposit(&mut self, bucket: Bucket) -> Bucket {
            let (quotient, remainder) = Self::split_int_and_fraction(bucket.amount());
            assert!(remainder.is_zero(),
                    "Please do not deposit fractional tokens. {}", bucket.amount()
            );

            self.water.put(bucket);

            let tickets_count: u32 = quotient.try_into().unwrap();

            let mut tickets: Bucket = Bucket::new(self.ticket_manager.address());
            for i in 0..tickets_count {
                let ticket_id = self.ticket_seq + i;
                let local_id = NonFungibleLocalId::integer(ticket_id.into());
                let ticket: Bucket = self.ticket_manager.mint_non_fungible(&local_id, RandomIceTicket {
                    result: None
                });
                tickets.put(ticket);

                self.add_ticket(ticket_id);
            }

            self.ticket_seq += tickets_count;

            return tickets;
        }

        pub fn withdraw(&mut self, tickets: Bucket) -> (Bucket, Bucket) {
            assert_eq!(tickets.resource_address(), self.ticket_manager.address(), "Withdrawal requires to burn your tickets.");

            let mut ice_ids: IndexSet<NonFungibleLocalId> = IndexSet::new();
            let mut water_count = 0u8;
            for non_fungible in tickets.as_non_fungible().non_fungibles::<RandomIceTicket>() {
                let data = non_fungible.data();
                match data.result {
                    Some(ice_id) => {
                        ice_ids.insert(ice_id);
                    }
                    None => {
                        water_count += 1;
                        let id = match non_fungible.local_id() {
                            NonFungibleLocalId::Integer(int_id) => int_id.value() as u32,
                            _ => u32::MAX,
                        };
                        self.remove_ticket(id);
                    }
                };
            }
            tickets.burn();
            return (self.ice.take_non_fungibles(&ice_ids).into(), self.water.take(water_count));
        }


        pub fn mint(&mut self, mint_count: u8, melt_count: u8) -> u32 {
            let address = Runtime::global_component().address();
            let method_name = "do_mint".into();
            let on_error = "".into();
            // `key` encodes two variables: (key = m * 100 + n)
            // 1) n - the number of NFTs to mint
            // 2) m - the number just minted NFTs to add to the "melt pool".
            // Using such a compound key allows to avoid maintaining one more KVS.
            let key: u32 = (melt_count as u32) * 100u32 + mint_count as u32;
            return RNG.request_random(address, method_name, on_error, key, None, 60u8);
        }

        pub fn do_mint(&mut self, key: u32, random_seed: Vec<u8>) {
            debug!("LOG:IceRandomizer::do_mint({:?}, {:?})", key, random_seed);

            let mint_count = key % 100;
            let mut melt_count = key / 100;

            let amount = self.water.amount().min(Decimal::from(mint_count));
            let bucket = self.water.take(amount);
            let (minted_ice_fungible, empty_bucket) = RRC404.freeze(bucket);

            let minted_ice = minted_ice_fungible.as_non_fungible();
            let nft_ids = minted_ice.non_fungible_local_ids();

            self.ice.put(minted_ice);

            let mut random: Random = Random::new(&random_seed);

            for ice_id in nft_ids {
                let winner_idx = random.roll::<u16>(self.tickets_count);
                let winner = *self.tickets_by_idx.get(&winner_idx).unwrap();
                self.remove_ticket(winner);
                let local_id = NonFungibleLocalId::integer(winner as u64);
                self.ticket_manager.update_non_fungible_data(
                    &local_id,
                    "result",
                    Some(ice_id),
                );
                if melt_count > 0 {
                    self.melt_list.push(winner);
                    melt_count -= 1;
                }
            }

            empty_bucket.drop_empty();
        }

        pub fn melt(&mut self) {
            debug!("LOG:IceRandomizer::melt()");

            let mut ice_to_melt: IndexSet<NonFungibleLocalId> = IndexSet::new();
            for ticket_id in self.melt_list.clone() {
                let local_id = NonFungibleLocalId::integer(ticket_id as u64);

                if self.ticket_manager.non_fungible_exists(&local_id) {
                    let data: RandomIceTicket = self.ticket_manager.get_non_fungible_data(&local_id);
                    match data.result {
                        Some(ice_id) => {
                            ice_to_melt.insert(ice_id);
                            self.add_ticket(ticket_id);
                            self.ticket_manager.update_non_fungible_data(
                                &local_id,
                                "result",
                                None::<NonFungibleLocalId>,
                            );
                        }
                        None => {
                            panic!("Not possible");
                        }
                    };
                } // ticket already burned - just ignore

            }

            let ice_bucket = self.ice.take_non_fungibles(&ice_to_melt).into();
            let water_bucket = RRC404.melt(ice_bucket);
            self.water.put(water_bucket);
            self.melt_list.clear();
        }


        fn add_ticket(&mut self, ticket_id: u32) {
            let index = self.tickets_count;
            self.tickets_by_idx.insert(index, ticket_id);
            self.tickets_id_to_idx.insert(ticket_id, index);
            self.tickets_count += 1;
        }

        fn remove_ticket(&mut self, id: u32) {
            let idx = self.tickets_id_to_idx.remove(&id).unwrap();
            self.tickets_by_idx.remove(&idx);

            // if there is a gap - move the last ticket there
            let last_idx = self.tickets_count - 1;
            if idx != last_idx {
                let last = self.tickets_by_idx.remove(&last_idx).unwrap();
                self.tickets_id_to_idx.insert(last, idx);
                self.tickets_by_idx.insert(idx, last);
            }
            self.tickets_count -= 1;
        }
    }
}