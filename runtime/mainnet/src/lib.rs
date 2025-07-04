// Build both the Native Rust binary and the WASM binary.
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub use bp_core::{AccountId, Address, Balance, BlockNumber, Hash, Header, Nonce, Signature};
use codec::{Decode, Encode};
use sp_std::marker::PhantomData;

pub use taker_mainnet_constants::{
	currency::{GWEI, UNITS as BFC, *},
	fee::*,
	time::*,
	BABE_GENESIS_EPOCH_CONFIG, DEFAULT_ADMIN, FEE_COLLECTOR,
};

use fp_rpc::TransactionStatus;
use fp_rpc_txpool::TxPoolResponse;
use sp_api::impl_runtime_apis;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{crypto::KeyTypeId, ConstU64, OpaqueMetadata, H160, H256, U256};
use sp_genesis_builder::PresetId;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		BlakeTwo256, Block as BlockT, DispatchInfoOf, Dispatchable, IdentityLookup, NumberFor,
		PostDispatchInfoOf, UniqueSaturatedInto,
	},
	transaction_validity::{TransactionSource, TransactionValidity, TransactionValidityError},
	ApplyExtrinsicResult,
};
pub use sp_runtime::{traits, ExtrinsicInclusionMode, Perbill, Percent, Permill};
use sp_std::prelude::*;

use sp_staking::currency_to_vote::U128CurrencyToVote;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use pallet_balances::{Call as BalancesCall, NegativeImbalance};
use pallet_ethereum::{
	Call::transact, EthereumBlockHashMapping, PostLogContent, Transaction as EthereumTransaction,
};
use pallet_evm::{
	Account as EVMAccount, EVMCurrencyAdapter, EnsureAddressNever, EnsureAddressRoot,
	FeeCalculator, GasWeightMapping, IdentityAddressMapping, Runner,
};
use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
pub use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as session_historical;
pub use pallet_staking::StakerStatus;
pub use pallet_staking_runtime_api::{NominatorInfo, ValidatorInfo};
pub use pallet_timestamp::Call as TimestampCall;
#[allow(deprecated)]
use pallet_transaction_payment::CurrencyAdapter;
use sp_runtime::{PerThing, SaturatedConversion};

use frame_election_provider_support::{onchain, SequentialPhragmen};
pub use frame_support::{
	derive_impl,
	dispatch::{DispatchClass, GetDispatchInfo},
	genesis_builder_helper::{build_state, get_preset},
	pallet_prelude::Get,
	parameter_types,
	traits::{
		ConstU128, ConstU32, ConstU8, Contains, Currency, EitherOfDiverse, EqualPrivilegeOnly,
		FindAuthor, Imbalance, InsideBoth, KeyOwnerProofSystem, LockIdentifier, NeverEnsureOrigin,
		OnFinalize, OnUnbalanced, Randomness, StorageInfo, StorageMapShim,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		ConstantMultiplier, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients,
		WeightToFeePolynomial,
	},
	ConsensusEngineId, PalletId, StorageValue,
};

mod precompiles;
pub use precompiles::TakerPrecompiles;
pub type Precompiles = TakerPrecompiles<Runtime>;

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
	fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;
	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub babe: Babe,
			pub grandpa: Grandpa,
			pub im_online: ImOnline,
		}
	}
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("frontier-template"),
	impl_name: create_runtime_str!("frontier-template"),
	// The version of the authorship interface.
	authoring_version: 1,
	// The version of the runtime spec.
	spec_version: 108,
	// The version of the implementation of the spec.
	impl_version: 1,
	// A list of supported runtime APIs along with their versions.
	apis: RUNTIME_API_VERSIONS,
	// The version of the interface for handling transactions.
	transaction_version: 1,
	// The version of the interface for handling state transitions.
	state_version: 1,
};

/// Maximum weight per block.
/// We allow for 1 second of compute with a 3 second average block time, with maximum proof size.
const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_div(2), u64::MAX);

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const BlockHashCount: BlockNumber = 256;
	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights
		::with_sensible_defaults(MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO);
	/// We allow for 5 MB blocks.
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

/// The System pallet defines the core data types used in a Substrate runtime
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
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
	/// The lookup mechanism to get the account ID from whatever is passed in dispatchers.
	type Lookup = IdentityLookup<AccountId>;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Provides information about the pallet setup in the runtime.
	type PalletInfo = PalletInfo;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The maximum number of consumers allowed on a single account.
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub const BabeEpochDuration: u64 = 10 * MINUTES as u64;
	pub const ExpectedBlockTime: u64 = MILLISECS_PER_BLOCK;
	pub const ReportLongevity: u64 = 0;
	pub const MaxAuthorities: u32 = 100;
	pub const MaxNominators: u32 = 100;
}

impl pallet_babe::Config for Runtime {
	type EpochDuration = BabeEpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type DisabledValidators = Session;
	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, BabeId)>>::Proof;
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
	type EquivocationReportSystem =
		pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
	type WeightInfo = ();
}

/// Provides the GRANDPA block finality gadget.
impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
	type MaxSetIdSessionEntries = ConstU64<0>;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;

	type Keys = crate::opaque::SessionKeys;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler =
		<crate::opaque::SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type WeightInfo = ();
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

parameter_types! {
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const MaxPeerDataEncodingSize: u32 = 1_000;
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
	type RuntimeEvent = RuntimeEvent;
	type ValidatorSet = Historical;
	type NextSessionRotation = ();
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
	type DefaultSlashFraction = ();
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = (Staking, ImOnline);
}

impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: sp_runtime::curve::PiecewiseLinear<'static> = curve!(
		min_inflation: 0_050_000,
		max_inflation: 0_053_200,
		ideal_stake: 0_400_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	pub const SessionsPerEra: sp_staking::SessionIndex = 3;
	pub const BondingDuration: sp_staking::EraIndex = 1;
	pub const SlashDeferDuration: sp_staking::EraIndex = 2;
	pub const RewardCurve: &'static sp_runtime::curve::PiecewiseLinear<'static> = &REWARD_CURVE;
	pub HistoryDepth: u32 = 84;
	pub const MaxNominatorRewardedPerValidator: u32 = 256;
	/// Maximum number of nominations per nominator.
	pub const MaxNominations: u32 = 16;
	pub const MaxIterations: u32 = 5;
	// 0.05%. The higher the value, the more strict solution acceptance becomes.
	pub MinSolutionScoreBump: Perbill = sp_runtime::PerThing::from_rational(5u32, 10_000);
	pub StakingUnsignedPriority: frame_support::pallet_prelude::TransactionPriority =
		Perbill::from_percent(90) * frame_support::pallet_prelude::TransactionPriority::max_value();
	pub const ImOnlineUnsignedPriority: frame_support::pallet_prelude::TransactionPriority = frame_support::pallet_prelude::TransactionPriority::max_value();
	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxNominators = ConstU32<1000>;
	type MaxValidators = ConstU32<1000>;
}

parameter_types! {
	pub MaxElectingVoters: u32 = 40_000;
	pub MaxActiveValidators: u32 = 1000;
	pub MaxOnChainElectingVoters: u32 = 5000;
	pub MaxOnChainElectableTargets: u16 = 1250;
	pub ElectionBounds: frame_election_provider_support::bounds::ElectionBounds =
	frame_election_provider_support::bounds::ElectionBoundsBuilder::default().voters_count(MaxElectingVoters::get().into()).build();
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Runtime;
	type Solver = SequentialPhragmen<AccountId, Perbill>;
	type DataProvider = Staking;
	type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Runtime>;
	type MaxWinners = MaxActiveValidators;
	type Bounds = ElectionBounds;
}

impl pallet_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type UnixTime = Timestamp;
	type GasCurrency = Balances;
	type Currency = AssetCurrency;
	type CurrencyToVote = U128CurrencyToVote;
	type RewardRemainder = (); // Treasury
	type Slash = (); // Treasury
	type Reward = (); // rewards are minted from the voi
	type SessionInterface = Self;
	type NextNewSession = Session;
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type EraPayout = pallet_staking::FixedRelease;
	type WeightInfo = ();
	// type CurrencyBalance = Balance;
	type CurrencyBalance = <Self as pallet_asset_currency::Config>::Balance;
	type GenesisElectionProvider = Self::ElectionProvider;
	type MaxNominations = MaxNominations;
	type HistoryDepth = HistoryDepth;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
	type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Runtime>;
	type TargetList = pallet_staking::UseValidatorsMap<Runtime>;
	type MaxUnlockingChunks = ConstU32<32>;
	type OnStakerSlash = ();
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type ElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

/// A timestamp: milliseconds since the unix epoch.
impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

/// Provides functionality for handling accounts and balances.
impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<0>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

parameter_types! {
	pub const AssetExistentialDeposit: u128 = 0;
	pub const AssetMaxLocks: u32 = 50;
	pub const AssetMaxReserves: u32 = 50;
	pub const DefaultAdmin: AccountId = DEFAULT_ADMIN;
	pub const AssetPalletId: PalletId = PalletId(*b"asset/id");
}

impl pallet_asset_currency::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type NativeCurrency = Balances;
	type Balance = Balance;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type HoldIdentifier = ();
	type DustRemoval = ();
	type ExistentialDeposit = AssetExistentialDeposit;
	type MaxLocks = AssetMaxLocks;
	type MaxReserves = AssetMaxReserves;
	type MaxHolds = ConstU32<1>;
	type MaxFreezes = ();
	type DefaultAdmin = DefaultAdmin;
	type GasFeeCollector = FeeCollector;
	type PalletId = AssetPalletId;
}

parameter_types! {
	pub const TransactionByteFee: Balance = TRANSACTION_BYTE_FEE;
	pub const FeeCollector: AccountId = FEE_COLLECTOR;
}

type NegativeImbalanceOf<C, T> =
	<C as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

pub struct DealWithFees<T, C>(sp_std::marker::PhantomData<(T, C)>);
impl<C, T> OnUnbalanced<NegativeImbalanceOf<C, T>> for DealWithFees<T, C>
where
	T: frame_system::Config + pallet_asset_currency::Config,
	C: Currency<<T as frame_system::Config>::AccountId>,
{
	fn on_nonzero_unbalanced(fees: NegativeImbalanceOf<C, T>) {
		C::resolve_creating(&<T as pallet_asset_currency::Config>::GasFeeCollector::get(), fees);
	}
}

/// Provides the basic logic needed to pay the absolute minimum amount needed for a transaction to
/// be included.
impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees<Runtime, Balances>>;
	type WeightToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = ();
	type OperationalFeeMultiplier = ConstU8<5>;
}

/// The Sudo module allows for a single account (called the "sudo key")
/// to execute dispatchable functions that require a `Root` call
/// or designate a new account to replace them as the sudo key.
impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub BlockGasLimit: U256 = U256::from(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT.ref_time() / WEIGHT_PER_GAS);
	pub WeightPerGas: Weight = Weight::from_parts(WEIGHT_PER_GAS, 0);
	/// The amount of gas per pov. A ratio of 4 if we convert ref_time to gas and we compare
	/// it with the pov_size for a block. E.g.
	/// ceil(
	///     (max_extrinsic.ref_time() / max_extrinsic.proof_size()) / WEIGHT_PER_GAS
	/// )
	pub PrecompilesValue: Precompiles = TakerPrecompiles::<_>::new();
	pub const GasLimitPovSizeRatio: u64 = 4;
	pub const GasLimitStorageGrowthRatio: u64 = 366;
}

pub struct TransactionConverter;
impl fp_rpc::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_unsigned(
			pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
		)
	}
}
impl fp_rpc::ConvertTransaction<opaque::UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(
		&self,
		transaction: pallet_ethereum::Transaction,
	) -> opaque::UncheckedExtrinsic {
		let extrinsic = UncheckedExtrinsic::new_unsigned(
			pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
		);
		let encoded = extrinsic.encode();
		opaque::UncheckedExtrinsic::decode(&mut &encoded[..])
			.expect("Encoded extrinsic is always valid")
	}
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> (U256, Weight) {
		(pallet_base_fee::Pallet::<Runtime>::min_gas_price().0, Weight::zero())
	}
}

pub struct EthereumFindAuthor<F>(PhantomData<F>);
impl<F: frame_support::traits::FindAuthor<u32>> frame_support::traits::FindAuthor<sp_core::H160>
	for EthereumFindAuthor<F>
{
	fn find_author<'a, I>(digests: I) -> Option<sp_core::H160>
	where
		I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
	{
		if let Some(author_index) = F::find_author(digests) {
			let authority_id = Babe::authorities()[author_index as usize].clone();
			let queued_keys = <pallet_session::Pallet<Runtime>>::queued_keys();
			for key in queued_keys {
				if key.1.babe == authority_id.0 {
					return Some(key.0.into());
				}
			}
		}
		None
	}
}

/// The EVM module allows unmodified EVM code to be executed in a Substrate-based blockchain.
impl pallet_evm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockGasLimit = BlockGasLimit;
	type ChainId = EVMChainId;
	type BlockHashMapping = EthereumBlockHashMapping<Self>;
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type CallOrigin = EnsureAddressRoot<AccountId>;
	type WithdrawOrigin = EnsureAddressNever<AccountId>;
	type AddressMapping = IdentityAddressMapping;
	type FeeCalculator = FixedGasPrice;
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
	type WeightPerGas = WeightPerGas;
	type OnChargeTransaction = EVMCurrencyAdapter<Balances, DealWithFees<Runtime, Balances>>;
	type FindAuthor = EthereumFindAuthor<Babe>;
	type PrecompilesType = TakerPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type OnCreate = ();
	type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
	type Timestamp = Timestamp;
	type SuicideQuickClearLimit = ConstU32<0>;
	type WeightInfo = pallet_evm::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

/// The Ethereum module is responsible for storing block data and provides RPC compatibility.
impl pallet_ethereum::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
	type PostLogContent = PostBlockAndTxnHashes;
	type ExtraDataLength = ConstU32<30>;
}

parameter_types! {
	pub DefaultBaseFeePerGas: U256 = (GWEI / 10).into();
	pub DefaultElasticity: Permill = Permill::zero();
}

pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
	fn lower() -> Permill {
		Permill::zero()
	}
	fn ideal() -> Permill {
		Permill::from_parts(500_000)
	}
	fn upper() -> Permill {
		Permill::from_parts(1_000_000)
	}
}

/// The Base fee module adds support for EIP-1559 transactions and handles base fee calculations.
impl pallet_base_fee::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Threshold = BaseFeeThreshold;
	type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
	type DefaultElasticity = DefaultElasticity;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

impl pallet_evm_chain_id::Config for Runtime {}

parameter_types! {
	pub BoundDivision: U256 = U256::from(1024);
}

impl pallet_dynamic_fee::Config for Runtime {
	type MinGasPriceBoundDivisor = BoundDivision;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u32 = 100;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Runtime;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	#[runtime::pallet_index(2)]
	pub type Timestamp = pallet_timestamp;

	#[runtime::pallet_index(3)]
	pub type Babe = pallet_babe;

	#[runtime::pallet_index(4)]
	pub type Balances = pallet_balances;

	#[runtime::pallet_index(6)]
	pub type AssetCurrency = pallet_asset_currency;

	#[runtime::pallet_index(7)]
	pub type TransactionPayment = pallet_transaction_payment;

	#[runtime::pallet_index(8)]
	pub type Utility = pallet_utility;

	#[runtime::pallet_index(14)]
	pub type Grandpa = pallet_grandpa;

	#[runtime::pallet_index(15)]
	pub type Session = pallet_session;

	#[runtime::pallet_index(16)]
	pub type Historical = session_historical;

	#[runtime::pallet_index(17)]
	pub type Staking = pallet_staking;

	#[runtime::pallet_index(18)]
	pub type Authorship = pallet_authorship;

	#[runtime::pallet_index(19)]
	pub type ImOnline = pallet_im_online;

	#[runtime::pallet_index(20)]
	pub type Offences = pallet_offences;

	#[runtime::pallet_index(40)]
	pub type EVM = pallet_evm;

	#[runtime::pallet_index(41)]
	pub type Ethereum = pallet_ethereum;

	#[runtime::pallet_index(42)]
	pub type BaseFee = pallet_base_fee;

	#[runtime::pallet_index(43)]
	pub type EVMChainId = pallet_evm_chain_id;

	#[runtime::pallet_index(44)]
	pub type DynamicFee = pallet_dynamic_fee;

	#[runtime::pallet_index(99)]
	pub type Sudo = pallet_sudo;

	#[runtime::pallet_index(100)]
	pub type Multisig = pallet_multisig;
}

taker_common_runtime::impl_common_runtime_apis!();
taker_common_runtime::impl_self_contained_call!();
