use sp_std::{fmt::Debug, marker::PhantomData, vec::Vec};
use fp_evm::{PrecompileHandle, PrecompileResult, Context, PrecompileOutput, ExitSucceed, Precompile};
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
use frame_support::sp_runtime;
use frame_support::sp_runtime::SaturatedConversion;
use pallet_evm::{AddressMapping, GasWeightMapping};
use sp_core::H160;
use precompile_utils::*;
use sp_runtime::traits::Dispatchable;
use precompile_utils::data::Address;
use sp_core::Encode;

#[generate_function_selector]
#[derive(Debug, PartialEq, num_enum::TryFromPrimitive)]
pub enum AssetCurrencyAction {
    BalanceOf = "balanceOf(address)",
    Metadata = "metadata()",
    WhitelistAdmin = "whitelistAdmin()",
    Whitelist = "whitelist()",
    MintTo = "mintTo(address,uint256)",
    Burn = "burn(address,uint256)",
    TransferWhitelistAdmin = "transferWhitelistAdmin(address)",
    UpdateWhitelist = "updateWhitelist(address,bool)",
    Transfer = "transfer(address,uint256)",
}

pub struct AssetCurrencyPrecompile<Runtime>(PhantomData<Runtime>);

impl<Runtime> Precompile for AssetCurrencyPrecompile<Runtime>
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
            AssetCurrencyAction::BalanceOf => Self::balance_of(input_reader),
            AssetCurrencyAction::Metadata => Self::metadata(),
            AssetCurrencyAction::WhitelistAdmin => Self::whitelist_admin(),
            AssetCurrencyAction::Whitelist => Self::whitelist(),
            // Tx call
            AssetCurrencyAction::MintTo => Self::mint_to(input_reader, target_gas, context),
            AssetCurrencyAction::Burn => {
                Self::burn(input_reader, target_gas, context)
            },
            AssetCurrencyAction::TransferWhitelistAdmin => Self::transfer_whitelist_admin(input_reader, target_gas, context),
            AssetCurrencyAction::UpdateWhitelist => Self::update_whitelist(input_reader, target_gas, context),
            AssetCurrencyAction::Transfer => Self::transfer(input_reader, target_gas, context),
        }
    }
}

impl<Runtime> AssetCurrencyPrecompile<Runtime>
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

        let available_balance =
            pallet_asset_currency::Pallet::<Runtime>::account_available_balance(&account_id);
        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write(available_balance.saturated_into::<u128>())
                    .build(),
            }
        )
    }

    fn metadata() -> PrecompileResult {
        let (symbol, dec) =
            pallet_asset_currency::Pallet::<Runtime>::token_metadata();
        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write::<precompile_utils::data::Bytes>(symbol.as_slice().into())
                    .write(dec)
                    .build(),
            }
        )
    }

    fn whitelist_admin() -> PrecompileResult {
        let admin =
            pallet_asset_currency::Pallet::<Runtime>::get_admin();
        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write::<Address>(Address::from(H160::from_slice(&admin.encode())))
                    .build(),
            }
        )
    }

    fn whitelist() -> PrecompileResult {
        let list =
            pallet_asset_currency::Pallet::<Runtime>::whitelist();
        let whitelist = list.iter().map(|acc| Address::from(H160::from_slice(&acc.encode()))).collect();
        Ok(
            PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                output: precompile_utils::data::EvmDataWriter::new()
                    .write::<Vec<Address>>(whitelist)
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
        let call = pallet_asset_currency::Call::<Runtime>::taker_mint_to {
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

    fn burn(
        mut input: EvmDataReader,
        _target_gas: Option<u64>,
        context: &Context,
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(2)?;
        let from = input.read::<Address>()?;
        let value = input.read::<u128>()?;

        let amount: <Runtime as pallet_asset_currency::Config>::Balance = SaturatedConversion::saturated_from(value);
        let from_account = Runtime::AddressMapping::into_account_id(from.0);
        let call = pallet_asset_currency::Call::<Runtime>::taker_burn {
            amount,
            from_account,
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

    fn transfer_whitelist_admin(
        mut input: EvmDataReader,
        _target_gas: Option<u64>,
        context: &Context,
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(1)?;
        let new_admin = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0).into();

        let call = pallet_asset_currency::Call::<Runtime>::transfer_whitelist_admin {
            new_admin,
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

    fn update_whitelist(
        mut input: EvmDataReader,
        _target_gas: Option<u64>,
        context: &Context,
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(2)?;
        let account = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0).into();
        let add = input.read::<bool>()?;
        let call = pallet_asset_currency::Call::<Runtime>::update_whitelist {
            account,
            add,
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

    fn transfer(
        mut input: EvmDataReader,
        _target_gas: Option<u64>,
        context: &Context,
    ) -> PrecompileResult {
        let origin = Runtime::AddressMapping::into_account_id(context.caller);
        input.expect_arguments(2)?;
        let to = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0).into();
        let amount = input.read::<u128>()?;
        let value: <Runtime as pallet_asset_currency::Config>::Balance = SaturatedConversion::saturated_from(amount);
        let call = pallet_asset_currency::Call::<Runtime>::transfer {
            to,
            value,
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
