use crate::cli_opt::EthApi as EthApiCmd;

use std::{collections::BTreeMap, sync::Arc};

use fc_rpc::{EthBlockDataCacheTask, StorageOverride};
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use sc_client_api::{backend::Backend, StorageProvider};
use sc_consensus_babe::BabeWorkerHandle;
use sc_consensus_grandpa::{
	FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState,
};
use sc_network::service::traits::NetworkService;
use sc_network_sync::SyncingService;
use sc_rpc::SubscriptionTaskExecutor;
use sc_rpc_api::DenyUnsafe;
use sc_service::TaskManager;
use sc_transaction_pool::{ChainApi, Pool};

use bp_core::{BlockNumber, Hash, Header};
use sp_core::H256;
use sp_runtime::{generic, traits::Block as BlockT, OpaqueExtrinsic as UncheckedExtrinsic};

pub type Block = generic::Block<Header, UncheckedExtrinsic>;

pub struct DefaultEthConfig<C, BE>(std::marker::PhantomData<(C, BE)>);

impl<C, BE> fc_rpc::EthConfig<Block, C> for DefaultEthConfig<C, BE>
where
	C: StorageProvider<Block, BE> + Sync + Send + 'static,
	BE: Backend<Block> + 'static,
{
	type EstimateGasAdapter = ();
	type RuntimeStorageOverride =
		fc_rpc::frontier_backend_client::SystemAccountId20StorageOverride<Block, C, BE>;
}

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps<B> {
	/// Voting round info.
	pub shared_voter_state: SharedVoterState,
	/// Authority set info.
	pub shared_authority_set: SharedAuthoritySet<Hash, BlockNumber>,
	/// Receives notifications about justification events from Grandpa.
	pub justification_stream: GrandpaJustificationStream<Block>,
	/// Executor to drive the subscription manager in the Grandpa RPC handler.
	pub subscription_executor: SubscriptionTaskExecutor,
	/// Finality proof provider.
	pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Extra dependencies for BABE.
pub struct BabeDeps {
	/// A handle to the BABE worker for issuing requests.
	pub babe_worker_handle: BabeWorkerHandle<Block>,
	/// The keystore that manages the keys of the node.
	pub keystore: sp_keystore::KeystorePtr,
}

/// Mainnet/Testnet client dependencies.
pub struct FullDeps<C, P, BE, SC, A: ChainApi, CIDP> {
	/// Client version.
	pub client_version: String,
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// The SelectChain Strategy
	pub select_chain: SC,
	/// A copy of the chain spec.
	pub chain_spec: Box<dyn sc_chain_spec::ChainSpec>,
	/// Graph pool instance.
	pub graph: Arc<Pool<A>>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<BE>,
	/// BABE specific dependencies.
	pub babe: BabeDeps,
	/// The Node authority flag
	pub is_authority: bool,
	/// Network service
	pub network: Arc<dyn NetworkService>,
	/// EthFilterApi pool.
	pub filter_pool: FilterPool,
	/// List of optional RPC extensions.
	pub ethapi_cmd: Vec<EthApiCmd>,
	/// Frontier backend.
	pub frontier_backend: Arc<dyn fc_api::Backend<Block> + Send + Sync>,
	/// Backend.
	pub backend: Arc<BE>,
	/// Maximum fee history cache size.
	pub fee_history_limit: u64,
	/// Fee history cache.
	pub fee_history_cache: FeeHistoryCache,
	/// Ethereum data access overrides.
	pub overrides: Arc<dyn StorageOverride<Block>>,
	/// Cache for Ethereum block data.
	pub block_data_cache: Arc<EthBlockDataCacheTask<Block>>,
	/// Maximum number of logs in one query.
	pub max_past_logs: u32,
	/// Timeout for eth logs query in seconds. (default 10)
	pub logs_request_timeout: u64,
	/// Mandated parent hashes for a given block hash.
	pub forced_parent_hashes: Option<BTreeMap<H256, H256>>,
	/// Chain syncing service
	pub sync_service: Arc<SyncingService<Block>>,
	/// Something that can create the inherent data providers for pending state
	pub pending_create_inherent_data_providers: CIDP,
}

pub struct SpawnTasksParams<'a, B: BlockT, C, BE> {
	pub task_manager: &'a TaskManager,
	pub client: Arc<C>,
	pub substrate_backend: Arc<BE>,
	pub frontier_backend: Arc<fc_db::Backend<B, C>>,
	pub filter_pool: Option<FilterPool>,
	pub overrides: Arc<dyn StorageOverride<B>>,
	pub fee_history_limit: u64,
	pub fee_history_cache: FeeHistoryCache,
}

pub struct TracingConfig {
	pub tracing_requesters: crate::tracing::RpcRequesters,
	pub trace_filter_max_count: u32,
}

pub mod staking {
	use codec::Encode;
	use jsonrpsee::{
		proc_macros::rpc,
		types::{ErrorObject, ErrorObjectOwned},
	};
	use pallet_staking::RewardDestination;
	use pallet_staking_runtime_api::AccountId20 as AccountId;
	use pallet_staking_runtime_api::StakingRpcApi;
	use sp_api::{ApiError, ProvideRuntimeApi};
	use sp_blockchain::HeaderBackend;
	use std::str::FromStr;
	use std::sync::Arc;

	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize)]
	pub enum RawRewardDestination {
		Staked,
		Stash,
		Controller,
		Account(String),
		None,
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	pub struct RawNominatorInfo {
		pub stash_account: String,
		pub target_validators: Vec<String>,
		pub total_staking: String,
		pub active_staking: String,
		pub rewrds_destination: RawRewardDestination,
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	pub struct RawValidatorInfo {
		pub stash_account: String,
		pub state: bool,
		pub total_staking: String,
		pub owner_staking: String,
		pub nominators: String,
		pub commission: String,
		pub can_nominated: bool,
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	pub struct RawProvider {
		pub pid: String,
		pub owner: String,
		pub cap_pledge: String,
		pub total_pledge: String,
		pub devices_num: String,
		pub total_punishment: String,
		pub total_rewards: String,
		pub unpaid_rewards: String,
	}

	#[derive(Debug, thiserror::Error)]
	/// Top-level error type for the RPC handler
	pub enum Error {
		#[error("parse account id failed")]
		InvalidAccount,
		#[error("call api error")]
		ApiCallErr(ApiError),
		#[error("no nominator storage for account")]
		NoStorage,
	}

	const STAKING_ERROR: i32 = 8100;

	impl From<Error> for ErrorObjectOwned {
		fn from(error: Error) -> Self {
			match error {
				Error::InvalidAccount => {
					ErrorObject::owned(STAKING_ERROR + 1, error.to_string(), None::<()>)
				},
				Error::ApiCallErr(_) => {
					ErrorObject::owned(STAKING_ERROR + 2, error.to_string(), None::<()>)
				},
				Error::NoStorage => {
					ErrorObject::owned(STAKING_ERROR + 3, error.to_string(), None::<()>)
				},
			}
		}
	}

	#[rpc(client, server)]
	pub trait StakingApi {
		#[method(name = "staking_nominatorInfo")]
		fn nominator_info(&self, account_id: Vec<String>) -> Result<Vec<RawNominatorInfo>, Error>;
		#[method(name = "staking_validatorInfo")]
		fn validator_info(&self, account_id: Vec<String>) -> Result<Vec<RawValidatorInfo>, Error>;
		#[method(name = "staking_getValidatorRewards")]
		fn get_validator_rewards(
			&self,
			account_id: String,
			era_index: u32,
		) -> Result<String, Error>;
		#[method(name = "staking_getNominatorRewards")]
		fn get_nominator_rewards(
			&self,
			account_id: String,
			era_index: u32,
		) -> Result<String, Error>;
		#[method(name = "staking_getAllValidatorsCanNominate")]
		fn all_validators_can_nominate(&self) -> Result<Vec<String>, Error>;
	}

	pub struct StakingClient<C, B> {
		client: Arc<C>,
		_marker: std::marker::PhantomData<B>,
	}

	impl<C, B> StakingClient<C, B> {
		pub fn new(client: Arc<C>) -> Self {
			StakingClient { client, _marker: Default::default() }
		}
	}

	impl<C, B> StakingApiServer for StakingClient<C, B>
	where
		C: ProvideRuntimeApi<B>,
		C: HeaderBackend<B> + 'static,
		C::Api: StakingRpcApi<B>,
		B: sp_runtime::traits::Block,
	{
		fn nominator_info(&self, accounts: Vec<String>) -> Result<Vec<RawNominatorInfo>, Error> {
			let api = self.client.runtime_api();
			let best = self.client.info().best_hash;
			let mut infos = Vec::new();
			for account in accounts {
				match AccountId::from_str(&account) {
					Ok(account_id) => match api.nominator_info(best, &account_id) {
						Ok(Some(nominator_info)) => {
							let raw_destination = match nominator_info.rewrds_destination {
								RewardDestination::Stash => RawRewardDestination::Stash,
								RewardDestination::Staked => RawRewardDestination::Staked,
								RewardDestination::Controller => RawRewardDestination::Controller,
								RewardDestination::Account(acc) => {
									RawRewardDestination::Account(acc.to_string())
								},
								_ => RawRewardDestination::None,
							};
							let raw = RawNominatorInfo {
								stash_account: "0x".to_string()
									+ &hex::encode(&nominator_info.stash_account.encode()),
								target_validators: nominator_info
									.target_validators
									.into_iter()
									.map(|acc| "0x".to_string() + &hex::encode(&acc.encode()))
									.collect(),
								total_staking: nominator_info.total_staking.to_string(),
								active_staking: nominator_info.active_staking.to_string(),
								rewrds_destination: raw_destination,
							};
							infos.push(raw);
						},
						Ok(None) => return Err(Error::NoStorage),
						Err(e) => return Err(Error::ApiCallErr(e)),
					},
					Err(_) => return Err(Error::InvalidAccount),
				}
			}
			Ok(infos)
		}

		fn validator_info(&self, accounts: Vec<String>) -> Result<Vec<RawValidatorInfo>, Error> {
			let api = self.client.runtime_api();
			let best = self.client.info().best_hash;
			let mut infos = Vec::new();
			for account in accounts {
				match AccountId::from_str(&account) {
					Ok(account_id) => match api.validator_info(best, &account_id) {
						Ok(Some(validator_info)) => {
							let raw = RawValidatorInfo {
								stash_account: "0x".to_string()
									+ &hex::encode(&validator_info.stash_account.encode()),
								state: validator_info.is_active,
								total_staking: validator_info.total_staking.to_string(),
								owner_staking: validator_info.owner_staking.to_string(),
								nominators: validator_info.nominators.to_string(),
								commission: (validator_info.commission.deconstruct() / 10000000)
									.to_string(),
								can_nominated: validator_info.can_nominated,
							};
							infos.push(raw);
						},
						Ok(None) => return Err(Error::NoStorage),
						Err(e) => return Err(Error::ApiCallErr(e)),
					},
					Err(_) => return Err(Error::InvalidAccount),
				}
			}
			Ok(infos)
		}

		fn get_validator_rewards(&self, account: String, era_index: u32) -> Result<String, Error> {
			let api = self.client.runtime_api();
			let best = self.client.info().best_hash;
			let account_id = match AccountId::from_str(&account) {
				Ok(acc) => acc,
				Err(_) => return Err(Error::InvalidAccount),
			};

			match api.get_validator_rewards(best, &account_id, era_index) {
				Ok(Some(rewards)) => Ok(rewards.to_string()),
				Ok(None) => return Err(Error::NoStorage),
				Err(e) => return Err(Error::ApiCallErr(e)),
			}
		}

		fn get_nominator_rewards(&self, account: String, era_index: u32) -> Result<String, Error> {
			let api = self.client.runtime_api();
			let best = self.client.info().best_hash;
			let account_id = match AccountId::from_str(&account) {
				Ok(acc) => acc,
				Err(_) => return Err(Error::InvalidAccount),
			};

			match api.get_nominator_rewards(best, &account_id, era_index) {
				Ok(Some(rewards)) => Ok(rewards.to_string()),
				Ok(None) => return Err(Error::NoStorage),
				Err(e) => return Err(Error::ApiCallErr(e)),
			}
		}

		fn all_validators_can_nominate(&self) -> Result<Vec<String>, Error> {
			let api = self.client.runtime_api();
			let best = self.client.info().best_hash;
			match api.all_validators_can_nominate(best) {
				Ok(accounts) => Ok(accounts
					.iter()
					.map(|acc| "0x".to_string() + &hex::encode(&acc.encode()))
					.collect()),
				Err(e) => return Err(Error::ApiCallErr(e)),
			}
		}
	}
}
