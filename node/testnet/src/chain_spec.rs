use fp_evm::GenesisAccount;
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{ed25519, sr25519, ByteArray, Pair, Public};
use taker_testnet_runtime::{
	opaque::SessionKeys, AccountId, Balance, ImOnlineId, Precompiles, SS58Prefix, StakerStatus,
	WASM_BINARY,
};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const INITIAL_STAKING: u128 = 2_000 * 10u128.pow(18);
const ENDOWED_AMOUNT: u128 = 40_000 * 10u128.pow(18);

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec;

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
	SessionKeys { babe, grandpa, im_online }
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
		sr25519::Public::from_slice(&hex::decode(babe_and_gran[0]).expect("babe pk decode failed"))
			.unwrap()
			.into(),
		ed25519::Public::from_slice(&hex::decode(babe_and_gran[1]).expect("gran pk decode failed"))
			.unwrap()
			.into(),
		sr25519::Public::from_slice(
			&hex::decode(babe_and_gran[2]).expect("im_online pk decode failed"),
		)
		.unwrap()
		.into(),
	)
}

pub fn testnet_config() -> ChainSpec {
	ChainSpec::builder(WASM_BINARY.expect("WASM not available"), Default::default())
		.with_name("taker Testnet")
		.with_id("Testnet")
		.with_protocol_id("taker Testnet")
		.with_chain_type(ChainType::Local)
		.with_properties(properties())
		.with_genesis_config_patch(testnet_genesis(
			// Sudo account
			AccountId::from(hex!("652555f193d647382AEeEE83162E1ADf3a08F967")),
			// Pre-funded accounts
			vec![
				(AccountId::from(hex!("652555f193d647382AEeEE83162E1ADf3a08F967")), ENDOWED_AMOUNT), //sudo account,
				(AccountId::from(hex!("Fe2Ff99839aa60dd60362B9ff44C7e8fc609d62C")), ENDOWED_AMOUNT),
				(AccountId::from(hex!("30caa273746ab5571c47B3EfAb9EC8d603038439")), ENDOWED_AMOUNT),
				(AccountId::from(hex!("de1D67D0B511993D0Fc38Ec5829EF7d69CDe379D")), ENDOWED_AMOUNT),
			],
			// Initial NPOS authorities
			vec![
				authority_id_from_pk(
					AccountId::from(hex!("Fe2Ff99839aa60dd60362B9ff44C7e8fc609d62C")),
					AccountId::from(hex!("Fe2Ff99839aa60dd60362B9ff44C7e8fc609d62C")),
					"0x0447d34c079f9a8f3d62ae6592e7c5c4e334d25cd18673b81c67cb910314cf65",
					"0xDFCE90621427FF95C38F10BCB2BC4020A0C629BF121F1C748265040640692289",
					"0x0447d34c079f9a8f3d62ae6592e7c5c4e334d25cd18673b81c67cb910314cf65",
				),
				authority_id_from_pk(
					AccountId::from(hex!("30caa273746ab5571c47B3EfAb9EC8d603038439")),
					AccountId::from(hex!("30caa273746ab5571c47B3EfAb9EC8d603038439")),
					"0x3e9562351b3ed2e1136b3d5e29263e6e4d03adb2a3611987750680fe283aac14",
					"0xC632AAB355782C01C275FFC7A863ED78C709895B4BBD23B53EE31B76C3F659A8",
					"0x3e9562351b3ed2e1136b3d5e29263e6e4d03adb2a3611987750680fe283aac14",
				),
				authority_id_from_pk(
					AccountId::from(hex!("de1D67D0B511993D0Fc38Ec5829EF7d69CDe379D")),
					AccountId::from(hex!("de1D67D0B511993D0Fc38Ec5829EF7d69CDe379D")),
					"0x142b1fe25707d2b580f4e7491599ebe3a301ea9f875f7d04fa9649373982fa63",
					"0xEDD2ACF928369505B7D52AE2D6F65F22B4B4040F2B993B5F24B9099055A80820",
					"0x142b1fe25707d2b580f4e7491599ebe3a301ea9f875f7d04fa9649373982fa63",
				),
			],
			2748,
		))
		.build()
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	sudo_key: AccountId,
	endowed_accounts: Vec<(AccountId, Balance)>,
	initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId)>,
	chain_id: u64,
) -> serde_json::Value {
	use taker_testnet_runtime::{
		AssetCurrencyConfig, BabeConfig, BalancesConfig, EVMChainIdConfig, EVMConfig,
		GrandpaConfig, ImOnlineConfig, Perbill, RuntimeGenesisConfig, SessionConfig, StakingConfig,
		SudoConfig,
	};
	// This is the simplest bytecode to revert without returning any data.
	// We will pre-deploy it under all of our precompiles to ensure they can be called from
	// within contracts.
	// (PUSH1 0x00 PUSH1 0x00 REVERT)
	let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];

	let config = RuntimeGenesisConfig {
		// System
		system: Default::default(),
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(sudo_key),
		},

		// Monetary
		balances: BalancesConfig { balances: endowed_accounts.clone() },
		asset_currency: AssetCurrencyConfig {
			symbol: "veTAKER".as_bytes().to_vec(),
			decimals: 18,
			balances: endowed_accounts,
		},
		transaction_payment: Default::default(),

		// Consensus
		babe: BabeConfig {
			authorities: vec![],
			epoch_config: taker_testnet_runtime::BABE_GENESIS_EPOCH_CONFIG,
			..Default::default()
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| (x.0, x.0, session_keys(x.2.clone(), x.3.clone(), x.4.clone())))
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

		grandpa: GrandpaConfig { authorities: vec![], ..Default::default() },

		// EVM compatibility
		evm_chain_id: EVMChainIdConfig { chain_id, ..Default::default() },
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
	};

	serde_json::to_value(&config).expect("Could not build genesis config.")
}
