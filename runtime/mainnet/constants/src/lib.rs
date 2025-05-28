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

/// admin for whitelist, 0xe17AcfCcDdEBaCAa5Fbc267FB3FEAE753DE5Aa39
pub const DEFAULT_ADMIN: AccountId = AccountId {
	0: [
		225u8, 122, 207, 204, 221, 235, 172, 170, 95, 188, 38, 127, 179, 254, 174, 117, 61, 229,
		170, 57,
	],
};

/// gas fee collector, 0xe17AcfCcDdEBaCAa5Fbc267FB3FEAE753DE5Aa39
pub const FEE_COLLECTOR: AccountId = AccountId {
	0: [
		225u8, 122, 207, 204, 221, 235, 172, 170, 95, 188, 38, 127, 179, 254, 174, 117, 61, 229,
		170, 57,
	],
};
