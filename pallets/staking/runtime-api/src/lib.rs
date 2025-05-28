// This file is part of Substrate.

// Copyright (C) 2023 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// #![feature(trivial_bounds)]
//! Runtime API definition for the staking pallet.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use codec::{Decode, Encode};
pub use fp_account::AccountId20;
use pallet_staking::RewardDestination;
use scale_info::TypeInfo;
use sp_runtime::{Perbill, RuntimeDebug};
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
	pub trait StakingApi<Balance>
		where
			Balance: Codec,
	{
		/// Returns the nominations quota for a nominator with a given balance.
		fn nominations_quota(balance: Balance) -> u32;
	}
}

#[derive(Encode, Decode, PartialEq, Clone, RuntimeDebug, TypeInfo)]
pub struct NominatorInfo {
	pub stash_account: AccountId20,
	pub target_validators: Vec<AccountId20>,
	pub total_staking: u128,
	pub active_staking: u128,
	pub rewrds_destination: RewardDestination<AccountId20>,
}
#[derive(Encode, Decode, PartialEq, Clone, RuntimeDebug, TypeInfo)]
pub struct ValidatorInfo {
	pub stash_account: AccountId20,
	pub is_active: bool,
	pub total_staking: u128,
	pub owner_staking: u128,
	pub nominators: u8,
	pub commission: Perbill,
	pub can_nominated: bool,
}
sp_api::decl_runtime_apis! {
	pub trait StakingRpcApi {
		 fn nominator_info(account: &AccountId20) -> Option<NominatorInfo>;
		 fn validator_info(account: &AccountId20) -> Option<ValidatorInfo>;
		fn get_validator_rewards(account: &AccountId20, era_index: u32) -> Option<u128>;
		fn get_nominator_rewards(account: &AccountId20, era_index: u32) -> Option<u128>;
		fn all_validators_can_nominate() -> Vec<AccountId20>;
	}
}
