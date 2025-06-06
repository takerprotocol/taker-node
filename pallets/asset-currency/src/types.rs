use codec::{Decode, Encode, MaxEncodedLen};
use core::ops::BitOr;
use frame_support::traits::{LockIdentifier, WithdrawReasons};
use scale_info::TypeInfo;
use sp_runtime::{RuntimeDebug, Saturating};

/// All balance information for an account.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct AccountData<Balance> {
	/// Non-reserved part of the balance which the account holder may be able to control.
	///
	/// This is the only balance that matters in terms of most operations on tokens.
	pub free: Balance,
	/// Balance which is has active holds on it and may not be used at all.
	///
	/// This is the sum of all individual holds together with any sums still under the (deprecated)
	/// reserves API.
	pub reserved: Balance,
	/// The amount that `free + reserved` may not drop below when reducing the balance, except for
	/// actions where the account owner cannot reasonably benefit from the balance reduction, such
	/// as slashing.
	pub frozen: Balance,
}

impl<Balance: Saturating + Copy + Ord> AccountData<Balance> {
	pub fn usable(&self) -> Balance {
		self.free.saturating_sub(self.frozen)
	}

	/// The total balance in this account including any that is reserved and ignoring any frozen.
	pub fn total(&self) -> Balance {
		self.free.saturating_add(self.reserved)
	}
}

/// A single lock on a balance. There can be many of these on an account and they "overlap", so the
/// same balance is frozen by multiple locks.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct BalanceLock<Balance> {
	/// An identifier for this lock. Only one lock may be in existence for each identifier.
	pub id: LockIdentifier,
	/// The amount which the free balance may not drop below when this lock is in effect.
	pub amount: Balance,
	/// If true, then the lock remains in effect even for payment of transaction fees.
	pub reasons: Reasons,
}

/// Simplified reasons for withdrawing balance.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum Reasons {
	/// Paying system transaction fees.
	Fee = 0,
	/// Any reason other than paying system transaction fees.
	Misc = 1,
	/// Any reason at all.
	All = 2,
}

impl From<WithdrawReasons> for Reasons {
	fn from(r: WithdrawReasons) -> Reasons {
		if r == WithdrawReasons::TRANSACTION_PAYMENT {
			Reasons::Fee
		} else if r.contains(WithdrawReasons::TRANSACTION_PAYMENT) {
			Reasons::All
		} else {
			Reasons::Misc
		}
	}
}

impl BitOr for Reasons {
	type Output = Reasons;
	fn bitor(self, other: Reasons) -> Reasons {
		if self == other {
			return self;
		}
		Reasons::All
	}
}

/// Store named reserved balance.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ReserveData<ReserveIdentifier, Balance> {
	/// The identifier for the named reserve.
	pub id: ReserveIdentifier,
	/// The amount of the named reserve.
	pub amount: Balance,
}

/// An identifier and balance.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct IdAmount<Id, Balance> {
	/// An identifier for this item.
	pub id: Id,
	/// Some amount for this item.
	pub amount: Balance,
}
