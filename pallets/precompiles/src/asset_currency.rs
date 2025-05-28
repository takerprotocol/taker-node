use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
use pallet_evm::AddressMapping;
use precompile_utils::prelude::*;
use sp_core::{H160, U256};
use sp_runtime::traits::Dispatchable;
use sp_runtime::SaturatedConversion;
use sp_std::marker::PhantomData;
use sp_std::vec::Vec;

pub struct AssetCurrencyPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> AssetCurrencyPrecompile<Runtime>
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
		let account_data =
			pallet_asset_currency::Pallet::<Runtime>::account_available_balance(&account_id);
		let available_balance = account_data.saturated_into::<u128>();
		Ok(available_balance.into())
	}

	#[precompile::public("metadata()")]
	#[precompile::view]
	fn metadata(handle: &mut impl PrecompileHandle) -> EvmResult<(UnboundedBytes, U256)> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let (symbol, dec) = pallet_asset_currency::Pallet::<Runtime>::token_metadata();
		Ok((UnboundedBytes::from(symbol), dec.into()))
	}

	#[precompile::public("whitelistAdmin()")]
	#[precompile::public("whitelist_admin()")]
	#[precompile::view]
	fn whitelist_admin(handle: &mut impl PrecompileHandle) -> EvmResult<Address> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let admin = pallet_asset_currency::Pallet::<Runtime>::get_admin();
		Ok(Address(admin.into()))
	}

	#[precompile::public("whitelist()")]
	#[precompile::view]
	fn whitelist(handle: &mut impl PrecompileHandle) -> EvmResult<Vec<Address>> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let list = pallet_asset_currency::Pallet::<Runtime>::whitelist();
		let whitelist = list.into_iter().map(|acc| Address(acc.into())).collect();
		Ok(whitelist)
	}

	// Dispatchable methods

	#[precompile::public("mintTo(address,uint256)")]
	#[precompile::public("mint_to(address,uint256)")]
	fn mint_to(handle: &mut impl PrecompileHandle, to: Address, value: u128) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let amount: <Runtime as pallet_asset_currency::Config>::Balance =
			SaturatedConversion::saturated_from(value);
		let to_account = Runtime::AddressMapping::into_account_id(to.0);
		let call = pallet_asset_currency::Call::<Runtime>::taker_mint_to { amount, to_account };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("burn(address,uint256)")]
	fn burn(handle: &mut impl PrecompileHandle, from: Address, value: u128) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let amount: <Runtime as pallet_asset_currency::Config>::Balance =
			SaturatedConversion::saturated_from(value);
		let from_account = Runtime::AddressMapping::into_account_id(from.0);
		let call = pallet_asset_currency::Call::<Runtime>::taker_burn { amount, from_account };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("transferWhitelistAdmin(address)")]
	#[precompile::public("transfer_whitelist_admin(address)")]
	fn transfer_whitelist_admin(handle: &mut impl PrecompileHandle, to: Address) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let new_admin = Runtime::AddressMapping::into_account_id(to.0);
		let call = pallet_asset_currency::Call::<Runtime>::transfer_whitelist_admin { new_admin };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("updateWhitelist(address,bool)")]
	#[precompile::public("update_whitelist(address,bool)")]
	fn update_whitelist(handle: &mut impl PrecompileHandle, to: Address, add: bool) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let account = Runtime::AddressMapping::into_account_id(to.0);
		let call = pallet_asset_currency::Call::<Runtime>::update_whitelist { account, add };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}

	#[precompile::public("transfer(address,uint256)")]
	fn transfer(handle: &mut impl PrecompileHandle, to: Address, value: u128) -> EvmResult {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let value: <Runtime as pallet_asset_currency::Config>::Balance =
			SaturatedConversion::saturated_from(value);
		let to = Runtime::AddressMapping::into_account_id(to.0);
		let call = pallet_asset_currency::Call::<Runtime>::transfer { to, value };
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
		Ok(())
	}
}
