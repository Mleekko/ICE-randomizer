use std::env;

use scrypto_unit::*;
use transaction::prelude::*;
use dot_random_test_utils::{deploy_random_component};
use dot_random_test_utils::cargo::get_repo_sub_dir;
use radix_engine::transaction::CommitResult;
use radix_engine::vm::NoExtension;
use scrypto::this_package;
use scrypto_test::prelude::InMemorySubstateDatabase;

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



#[test]
fn test_something() {
    // Arrange
    let mut test_runner = TestRunnerBuilder::new().without_trace().build();
    let (public_key, _, owner_account) = test_runner.new_allocated_account();

    // Deploy RandomComponent
    let mut random_env = deploy_random_component(&mut test_runner, "55cf37d");

    // Deploy ICE-RRC404
    let rrc404v1_path = get_repo_sub_dir("ice_rrc404v1", "d99f72d", "");
    deploy_rrc404(&mut test_runner, rrc404v1_path.to_str().unwrap(), &public_key, &owner_account);


    // Deploy Ice Randomizer
    let package_address = test_runner.publish_package_simple(
        this_package!()
    );
    let receipt = test_runner.execute_manifest(
        ManifestBuilder::new()
            .lock_fee_from_faucet()
            .call_function(
                package_address,
                "IceRandomizer",
                "instantiate",
                manifest_args!(),
            )
            .deposit_batch(owner_account)
            .build(), vec![NonFungibleGlobalId::from_public_key(&public_key)]);

    let result = receipt.expect_commit_success();
    let ice_randomizer = result.new_component_addresses()[0];
    let kv_store = get_kv_store(result);

}

fn deploy_rrc404(test_runner: &mut TestRunner<NoExtension, InMemorySubstateDatabase>, dir_rrc404v1: &str,
                 public_key: &Secp256k1PublicKey, owner_account: &ComponentAddress) -> ComponentAddress {
    test_runner.compile_and_publish_at_address(dir_rrc404v1, RRC404_PACKAGE);

    let receipt = test_runner.execute_system_transaction_with_preallocated_addresses(
        vec![
            InstructionV1::CallFunction {
                package_address: DynamicPackageAddress::Static(RRC404_PACKAGE),
                blueprint_name: "Rrc404".to_string(),
                function_name: "instantiate".to_string(),
                args: manifest_args!(Some(ManifestAddressReservation(0)), Some(ManifestAddressReservation(1)), Some(ManifestAddressReservation(2))).into(),
            },
            InstructionV1::CallMethod {
                address: DynamicGlobalAddress::Static(GlobalAddress::new_or_panic((*owner_account).into())),
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
        btreeset!(NonFungibleGlobalId::from_public_key(public_key)),
    );
    let result = receipt.expect_commit_success();
    let rrc404_component = result.new_component_addresses()[0];

    println!("rrc404_component: {:?}", rrc404_component);
    return rrc404_component;
}

fn get_kv_store(result: &CommitResult) -> &NodeId {
    // Our KVS is last created.
    let mut last: &NodeId = &NodeId([1u8; 30]);
    for (node_id, _) in &result.state_updates.by_node {
        if node_id.is_internal_kv_store() {
            last = node_id;
        }
    }
    return last;
}