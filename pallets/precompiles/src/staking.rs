use fp_evm::ExitError;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
use pallet_evm::AddressMapping;
use pallet_staking::RewardDestination;
use precompile_utils::prelude::*;
use sp_core::{Decode, H160, U256};
use sp_runtime::traits::Dispatchable;
use sp_runtime::traits::StaticLookup;
use sp_runtime::{PerThing, Perbill, SaturatedConversion};
use sp_std::marker::PhantomData;
use sp_std::{vec, vec::Vec};

type BalanceOf<Runtime> = <Runtime as pallet_staking::Config>::CurrencyBalance;

pub struct StakingPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> StakingPrecompile<Runtime>
where
	Runtime: pallet_staking::Config
		+ pallet_utility::Config
		+ pallet_session::Config
		+ pallet_evm::Config
		+ pallet_balances::Config
		+ frame_system::Config,
	<Runtime as pallet_utility::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<<Runtime as pallet_utility::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
	<Runtime as frame_system::Config>::RuntimeCall:
		Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
	<<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source:
		From<<Runtime as frame_system::Config>::AccountId>,
	<Runtime as pallet_utility::Config>::RuntimeCall: From<pallet_staking::Call<Runtime>>,
	<Runtime as pallet_utility::Config>::RuntimeCall: From<pallet_session::Call<Runtime>>,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_staking::Call<Runtime>>,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_session::Call<Runtime>>,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_utility::Call<Runtime>>,
	BalanceOf<Runtime>: TryFrom<U256> + Into<U256>,
	Runtime::AccountId: Into<H160>,
	Runtime::AccountId: Into<<<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source>,
{
	// Storage getters

	#[precompile::public("validatorCount()")]
	#[precompile::public("validator_count()")]
	#[precompile::view]
	fn validator_count(handle: &mut impl PrecompileHandle) -> EvmResult<U256> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let count = pallet_staking::Pallet::<Runtime>::validator_count();
		Ok(count.into())
	}

	#[precompile::public("stashAccount(address)")]
	#[precompile::public("stash_account(address)")]
	#[precompile::view]
	fn stash_account(handle: &mut impl PrecompileHandle, who: Address) -> EvmResult<Address> {
		let account_id = Runtime::AddressMapping::into_account_id(who.0);
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let stash_account: H160 = pallet_staking::Pallet::<Runtime>::bonded(&account_id)
			.map(|v| v.into())
			.unwrap_or_default();
		Ok(Address(stash_account))
	}

	#[precompile::public("stakingLedger(address)")]
	#[precompile::public("staking_ledger(address)")]
	#[precompile::view]
	fn staking_ledger(handle: &mut impl PrecompileHandle, who: Address) -> EvmResult<(U256, U256)> {
		let account_id = Runtime::AddressMapping::into_account_id(who.0);
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let ledger = pallet_staking::Pallet::<Runtime>::ledger(&account_id)
			.map(|v| (v.total.saturated_into::<u128>(), v.active.saturated_into::<u128>()))
			.unwrap_or_default();
		Ok((ledger.0.into(), ledger.1.into()))
	}

	#[precompile::public("payee(address)")]
	#[precompile::view]
	fn payee(handle: &mut impl PrecompileHandle, who: Address) -> EvmResult<Address> {
		let account_id = Runtime::AddressMapping::into_account_id(who.0);
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let payee = pallet_staking::Pallet::<Runtime>::payee(&account_id);
		let res = match payee {
			RewardDestination::Staked => H160::from_low_u64_be(1),
			RewardDestination::Stash => H160::from_low_u64_be(2),
			RewardDestination::Controller => H160::from_low_u64_be(3),
			RewardDestination::Account(acc) => acc.into(),
			RewardDestination::None => H160::from_low_u64_be(0),
		};
		Ok(Address(res))
	}

	#[precompile::public("activeEra()")]
	#[precompile::public("active_era()")]
	#[precompile::view]
	fn active_era(handle: &mut impl PrecompileHandle) -> EvmResult<U256> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let active_era = pallet_staking::Pallet::<Runtime>::active_era()
			.map(|v| v.index)
			.unwrap_or_default();
		Ok(active_era.into())
	}

	#[precompile::public("erasStakers(uint256,address)")]
	#[precompile::public("eras_stakers(uint256,address)")]
	#[precompile::view]
	fn eras_stakers(
		handle: &mut impl PrecompileHandle,
		index: u32,
		who: Address,
	) -> EvmResult<(Vec<Address>, Vec<U256>)> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let account_id = Runtime::AddressMapping::into_account_id(who.0);

		let exposure = pallet_staking::Pallet::<Runtime>::eras_stakers(index, account_id);

		let others: Vec<(Address, u128)> = exposure
			.others
			.iter()
			.map(|individual| {
				(Address(individual.who.clone().into()), individual.value.saturated_into::<u128>())
			})
			.collect();
		let (nominators, amounts): (Vec<Address>, Vec<u128>) = others.into_iter().unzip();
		let amounts = amounts.into_iter().map(|v| v.into()).collect();
		Ok((nominators, amounts))
	}

	#[precompile::public("erasValidatorPrefs(uint256,address)")]
	#[precompile::public("eras_validator_prefs(uint256,address)")]
	#[precompile::view]
	fn eras_validator_prefs(
		handle: &mut impl PrecompileHandle,
		index: u32,
		who: Address,
	) -> EvmResult<(U256, bool)> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let account_id = Runtime::AddressMapping::into_account_id(who.0);

		let pref = pallet_staking::Pallet::<Runtime>::eras_validator_prefs(index, &account_id);

		Ok((pref.commission.deconstruct().into(), !pref.blocked))
	}

	#[precompile::public("nominators(address)")]
	#[precompile::view]
	fn nominators(handle: &mut impl PrecompileHandle, who: Address) -> EvmResult<Vec<Address>> {
		let account_id = Runtime::AddressMapping::into_account_id(who.0);
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let nominations: Vec<Address> = pallet_staking::Pallet::<Runtime>::nominators(&account_id)
			.map(|v| v.targets.iter().map(|acc| Address(acc.clone().into())).collect())
			.unwrap_or_default();
		Ok(nominations)
	}

	#[precompile::public("eraValidatorReward(uint256,address)")]
	#[precompile::public("era_validator_reward(uint256,address)")]
	#[precompile::view]
	fn era_validator_reward(
		handle: &mut impl PrecompileHandle,
		era_index: u32,
		who: Address,
	) -> EvmResult<U256> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let account = Runtime::AddressMapping::into_account_id(who.0);
		let reward = {
			let era_payout = pallet_staking::Pallet::<Runtime>::eras_validator_reward(era_index)
				.unwrap_or_default();
			let era_reward_points =
				pallet_staking::Pallet::<Runtime>::eras_reward_points(era_index);
			if era_reward_points.total == 0 {
				return Ok(0.into());
			}
			let exposure =
				pallet_staking::Pallet::<Runtime>::eras_stakers_clipped(era_index, account.clone());
			if exposure.total.saturated_into::<u128>() == 0 {
				return Ok(0.into());
			}
			let validator_point = era_reward_points
				.individual
				.get(&account)
				.map(|points| *points)
				.unwrap_or_else(|| 0);
			// validator's rewards(contain commission and stakers)
			let validator_part: Perbill =
				PerThing::from_rational(validator_point, era_reward_points.total);
			let validator_total_payout = validator_part * era_payout;

			let part_info = pallet_staking::Pallet::<Runtime>::validators(&account);
			// validator's commission reward
			let validator_commission_payout = part_info.commission * validator_total_payout;
			let validator_leftover_payout = validator_total_payout - validator_commission_payout;

			// distribute rewards by stake
			// validators staking rewards
			let validator_staking_part: Perbill = PerThing::from_rational(
				exposure.own.saturated_into::<u128>(),
				exposure.total.saturated_into::<u128>(),
			);
			// let validator_staking_payout = (U256::from(validator_leftover_payout) * U256::from(exposure.own)).as_u128() / exposure.total;
			let validator_staking_payout = validator_staking_part * validator_leftover_payout;
			(validator_staking_payout + validator_commission_payout).saturated_into::<u128>()
		};
		Ok(reward.into())
	}

	#[precompile::public("eraNominatorReward(uint256,address)")]
	#[precompile::public("era_nominator_reward(uint256,address)")]
	#[precompile::view]
	fn era_nominator_reward(
		handle: &mut impl PrecompileHandle,
		era_index: u32,
		who: Address,
	) -> EvmResult<U256> {
		let account = Runtime::AddressMapping::into_account_id(who.0);
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let reward = {
			let nominations = match pallet_staking::Pallet::<Runtime>::nominators(&account) {
				Some(nominations) => nominations,
				None => return Ok(0.into()),
			};
			let era_payout =
				match pallet_staking::Pallet::<Runtime>::eras_validator_reward(era_index) {
					Some(era_payout) => era_payout,
					None => return Ok(0.into()),
				};
			let era_reward_points =
				pallet_staking::Pallet::<Runtime>::eras_reward_points(era_index);
			if era_reward_points.total == 0 {
				return Ok(0.into());
			}
			let validators = nominations.targets;
			let mut nominator_rewards = 0;
			// check all validators about the nominator
			for validator in validators {
				let exposure = pallet_staking::Pallet::<Runtime>::eras_stakers_clipped(
					era_index,
					validator.clone(),
				);
				if exposure.total.saturated_into::<u128>() == 0 {
					continue;
				}
				let validator_point = era_reward_points
					.individual
					.get(&validator)
					.map(|points| *points)
					.unwrap_or_else(|| 0);
				let validator_part: Perbill =
					PerThing::from_rational(validator_point, era_reward_points.total);
				let validator_total_payout = validator_part * era_payout;

				let part_info = pallet_staking::Pallet::<Runtime>::validators(validator);
				// validator's commission reward
				let validator_commission_payout = part_info.commission * validator_total_payout;
				let validator_leftover_payout =
					validator_total_payout - validator_commission_payout;

				for individual in exposure.others.into_iter() {
					if individual.who == account {
						let nominator_staking_part: Perbill = PerThing::from_rational(
							individual.value.saturated_into::<u128>(),
							exposure.total.saturated_into::<u128>(),
						);
						nominator_rewards += nominator_staking_part
							* validator_leftover_payout.saturated_into::<u128>();
						break;
					}
				}
			}
			nominator_rewards.saturated_into::<u128>()
		};
		Ok(reward.into())
	}

	#[precompile::public("erasTotalStake(uint256)")]
	#[precompile::public("eras_total_stake(uint256)")]
	#[precompile::view]
	fn eras_total_stake(handle: &mut impl PrecompileHandle, era_index: u32) -> EvmResult<U256> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let total = pallet_staking::Pallet::<Runtime>::eras_total_stake(era_index);

		Ok(total.into())
	}

	// Dispatchable methods

	#[precompile::public("bondAndNominate(uint256,uint256,address[])")]
	#[precompile::public("bond_and_nominate(uint256,uint256,address[])")]
	fn bond_and_nominate(
		handle: &mut impl PrecompileHandle,
		bond_value: u128,
		payee: u8,
		accounts: Vec<Address>,
	) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let value: BalanceOf<Runtime> = SaturatedConversion::saturated_from(bond_value);

		let payee = match payee {
			0 => RewardDestination::Staked,
			1 => RewardDestination::Stash,
			_ => RewardDestination::default(),
		};
		let controller: pallet_staking::AccountIdLookupOf<Runtime> = origin.clone().into();
		let bond_call = pallet_staking::Call::<Runtime>::bond { controller, value, payee }.into();
		let targets = accounts
			.iter()
			.map(|validator| Runtime::AddressMapping::into_account_id(validator.0).into())
			.collect();
		let nominate_call = pallet_staking::Call::<Runtime>::nominate { targets }.into();
		let calls = vec![bond_call, nominate_call];
		let batch_call = pallet_utility::Call::<Runtime>::batch_all { calls };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), batch_call)?;
		Ok(())
	}

	#[precompile::public("bondAndValidate(uint256,uint256,uint256,bool,bytes,bytes)")]
	#[precompile::public("bond_and_validate(uint256,uint256,uint256,bool,bytes,bytes)")]
	fn bond_and_validate(
		handle: &mut impl PrecompileHandle,
		bond_value: u128,
		payee: u8,
		commission: u32,
		can_nominated: bool,
		raw_keys: UnboundedBytes,
		proof: UnboundedBytes,
	) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let value: BalanceOf<Runtime> = SaturatedConversion::saturated_from(bond_value);
		let payee = match payee {
			0 => RewardDestination::Staked,
			1 => RewardDestination::Stash,
			_ => RewardDestination::default(),
		};
		let commission = sp_runtime::Perbill::from_percent(commission);
		let keys = Runtime::Keys::decode(&mut raw_keys.as_bytes())
			.map_err(|_| ExitError::Other("Incorrect session keys".into()))?;
		let prefs = pallet_staking::ValidatorPrefs { commission, blocked: !can_nominated };
		let controller: pallet_staking::AccountIdLookupOf<Runtime> = origin.clone().into();
		let bond_call = pallet_staking::Call::<Runtime>::bond { controller, value, payee }.into();
		let set_session_key_call =
			pallet_session::Call::<Runtime>::set_keys { keys, proof: proof.as_bytes().to_vec() }
				.into();
		let validate_call = pallet_staking::Call::<Runtime>::validate { prefs }.into();

		let calls = vec![bond_call, set_session_key_call, validate_call];
		let batch_call = pallet_utility::Call::<Runtime>::batch_all { calls };

		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), batch_call)?;
		Ok(())
	}

	#[precompile::public("chill()")]
	fn chill(handle: &mut impl PrecompileHandle) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let call = pallet_staking::Call::<Runtime>::chill {};
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("bondExtra(uint256)")]
	#[precompile::public("bond_extra(uint256)")]
	fn bond_extra(handle: &mut impl PrecompileHandle, value: u128) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let max_additional: BalanceOf<Runtime> = SaturatedConversion::saturated_from(value);
		let call = pallet_staking::Call::<Runtime>::bond_extra { max_additional };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("unbond(uint256)")]
	fn unbond(handle: &mut impl PrecompileHandle, bond_value: u128) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let value: BalanceOf<Runtime> = SaturatedConversion::saturated_from(bond_value);
		let call = pallet_staking::Call::<Runtime>::unbond { value };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("payoutStakers(address,uint256[])")]
	#[precompile::public("payout_stakers(address,uint256[])")]
	fn payout_stakers(
		handle: &mut impl PrecompileHandle,
		who: Address,
		era_indexs: Vec<u32>,
	) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let validator_stash = Runtime::AddressMapping::into_account_id(who.0);
		let mut calls = Vec::new();
		for era in era_indexs {
			let call = pallet_staking::Call::<Runtime>::payout_stakers {
				validator_stash: validator_stash.clone(),
				era,
			};
			calls.push(call.into());
		}
		let call = pallet_utility::Call::<Runtime>::batch_all { calls };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("setPayee(uint256)")]
	#[precompile::public("set_payee(uint256)")]
	fn set_payee(handle: &mut impl PrecompileHandle, payee: u8) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let payee = match payee {
			0 => RewardDestination::Staked,
			1 => RewardDestination::Stash,
			_ => RewardDestination::default(),
		};
		let call = pallet_staking::Call::<Runtime>::set_payee { payee };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("withdrawUnbonded(uint256)")]
	#[precompile::public("withdraw_unbonded(uint256)")]
	fn withdraw_unbonded(handle: &mut impl PrecompileHandle, num_slashing_spans: u32) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		if pallet_staking::Pallet::<Runtime>::ledger(&origin).is_none() {
			return Ok(());
		}
		let call = pallet_staking::Call::<Runtime>::withdraw_unbonded { num_slashing_spans };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("nominate(address[])")]
	fn nominate(handle: &mut impl PrecompileHandle, accounts: Vec<Address>) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let targets = accounts
			.iter()
			.map(|validator| Runtime::AddressMapping::into_account_id(validator.0).into())
			.collect();
		let call = pallet_staking::Call::<Runtime>::nominate { targets };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("validate(uint256,bool)")]
	fn validate(
		handle: &mut impl PrecompileHandle,
		commission: u32,
		can_nominated: bool,
	) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let commission = sp_runtime::Perbill::from_percent(commission);
		let prefs = pallet_staking::ValidatorPrefs { commission, blocked: !can_nominated };
		let call = pallet_staking::Call::<Runtime>::validate { prefs };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("setSessionKey(bytes,bytes)")]
	#[precompile::public("set_session_key(bytes,bytes)")]
	fn set_session_key(
		handle: &mut impl PrecompileHandle,
		raw_keys: UnboundedBytes,
		proof: UnboundedBytes,
	) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let keys = Runtime::Keys::decode(&mut raw_keys.as_bytes())
			.map_err(|_| ExitError::Other("Incorrect session keys".into()))?;
		let call =
			pallet_session::Call::<Runtime>::set_keys { keys, proof: proof.as_bytes().to_vec() };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("setSessionKeysAndValidate(uint256,bool,bytes,bytes)")]
	#[precompile::public("set_session_keys_and_validate(uint256,bool,bytes,bytes)")]
	fn set_session_keys_and_validate(
		handle: &mut impl PrecompileHandle,
		commission: u32,
		can_nominated: bool,
		raw_keys: UnboundedBytes,
		proof: UnboundedBytes,
	) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let commission = sp_runtime::Perbill::from_percent(commission);
		let prefs = pallet_staking::ValidatorPrefs { commission, blocked: !can_nominated };
		let validate_call = pallet_staking::Call::<Runtime>::validate { prefs }.into();

		let keys = Runtime::Keys::decode(&mut raw_keys.as_bytes())
			.map_err(|_| ExitError::Other("Incorrect session keys".into()))?;
		let set_session_keys_call =
			pallet_session::Call::<Runtime>::set_keys { keys, proof: proof.as_bytes().to_vec() }
				.into();
		let calls = vec![validate_call, set_session_keys_call];
		let call = pallet_utility::Call::<Runtime>::batch_all { calls };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("chillAndUnbonded(uint256)")]
	#[precompile::public("chill_and_unbonded(uint256)")]
	fn chill_and_unbonded(handle: &mut impl PrecompileHandle, unbond_value: u128) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let chill_call = pallet_staking::Call::<Runtime>::chill {}.into();
		let value: BalanceOf<Runtime> = SaturatedConversion::saturated_from(unbond_value);
		let unbond_call = pallet_staking::Call::<Runtime>::unbond { value }.into();
		let calls = vec![chill_call, unbond_call];
		let call = pallet_utility::Call::<Runtime>::batch_all { calls };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("bondExtraAndNominate(uint256,address[])")]
	#[precompile::public("bond_extra_and_nominate(uint256,address[])")]
	fn bond_extra_and_nominate(
		handle: &mut impl PrecompileHandle,
		bond_value: u128,
		accounts: Vec<Address>,
	) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let max_additional: BalanceOf<Runtime> = SaturatedConversion::saturated_from(bond_value);
		let bond_extra_call = pallet_staking::Call::<Runtime>::bond_extra { max_additional }.into();
		let targets = accounts
			.iter()
			.map(|validator| Runtime::AddressMapping::into_account_id(validator.0).into())
			.collect();
		let nominate_call = pallet_staking::Call::<Runtime>::nominate { targets }.into();
		let calls = vec![bond_extra_call, nominate_call];
		let call = pallet_utility::Call::<Runtime>::batch_all { calls };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("bondExtraAndValidate(uint256,uint256,bool,bytes,bytes)")]
	#[precompile::public("bond_extra_and_validate(uint256,uint256,bool,bytes,bytes)")]
	fn bond_extra_and_validate(
		handle: &mut impl PrecompileHandle,
		bond_value: u128,
		commission: u32,
		can_nominated: bool,
		raw_keys: UnboundedBytes,
		proof: UnboundedBytes,
	) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let max_additional: BalanceOf<Runtime> = SaturatedConversion::saturated_from(bond_value);
		let commission = sp_runtime::Perbill::from_percent(commission);
		let keys = Runtime::Keys::decode(&mut raw_keys.as_bytes())
			.map_err(|_| ExitError::Other("Incorrect session keys".into()))?;
		let prefs = pallet_staking::ValidatorPrefs { commission, blocked: !can_nominated };
		let bond_extra_call = pallet_staking::Call::<Runtime>::bond_extra { max_additional }.into();
		let set_session_key_call =
			pallet_session::Call::<Runtime>::set_keys { keys, proof: proof.as_bytes().to_vec() }
				.into();
		let validate_call = pallet_staking::Call::<Runtime>::validate { prefs }.into();
		let calls = vec![bond_extra_call, set_session_key_call, validate_call];

		let call = pallet_utility::Call::<Runtime>::batch_all { calls };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}
}
