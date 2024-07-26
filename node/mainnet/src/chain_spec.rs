use fp_evm::GenesisAccount;
use taker_mainnet_runtime::{AccountId, Balance, WASM_BINARY, GenesisConfig, SS58Prefix, StakerStatus, ImOnlineId, opaque::SessionKeys, Precompiles};
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

pub fn mainnet_config() -> ChainSpec {
	let wasm_binary = WASM_BINARY.expect("WASM not available");
	ChainSpec::from_genesis(
		// Name
		"Taker Mainnet",
		// ID
		"Mainnet",
		ChainType::Local,
		move || {
			mainnet_genesis(
				wasm_binary,
				// Sudo account
				AccountId::from(hex!("86877CA251E15Add75d140a3f8C5707D4e47D88a")),
				// Pre-funded accounts
				vec![
					(AccountId::from(hex!("86877CA251E15Add75d140a3f8C5707D4e47D88a")), ENDOWED_AMOUNT), //sudo account,
					(AccountId::from(hex!("F6EF71fB82fD1d8a90a4779ef1eA23a11Ee012e2")), ENDOWED_AMOUNT),
					(AccountId::from(hex!("329DABfc5148A49Bd5ae1129aaAd5a7294DDA206")), ENDOWED_AMOUNT),
					(AccountId::from(hex!("AE100Ed673FF338Ede70236fa3D971C57c325f89")), ENDOWED_AMOUNT),
				],
				// Initial NPOS authorities
				vec![
					authority_id_from_pk(
						AccountId::from(hex!("F6EF71fB82fD1d8a90a4779ef1eA23a11Ee012e2")),
						AccountId::from(hex!("F6EF71fB82fD1d8a90a4779ef1eA23a11Ee012e2")),
						"0xb88dc3a5819e681b0857ce9513f42b1f5c9d006160faa368011af96b100cb256",
						"0x1D1E813EF5CEE7DA27F6DEF94FD04491FA1098DB3B9A8646DA0B0417E490ADDC",
						"0xb88dc3a5819e681b0857ce9513f42b1f5c9d006160faa368011af96b100cb256",
					),
					authority_id_from_pk(
						AccountId::from(hex!("329DABfc5148A49Bd5ae1129aaAd5a7294DDA206")),
						AccountId::from(hex!("329DABfc5148A49Bd5ae1129aaAd5a7294DDA206")),
						"0x5675a3732e4b911128b7082ba5936266da0f893c8fd87b4a4f900fb84a62d37d",
						"0xC36EBA7DE02BFA88E3910C82B7133AA6B5917B0FD2B0992B045E3EF84FC2AD4B",
						"0x5675a3732e4b911128b7082ba5936266da0f893c8fd87b4a4f900fb84a62d37d",
					),
					authority_id_from_pk(
						AccountId::from(hex!("AE100Ed673FF338Ede70236fa3D971C57c325f89")),
						AccountId::from(hex!("AE100Ed673FF338Ede70236fa3D971C57c325f89")),
						"0x347e3ba27df3ccf65d3ec16c540c87b6b558212a76295eab28f377c4f6350757",
						"0xE84F5112EE68425F306001E9729283F74ABA3BA0C7704D48226683D67888A500",
						"0x347e3ba27df3ccf65d3ec16c540c87b6b558212a76295eab28f377c4f6350757",
					)
				],
				2749,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		Some("taker Mainnet"),
		// Fork ID
		None,
		// Properties
		Some(properties()),
		// Extensions
		None,
	)
}

/// Configure initial storage state for FRAME modules.
fn mainnet_genesis(
	wasm_binary: &[u8],
	sudo_key: AccountId,
	endowed_accounts: Vec<(AccountId, Balance)>,
	initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId)>,
	// initial_authorities: Vec<(AuraId, GrandpaId, AccountId)>,
	chain_id: u64,
) -> GenesisConfig {
	use taker_mainnet_runtime::{
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
			epoch_config: Some(taker_mainnet_runtime::BABE_GENESIS_EPOCH_CONFIG),
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
