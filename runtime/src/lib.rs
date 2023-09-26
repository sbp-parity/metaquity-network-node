#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use codec::{Decode, Encode, MaxEncodedLen};
use pallet_grandpa::AuthorityId as GrandpaId;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		AccountIdLookup, BlakeTwo256, Block as BlockT, IdentifyAccount, NumberFor, One, Verify,
	},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, MultiSignature,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		AsEnsureOriginWithArg, ConstBool, ConstU128, ConstU32, ConstU64, ConstU8,
		KeyOwnerProofSystem, Randomness, StorageInfo,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		IdentityFee, Weight,
	},
	BoundedVec, PalletId, StorageValue,
};
pub use frame_system::Call as SystemCall;
use frame_system::{EnsureRoot, EnsureSigned};
pub use pallet_balances::Call as BalancesCall;
use pallet_nfts::PalletFeatures;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{ConstFeeMultiplier, CurrencyAdapter, Multiplier};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

/// An index to a block.
pub type BlockNumber = u32;

/// Function used in fee configurations
pub const fn deposit(items: u32, bytes: u32) -> Balance {
	// SBP-M1 review: implementation may increase likelihood of chain storage bloat by returning a relatively small value for a deposit, based on the number of decimals currently used on the chain.
	// SBP-M1 review: typical implementations include an additional multiplier. See deposit function implementations within runtimes at https://github.com/paritytech/polkadot-sdk/tree/master/polkadot/runtime and https://github.com/paritytech/extended-parachain-template/blob/main/runtime/mainnet/src/lib.rs#L223 as examples.
	items as Balance + (bytes as Balance) * 100
}
/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
		}
	}
}

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("metaquity-network"),
	impl_name: create_runtime_str!("metaquity-network"),
	authoring_version: 1,
	spec_version: 100,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
// SBP-M1 review: consider effects of block time increasing when migrating from solo chain to parachain (if applicable): https://github.com/paritytech/extended-parachain-template/blob/3bec37d7844880d13e0a1f3253d1402500f83789/runtime/common/src/lib.rs#L22
// SBP-M1 review: note also that `asynchronous backing` is set to reduce parachain block times to 6 seconds once available
pub const MILLISECS_PER_BLOCK: u64 = 6000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// SBP-M1 review: consider generic unit naming (or MQTY instead of UNITS/DOLLARS) - e.g. https://github.com/paritytech/extended-parachain-template/blob/3bec37d7844880d13e0a1f3253d1402500f83789/runtime/mainnet/src/lib.rs#L218
pub const MILLICENTS: Balance = 1_000_000_000;
pub const CENTS: Balance = 1_000 * MILLICENTS; // assume this is worth about a cent.
// SBP-M1 review: is 14 decimal places intentional? 18 is specified at https://github.com/paritytech/ss58-registry/blob/main/ss58-registry.json#L882. Suggest setting UNITS/DOLLARS/MQTY to 18 decimal value and then divide accordingly for sub-units for clarity. Consider adding additional metadata in the chain_spec.rs as well - e.g. https://github.com/paritytech/extended-parachain-template/blob/3bec37d7844880d13e0a1f3253d1402500f83789/node/src/chain_spec.rs#L136
pub const DOLLARS: Balance = 100 * CENTS;
// SBP-M1 review: should UNITS simply be a type alias to DOLLARS?
pub const UNITS: Balance = 1;
/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::with_sensible_defaults(
			Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
			NORMAL_DISPATCH_RATIO,
		);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u16 = 666;
}

// Configure FRAME pallets to include in runtime.

// SBP-M1 review: consider matching member order with that of trait
impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;
	/// The block type for the runtime.
	type Block = Block;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
	// SBP-M1 review: unnecessary qualification
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

// SBP-M1 review: consider matching member order with that of trait
impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<32>;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = ();
	type MaxAuthorities = ConstU32<32>;
	type MaxSetIdSessionEntries = ConstU64<0>;

	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

/// Existential deposit.
// SBP-M1 review: very small number for ED, especially for a chain with 18 decimals. Update to some fraction of a UNIT - e.g. https://github.com/paritytech/extended-parachain-template/blob/3bec37d7844880d13e0a1f3253d1402500f83789/runtime/mainnet/src/lib.rs#L220
// SBP-M1 review: see https://wiki.polkadot.network/docs/build-protocol-info#existential-deposit for more information
pub const EXISTENTIAL_DEPOSIT: u128 = 1000;

// SBP-M1 review: consider matching member order with that of trait
impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
  // SBP-M1 review: add comment noting why this is set to one - i.e. HoldReason::NftFractionalization
	type MaxHolds = ConstU32<1>;
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// SBP-M1 review: consider a mechanism for dealing with transaction fees - e.g. DealWithFees
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	// SBP-M1 review: consider non-default weight to fee mechanisms - e.g. https://github.com/paritytech/extended-parachain-template/blob/3bec37d7844880d13e0a1f3253d1402500f83789/runtime/mainnet/src/lib.rs#L126
	type WeightToFee = IdentityFee<Balance>;
	// SBP-M1 review: consider non-default length to fee mechanisms
	type LengthToFee = IdentityFee<Balance>;
	// SBP-M1 review: consider non-default fee multiplier update mechanisms - e.g. SlowAdjustingFeeUpdate
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// Minimum 100bytes
	// SBP-M1 review: consider using deposit() function for consistency with other pallets deposits (and as per Polkadot runtime).
	pub const BasicDeposit: Balance = 1000 * CENTS;        //258 bytes on-chain
	// SBP-M1 review: consider using deposit() function for consistency with other pallets deposits (and as per Polkadot runtime).
	pub const FieldDeposit: Balance = 250 * CENTS;         //66 bytes on-chain
	// SBP-M1 review: consider using deposit() function for consistency with other pallets deposits (and as per Polkadot runtime).
	pub const SubAccountDeposit: Balance = 200 * CENTS;    // 53 bytes on-chain
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxRegistrars: u32= 20;
}

impl pallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type FieldDeposit = FieldDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type MaxAdditionalFields = MaxAdditionalFields;
	type MaxRegistrars = MaxRegistrars;
	// SBP-M1 review: consider what happens with slashed funds - e.g. treasury
	type Slashed = ();
	// SBP-M1 review: should be EnsureRoot
	type ForceOrigin = EnsureSigned<Self::AccountId>;
	// SBP-M1 review: should be EnsureRoot or MQTY admin origin to maintain registrar integrity
	type RegistrarOrigin = EnsureSigned<Self::AccountId>;
	type WeightInfo = ();
}

parameter_types! {
	// SBP-M1 review: re-consider value after adjusting units mentioned above.
	pub const AssetDeposit: Balance = 100 * DOLLARS;
	// SBP-M1 review: prefer inlining if type only used once - e.g. ConstU128. Also re-consider value after adjusting units mentioned above.
	pub const ApprovalDeposit: Balance = 1 * DOLLARS;
	pub const StringLimit: u32 = 50;
	// SBP-M1 review: prefer inlining if type only used once - e.g. ConstU128. Also re-consider value after adjusting units mentioned above.
	pub const MetadataDepositBase: Balance = 10 * DOLLARS;
	// SBP-M1 review: prefer inlining if type only used once - e.g. ConstU128. Also re-consider value after adjusting units mentioned above.
	pub const MetadataDepositPerByte: Balance = 1 * DOLLARS;
}

// SBP-M1 review: consider matching member order with that of trait
impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// SBP-M1 review: reuse Balance type rather than explicit u128?
	type Balance = u128;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	// SBP-M1 review: consider whether anyone should be able to permissionlessly create an asset - should probably be set to MQTY admin origin only.
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	// SBP-M1 review: may need to be root or MQTY admin origin to allow force_set_metadata for fractionalised assets - see EitherOf<L, R>.
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	// SBP-M1 review: re-consider this after adjusting the units mentioned above
	type AssetAccountDeposit = ConstU128<DOLLARS>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
	// SBP-M1 review: use separator for consistency - i.e. 1_000
	type RemoveItemsLimit = ConstU32<1000>;
}

// SBP-M1 review: pallet_uniques is not used, so these parameters can be removed, with values moved to Nfts* parameter types
parameter_types! {
	// SBP-M1 review: UNITS is 1, resulting in deposit of zero. This needs to be fixed.
	// SBP-M1 review: only used once, move implementation to usage to remove this parameter
	pub const UniquesCollectionDeposit: Balance = UNITS /10;
	// SBP-M1 review: UNITS is 1, resulting in deposit of zero. This needs to be fixed.
	// SBP-M1 review: only used once, move implementation to usage to remove this parameter
	pub const UniquesItemDeposit: Balance = UNITS / 1_000;
	// SBP-M1 review: only used once, move implementation to usage to remove this parameter
	// SBP-M1 review: provide justification as to how 129 is determined. I do see that it is configured this way on Asset Hub on Polkadot/Kusama though.
	pub const UniquesMetadataDepositsBase: Balance = deposit(1, 129);
	// SBP-M1 review: only used once, move implementation to usage to remove this parameter
	// SBP-M1 review: provide justification as to how 129 is determined. Asset Hub on Polkadot/Kusama has this configured as deposit(1, 0).
	pub const UniquesAttributeDepositsBase: Balance = deposit(1, 129);
	// SBP-M1 review: only used once, move implementation to usage to remove this parameter
	pub const UniquesDepositPerByte: Balance = deposit(0, 1);
}

parameter_types! {
	pub NftsPalletFeatures: PalletFeatures = PalletFeatures::all_enabled();
	// SBP-M1 review: only used once, inline value via ConstU32 to remove this parameter
	pub const NftsMaxDeadlineDuration: BlockNumber = 12 * 30 * DAYS;

	// SBP-M1 review: typo
	// reuse the unique deopsits
	// SBP-M1 review: move impls from above for each of the following to eliminate Uniques* parameter types above
	pub const NftsCollectionDeposit: Balance = UniquesCollectionDeposit::get();
	pub const NftsItemDeposit: Balance = UniquesItemDeposit::get();
	pub const NftsMetadataDepositsBase: Balance = UniquesMetadataDepositsBase::get();
	pub const NftsAttributeDepositsBase: Balance = UniquesAttributeDepositsBase::get();
	pub const NftsDepositPerByte: Balance = UniquesDepositPerByte::get();
}

// SBP-M1 review: consider matching member order with that of trait
impl pallet_nfts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	// SBP-M1 review: consider whether any user with access to public chain should be able to permissionlessly create collections, which is currently the case here. The use-case/UI screenshots imply that asset verification is required, so assume the onchain creation of collections should only be carried out by MQTY admin (e.g. configure CreateOrigin as MQTY admin) and then assets (NFTs) minted by the collection admin once verified.
	// SBP-M1 review: unnecessary qualification (frame_system::)
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
	// SBP-M1 review: unnecessary qualification (frame_system::)
	type ForceOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type Locker = ();
	type CollectionDeposit = NftsCollectionDeposit;
	type ItemDeposit = NftsItemDeposit;
	type MetadataDepositBase = NftsMetadataDepositsBase;
	type AttributeDepositBase = NftsAttributeDepositsBase;
	type DepositPerByte = NftsDepositPerByte;
	type StringLimit = ConstU32<50>;
	type KeyLimit = ConstU32<50>;
	type ValueLimit = ConstU32<50>;
	type ApprovalsLimit = ConstU32<10>;
	type ItemAttributesApprovalsLimit = ConstU32<2>;
	type MaxTips = ConstU32<10>;
	type MaxDeadlineDuration = NftsMaxDeadlineDuration;
	type MaxAttributesPerCall = ConstU32<2>;
	type Features = NftsPalletFeatures;
	/// Off-chain = signature On-chain - therefore no conversion needed.
	/// It needs to be From<MultiSignature> for benchmarking.
	type OffchainSignature = Signature;
	type OffchainPublic = <Signature as Verify>::Signer;
	type WeightInfo = ();
}

/// A reason for placing a hold on funds.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	MaxEncodedLen,
	Debug,
	scale_info::TypeInfo,
)]
pub enum HoldReason {
	/// Used by the NFT Fractionalization Pallet.
	NftFractionalization,
}

parameter_types! {
	pub const NftFractionalizationPalletId: PalletId = PalletId(*b"fraction");
	// SBP-M1 review: consider BoundedVec::unchecked_from() or use .expect("reason") rather than .unwrap(). I see the Asset Hub on Kusama is configured this way though.
	pub NewAssetSymbol: BoundedVec<u8, StringLimit> = (*b"FRAC").to_vec().try_into().unwrap();
	// SBP-M1 review: consider something more informative like 'Fractionalized Asset'. May not matter though, as it will probably require an assets::force_set_metadata to customise the fractionalized asset metadata after the NFT has been fractionalized.
	pub NewAssetName: BoundedVec<u8, StringLimit> = (*b"Frac").to_vec().try_into().unwrap();
}

// SBP-M1 review: consider matching member order with that of trait
impl pallet_nft_fractionalization::Config for Runtime {
	// SBP-M1 review: whilst it resolves to the same type, consider using <Self as pallet_assets::Config>::Balance as it would better align with AssetId and Assets type definitions below. I see the Asset Hub on Kusama is configured this way though.
	type AssetBalance = <Self as pallet_balances::Config>::Balance;
	type AssetId = <Self as pallet_assets::Config>::AssetId;
	type Assets = Assets;
	type Currency = Balances;
  // SBP-M1 review: uses AssetDeposit rather than NftsCollectionDeposit and cannot determine whether this is intentional. I see the Asset Hub on Kusama is configured this way though. Suggest adding NftFractionalizationDeposit alias to AssetDeposit or NftsCollectionDeposit with a comment as to why it is being used in your runtime for clarity.
	type Deposit = AssetDeposit;
	type NewAssetName = NewAssetName;
	type NewAssetSymbol = NewAssetSymbol;
	type NftCollectionId = <Self as pallet_nfts::Config>::CollectionId;
	type NftId = <Self as pallet_nfts::Config>::ItemId;
	type Nfts = Nfts;
	type PalletId = NftFractionalizationPalletId;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type StringLimit = StringLimit;
	type WeightInfo = pallet_nft_fractionalization::weights::SubstrateWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime {
    // SBP-M1 review: explicit pallet indices preferred - e.g. https://github.com/paritytech/extended-parachain-template/blob/3bec37d7844880d13e0a1f3253d1402500f83789/runtime/mainnet/src/lib.rs#L564
		System: frame_system,
		Timestamp: pallet_timestamp,
		Aura: pallet_aura,
		Grandpa: pallet_grandpa,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,
		Sudo: pallet_sudo,

		Utility: pallet_utility,
		Identity: pallet_identity,
		Assets: pallet_assets,
		Nfts: pallet_nfts,
		NftFractionalization: pallet_nft_fractionalization,
	}
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_sudo, Sudo]
    // SBP-M1 review: add missing pallets: benchmarks should be re-run on reference hardware based on how they are configured/used by your runtime
	);
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_consensus_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			_authority_id: GrandpaId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, TrackedStorageKey};

			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, BlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
		}
	}
}
