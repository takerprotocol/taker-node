#![cfg_attr(not(feature = "std"), no_std)]

use bp_core::AccountId;
pub use taker_common_constants::{currency, time};

pub mod fee {
	use frame_support::weights::constants::WEIGHT_REF_TIME_PER_SECOND;

	/// Current approximation of the gas/s consumption considering
	/// EVM execution over compiled WASM (on 4.4Ghz CPU).
	/// Given the 500ms Weight, from which 75% only are used for transactions,
	/// the total EVM execution gas limit is: GAS_PER_SECOND * 0.500 * 0.75 ~= 50_000_000.
	pub const GAS_PER_SECOND: u64 = 133_333_333;

	/// Approximate ratio of the amount of Weight per Gas.
	/// u64 works for approximations because Weight is a very small unit compared to gas.
	pub const WEIGHT_PER_GAS: u64 = WEIGHT_REF_TIME_PER_SECOND / GAS_PER_SECOND;
}

pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
	sp_consensus_babe::BabeEpochConfiguration {
		c: (1, 4),
		allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
	};

/// admin for whitelist, 0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac
pub const DEFAULT_ADMIN: AccountId = AccountId {
	0: [
		242u8, 79, 243, 169, 207, 4, 199, 29, 188, 148, 208, 181, 102, 247, 162, 123, 148, 86, 108,
		172,
	],
};

/// gas fee collector, 0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac
pub const FEE_COLLECTOR: AccountId = AccountId {
	0: [
		242u8, 79, 243, 169, 207, 4, 199, 29, 188, 148, 208, 181, 102, 247, 162, 123, 148, 86, 108,
		172,
	],
};
