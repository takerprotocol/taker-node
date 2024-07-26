#![allow(unused_assignments)]
use fp_evm::{
    PrecompileHandle, PrecompileResult, Context, PrecompileOutput, ExitSucceed,
    ExitError, Precompile
};
use frame_support::sp_runtime;
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
};
use pallet_evm::{AddressMapping, GasWeightMapping};
use precompile_utils::data::{Address, Bytes};
use precompile_utils::*;
use frame_support::sp_runtime::SaturatedConversion;
use sp_core::{H160, U256};
use sp_std::{
    convert::{TryFrom, TryInto},
    fmt::Debug,
    marker::PhantomData,
    vec,
    vec::Vec,
};
use pallet_staking::RewardDestination;
use sp_runtime::{Perbill, PerThing, traits::{Dispatchable, StaticLookup}};
use sp_core::Decode;

type BalanceOf<Runtime> = <Runtime as pallet_staking::Config>::CurrencyBalance;

#[generate_function_selector]
#[derive(Debug, PartialEq, num_enum::TryFromPrimitive)]
enum StakingAction {
    // storage
    ValidatorCount = "validatorCount()",
    StashAccount = "stashAccount(address)",
    StakingLedger = "stakingLedger(address)",
    Payee = "payee(address)",
    ActiveEra = "activeEra()",
    ErasStakers = "erasStakers(uint256,address)",
    ErasValidatorPrefs = "erasValidatorPrefs(uint256,address)",
    Nominators = "nominators(address)",
    EraValidatorReward = "eraValidatorReward(uint256,address)",
    EraNominatorReward = "eraNominatorReward(uint256,address)",
    ErasTotalStake = "erasTotalStake(uint256)",
    // ValidatorSlashInEra = "validatorSlashInEra(uint256,address)", // (perbill(deconstruct), balance)
    // NominatorSlashInEra = "nominatorSlashInEra(uint256,address)", // balance

    // tx
    BondAndNominate = "bondAndNominate(uint256,uint256,address[])",
    BondAndValidate = "bondAndValidate(uint256,uint256,uint256,bool,bytes,bytes)",
    Chill = "chill()",
    BondExtra = "bondExtra(uint256)",
    Unbond = "unbond(uint256)",
    PayoutStakers = "payoutStakers(address,uint256[])",
    SetPayee = "setPayee(uint256)",
    WithdrawUnbonded = "withdrawUnbonded(uint256)",
    Nominate = "nominate(address[])",
    Validate = "validate(uint256,bool)",
    SetSessionKeys = "setSessionKey(bytes,bytes)",
    SetSessionKeysAndValidate = "setSessionKeysAndValidate(uint256,bool,bytes,bytes)",
    ChillAndUnbonded = "chillAndUnbonded(uint256)",
    BondExtraAndNominate = "bondExtraAndNominate(uint256,address[])",
    BondExtraAndValidate = "bondExtraAndValidate(uint256,uint256,bool,bytes,bytes)",
}

pub struct StakingPrecompile<Runtime>(PhantomData<Runtime>);

impl<Runtime> Precompile for StakingPrecompile<Runtime>
    where
        Runtime: pallet_staking::Config
        + pallet_evm::Config
        + pallet_utility::Config
        + pallet_session::Config,
        BalanceOf<Runtime>: TryFrom<U256> + Debug,
        <<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source: From<<Runtime as frame_system::Config>::AccountId>,
        <Runtime as pallet_utility::Config>::RuntimeCall: From<pallet_staking::Call<Runtime>>,
        <Runtime as pallet_utility::Config>::RuntimeCall: From<pallet_session::Call<Runtime>>,
        <Runtime as frame_system::Config>::RuntimeCall: From<pallet_staking::Call<Runtime>>,
        <Runtime as frame_system::Config>::RuntimeCall: From<pallet_session::Call<Runtime>>,
        <Runtime as frame_system::Config>::RuntimeCall: From<pallet_utility::Call<Runtime>>,
        <Runtime as frame_system::Config>::RuntimeOrigin: From<Option<<Runtime as frame_system::Config>::AccountId>>,
        <Runtime as frame_system::Config>::RuntimeCall: frame_support::dispatch::GetDispatchInfo,
        <Runtime as frame_system::Config>::RuntimeCall:
        Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
        Runtime::AccountId: Into<H160>,
        Runtime::AccountId: Into<<<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source>,
{
    fn execute(
        handle: &mut impl PrecompileHandle
    ) -> PrecompileResult {
        let input = handle.input();
        let target_gas = handle.gas_limit();
        let context = handle.context();
        let (input_reader, selector) = EvmDataReader::new_with_selector(input)?;
        match selector {
            // Storage call
            StakingAction::ValidatorCount => Self::validator_count(input_reader),
            StakingAction::StashAccount => Self::stash_account(input_reader),
            StakingAction::StakingLedger => Self::staking_ledger(input_reader),
            StakingAction::Payee => Self::payee(input_reader),
            StakingAction::ActiveEra => Self::active_era(input_reader),
            StakingAction::ErasStakers => Self::eras_stakers(input_reader),
            StakingAction::ErasValidatorPrefs => Self::eras_validator_prefs(input_reader),
            StakingAction::Nominators => Self::nominators(input_reader),
            StakingAction::EraValidatorReward => Self::era_validator_reward(input_reader),
            StakingAction::EraNominatorReward => Self::era_nominator_reward(input_reader),
            StakingAction::ErasTotalStake => Self::eras_total_stake(input_reader),
            // StakingAction::ValidatorSlashInEra => Self::validator_slash_in_era(input_reader),
            // StakingAction::NominatorSlashInEra => Self::nominator_slash_in_era(input_reader),
            // Tx call
            StakingAction::BondAndNominate => Self::bond_and_nominate(input_reader, target_gas, context),
            StakingAction::BondAndValidate => Self::bond_and_validate(input_reader, target_gas, context),
            StakingAction::Chill => Self::chill(input_reader, target_gas, context),
            StakingAction::BondExtra => Self::bond_extra(input_reader, target_gas, context),
            StakingAction::Unbond => Self::unbond(input_reader, target_gas, context),
            StakingAction::PayoutStakers => Self::payout_stakers(input_reader, target_gas, context),
            StakingAction::SetPayee => Self::set_payee(input_reader, target_gas, context),
            StakingAction::WithdrawUnbonded => {
                Self::withdraw_unbonded(input_reader, target_gas, context)
            }
            StakingAction::Nominate => Self::nominate(input_reader, target_gas, context),
            StakingAction::Validate => Self::validate(input_reader, target_gas, context),
            StakingAction::SetSessionKeys => Self::set_session_keys(input_reader, target_gas, context),
            StakingAction::SetSessionKeysAndValidate => Self::set_session_keys_and_validate(input_reader, target_gas, context),
            StakingAction::ChillAndUnbonded => Self::chill_and_unbonded(input_reader, target_gas, context),
            StakingAction::BondExtraAndNominate => Self::bond_extra_and_nominate(input_reader, target_gas, context),
            StakingAction::BondExtraAndValidate => Self::bond_extra_and_validate(input_reader, target_gas, context),
        }
    }
}

impl<Runtime> StakingPrecompile<Runtime>
    where
        Runtime: pallet_staking::Config
        + pallet_evm::Config
        + pallet_utility::Config
        + pallet_session::Config,
        BalanceOf<Runtime>: TryFrom<U256> + TryInto<u128> + Debug,
        <<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source: From<<Runtime as frame_system::Config>::AccountId>,
        <Runtime as pallet_utility::Config>::RuntimeCall: From<pallet_staking::Call<Runtime>>,
        <Runtime as pallet_utility::Config>::RuntimeCall: From<pallet_session::Call<Runtime>>,
        <Runtime as frame_system::Config>::RuntimeCall: From<pallet_staking::Call<Runtime>>,
        <Runtime as frame_system::Config>::RuntimeCall: From<pallet_session::Call<Runtime>>,
        <Runtime as frame_system::Config>::RuntimeCall: From<pallet_utility::Call<Runtime>>,
        <Runtime as frame_system::Config>::RuntimeOrigin: From<Option<<Runtime as frame_system::Config>::AccountId>>,
        <Runtime as frame_system::Config>::RuntimeCall: frame_support::dispatch::GetDispatchInfo,
        <Runtime as frame_system::Config>::RuntimeCall:
        Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
        Runtime::AccountId: Into<H160>,
        Runtime::AccountId: Into<<<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source>,
{
    // storage call
    fn validator_count(
        input: EvmDataReader,
    ) -> PrecompileResult {
        input.expect_arguments(0)?;
        // let address = input.read::<Address>()?;
        // let account_id = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0);

        let count =
            pallet_staking::Pallet::<Runtime>::validator_count();
        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write(count)
                    .build(),
            }
        )
    }

    fn stash_account(
        mut input: EvmDataReader,
    ) -> PrecompileResult {
        input.expect_arguments(1)?;
        let account_id = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0);

        let stash_account: H160 =
            pallet_staking::Pallet::<Runtime>::bonded(&account_id)
                .map(|v| v.into())
                .unwrap_or_default();
        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write(Address(stash_account))
                    .build(),
            }
        )
    }

    fn staking_ledger(
        mut input: EvmDataReader,
    ) -> PrecompileResult {
        input.expect_arguments(1)?;
        let account_id = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0);
        let ledger =
            pallet_staking::Pallet::<Runtime>::ledger(&account_id)
                .map(|v| (v.total.saturated_into::<u128>(), v.active.saturated_into::<u128>()))
                .unwrap_or_default();
        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write(ledger.0)
                    .write(ledger.1)
                    .build(),
            }
        )
    }

    fn payee(
        mut input: EvmDataReader,
    ) -> PrecompileResult {
        input.expect_arguments(1)?;
        let account_id = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0);
        let payee =
            pallet_staking::Pallet::<Runtime>::payee(&account_id);
        let res = match payee {
            RewardDestination::Staked => H160::from_low_u64_be(1),
            RewardDestination::Stash => H160::from_low_u64_be(2),
            RewardDestination::Controller => H160::from_low_u64_be(3),
            RewardDestination::Account(acc) => acc.into(),
            RewardDestination::None => H160::from_low_u64_be(0),
        };
        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write(Address(res))
                    .build(),
            }
        )
    }

    fn active_era(
        input: EvmDataReader,
    ) -> PrecompileResult {
        input.expect_arguments(0)?;
        let active_era =
            pallet_staking::Pallet::<Runtime>::active_era().map(|v| v.index).unwrap_or_default();
        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write(active_era)
                    .build(),
            }
        )
    }

    fn eras_stakers(
        mut input: EvmDataReader,
    ) -> PrecompileResult {
        input.expect_arguments(2)?;
        let index = input.read::<u32>()?;
        let account_id = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0);

        let exposure =
            pallet_staking::Pallet::<Runtime>::eras_stakers(index, account_id);

        let others: Vec<(Address, u128)> = exposure.others
            .iter()
            .map(|individual| (Address(individual.who.clone().into()), individual.value.saturated_into::<u128>()))
            .collect();
        let (nominators, amounts): (Vec<Address>, Vec<u128>) = others.into_iter().unzip();
        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write(nominators)
                    .write(amounts)
                    .build(),
            }
        )
    }

    fn eras_validator_prefs(
        mut input: EvmDataReader,
    ) -> PrecompileResult {
        input.expect_arguments(2)?;
        let index = input.read::<u32>()?;
        let account_id = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0);

        let pref =
            pallet_staking::Pallet::<Runtime>::eras_validator_prefs(index, account_id);

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write(pref.commission.deconstruct())
                    .write(!pref.blocked) // can nominate by others
                    .build(),
            }
        )
    }

    fn nominators(
        mut input: EvmDataReader,
    ) -> PrecompileResult {
        input.expect_arguments(1)?;
        let account_id = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0);

        let nominations: Vec<Address> =
            pallet_staking::Pallet::<Runtime>::nominators(account_id)
                .map(|v| v.targets.iter().map(|acc| Address(acc.clone().into())).collect())
                .unwrap_or_default();

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write(nominations)
                    .build(),
            }
        )
    }

    fn era_validator_reward(
        mut input: EvmDataReader,
    ) -> PrecompileResult {
        input.expect_arguments(2)?;
        let era_index = input.read::<u32>()?;
        let account = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0);

        let reward = {
            let era_payout = pallet_staking::Pallet::<Runtime>::eras_validator_reward(era_index).unwrap_or_default();
            let era_reward_points = pallet_staking::Pallet::<Runtime>::eras_reward_points(era_index);
            if era_reward_points.total == 0 {
                return Ok(
                    PrecompileOutput {
                        exit_status: ExitSucceed::Returned,
                        output: precompile_utils::data::EvmDataWriter::new()
                            .write(0u128)
                            .build(),
                    }
                )
            }
            let exposure = pallet_staking::Pallet::<Runtime>::eras_stakers_clipped(era_index, account.clone());
            if exposure.total.saturated_into::<u128>() == 0 {
                return Ok(
                    PrecompileOutput {
                        exit_status: ExitSucceed::Returned,
                        output: precompile_utils::data::EvmDataWriter::new()
                            .write(0u128)
                            .build(),
                    }
                )
            }
            let validator_point = era_reward_points.individual.get(&account)
                .map(|points| *points)
                .unwrap_or_else(|| 0);
            // validator's rewards(contain commission and stakers)
            let validator_part: Perbill = PerThing::from_rational(validator_point, era_reward_points.total);
            let validator_total_payout = validator_part * era_payout;

            let part_info = pallet_staking::Pallet::<Runtime>::validators(account);
            // validator's commission reward
            let validator_commission_payout = part_info.commission * validator_total_payout;
            let validator_leftover_payout = validator_total_payout - validator_commission_payout;

            // distribute rewards by stake
            // validators staking rewards
            let validator_staking_part: Perbill = PerThing::from_rational(exposure.own.saturated_into::<u128>(), exposure.total.saturated_into::<u128>());
            // let validator_staking_payout = (U256::from(validator_leftover_payout) * U256::from(exposure.own)).as_u128() / exposure.total;
            let validator_staking_payout = validator_staking_part * validator_leftover_payout;
            (validator_staking_payout + validator_commission_payout).saturated_into::<u128>()
        };

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write(reward)
                    .build(),
            }
        )
    }

    fn era_nominator_reward(
        mut input: EvmDataReader,
    ) -> PrecompileResult {
        input.expect_arguments(2)?;
        let era_index = input.read::<u32>()?;
        let account = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0);

        let reward = {
            let nominations = match pallet_staking::Pallet::<Runtime>::nominators(account.clone()) {
                Some(nominations) => nominations,
                None => return Ok(
                    PrecompileOutput {
                        exit_status: ExitSucceed::Returned,
                        output: precompile_utils::data::EvmDataWriter::new()
                            .write(0u128)
                            .build(),
                    }
                ),
            };
            let era_payout = match pallet_staking::Pallet::<Runtime>::eras_validator_reward(era_index) {
                Some(era_payout) => era_payout,
                None => return Ok(
                    PrecompileOutput {
                        exit_status: ExitSucceed::Returned,
                        output: precompile_utils::data::EvmDataWriter::new()
                            .write(0u128)
                            .build(),
                    }
                ),
            };
            let era_reward_points = pallet_staking::Pallet::<Runtime>::eras_reward_points(era_index);
            if era_reward_points.total == 0 {
                return Ok(
                    PrecompileOutput {
                        exit_status: ExitSucceed::Returned,
                        output: precompile_utils::data::EvmDataWriter::new()
                            .write(0u128)
                            .build(),
                    }
                )
            }
            let validators = nominations.targets;
            let mut nominator_rewards = 0;
            // check all validators about the nominator
            for validator in validators {
                let exposure = pallet_staking::Pallet::<Runtime>::eras_stakers_clipped(era_index, validator.clone());
                if exposure.total.saturated_into::<u128>() == 0 {
                   continue;
                }
                let validator_point = era_reward_points.individual.get(&validator)
                    .map(|points| *points)
                    .unwrap_or_else(|| 0);
                let validator_part: Perbill = PerThing::from_rational(validator_point, era_reward_points.total);
                let validator_total_payout = validator_part * era_payout;

                let part_info = pallet_staking::Pallet::<Runtime>::validators(validator);
                // validator's commission reward
                let validator_commission_payout = part_info.commission * validator_total_payout;
                let validator_leftover_payout = validator_total_payout - validator_commission_payout;

                for individual in exposure.others.into_iter() {
                    if individual.who == account {
                        let nominator_staking_part: Perbill = PerThing::from_rational(individual.value.saturated_into::<u128>(), exposure.total.saturated_into::<u128>());
                        nominator_rewards += nominator_staking_part * validator_leftover_payout.saturated_into::<u128>();
                        break;
                    }
                }
            }
            nominator_rewards.saturated_into::<u128>()
        };

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write(reward)
                    .build(),
            }
        )
    }

    fn eras_total_stake(
        mut input: EvmDataReader,
    ) -> PrecompileResult {
        input.expect_arguments(1)?;
        let era_index = input.read::<u32>()?;

        let total =
            pallet_staking::Pallet::<Runtime>::eras_total_stake(era_index);

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write(total.saturated_into::<u128>())
                    .build(),
            }
        )
    }

    // tx call
    fn bond_and_nominate(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context,
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(3)?;

        // let controller: pallet_staking::AccountIdLookupOf<Runtime> = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0).into();
        let bond_value = input.read::<u128>()?;
        let value: BalanceOf<Runtime> = SaturatedConversion::saturated_from(bond_value);

        let payee = match input.read::<u8>()? {
            0 => RewardDestination::Staked,
            1 => RewardDestination::Stash,
            _ => RewardDestination::default(),
        };
        let targets: Vec<<Runtime::Lookup as StaticLookup>::Source> = input
            .read::<Vec<Address>>()?
            .into_iter()
            .map(|addr| Runtime::AddressMapping::into_account_id(addr.0).into())
            .collect();
        let controller: pallet_staking::AccountIdLookupOf<Runtime> = origin.clone().into();
        let bond_call = pallet_staking::Call::<Runtime>::bond {
            controller,
            value,
            payee,
        }.into();

        let nominate_call = pallet_staking::Call::<Runtime>::nominate{ targets }.into();

        let calls = vec![bond_call, nominate_call];
        let batch_call: <Runtime as frame_system::Config>::RuntimeCall = pallet_utility::Call::<Runtime>::batch_all{ calls }.into();
        let dispatch_info = batch_call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            batch_call,
            gasometer.remaining_gas()?,
        )?;

        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn bond_and_validate(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context,
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(6)?;

        let bond_value = input.read::<u128>()?;
        let value: BalanceOf<Runtime> = SaturatedConversion::saturated_from(bond_value);

        let payee = match input.read::<u8>()? {
            0 => RewardDestination::Staked,
            1 => RewardDestination::Stash,
            _ => RewardDestination::default(),
        };

        let commission = sp_runtime::Perbill::from_percent(input.read::<u32>()?);
        let can_nominated = input.read::<bool>()?;
        let raw_keys = input.read::<Bytes>()?;
        let proof = input.read::<Bytes>()?.0;
        let keys = Runtime::Keys::decode(&mut raw_keys.0.as_slice()).map_err(|_| ExitError::Other("Incorrect session keys".into()))?;
        let prefs = pallet_staking::ValidatorPrefs {
            commission,
            blocked: !can_nominated,
        };
        let controller: pallet_staking::AccountIdLookupOf<Runtime> = origin.clone().into();
        let bond_call = pallet_staking::Call::<Runtime>::bond{ controller, value, payee }.into();
        let set_session_key_call = pallet_session::Call::<Runtime>::set_keys{ keys, proof }.into();
        let validate_call = pallet_staking::Call::<Runtime>::validate{ prefs }.into();

        let calls = vec![bond_call, set_session_key_call, validate_call];
        let batch_call: <Runtime as frame_system::Config>::RuntimeCall = pallet_utility::Call::<Runtime>::batch_all { calls }.into();
        let dispatch_info = batch_call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            batch_call,
            gasometer.remaining_gas()?,
        )?;

        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn chill(
        _input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context,
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        let call: <Runtime as frame_system::Config>::RuntimeCall = pallet_staking::Call::<Runtime>::chill{}.into();
        let dispatch_info = call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            call,
            gasometer.remaining_gas()?,
        )?;

        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn bond_extra(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context,
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(1)?;
        let value = input.read::<u128>()?;
        let max_additional: BalanceOf<Runtime> = SaturatedConversion::saturated_from(value);
        let call: <Runtime as frame_system::Config>::RuntimeCall = pallet_staking::Call::<Runtime>::bond_extra{ max_additional }.into();
        let dispatch_info = call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            call,
            gasometer.remaining_gas()?,
        )?;

        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn unbond(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context,
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(1)?;
        let bond_value = input.read::<u128>()?;
        let value: BalanceOf<Runtime> = SaturatedConversion::saturated_from(bond_value);
        let call: <Runtime as frame_system::Config>::RuntimeCall = pallet_staking::Call::<Runtime>::unbond{ value }.into();
        let dispatch_info = call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            call,
            gasometer.remaining_gas()?,
        )?;

        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn payout_stakers(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context,
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(2)?;
        let validator_stash = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0);
        let era_indexs = input.read::<Vec<u32>>()?;

        let mut calls = Vec::new();
        for era in era_indexs {
            let call = pallet_staking::Call::<Runtime>::payout_stakers{ validator_stash: validator_stash.clone(), era };
            calls.push(call.into());
        }

        let batch_call: <Runtime as frame_system::Config>::RuntimeCall = pallet_utility::Call::<Runtime>::batch_all{ calls }.into();
        let dispatch_info = batch_call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            batch_call,
            gasometer.remaining_gas()?,
        )?;

        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn set_payee(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(1)?;
        let payee = match input.read::<u8>()? {
            0 => RewardDestination::Staked,
            1 => RewardDestination::Stash,
            _ => RewardDestination::default(),
        };

        let call: <Runtime as frame_system::Config>::RuntimeCall = pallet_staking::Call::<Runtime>::set_payee{ payee }.into();
        let dispatch_info = call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = match RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            call,
            gasometer.remaining_gas()?,
        ) {
            Ok(used_gas) => {
                used_gas
            },
            Err(e) => {
                return Err(e);
            }
        };
        gasometer.record_cost(used_gas)?;
        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn withdraw_unbonded(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(1)?;
        if pallet_staking::Pallet::<Runtime>::ledger(&origin).is_none() {
            return Ok(
                PrecompileOutput {
                    exit_status: ExitSucceed::Stopped,
                    output: Default::default(),
                }
            )
        }
        let num_slashing_spans = input.read::<u32>()?;
        let call: <Runtime as frame_system::Config>::RuntimeCall = pallet_staking::Call::<Runtime>::withdraw_unbonded{ num_slashing_spans }.into();
        // let dispatch_info = <Runtime as pallet_staking::Config>::Call::from(call.clone()).get_dispatch_info();
        let dispatch_info = call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            call,
            gasometer.remaining_gas()?,
        )?;
        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn nominate(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(1)?;
        let targets: Vec<<Runtime::Lookup as StaticLookup>::Source> = input.read::<Vec<Address>>()?
            .into_iter()
            .map(|addr| Runtime::AddressMapping::into_account_id(addr.0).into())
            .collect();
        let nominate_call: <Runtime as frame_system::Config>::RuntimeCall = pallet_staking::Call::<Runtime>::nominate{ targets }.into();
        let dispatch_info = nominate_call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            nominate_call,
            gasometer.remaining_gas()?,
        )?;

        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn validate(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(2)?;

        let commission = sp_runtime::Perbill::from_percent(input.read::<u32>()?);
        let can_nominated = input.read::<bool>()?;

        let prefs = pallet_staking::ValidatorPrefs {
            commission,
            blocked: !can_nominated
        };

        let validate_call: <Runtime as frame_system::Config>::RuntimeCall = pallet_staking::Call::<Runtime>::validate{ prefs }.into();
        let dispatch_info = validate_call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            validate_call,
            gasometer.remaining_gas()?,
        )?;

        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn set_session_keys(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(2)?;

        let raw_keys = input.read::<Bytes>()?;
        let proof = input.read::<Bytes>()?.0;
        let keys = Runtime::Keys::decode(&mut raw_keys.0.as_slice()).map_err(|_| ExitError::Other("Incorrect session keys".into()))?;
        let call: <Runtime as frame_system::Config>::RuntimeCall = pallet_session::Call::<Runtime>::set_keys{ keys, proof }.into();
        let dispatch_info = call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            call,
            gasometer.remaining_gas()?,
        )?;

        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn set_session_keys_and_validate(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(4)?;

        let commission = sp_runtime::Perbill::from_percent(input.read::<u32>()?);
        let can_nominated = input.read::<bool>()?;

        let prefs = pallet_staking::ValidatorPrefs {
            commission,
            blocked: !can_nominated
        };
        let validate_call = pallet_staking::Call::<Runtime>::validate{ prefs }.into();

        let raw_keys = input.read::<Bytes>()?;
        let proof = input.read::<Bytes>()?.0;
        let keys = Runtime::Keys::decode(&mut raw_keys.0.as_slice()).map_err(|_| ExitError::Other("Incorrect session keys".into()))?;
        let set_session_keys_call = pallet_session::Call::<Runtime>::set_keys{ keys, proof }.into();
        let calls = vec![validate_call, set_session_keys_call];
        let batch_call: <Runtime as frame_system::Config>::RuntimeCall = pallet_utility::Call::<Runtime>::batch_all{ calls }.into();
        let dispatch_info = batch_call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            batch_call,
            gasometer.remaining_gas()?,
        )?;

        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn chill_and_unbonded(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context
    ) -> PrecompileResult {
        input.expect_arguments(1)?;
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        let chill_call = pallet_staking::Call::<Runtime>::chill{}.into();
        let unbond_value = input.read::<u128>()?;
        let value: BalanceOf<Runtime> = SaturatedConversion::saturated_from(unbond_value);
        let unbond_call = pallet_staking::Call::<Runtime>::unbond{ value }.into();

        let calls = vec![chill_call, unbond_call];
        let batch_call: <Runtime as frame_system::Config>::RuntimeCall = pallet_utility::Call::<Runtime>::batch_all{ calls }.into();
        let dispatch_info = batch_call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            batch_call,
            gasometer.remaining_gas()?,
        )?;

        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn bond_extra_and_nominate(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context,
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(2)?;

        // let controller: pallet_staking::AccountIdLookupOf<Runtime> = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0).into();
        let bond_value = input.read::<u128>()?;
        let max_additional: BalanceOf<Runtime> = SaturatedConversion::saturated_from(bond_value);

        let targets: Vec<<Runtime::Lookup as StaticLookup>::Source> = input
            .read::<Vec<Address>>()?
            .into_iter()
            .map(|addr| Runtime::AddressMapping::into_account_id(addr.0).into())
            .collect();
        let bond_extra_call = pallet_staking::Call::<Runtime>::bond_extra {
            max_additional,
        }.into();

        let nominate_call = pallet_staking::Call::<Runtime>::nominate{ targets }.into();

        let calls = vec![bond_extra_call, nominate_call];
        let batch_call: <Runtime as frame_system::Config>::RuntimeCall = pallet_utility::Call::<Runtime>::batch_all{ calls }.into();
        let dispatch_info = batch_call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            batch_call,
            gasometer.remaining_gas()?,
        )?;

        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

    fn bond_extra_and_validate(
        mut input: EvmDataReader,
        mut target_gas: Option<u64>,
        context: &Context,
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(5)?;

        let bond_value = input.read::<u128>()?;
        let max_additional: BalanceOf<Runtime> = SaturatedConversion::saturated_from(bond_value);

        let commission = sp_runtime::Perbill::from_percent(input.read::<u32>()?);
        let can_nominated = input.read::<bool>()?;
        let raw_keys = input.read::<Bytes>()?;
        let proof = input.read::<Bytes>()?.0;
        let keys = Runtime::Keys::decode(&mut raw_keys.0.as_slice()).map_err(|_| ExitError::Other("Incorrect session keys".into()))?;
        let prefs = pallet_staking::ValidatorPrefs {
            commission,
            blocked: !can_nominated,
        };
        let bond_extra_call = pallet_staking::Call::<Runtime>::bond_extra {
            max_additional,
        }.into();
        let set_session_key_call = pallet_session::Call::<Runtime>::set_keys{ keys, proof }.into();
        let validate_call = pallet_staking::Call::<Runtime>::validate{ prefs }.into();

        let calls = vec![bond_extra_call, set_session_key_call, validate_call];
        let batch_call: <Runtime as frame_system::Config>::RuntimeCall = pallet_utility::Call::<Runtime>::batch_all { calls }.into();
        let dispatch_info = batch_call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        target_gas = Some(required_gas);
        let mut gasometer = Gasometer::new(target_gas);
        let used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            batch_call,
            gasometer.remaining_gas()?,
        )?;

        gasometer.record_cost(used_gas)?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }

}
