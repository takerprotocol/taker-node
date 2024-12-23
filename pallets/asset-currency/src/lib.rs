#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unused_crate_dependencies)]

pub mod impl_currency;
pub mod types;
mod impl_fungible;

// use frame_support::ensure;
pub use pallet::*;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
    use sp_std::fmt::Debug;
    use parity_scale_codec::Codec;
    use sp_std::vec::Vec;
    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, BoundedSlice, transactional, WeakBoundedVec, PalletId};
    use frame_support::sp_runtime::{FixedPointOperand, Saturating};
    use frame_support::sp_runtime::traits::{AtLeast32BitUnsigned, CheckedAdd, Zero};
    use frame_support::traits::{fungible, BalanceStatus as Status, ReservableCurrency, OnUnbalanced, Defensive, Currency};
    use frame_support::traits::fungible::{Credit, Inspect, Mutate};
    use frame_support::traits::tokens::{Fortitude, Precision};
    use frame_support::traits::tokens::Preservation::Expendable;
    use frame_system::pallet_prelude::*;
    use sp_runtime::{ArithmeticError, SaturatedConversion};
    use sp_runtime::traits::AccountIdConversion;
    use crate::types::*;

    pub type CreditOf<T> = Credit<<T as frame_system::Config>::AccountId, Pallet<T>>;
    const LOG_TARGET: &str = "runtime::asset-currency";

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Native Currency
        type NativeCurrency: Currency<Self::AccountId> + fungible::Mutate<Self::AccountId>;
        /// The balance of an account.
        type Balance: Parameter
        + Member
        + AtLeast32BitUnsigned
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + MaxEncodedLen
        + TypeInfo
        + FixedPointOperand;
        /// The ID type for reserves.
        ///
        /// Use of reserves is deprecated in favour of holds. See `https://github.com/paritytech/substrate/pull/12951/`
        type ReserveIdentifier: Parameter + Member + MaxEncodedLen + Ord + Copy;
        /// The ID type for freezes.
        type FreezeIdentifier: Parameter + Member + MaxEncodedLen + Copy;
        /// The ID type for holds.
        type HoldIdentifier: Parameter + Member + MaxEncodedLen + Ord + Copy;
        /// Handler for the unbalanced reduction when removing a dust account.
        type DustRemoval: OnUnbalanced<CreditOf<Self>>;
        /// The minimum amount required to keep an account open. MUST BE GREATER THAN ZERO!
        ///
        /// If you *really* need it to be zero, you can enable the feature `insecure_zero_ed` for
        /// this pallet. However, you do so at your own risk: this will open up a major DoS vector.
        /// In case you have multiple sources of provider references, you may also get unexpected
        /// behaviour if you set this to zero.
        ///
        /// Bottom line: Do yourself a favour and make it at least one!
        #[pallet::constant]
        type ExistentialDeposit: Get<Self::Balance>;
        /// The maximum number of locks that should exist on an account.
        /// Not strictly enforced, but used for weight estimation.
        #[pallet::constant]
        type MaxLocks: Get<u32>;

        /// The maximum number of named reserves that can exist on an account.
        #[pallet::constant]
        type MaxReserves: Get<u32>;

        /// The maximum number of holds that can exist on an account at any time.
        #[pallet::constant]
        type MaxHolds: Get<u32>;

        /// The maximum number of individual freeze locks that can exist on an account at any time.
        #[pallet::constant]
        type MaxFreezes: Get<u32>;
        #[pallet::constant]
        type DefaultAdmin: Get<Self::AccountId>;
        #[pallet::constant]
        type GasFeeCollector: Get<Self::AccountId>;
        /// The pallet id used for deriving its sovereign account ID.
        #[pallet::constant]
        type PalletId: Get<PalletId>;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The total units issued in the system.
    #[pallet::storage]
    #[pallet::getter(fn total_issuance)]
    #[pallet::whitelist_storage]
    pub type TotalIssuance<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

    #[pallet::storage]
    pub type Account<T: Config> =
    StorageMap<_, Blake2_128Concat, T::AccountId, AccountData<T::Balance>, ValueQuery>;

    /// Any liquidity locks on some account balances.
    /// NOTE: Should only be accessed when setting, changing and freeing a lock.
    #[pallet::storage]
    #[pallet::getter(fn locks)]
    pub type Locks<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        WeakBoundedVec<BalanceLock<T::Balance>, T::MaxLocks>,
        ValueQuery,
    >;

    /// Named reserves on some account balances.
    #[pallet::storage]
    #[pallet::getter(fn reserves)]
    pub type Reserves<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<ReserveData<T::ReserveIdentifier, T::Balance>, T::MaxReserves>,
        ValueQuery,
    >;

    /// Holds on account balances.
    #[pallet::storage]
    pub type Holds<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<IdAmount<T::HoldIdentifier, T::Balance>, T::MaxHolds>,
        ValueQuery,
    >;

    /// The total units of outstanding deactivated balance in the system.
    #[pallet::storage]
    #[pallet::getter(fn inactive_issuance)]
    #[pallet::whitelist_storage]
    pub type InactiveIssuance<T: Config> =
    StorageValue<_, T::Balance, ValueQuery>;

    /// Freeze locks on account balances.
    #[pallet::storage]
    pub type Freezes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<IdAmount<T::FreezeIdentifier, T::Balance>, T::MaxFreezes>,
        ValueQuery,
    >;

    /// The controllers list to burn and mint token
    #[pallet::storage]
    #[pallet::getter(fn token_controllers)]
    pub type TokenControllers<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    /// The asset metadata
    #[pallet::storage]
    #[pallet::getter(fn token_metadata)]
    pub type TokenMetadata<T: Config> = StorageValue<_, (Vec<u8>, u8), ValueQuery>;

    /// The whitelist for 'Transfer'
    #[pallet::storage]
    #[pallet::getter(fn whitelist)]
    pub type Whitelist<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    /// The controller for 'TransferWhitelist'
    #[pallet::storage]
    #[pallet::getter(fn whitelist_admin)]
    pub type WhitelistAdmin<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub symbol: Vec<u8>,
        pub decimals: u8,
        pub balances: Vec<(T::AccountId, T::Balance)>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                symbol: "veTAKER".as_bytes().to_vec(),
                decimals: 18,
                balances: Default::default()
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            let total = self.balances.iter().fold(Zero::zero(), |acc: T::Balance, &(_, n)| acc + n);

            <TotalIssuance<T>>::put(total);

            for (_, balance) in &self.balances {
                assert!(
                    *balance >= <T as Config>::ExistentialDeposit::get(),
                    "the balance of any account should always be at least the existential deposit.",
                )
            }

            // ensure no duplicates exist.
            let endowed_accounts = self
                .balances
                .iter()
                .map(|(x, _)| x)
                .cloned()
                .collect::<sp_std::collections::btree_set::BTreeSet<_>>();

            assert!(
                endowed_accounts.len() == self.balances.len(),
                "duplicate balances in genesis."
            );

            for &(ref who, free) in self.balances.iter() {
                frame_system::Pallet::<T>::inc_providers(who);
                Account::<T>::insert(who, AccountData { free, ..Default::default() });
            }

            TokenMetadata::<T>::put((self.symbol.clone(), self.decimals));
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An account was created with some free balance.
        Endowed { account: T::AccountId, free_balance: T::Balance },
        /// An account was removed whose balance was non-zero but below ExistentialDeposit,
        /// resulting in an outright loss.
        DustLost { account: T::AccountId, amount: T::Balance },
        /// Transfer succeeded.
        Transfer { from: T::AccountId, to: T::AccountId, amount: T::Balance },
        /// A balance was set by root.
        BalanceSet { who: T::AccountId, free: T::Balance },
        /// Some balance was reserved (moved from free to reserved).
        Reserved { who: T::AccountId, amount: T::Balance },
        /// Some balance was unreserved (moved from reserved to free).
        Unreserved { who: T::AccountId, amount: T::Balance },
        /// Some balance was moved from the reserve of the first account to the second account.
        /// Final argument indicates the destination balance type.
        ReserveRepatriated {
            from: T::AccountId,
            to: T::AccountId,
            amount: T::Balance,
            destination_status: Status,
        },
        /// Some amount was deposited (e.g. for transaction fees).
        Deposit { who: T::AccountId, amount: T::Balance },
        /// Some amount was withdrawn from the account (e.g. for transaction fees).
        Withdraw { who: T::AccountId, amount: T::Balance },
        /// Some amount was removed from the account (e.g. for misbehavior).
        Slashed { who: T::AccountId, amount: T::Balance },
        /// Some amount was minted into an account.
        Minted { who: T::AccountId, amount: T::Balance },
        /// Some amount was burned from an account.
        Burned { who: T::AccountId, amount: T::Balance },
        /// Some amount was suspended from an account (it can be restored later).
        Suspended { who: T::AccountId, amount: T::Balance },
        /// Some amount was restored into an account.
        Restored { who: T::AccountId, amount: T::Balance },
        /// An account was upgraded.
        Upgraded { who: T::AccountId },
        /// Total issuance was increased by `amount`, creating a credit to be balanced.
        Issued { amount: T::Balance },
        /// Total issuance was decreased by `amount`, creating a debt to be balanced.
        Rescinded { amount: T::Balance },
        /// Some balance was locked.
        Locked { who: T::AccountId, amount: T::Balance },
        /// Some balance was unlocked.
        Unlocked { who: T::AccountId, amount: T::Balance },
        /// Some balance was frozen.
        Frozen { who: T::AccountId, amount: T::Balance },
        /// Some balance was thawed.
        Thawed { who: T::AccountId, amount: T::Balance },
    }

    #[pallet::error]
    pub enum Error<T> {
        MintInvalidImbalance,
        SlashInvalidImbalance,
        NoTransfer,
        LiquidityRestrictions,
        DeadAccount,
        ExistentialDeposit,
        InsufficientBalance,
        Expendability,
        TooManyReserves,
        TooManyHolds,
        TooManyFreezes,
        SwapEmpty,
        BurnEmpty,
        NotController,
        NotAdmin,
        AlreadyWhitelist,
        NotWhitelisted,
        BurnOverflow,
		MintEmpty,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        #[transactional]
        pub fn taker_mint_to(
            origin: OriginFor<T>,
            amount: T::Balance,
            to_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
			ensure!(TokenControllers::<T>::get().contains(&sender), Error::<T>::NotController);
			if amount.is_zero() {
                return Err(Error::<T>::MintEmpty.into())
            }
			let balance_can_burn = Self::reducible_balance(&Self::account_id(), Expendable, Fortitude::Polite);
            if balance_can_burn < amount {
                return Err(Error::<T>::BurnOverflow.into())
            }
            Account::<T>::mutate(&Self::account_id(), |account| account.free = account.free.saturating_sub(amount));
            Account::<T>::mutate(&to_account, |account| account.free = account.free.saturating_add(amount));
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn taker_burn(
            origin: OriginFor<T>,
            amount: T::Balance,
            from_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(TokenControllers::<T>::get().contains(&sender), Error::<T>::NotController);
            if amount.is_zero() {
                return Err(Error::<T>::BurnEmpty.into())
            }
            let balance_can_burn = Self::reducible_balance(&from_account, Expendable, Fortitude::Polite);
            if balance_can_burn < amount {
                return Err(Error::<T>::BurnOverflow.into())
            }
            Account::<T>::mutate(&Self::account_id(), |account| account.free = account.free.saturating_add(amount));
            Account::<T>::mutate(&from_account, |account| account.free = account.free.saturating_sub(amount));
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn native_mint_to(
            origin: OriginFor<T>,
            amount: T::Balance,
            to_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(TokenControllers::<T>::get().contains(&sender), Error::<T>::NotController);
            if amount.is_zero() {
                return Err(Error::<T>::SwapEmpty.into())
            }
            // deposit native balance
            let tmp = amount.saturated_into::<u128>();
            T::NativeCurrency::deposit_creating(&to_account, SaturatedConversion::saturated_from(tmp));
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn set_controller(
            origin: OriginFor<T>,
            new: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            let mut list = TokenControllers::<T>::get();
            if !list.contains(&new) {
                list.push(new);
                TokenControllers::<T>::put(list);
            }
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn transfer_whitelist_admin(
            origin: OriginFor<T>,
            new_admin: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            Self::verify_admin(origin)?;
            WhitelistAdmin::<T>::put(new_admin);
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn update_whitelist(
            origin: OriginFor<T>,
            account: T::AccountId,
            add: bool,
        ) -> DispatchResultWithPostInfo {
            Self::verify_admin(origin)?;
            let mut whitelist = Whitelist::<T>::get();
            if add {
                ensure!(!whitelist.contains(&account), Error::<T>::AlreadyWhitelist);
                whitelist.push(account);
            } else {
                if let Some(index) = whitelist.iter().position(|x| x == &account) {
                    whitelist.remove(index);
                } else {
                    return Err(Error::<T>::NotWhitelisted.into());
                }
            }
            Whitelist::<T>::put(whitelist);
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn transfer(
            origin: OriginFor<T>,
            to: T::AccountId,
            value: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(Whitelist::<T>::get().contains(&sender), Error::<T>::NotWhitelisted);
            <Self as Mutate<_>>::transfer(&sender, &to, value, Expendable)?;
            Ok(().into())
        }
    }
    impl<T: Config> Pallet<T> {
        /// Get account id for this pallet.
        pub fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }
        fn ed() -> T::Balance {
            T::ExistentialDeposit::get()
        }
        /// Get both the free and reserved balances of an account.
        pub(crate) fn account(who: &T::AccountId) -> AccountData<T::Balance> {
            Account::<T>::get(who)
        }
        /// Get both the free and reserved balances of an account.
        pub fn account_available_balance(who: &T::AccountId) -> T::Balance {
            let data = Account::<T>::get(who);
            data.free.saturating_sub(data.frozen)
        }
        fn have_providers_or_no_zero_ed(_: &T::AccountId) -> bool {
            true
        }
        /// Mutate an account to some new value, or delete it entirely with `None`. Will enforce
        /// `ExistentialDeposit` law, annulling the account as needed.
        ///
        /// It returns the result from the closure. Any dust is handled through the low-level
        /// `fungible::Unbalanced` trap-door for legacy dust management.
        ///
        /// NOTE: Doesn't do any preparatory work for creating a new account, so should only be used
        /// when it is known that the account already exists.
        ///
        /// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is expected that
        /// the caller will do this.
        pub(crate) fn try_mutate_account_handling_dust<R, E: From<DispatchError>>(
            who: &T::AccountId,
            f: impl FnOnce(&mut AccountData<T::Balance>, bool) -> Result<R, E>,
        ) -> Result<R, E> {
            let (r, maybe_dust) = Self::try_mutate_account(who, f)?;
            if let Some(dust) = maybe_dust {
                <Self as fungible::Unbalanced<_>>::handle_raw_dust(dust);
            }
            Ok(r)
        }

        /// Mutate an account to some new value, or delete it entirely with `None`. Will enforce
        /// `ExistentialDeposit` law, annulling the account as needed. This will do nothing if the
        /// result of `f` is an `Err`.
        ///
        /// It returns both the result from the closure, and an optional amount of dust
        /// which should be handled once it is known that all nested mutates that could affect
        /// storage items what the dust handler touches have completed.
        ///
        /// NOTE: Doesn't do any preparatory work for creating a new account, so should only be used
        /// when it is known that the account already exists.
        ///
        /// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is expected that
        /// the caller will do this.
        pub(crate) fn try_mutate_account<R, E: From<DispatchError>>(
            who: &T::AccountId,
            f: impl FnOnce(&mut AccountData<T::Balance>, bool) -> Result<R, E>,
        ) -> Result<(R, Option<T::Balance>), E> {
            let result = Account::<T>::try_mutate_exists(who, |maybe_account| {
                let is_new = maybe_account.is_none();
                let mut account = maybe_account.take().unwrap_or_default();
                let did_provide =
                    account.free >= Self::ed() && Self::have_providers_or_no_zero_ed(who);
                let did_consume =
                    !is_new && (!account.reserved.is_zero() || !account.frozen.is_zero());

                let result = f(&mut account, is_new)?;

                let does_provide = account.free >= Self::ed();
                let does_consume = !account.reserved.is_zero() || !account.frozen.is_zero();

                if !did_provide && does_provide {
                    frame_system::Pallet::<T>::inc_providers(who);
                }
                if did_consume && !does_consume {
                    frame_system::Pallet::<T>::dec_consumers(who);
                }
                if !did_consume && does_consume {
                    frame_system::Pallet::<T>::inc_consumers(who)?;
                }
                if did_provide && !does_provide {
                    // This could reap the account so must go last.
                    frame_system::Pallet::<T>::dec_providers(who).map_err(|r| {
                        // best-effort revert consumer change.
                        if did_consume && !does_consume {
                            let _ = frame_system::Pallet::<T>::inc_consumers(who).defensive();
                        }
                        if !did_consume && does_consume {
                            let _ = frame_system::Pallet::<T>::dec_consumers(who);
                        }
                        r
                    })?;
                }

                let maybe_endowed = if is_new { Some(account.free) } else { None };

                // Handle any steps needed after mutating an account.
                //
                // This includes DustRemoval unbalancing, in the case than the `new` account's total
                // balance is non-zero but below ED.
                //
                // Updates `maybe_account` to `Some` iff the account has sufficient balance.
                // Evaluates `maybe_dust`, which is `Some` containing the dust to be dropped, iff
                // some dust should be dropped.
                //
                // We should never be dropping if reserved is non-zero. Reserved being non-zero
                // should imply that we have a consumer ref, so this is economically safe.
                let ed = Self::ed();
                let maybe_dust = if account.free < ed && account.reserved.is_zero() {
                    if account.free.is_zero() {
                        None
                    } else {
                        Some(account.free)
                    }
                } else {
                    assert!(
                        account.free.is_zero() || account.free >= ed || !account.reserved.is_zero()
                    );
                    *maybe_account = Some(account);
                    None
                };
                Ok((maybe_endowed, maybe_dust, result))
            });
            result.map(|(maybe_endowed, maybe_dust, result)| {
                if let Some(endowed) = maybe_endowed {
                    Self::deposit_event(Event::Endowed {
                        account: who.clone(),
                        free_balance: endowed,
                    });
                }
                if let Some(amount) = maybe_dust {
                    Pallet::<T>::deposit_event(Event::DustLost { account: who.clone(), amount });
                }
                (result, maybe_dust)
            })
        }

        /// Mutate an account to some new value, or delete it entirely with `None`. Will enforce
        /// `ExistentialDeposit` law, annulling the account as needed.
        ///
        /// It returns the result from the closure. Any dust is handled through the low-level
        /// `fungible::Unbalanced` trap-door for legacy dust management.
        ///
        /// NOTE: Doesn't do any preparatory work for creating a new account, so should only be used
        /// when it is known that the account already exists.
        ///
        /// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is expected that
        /// the caller will do this.
        pub(crate) fn mutate_account_handling_dust<R>(
            who: &T::AccountId,
            f: impl FnOnce(&mut AccountData<T::Balance>) -> R,
        ) -> Result<R, DispatchError> {
            let (r, maybe_dust) = Self::mutate_account(who, f)?;
            if let Some(dust) = maybe_dust {
                <Self as fungible::Unbalanced<_>>::handle_raw_dust(dust);
            }
            Ok(r)
        }

        /// Mutate an account to some new value, or delete it entirely with `None`. Will enforce
        /// `ExistentialDeposit` law, annulling the account as needed.
        ///
        /// It returns both the result from the closure, and an optional amount of dust
        /// which should be handled once it is known that all nested mutates that could affect
        /// storage items what the dust handler touches have completed.
        ///
        /// NOTE: Doesn't do any preparatory work for creating a new account, so should only be used
        /// when it is known that the account already exists.
        ///
        /// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is expected that
        /// the caller will do this.
        pub(crate) fn mutate_account<R>(
            who: &T::AccountId,
            f: impl FnOnce(&mut AccountData<T::Balance>) -> R,
        ) -> Result<(R, Option<T::Balance>), DispatchError> {
            Self::try_mutate_account(who, |a, _| -> Result<R, DispatchError> { Ok(f(a)) })
        }

        /// Move the reserved balance of one account into the balance of another, according to
        /// `status`. This will respect freezes/locks only if `fortitude` is `Polite`.
        ///
        /// Is a no-op if the value to be moved is zero.
        ///
        /// NOTE: returns actual amount of transferred value in `Ok` case.
        pub(crate) fn do_transfer_reserved(
            slashed: &T::AccountId,
            beneficiary: &T::AccountId,
            value: T::Balance,
            precision: Precision,
            fortitude: Fortitude,
            status: Status,
        ) -> Result<T::Balance, DispatchError> {
            if value.is_zero() {
                return Ok(Zero::zero())
            }

            let max = <Self as fungible::InspectHold<_>>::reducible_total_balance_on_hold(
                slashed, fortitude,
            );
            let actual = match precision {
                Precision::BestEffort => value.min(max),
                Precision::Exact => value,
            };
            ensure!(actual <= max, sp_runtime::TokenError::FundsUnavailable);
            if slashed == beneficiary {
                return match status {
                    Status::Free => Ok(actual.saturating_sub(Self::unreserve(slashed, actual))),
                    Status::Reserved => Ok(actual),
                }
            }

            let ((_, maybe_dust_1), maybe_dust_2) = Self::try_mutate_account(
                beneficiary,
                |to_account, is_new| -> Result<((), Option<T::Balance>), DispatchError> {
                    ensure!(!is_new, Error::<T>::DeadAccount);
                    Self::try_mutate_account(slashed, |from_account, _| -> DispatchResult {
                        match status {
                            Status::Free =>
                                to_account.free = to_account
                                    .free
                                    .checked_add(&actual)
                                    .ok_or(ArithmeticError::Overflow)?,
                            Status::Reserved =>
                                to_account.reserved = to_account
                                    .reserved
                                    .checked_add(&actual)
                                    .ok_or(ArithmeticError::Overflow)?,
                        }
                        from_account.reserved.saturating_reduce(actual);
                        Ok(())
                    })
                },
            )?;

            if let Some(dust) = maybe_dust_1 {
                <Self as fungible::Unbalanced<_>>::handle_raw_dust(dust);
            }
            if let Some(dust) = maybe_dust_2 {
                <Self as fungible::Unbalanced<_>>::handle_raw_dust(dust);
            }

            Self::deposit_event(Event::ReserveRepatriated {
                from: slashed.clone(),
                to: beneficiary.clone(),
                amount: actual,
                destination_status: status,
            });
            Ok(actual)
        }

        /// Update the account entry for `who`, given the locks.
        pub(crate) fn update_locks(who: &T::AccountId, locks: &[BalanceLock<T::Balance>]) {
            let bounded_locks = WeakBoundedVec::<_, T::MaxLocks>::force_from(
                locks.to_vec(),
                Some("Balances Update Locks"),
            );

            if locks.len() as u32 > T::MaxLocks::get() {
                log::warn!(
					target: LOG_TARGET,
					"Warning: A user has more currency locks than expected. \
					A runtime configuration adjustment may be needed."
				);
            }
            let freezes = Freezes::<T>::get(who);
            let mut prev_frozen = Zero::zero();
            let mut after_frozen = Zero::zero();
            // No way this can fail since we do not alter the existential balances.
            // TODO: Revisit this assumption.
            let res = Self::mutate_account(who, |b| {
                prev_frozen = b.frozen;
                b.frozen = Zero::zero();
                for l in locks.iter() {
                    b.frozen = b.frozen.max(l.amount);
                }
                for l in freezes.iter() {
                    b.frozen = b.frozen.max(l.amount);
                }
                after_frozen = b.frozen;
            });
            debug_assert!(res.is_ok());
            if let Ok((_, maybe_dust)) = res {
                debug_assert!(maybe_dust.is_none(), "Not altering main balance; qed");
            }

            match locks.is_empty() {
                true => Locks::<T>::remove(who),
                false => Locks::<T>::insert(who, bounded_locks),
            }

            if prev_frozen > after_frozen {
                let amount = prev_frozen.saturating_sub(after_frozen);
                Self::deposit_event(Event::Unlocked { who: who.clone(), amount });
            } else if after_frozen > prev_frozen {
                let amount = after_frozen.saturating_sub(prev_frozen);
                Self::deposit_event(Event::Locked { who: who.clone(), amount });
            }
        }
        /// Update the account entry for `who`, given the locks.
        pub(crate) fn update_freezes(
            who: &T::AccountId,
            freezes: BoundedSlice<IdAmount<T::FreezeIdentifier, T::Balance>, T::MaxFreezes>,
        ) -> DispatchResult {
            let mut prev_frozen = Zero::zero();
            let mut after_frozen = Zero::zero();
            let (_, maybe_dust) = Self::mutate_account(who, |b| {
                prev_frozen = b.frozen;
                b.frozen = Zero::zero();
                for l in Locks::<T>::get(who).iter() {
                    b.frozen = b.frozen.max(l.amount);
                }
                for l in freezes.iter() {
                    b.frozen = b.frozen.max(l.amount);
                }
                after_frozen = b.frozen;
            })?;
            debug_assert!(maybe_dust.is_none(), "Not altering main balance; qed");
            if freezes.is_empty() {
                Freezes::<T>::remove(who);
            } else {
                Freezes::<T>::insert(who, freezes);
            }
            if prev_frozen > after_frozen {
                let amount = prev_frozen.saturating_sub(after_frozen);
                Self::deposit_event(Event::Thawed { who: who.clone(), amount });
            } else if after_frozen > prev_frozen {
                let amount = after_frozen.saturating_sub(prev_frozen);
                Self::deposit_event(Event::Frozen { who: who.clone(), amount });
            }
            Ok(())
        }

        /// Verify whitelist admin
        fn verify_admin(
            origin: OriginFor<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let old_admin = WhitelistAdmin::<T>::get().unwrap_or(T::DefaultAdmin::get());
            ensure!(sender == old_admin, Error::<T>::NotAdmin);
            Ok(().into())
        }

        pub fn get_admin() -> T::AccountId {
            WhitelistAdmin::<T>::get().unwrap_or(T::DefaultAdmin::get())
        }
    }

}
