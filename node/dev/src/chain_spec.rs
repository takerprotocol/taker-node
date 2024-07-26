use fp_evm::GenesisAccount;
use taker_dev_runtime::{AccountId, Balance, WASM_BINARY, GenesisConfig, SS58Prefix, StakerStatus, ImOnlineId, opaque::SessionKeys, Precompiles};
use sc_service::ChainType;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{ByteArray, ed25519, Pair, Public, sr25519};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sp_runtime::Perbill;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const INITIAL_STAKING: u128 = 2_000 * 10u128.pow(18);
const ENDOWED_AMOUNT: u128 = 40_000 * 10u128.pow(18);

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (BabeId, GrandpaId) {
	(get_from_seed::<BabeId>(s), get_from_seed::<GrandpaId>(s))
}

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

fn properties() -> Properties {
	let mut properties = Properties::new();
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("tokenSymbol".into(), "TAKER".into());
	properties.insert("ss58Format".into(), SS58Prefix::get().into());
	properties
}

fn session_keys(babe: BabeId, grandpa: GrandpaId, im_online: ImOnlineId) -> SessionKeys {
	SessionKeys {
		babe,
		grandpa,
		im_online,
	}
}

pub fn authority_id_from_pk(
	accountid1: AccountId,
	accountid2: AccountId,
	babe_pk: &str,
	gran_pk: &str,
	imon_pk: &str,
) -> (AccountId, AccountId, BabeId, GrandpaId, ImOnlineId) {
	let mut babe_and_gran = Vec::new();
	for pk in vec![babe_pk, gran_pk, imon_pk] {
		if pk.starts_with("0x") {
			babe_and_gran.push(&pk[2..]);
		} else {
			babe_and_gran.push(pk);
		}
	}
	(
		accountid1,
		accountid2,
		sr25519::Public::from_slice(&hex::decode(babe_and_gran[0]).expect("babe pk decode failed")).unwrap()
			.into(),
		ed25519::Public::from_slice(&hex::decode(babe_and_gran[1]).expect("gran pk decode failed")).unwrap()
			.into(),
		sr25519::Public::from_slice(
			&hex::decode(babe_and_gran[2]).expect("im_online pk decode failed"),
		).unwrap()
			.into(),
	)
}

pub fn development_config() -> ChainSpec {
	let wasm_binary = WASM_BINARY.expect("WASM not available");
	ChainSpec::from_genesis(
		// Name
		"Taker Devnet",
		// ID
		"Devnet",
		ChainType::Local,
		move || {
			dev_genesis(
				wasm_binary,
				// Sudo account
				AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")),
				// Pre-funded accounts
				vec![
					(AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")), ENDOWED_AMOUNT), // Alith
					(AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0")), ENDOWED_AMOUNT), // Baltathar
					(AccountId::from(hex!("798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc")), ENDOWED_AMOUNT), // Charleth
					(AccountId::from(hex!("773539d4Ac0e786233D90A233654ccEE26a613D9")), ENDOWED_AMOUNT), // Dorothy
					(AccountId::from(hex!("Ff64d3F6efE2317EE2807d223a0Bdc4c0c49dfDB")), ENDOWED_AMOUNT), // Ethan
					(AccountId::from(hex!("C0F0f4ab324C46e55D02D0033343B4Be8A55532d")), ENDOWED_AMOUNT), // Faith
				],
				// Initial NPOS authorities
				vec![
					(
						AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")),
						AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")),
						get_from_seed::<BabeId>("Alice").into(),
						get_from_seed::<GrandpaId>("Alice").into(),
						get_from_seed::<ImOnlineId>("Alice").into(),
					),
				],
				2747,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		Some("taker devnet"),
		// Fork ID
		None,
		// Properties
		Some(properties()),
		// Extensions
		None,
	)
}

/// Configure initial storage state for FRAME modules.
fn dev_genesis(
	wasm_binary: &[u8],
	sudo_key: AccountId,
	endowed_accounts: Vec<(AccountId, Balance)>,
	initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId)>,
	// initial_authorities: Vec<(AuraId, GrandpaId, AccountId)>,
	chain_id: u64,
) -> GenesisConfig {
	use taker_dev_runtime::{
		BalancesConfig, EVMChainIdConfig, EVMConfig, GrandpaConfig, SudoConfig, SystemConfig,
		BabeConfig, SessionConfig, StakingConfig, ImOnlineConfig, AssetCurrencyConfig,
	};
	// This is the simplest bytecode to revert without returning any data.
	// We will pre-deploy it under all of our precompiles to ensure they can be called from
	// within contracts.
	// (PUSH1 0x00 PUSH1 0x00 REVERT)
	let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];

	GenesisConfig {
		// System
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			..Default::default()
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(sudo_key),
		},

		// Monetary
		balances: BalancesConfig {
			balances: endowed_accounts.clone(),
		},
		asset_currency: AssetCurrencyConfig {
			symbol: "veTAKER".as_bytes().to_vec(),
			decimals: 18,
			balances: endowed_accounts,
		},
		transaction_payment: Default::default(),

		// Consensus
		// aura: AuraConfig {
		// 	authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		// },
		babe: BabeConfig {
			authorities: vec![],
			epoch_config: Some(taker_dev_runtime::BABE_GENESIS_EPOCH_CONFIG),
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0,
						x.0,
						session_keys(x.2.clone(), x.3.clone(), x.4.clone()),
					)
				})
				.collect::<Vec<_>>(),
		},
		staking: StakingConfig {
			validator_count: initial_authorities.len() as u32 * 2,
			minimum_validator_count: initial_authorities.len() as u32,
			stakers: initial_authorities
				.iter()
				.map(|x| (x.0, x.1, INITIAL_STAKING, StakerStatus::Validator))
				.collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			min_nominator_bond: 2_000 * 10u128.pow(18),
			min_validator_bond: 2_000 * 10u128.pow(18),
			max_validator_count: Some(1000),
			max_nominator_count: Some(4000),
			..Default::default()
		},
		im_online: ImOnlineConfig { keys: vec![] },

		grandpa: GrandpaConfig {
			authorities: vec![],
		},

		// EVM compatibility
		evm_chain_id: EVMChainIdConfig {
			chain_id,
			..Default::default()
		},
		evm: EVMConfig {
			// We need _some_ code inserted at the precompile address so that
			// the evm will actually call the address.
			accounts: Precompiles::used_addresses()
				.into_iter()
				.map(|addr| {
					(
						addr.into(),
						GenesisAccount {
							nonce: Default::default(),
							balance: Default::default(),
							storage: Default::default(),
							code: revert_bytecode.clone(),
						},
					)
				})
				.collect(),
			..Default::default()
		},
		ethereum: Default::default(),
		dynamic_fee: Default::default(),
		base_fee: Default::default(),

	}
}
