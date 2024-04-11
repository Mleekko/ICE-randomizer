use std::env;

use dot_random_test_utils::{deploy_random_component, RandomTestEnv};
use dot_random_test_utils::cargo::get_repo_sub_dir;
use radix_engine::vm::NoExtension;
use scrypto::this_package;
use scrypto_test::prelude::InMemorySubstateDatabase;
use scrypto_unit::*;
use transaction::prelude::*;

pub const RRC404_PACKAGE: PackageAddress = PackageAddress::new_or_panic([
    13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 28, 225, 206, 28, 224, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
]); // package_sim1p5qqqqqqqqqqqqqqqqqpecwwrnsqqqqqqqqqqqqqqqqqqqqqj5zvnh

pub const RRC404_COMPONENT: ComponentAddress = ComponentAddress::new_or_panic([
    192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 28, 225, 206, 28, 224, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
]); // component_sim1cqqqqqqqqqqqqqqqqqqpecwwrnsqqqqqqqqqqqqqqqqqqqqqgguvvr

pub const RRC404_WATER: ResourceAddress = ResourceAddress::new_or_panic([
    93, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 28, 225, 206, 28, 224, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
]); // resource_sim1t5qqqqqqqqqqqqqqqqqpecwwrnsqqqqqqqqqqqqqqqqqqqqqs3ask4

pub const RRC404_ICE: ResourceAddress = ResourceAddress::new_or_panic([
    154, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 28, 225, 206, 28, 224, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
]); // resource_sim1ngqqqqqqqqqqqqqqqqqpecwwrnsqqqqqqqqqqqqqqqqqqqqq6lw2hr


pub const AMOUNTS: [Decimal; 5] = [dec!(10), dec!(40), dec!(5), dec!(25), dec!(60)];


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

    pub fn deploy(mut self, runner: &mut TestRunner<NoExtension, InMemorySubstateDatabase>) -> (RandomTestEnv<NoExtension, InMemorySubstateDatabase>, DeployedEnv) {
        // Deploy RandomComponent
        let mut random_env = deploy_random_component(runner, "55cf37d");

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
fn test_something() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().build();
    let mut env = TestEnv::init(&mut test_runner);
    let (mut random_env, mut test) = env.deploy(&mut test_runner);

    allocate_tokens(&mut test_runner, test);

    // Act
    // 1. Users lock tokens
    let mut index = 0usize;
    while index < AMOUNTS.len() {
        deposit_water(&mut test_runner, test, env.users[index], AMOUNTS[index]);
        index += 1;
    }

    // 2. Owner triggers random mint - should return callback id: 1
    let receipt = test_runner.execute_manifest_ignoring_fee(
        ManifestBuilder::new()
            .create_proof_from_account_of_amount(env.owner.address, test.randomizer_owner, dec!(1))
            .call_method(
                test.ice_randomizer,
                "mint",
                manifest_args!(80u32),
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
    let mut index = 0usize;
    while index < AMOUNTS.len() {
        let account = env.users[index];
        withdraw_ice(&mut test_runner, test, account, AMOUNTS[index]);
        println!("Balance: {} -> {:?}/{:?}", index,
                 test_runner.get_component_balance(account.address, RRC404_WATER),
                 test_runner.get_component_balance(account.address, RRC404_ICE)
        );
        index += 1;
    }

    // Assert component is empty
    let balance_ice = test_runner.get_component_balance(test.ice_randomizer, RRC404_ICE);
    assert_eq!(dec!(0), balance_ice);
    let balance_water = test_runner.get_component_balance(test.ice_randomizer, RRC404_WATER);
    assert_eq!(dec!(0), balance_water);
}


pub fn allocate_tokens(runner: &mut TestRunner<NoExtension, InMemorySubstateDatabase>, test: DeployedEnv) {
    let receipt = runner.execute_manifest_ignoring_fee(
        ManifestBuilder::new()
            .withdraw_from_account(test.env.owner.address, RRC404_WATER, dec!(140))
            .take_from_worktop(RRC404_WATER, AMOUNTS[0], "b1")
            .take_from_worktop(RRC404_WATER, AMOUNTS[1], "b2")
            .take_from_worktop(RRC404_WATER, AMOUNTS[2], "b3")
            .take_from_worktop(RRC404_WATER, AMOUNTS[3], "b4")
            .take_from_worktop(RRC404_WATER, AMOUNTS[4], "b5")
            .try_deposit_or_abort(test.env.users[0].address, None, "b1")
            .try_deposit_or_abort(test.env.users[1].address, None, "b2")
            .try_deposit_or_abort(test.env.users[2].address, None, "b3")
            .try_deposit_or_abort(test.env.users[3].address, None, "b4")
            .try_deposit_or_abort(test.env.users[4].address, None, "b5")
            .build(), vec![NonFungibleGlobalId::from_public_key(&test.env.owner.key)]);
    let result = receipt.expect_commit_success();
    let out = result.outcome.expect_success();
}

pub fn deposit_water(runner: &mut TestRunner<NoExtension, InMemorySubstateDatabase>, test: DeployedEnv, user: Account, amount: Decimal) {
    let receipt = runner.execute_manifest_ignoring_fee(
        ManifestBuilder::new()
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
    let receipt = runner.execute_manifest_ignoring_fee(
        ManifestBuilder::new()
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
