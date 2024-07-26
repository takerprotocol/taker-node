use sp_std::{fmt::Debug, marker::PhantomData};
use fp_evm::{
    PrecompileHandle, PrecompileResult, Context, PrecompileOutput, ExitSucceed, Precompile,
};
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
use frame_support::sp_runtime;
use frame_support::sp_runtime::SaturatedConversion;
use frame_support::traits::StoredMap;
use pallet_evm::{AddressMapping, GasWeightMapping};
use precompile_utils::*;
use sp_runtime::traits::Dispatchable;
use precompile_utils::data::Address;

#[generate_function_selector]
#[derive(Debug, PartialEq, num_enum::TryFromPrimitive)]
pub enum NativeCurrencyAction {
    BalanceOf = "balanceOf(address)",
    MintTo = "mintTo(address,uint256)",
}

pub struct NativeCurrencyPrecompile<Runtime>(PhantomData<Runtime>);

impl<Runtime> Precompile for NativeCurrencyPrecompile<Runtime>
    where
        Runtime: pallet_asset_currency::Config + pallet_evm::Config + pallet_balances::Config,
        <Runtime as frame_system::Config>::RuntimeCall:
        Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
        <<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<Runtime::AccountId>>,
        <Runtime as frame_system::Config>::RuntimeCall: From<pallet_asset_currency::Call<Runtime>>,
{
    fn execute(
        handle: &mut impl PrecompileHandle
    ) -> PrecompileResult {
        let input = handle.input();
        let target_gas = handle.gas_limit();
        let context = handle.context();
        let (input_reader, selector) = EvmDataReader::new_with_selector(input)?;
        match selector {
            // Storage
            NativeCurrencyAction::BalanceOf => Self::balance_of(input_reader),
            // Tx call
            NativeCurrencyAction::MintTo => Self::mint_to(input_reader, target_gas, context),
        }
    }
}

impl<Runtime> NativeCurrencyPrecompile<Runtime>
    where
        Runtime: pallet_asset_currency::Config + pallet_evm::Config + pallet_balances::Config,
        <Runtime as frame_system::Config>::RuntimeCall:
        Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
        <<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<Runtime::AccountId>>,
        <Runtime as frame_system::Config>::RuntimeCall: From<pallet_asset_currency::Call<Runtime>>,
{
    fn balance_of(
        mut input: EvmDataReader,
    ) -> PrecompileResult {
        input.expect_arguments(1)?;
        let account_id = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0);

        let account_data =
            <Runtime as pallet_balances::Config>::AccountStore::get(&account_id);
        let available_balance = account_data.usable().saturated_into::<u128>();
        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write(available_balance)
                    .build(),
            }
        )
    }

    fn mint_to(
        mut input: EvmDataReader,
        _target_gas: Option<u64>,
        context: &Context,
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(2)?;
        let to = input.read::<Address>()?;
        let value = input.read::<u128>()?;

        let amount: <Runtime as pallet_asset_currency::Config>::Balance = SaturatedConversion::saturated_from(value);
        let to_account = Runtime::AddressMapping::into_account_id(to.0);
        let call = pallet_asset_currency::Call::<Runtime>::native_mint_to {
            amount,
            to_account,
        };
        let dispatch_info = call.get_dispatch_info();
        let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
        let gasometer = Gasometer::new(Some(required_gas));
        let _used_gas = RuntimeHelper::<Runtime>::try_dispatch(
            Some(origin).into(),
            call,
            gasometer.remaining_gas()?,
        )?;

        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Stopped,
                output: Default::default(),
            }
        )
    }
}
