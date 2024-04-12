use std::env;
use std::ops::Add;

use dot_random_test_utils::{deploy_random_component, RandomTestEnv};
use dot_random_test_utils::cargo::get_repo_sub_dir;
use radix_engine::vm::NoExtension;
use scrypto::this_package;
use scrypto_test::prelude::InMemorySubstateDatabase;
use scrypto_unit::*;
use transaction::prelude::*;

use consts::{RRC404_COMPONENT, RRC404_ICE, RRC404_PACKAGE, RRC404_WATER};
use structs::{Account, DeployedEnv, IceRandomizerState, TestEnv};

mod consts;
mod structs;


pub const AMOUNTS: [Decimal; 5] = [dec!(10), dec!(40), dec!(5), dec!(25), dec!(60)];

impl TestEnv {
    pub fn init(test_runner: &mut TestRunner<NoExtension, InMemorySubstateDatabase>) -> Self {
        let (public_key, _, owner_account) = test_runner.new_allocated_account();

        let (key1, _, user1) = test_runner.new_allocated_account();
        let (key2, _, user2) = test_runner.new_allocated_account();
        let (key3, _, user3) = test_runner.new_allocated_account();
        let (key4, _, user4) = test_runner.new_allocated_account();
        let (key5, _, user5) = test_runner.new_allocated_account();

        return TestEnv {
            owner: Account { key: public_key, address: owner_account },
            users: [
                Account { key: key1, address: user1 },
                Account { key: key2, address: user2 },
                Account { key: key3, address: user3 },
                Account { key: key4, address: user4 },
                Account { key: key5, address: user5 }
            ],
        };
    }

    pub fn deploy(self, runner: &mut TestRunner<NoExtension, InMemorySubstateDatabase>) -> (RandomTestEnv<NoExtension, InMemorySubstateDatabase>, DeployedEnv) {
        // Deploy RandomComponent
        let random_env = deploy_random_component(runner, "55cf37d");

        // Deploy ICE-RRC404
        let rrc404v1_path = get_repo_sub_dir("ice_rrc404v1", "d99f72d", "");
        let rrc404_component = self.deploy_rrc404(runner, rrc404v1_path.to_str().unwrap());

        // Deploy Ice Randomizer
        let (ice_randomizer, randomizer_owner, ticket_address) = self.deploy_randomizer(runner);

        return (random_env, DeployedEnv {
            env: self,
            rrc404_component,
            ice_randomizer,
            randomizer_owner,
            ticket_address,
        });
    }

    fn deploy_rrc404(self, runner: &mut TestRunner<NoExtension, InMemorySubstateDatabase>, dir_rrc404v1: &str) -> ComponentAddress {
        runner.compile_and_publish_at_address(dir_rrc404v1, RRC404_PACKAGE);

        let receipt = runner.execute_system_transaction_with_preallocated_addresses(
            vec![
                InstructionV1::CallFunction {
                    package_address: DynamicPackageAddress::Static(RRC404_PACKAGE),
                    blueprint_name: "Rrc404".to_string(),
                    function_name: "instantiate".to_string(),
                    args: manifest_args!(Some(ManifestAddressReservation(0)), Some(ManifestAddressReservation(1)), Some(ManifestAddressReservation(2))).into(),
                },
                InstructionV1::CallMethod {
                    address: DynamicGlobalAddress::Static(GlobalAddress::new_or_panic(self.owner.address.into())),
                    method_name: "deposit_batch".to_string(),
                    args: manifest_args!(ManifestExpression::EntireWorktop).into(),
                }],
            vec![(
                     BlueprintId::new(&RRC404_PACKAGE, "Rrc404"),
                     GlobalAddress::new_or_panic(RRC404_COMPONENT.into()),
                 )
                     .into(),
                 (
                     BlueprintId::new(&RESOURCE_PACKAGE, FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT.to_owned()),
                     GlobalAddress::new_or_panic(RRC404_WATER.into()),
                 )
                     .into(),
                 (
                     BlueprintId::new(&RESOURCE_PACKAGE, NON_FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT.to_owned()),
                     GlobalAddress::new_or_panic(RRC404_ICE.into()),
                 )
                     .into()],
            btreeset!(NonFungibleGlobalId::from_public_key(&self.owner.key)),
        );
        let result = receipt.expect_commit_success();
        let rrc404_component = result.new_component_addresses()[0];

        println!("rrc404_component: {:?}", rrc404_component);
        return rrc404_component;
    }

    fn deploy_randomizer(self, runner: &mut TestRunner<NoExtension, InMemorySubstateDatabase>) -> (ComponentAddress, ResourceAddress, ResourceAddress) {
        let package_address = runner.publish_package_simple(
            this_package!()
        );
        let receipt = runner.execute_manifest(
            ManifestBuilder::new()
                .lock_fee_from_faucet()
                .call_function(
                    package_address,
                    "IceRandomizer",
                    "instantiate",
                    manifest_args!(),
                )
                .deposit_batch(self.owner.address)
                .build(), vec![NonFungibleGlobalId::from_public_key(&self.owner.key)]);

        let result = receipt.expect_commit_success();

        return (result.new_component_addresses()[0], result.new_resource_addresses()[0], result.new_resource_addresses()[1]);
    }
}

#[test]
fn test_mint_partial() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().without_trace().build();
    let env = TestEnv::init(&mut test_runner);
    let (mut random_env, test) = env.deploy(&mut test_runner);

    let amounts = AMOUNTS;
    allocate_tokens(&mut test_runner, test, &amounts);

    // Act
    // 1. Users lock tokens
    for index in 0..amounts.len() {
        deposit_water(&mut test_runner, test, env.users[index], amounts[index]);
    }

    // 2. Owner triggers random mint - should return callback id: 1
    let receipt = test_runner.execute_manifest(
        ManifestBuilder::new()
            .lock_fee_from_faucet()
            .create_proof_from_account_of_amount(env.owner.address, test.randomizer_owner, dec!(1))
            .call_method(
                test.ice_randomizer,
                "mint",
                manifest_args!(80u8, 0u8),
            )
            .build(), vec![NonFungibleGlobalId::from_public_key(&env.owner.key)]);
    let result = receipt.expect_commit_success();
    let out = result.outcome.expect_success();
    out[2].expect_return_value(&1u32);

    // 3. Simulate a TX that calls RandomComponent.execute() to do the actual mint - should mint an NFT
    random_env.execute_next(&mut test_runner, 1);

    // Assert minted 80 ICE
    let balance_ice = test_runner.get_component_balance(test.ice_randomizer, RRC404_ICE);
    assert_eq!(dec!(80), balance_ice);
    let balance_water = test_runner.get_component_balance(test.ice_randomizer, RRC404_WATER);
    assert_eq!(dec!(60), balance_water);

    // 4. Users withdraw ICE
    for index in 0..amounts.len() {
        let account = env.users[index];
        withdraw_ice(&mut test_runner, test, account, amounts[index]);
        println!("Balance: {} -> {:?}/{:?}", index,
                 test_runner.get_component_balance(account.address, RRC404_WATER),
                 test_runner.get_component_balance(account.address, RRC404_ICE)
        );
    }

    // Assert component is empty
    let balance_ice = test_runner.get_component_balance(test.ice_randomizer, RRC404_ICE);
    assert_eq!(dec!(0), balance_ice);
    let balance_water = test_runner.get_component_balance(test.ice_randomizer, RRC404_WATER);
    assert_eq!(dec!(0), balance_water);
}

#[test]
fn test_mint_in_batches() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().without_trace().build();
    let env = TestEnv::init(&mut test_runner);
    let (mut random_env, test) = env.deploy(&mut test_runner);

    let amounts = AMOUNTS;
    allocate_tokens(&mut test_runner, test, &amounts);

    // Act
    // 1. Users lock tokens
    for index in 0..amounts.len() {
        deposit_water(&mut test_runner, test, env.users[index], amounts[index]);
    }

    // 2. Owner triggers random mint in batches [140 = 28 x 5]
    for index in 0u32..5 {
        let receipt = test_runner.execute_manifest(
            ManifestBuilder::new()
                .lock_fee_from_faucet()
                .create_proof_from_account_of_amount(env.owner.address, test.randomizer_owner, dec!(1))
                .call_method(
                    test.ice_randomizer,
                    "mint",
                    manifest_args!(28u8, 0u8),
                )
                .build(), vec![NonFungibleGlobalId::from_public_key(&env.owner.key)]);
        let result = receipt.expect_commit_success();
        result.outcome.expect_success();

        // 3. Simulate a TX that calls RandomComponent.execute() to do the actual mint - should mint an NFT
        random_env.execute_next(&mut test_runner, index + 1);
    }


    // Assert minted 140 ICE
    let balance_ice = test_runner.get_component_balance(test.ice_randomizer, RRC404_ICE);
    assert_eq!(dec!(140), balance_ice);
    let balance_water = test_runner.get_component_balance(test.ice_randomizer, RRC404_WATER);
    assert_eq!(dec!(0), balance_water);

    // 4. Users withdraw ICE
    for index in 0..amounts.len() {
        let account = env.users[index];
        withdraw_ice(&mut test_runner, test, account, amounts[index]);
        let balance_water = test_runner.get_component_balance(account.address, RRC404_WATER);
        let balance_ice = test_runner.get_component_balance(account.address, RRC404_ICE);
        println!("Balance: {} -> {:?}/{:?}", index,
                 balance_water,
                 balance_ice
        );
        assert_eq!(dec!(0), balance_water);
        assert_eq!(Decimal::from(amounts[index]), balance_ice);
    }

    // Assert component is empty
    let balance_ice = test_runner.get_component_balance(test.ice_randomizer, RRC404_ICE);
    assert_eq!(dec!(0), balance_ice);
    let balance_water = test_runner.get_component_balance(test.ice_randomizer, RRC404_WATER);
    assert_eq!(dec!(0), balance_water);
}

#[test]
fn test_whole_flow() {
    // Arrange
    // No idea why, but `advance_to_round_at_timestamp()` requires this custom genesis to succeed.
    let custom_genesis = CustomGenesis::default(Epoch::of(1), CustomGenesis::default_consensus_manager_config());
    let mut test_runner = TestRunnerBuilder::new().with_custom_genesis(custom_genesis).without_trace().build();
    let env = TestEnv::init(&mut test_runner);
    let (mut random_env, test) = env.deploy(&mut test_runner);

    let amounts = [dec!(20), dec!(70), dec!(15), dec!(55), dec!(120)];
    allocate_tokens(&mut test_runner, test, &amounts);

    // Act
    // 1. Users lock tokens
    for index in 0..amounts.len() {
        deposit_water(&mut test_runner, test, env.users[index], amounts[index]);
    }

    // 2. Owner triggers random mint in batches [280 = 56 x 5], with the first batch marking 40 to be melted
    for index in 0u32..5 {
        let manifest_arguments = match index {
            0 => { manifest_args!(56u8, 40u8) }
            _ => { manifest_args!(56u8, 0u8) }
        };
        let receipt = test_runner.execute_manifest(
            ManifestBuilder::new()
                .lock_fee_from_faucet()
                .create_proof_from_account_of_amount(env.owner.address, test.randomizer_owner, dec!(1))
                .call_method(
                    test.ice_randomizer,
                    "mint",
                    manifest_arguments,
                )
                .build(), vec![NonFungibleGlobalId::from_public_key(&env.owner.key)]);
        let result = receipt.expect_commit_success();
        result.outcome.expect_success();

        // 3. Simulate a TX that calls RandomComponent.execute() to do the actual mint - should mint an NFT
        random_env.execute_next(&mut test_runner, index + 1);

        if index == 0 {
            advance_time(&mut test_runner);
        }
    }

    // Assert minted 280 ICE
    let balance_ice = test_runner.get_component_balance(test.ice_randomizer, RRC404_ICE);
    assert_eq!(dec!(280), balance_ice);
    let balance_water = test_runner.get_component_balance(test.ice_randomizer, RRC404_WATER);
    assert_eq!(dec!(0), balance_water);
    // Assert 40 NFTs to melt
    let state: IceRandomizerState = test_runner.component_state::<IceRandomizerState>(test.ice_randomizer);
    assert_eq!(40, state.melt_list.len());

    // 4. Owner melts the first 40 minted NFTS
    let receipt = test_runner.execute_manifest(
        ManifestBuilder::new()
            .lock_fee_from_faucet()
            .create_proof_from_account_of_amount(env.owner.address, test.randomizer_owner, dec!(1))
            .call_method(
                test.ice_randomizer,
                "melt",
                manifest_args!(),
            )
            .build(), vec![NonFungibleGlobalId::from_public_key(&env.owner.key)]);
    let result = receipt.expect_commit_success();
    result.outcome.expect_success();

    // Assert melted 40 ICE
    let balance_ice = test_runner.get_component_balance(test.ice_randomizer, RRC404_ICE);
    assert_eq!(dec!(240), balance_ice);
    let balance_water = test_runner.get_component_balance(test.ice_randomizer, RRC404_WATER);
    assert_eq!(dec!(40), balance_water);
    // Assert 0 NFTs to melt
    let state: IceRandomizerState = test_runner.component_state::<IceRandomizerState>(test.ice_randomizer);
    assert_eq!(0, state.melt_list.len());


    // 5. Users withdraw ICE
    for index in 0..amounts.len() {
        let account = env.users[index];
        withdraw_ice(&mut test_runner, test, account, amounts[index]);
        let balance_water = test_runner.get_component_balance(account.address, RRC404_WATER);
        let balance_ice = test_runner.get_component_balance(account.address, RRC404_ICE);
        println!("Balance: {} -> {:?}/{:?}", index,
                 balance_water,
                 balance_ice
        );
        assert_eq!(Decimal::from(amounts[index]), balance_ice + balance_water);
    }

    // Assert component is empty
    let balance_ice = test_runner.get_component_balance(test.ice_randomizer, RRC404_ICE);
    assert_eq!(dec!(0), balance_ice);
    let balance_water = test_runner.get_component_balance(test.ice_randomizer, RRC404_WATER);
    assert_eq!(dec!(0), balance_water);
}

#[test]
fn test_can_withdraw_during_mint() {
    // Arrange
    // No idea why, but `advance_to_round_at_timestamp()` requires this custom genesis to succeed.
    let custom_genesis = CustomGenesis::default(Epoch::of(1), CustomGenesis::default_consensus_manager_config());
    let mut test_runner = TestRunnerBuilder::new().with_custom_genesis(custom_genesis).without_trace().build();
    let env = TestEnv::init(&mut test_runner);
    let (mut random_env, test) = env.deploy(&mut test_runner);

    let amounts = [dec!(4), dec!(5), dec!(6)];
    allocate_tokens(&mut test_runner, test, &amounts);

    // Act
    // 1. Users lock tokens
    for index in 0..amounts.len() {
        deposit_water(&mut test_runner, test, env.users[index], amounts[index]);
    }

    // 2. Owner triggers random mint with the first batch marking 10 to be melted
    let receipt = test_runner.execute_manifest(
        ManifestBuilder::new()
            .lock_fee_from_faucet()
            .create_proof_from_account_of_amount(env.owner.address, test.randomizer_owner, dec!(1))
            .call_method(
                test.ice_randomizer,
                "mint",
                manifest_args!(40u8, 10u8),
            )
            .build(), vec![NonFungibleGlobalId::from_public_key(&env.owner.key)]);
    let result = receipt.expect_commit_success();
    result.outcome.expect_success();

    // 3. Simulate a TX that calls RandomComponent.execute() to do the actual mint - should mint an NFT
    random_env.execute_next(&mut test_runner, 1);

    // Assert minted 15 ICE
    let balance_ice = test_runner.get_component_balance(test.ice_randomizer, RRC404_ICE);
    assert_eq!(dec!(15), balance_ice);
    let balance_water = test_runner.get_component_balance(test.ice_randomizer, RRC404_WATER);
    assert_eq!(dec!(0), balance_water);
    // Assert 10 NFTs to melt
    let state: IceRandomizerState = test_runner.component_state::<IceRandomizerState>(test.ice_randomizer);
    assert_eq!(10, state.melt_list.len());

    // 4. One user withdraws [-4 ICE]
    withdraw_ice(&mut test_runner, test, env.users[0], amounts[0]);

    advance_time(&mut test_runner);

    // 5. Owner melts the first 10 minted NFTS - just N actually melt
    let receipt = test_runner.execute_manifest(
        ManifestBuilder::new()
            .lock_fee_from_faucet()
            .create_proof_from_account_of_amount(env.owner.address, test.randomizer_owner, dec!(1))
            .call_method(
                test.ice_randomizer,
                "melt",
                manifest_args!(),
            )
            .build(), vec![NonFungibleGlobalId::from_public_key(&env.owner.key)]);
    let result = receipt.expect_commit_success();
    result.outcome.expect_success();

    let balance_ice = test_runner.get_component_balance(test.ice_randomizer, RRC404_ICE);
    assert_eq!(dec!(3), balance_ice);
    let balance_water = test_runner.get_component_balance(test.ice_randomizer, RRC404_WATER);
    assert_eq!(dec!(8), balance_water);
    // Assert 0 NFTs to melt
    let state: IceRandomizerState = test_runner.component_state::<IceRandomizerState>(test.ice_randomizer);
    assert_eq!(0, state.melt_list.len());


    // 5. Users withdraw ICE
    for index in 1..amounts.len() {
        let account = env.users[index];
        withdraw_ice(&mut test_runner, test, account, amounts[index]);
        let balance_water = test_runner.get_component_balance(account.address, RRC404_WATER);
        let balance_ice = test_runner.get_component_balance(account.address, RRC404_ICE);
        println!("Balance: {} -> {:?}/{:?}", index,
                 balance_water,
                 balance_ice
        );
        assert_eq!(Decimal::from(amounts[index]), balance_ice + balance_water);
    }

    // Assert component is empty
    let balance_ice = test_runner.get_component_balance(test.ice_randomizer, RRC404_ICE);
    assert_eq!(dec!(0), balance_ice);
    let balance_water = test_runner.get_component_balance(test.ice_randomizer, RRC404_WATER);
    assert_eq!(dec!(0), balance_water);
}


fn sum(amounts: &[Decimal]) -> Decimal {
    let mut sum = Decimal::zero();
    for amount in amounts {
        sum += *amount;
    }
    return sum;
}

pub fn allocate_tokens(runner: &mut TestRunner<NoExtension, InMemorySubstateDatabase>, test: DeployedEnv, amounts: &[Decimal]) {
    let mut builder = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .withdraw_from_account(test.env.owner.address, RRC404_WATER, sum(&amounts));
    for i in 0..amounts.len() {
        let bucket = format!("b{}", i);
        builder = builder
            .take_from_worktop(RRC404_WATER, amounts[i], bucket.clone())
            .try_deposit_or_abort(test.env.users[i].address, None, bucket);
    }
    let receipt = runner.execute_manifest(
        builder.build(), vec![NonFungibleGlobalId::from_public_key(&test.env.owner.key)]);
    let result = receipt.expect_commit_success();
    result.outcome.expect_success();
}

pub fn deposit_water(runner: &mut TestRunner<NoExtension, InMemorySubstateDatabase>, test: DeployedEnv, user: Account, amount: Decimal) {
    let receipt = runner.execute_manifest(
        ManifestBuilder::new()
            .lock_fee_from_faucet()
            .withdraw_from_account(user.address, RRC404_WATER, amount)
            .take_all_from_worktop(RRC404_WATER, "bucket1")
            .with_name_lookup(|builder, lookup| {
                builder.call_method(
                    test.ice_randomizer,
                    "deposit",
                    manifest_args!(lookup.bucket("bucket1")),
                )
            })
            .deposit_batch(user.address)
            .build(), vec![NonFungibleGlobalId::from_public_key(&user.key)]);
    let result = receipt.expect_commit_success();
    result.outcome.expect_success();
}

pub fn withdraw_ice(runner: &mut TestRunner<NoExtension, InMemorySubstateDatabase>, test: DeployedEnv, user: Account, amount: Decimal) {
    let receipt = runner.execute_manifest(
        ManifestBuilder::new()
            .lock_fee_from_faucet()
            .withdraw_from_account(user.address, test.ticket_address, amount)
            .take_all_from_worktop(test.ticket_address, "bucket1")
            .with_name_lookup(|builder, lookup| {
                builder.call_method(
                    test.ice_randomizer,
                    "withdraw",
                    manifest_args!(lookup.bucket("bucket1")),
                )
            })
            .deposit_batch(user.address)
            .build(), vec![NonFungibleGlobalId::from_public_key(&user.key)]);
    let result = receipt.expect_commit_success();
    result.outcome.expect_success();
}


/// advance time by 4hrs, so later we can melt
fn advance_time(test_runner: &mut TestRunner<NoExtension, InMemorySubstateDatabase>) {
    let round = test_runner.get_consensus_manager_state().round.number();
    let current_time = test_runner.get_current_proposer_timestamp_ms();
    let ts = current_time + 4 * 60 * 60 * 1000;
    let res = test_runner.advance_to_round_at_timestamp(Round::of(round + 1), ts);
    res.expect_commit_success().outcome.expect_success();
}
