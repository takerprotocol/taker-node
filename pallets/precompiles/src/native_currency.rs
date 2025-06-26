use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
use frame_support::traits::StoredMap;

use pallet_evm::AddressMapping;
use precompile_utils::prelude::*;
use sp_core::{H160, U256};
use sp_runtime::traits::Dispatchable;
use sp_runtime::SaturatedConversion;
use sp_std::marker::PhantomData;

pub struct NativeCurrencyPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> NativeCurrencyPrecompile<Runtime>
where
	Runtime: pallet_asset_currency::Config
		+ pallet_balances::Config
		+ pallet_evm::Config
		+ frame_system::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::RuntimeCall: From<pallet_asset_currency::Call<Runtime>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	<Runtime as pallet_balances::Config>::Balance: Into<U256>,
	Runtime::AccountId: Into<H160>,
{
	// Storage getters

	#[precompile::public("balanceOf(address)")]
	#[precompile::public("balance_of(address)")]
	#[precompile::view]
	fn balance_of(handle: &mut impl PrecompileHandle, who: Address) -> EvmResult<U256> {
		let account_id = Runtime::AddressMapping::into_account_id(who.0);
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let account_data = <Runtime as pallet_balances::Config>::AccountStore::get(&account_id);
		let available_balance = account_data.usable().saturated_into::<u128>();
		Ok(available_balance.into())
	}

	// Dispatchable methods

	#[precompile::public("mintTo(address,uint256)")]
	#[precompile::public("mint_to(address,uint256)")]
	fn mint_to(handle: &mut impl PrecompileHandle, to: Address, value: u128) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let amount: <Runtime as pallet_asset_currency::Config>::Balance =
			SaturatedConversion::saturated_from(value);
		let to_account = Runtime::AddressMapping::into_account_id(to.0);
		let call = pallet_asset_currency::Call::<Runtime>::native_mint_to { amount, to_account };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("burnFrom(address,uint256)")]
	#[precompile::public("burn_from(address,uint256)")]
	fn burn_from(handle: &mut impl PrecompileHandle, from: Address, value: u128) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let amount: <Runtime as pallet_asset_currency::Config>::Balance =
			SaturatedConversion::saturated_from(value);
		let from_account = Runtime::AddressMapping::into_account_id(from.0);
		let call = pallet_asset_currency::Call::<Runtime>::native_burn { amount, from_account };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}
}
