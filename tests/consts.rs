use radix_engine::prelude::{ComponentAddress, PackageAddress, ResourceAddress};

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
