#![deny(unused_crate_dependencies)]
extern crate alloc;
pub use crate::data::{EvmData, EvmDataReader};
use fp_evm::ExitError;
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    sp_runtime,
    traits::Get,
};
use frame_support::weights::Weight;
pub use precompile_utils_macro::generate_function_selector;
pub mod data;
use pallet_evm::{GasWeightMapping, PrecompileFailure};
use sp_std::marker::PhantomData;
use crate::sp_runtime::{traits::Dispatchable, ModuleError};

pub type EvmResult<T = ()> = Result<T, PrecompileFailure>;
/// Return an error with provided (static) text.
pub fn error<T: Into<alloc::borrow::Cow<'static, str>>>(text: T) -> PrecompileFailure {
    PrecompileFailure::Error {exit_status: ExitError::Other(text.into())}
}

/// Represents modifiers a Solidity function can be annotated with.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum FunctionModifier {
    /// Function that doesn't modify the state.
    View,
    /// Function that modifies the state but refuse receiving funds.
    /// Correspond to a Solidity function with no modifiers.
    NonPayable,
    /// Function that modifies the state and accept funds.
    Payable,
}

/// Helper functions requiring a Runtime.
/// This runtime must of course implement `pallet_evm::Config`.
#[derive(Clone, Copy, Debug)]
pub struct RuntimeHelper<Runtime>(PhantomData<Runtime>);

impl<Runtime> RuntimeHelper<Runtime>
where
    Runtime: pallet_evm::Config,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
{
    /// Try to dispatch a Substrate call.
    /// Return an error if there are not enough gas, or if the call fails.
    /// If successful returns the used gas using the Runtime GasWeightMapping.
    pub fn try_dispatch<Call>(
        origin: <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin,
        call: Call,
        target_gas: Option<u64>,
    ) -> EvmResult<u64>
    where
        Runtime::RuntimeCall: From<Call>,
    {
        use sp_runtime::DispatchError;
        let call = Runtime::RuntimeCall::from(call);
        let dispatch_info = call.get_dispatch_info();

        // Make sure there is enough gas.
        if let Some(gas_limit) = target_gas {
            let required_gas = Runtime::GasWeightMapping::weight_to_gas(dispatch_info.weight);
            if required_gas > gas_limit {
                return Err(PrecompileFailure::Error {exit_status: ExitError::OutOfGas});
            }
        }

        // Dispatch call.
        // It may be possible to not record gas cost if the call returns Pays::No.
        // However while Substrate handle checking weight while not making the sender pay for it,
        // the EVM doesn't. It seems this safer to always record the costs to avoid unmetered
        // computations.
        let used_weight = call
            .dispatch(origin)
            .map_err(|e| {
                let err = match e.error {
                    DispatchError::Module(
                        ModuleError {
                            index,
                            error,
                            message,
                        }
                    ) => alloc::format!(
                        "index: {:?} \n error: {:?}, msg: {:?}",
                        index,
                        error,
                        message
                    ),
                    DispatchError::Other(msg) => msg.to_string(),
                    DispatchError::BadOrigin => "BadOrigin".to_string(),
                    DispatchError::CannotLookup => "CannotLookup".to_string(),
                    DispatchError::ConsumerRemaining => "ConsumerRemaining".to_string(),
                    DispatchError::NoProviders => "NoProviders".to_string(),
                    DispatchError::TooManyConsumers => "TooManyConsumers".to_string(),
                    DispatchError::Token(e) => Into::<&str>::into(e).to_string(),
                    DispatchError::Arithmetic(e) => Into::<&str>::into(e).to_string(),
                    DispatchError::Transactional(e) => Into::<&str>::into(e).to_string(),
                    DispatchError::Exhausted => "Exhausted".to_string(),
                    DispatchError::Corruption => "Corruption".to_string(),
                    DispatchError::Unavailable => "Unavailable".to_string(),
                    DispatchError::RootNotAllowed => "RootNotAllowed".to_string(),
                };
                error(alloc::format!(
                    "Dispatched call failed with error: {:?}",
                    err
                ))
            })?
            .actual_weight;

        // Return used weight by converting weight to gas.
        Ok(Runtime::GasWeightMapping::weight_to_gas(
            used_weight.unwrap_or(dispatch_info.weight),
        ))
    }

    /// Cost of a Substrate DB write in gas.
    pub fn db_write_gas_cost() -> u64 {
        <Runtime as pallet_evm::Config>::GasWeightMapping::weight_to_gas(
            Weight::from_all(<Runtime as frame_system::Config>::DbWeight::get().write),
        )
    }

    /// Cost of a Substrate DB read in gas.
    pub fn db_read_gas_cost() -> u64 {
        <Runtime as pallet_evm::Config>::GasWeightMapping::weight_to_gas(
            Weight::from_all(<Runtime as frame_system::Config>::DbWeight::get().read),
        )
    }
}

/// Custom Gasometer to record costs in precompiles.
/// It is advised to record known costs as early as possible to
/// avoid unecessary computations if there is an Out of Gas.
#[derive(Clone, Copy, Debug)]
pub struct Gasometer {
    target_gas: Option<u64>,
    used_gas: u64,
}

impl Gasometer {
    /// Create a new Gasometer with provided gas limit.
    /// None is no limit.
    pub fn new(target_gas: Option<u64>) -> Self {
        Self {
            target_gas,
            used_gas: 0,
        }
    }

    /// Get used gas.
    pub fn used_gas(&self) -> u64 {
        self.used_gas
    }

    /// Record cost, and return error if it goes out of gas.
    pub fn record_cost(&mut self, cost: u64) -> EvmResult {
        self.used_gas = self.used_gas.checked_add(cost).ok_or(PrecompileFailure::Error {exit_status: ExitError::OutOfGas})?;

        match self.target_gas {
            Some(gas_limit) if self.used_gas > gas_limit => Err(PrecompileFailure::Error {exit_status: ExitError::OutOfGas}),
            _ => Ok(()),
        }
    }

    /// Record cost of a log manualy.
    /// This can be useful to record log costs early when their content have static size.
    pub fn record_log_costs_manual(&mut self, topics: usize, data_len: usize) -> EvmResult {
        // Cost calculation is copied from EVM code that is not publicly exposed by the crates.
        // https://github.com/rust-blockchain/evm/blob/master/gasometer/src/costs.rs#L148

        const G_LOG: u64 = 375;
        const G_LOGDATA: u64 = 8;
        const G_LOGTOPIC: u64 = 375;

        let topic_cost = G_LOGTOPIC
            .checked_mul(topics as u64)
            .ok_or(PrecompileFailure::Error {exit_status: ExitError::OutOfGas})?;

        let data_cost = G_LOGDATA
            .checked_mul(data_len as u64)
            .ok_or(PrecompileFailure::Error {exit_status: ExitError::OutOfGas})?;

        self.record_cost(G_LOG)?;
        self.record_cost(topic_cost)?;
        self.record_cost(data_cost)?;

        Ok(())
    }

    /// Record cost of logs.
    pub fn record_log_costs(&mut self, logs: &[ethereum::Log]) -> EvmResult {
        for log in logs {
            self.record_log_costs_manual(log.topics.len(), log.data.len())?;
        }

        Ok(())
    }

    /// Compute remaining gas.
    /// Returns error if out of gas.
    /// Returns None if no gas limit.
    pub fn remaining_gas(&self) -> EvmResult<Option<u64>> {
        Ok(match self.target_gas {
            None => None,
            Some(gas_limit) => Some(
                gas_limit
                    .checked_sub(self.used_gas)
                    .ok_or(PrecompileFailure::Error {exit_status: ExitError::OutOfGas})?,
            ),
        })
    }
}
