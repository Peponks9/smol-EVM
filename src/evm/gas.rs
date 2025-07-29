//! EVM Gas Metering System
//!
//! Implements the EVM's gas accounting system as specified in the Ethereum Yellow Paper.
//! This module provides gas tracking, consumption, and refund mechanisms.
//! It coordinates with other EVM components (memory, stack, opcodes) to ensure
//! accurate gas accounting throughout execution.

use super::memory::Memory;
use crate::types::U256;

/// Gas-related errors that can occur during EVM execution.
#[derive(Debug, PartialEq, Eq)]
pub enum GasError {
    /// Attempted to consume more gas than available.
    OutOfGas,
    /// Gas limit exceeded during execution
    GasLimitExceeded,
    /// Ivalid gas amount
    InvalidGasAmount,
}

/// The EVM gas meter, responsible for tracking gas consumption and limits.
///
/// # Design Principles
/// - Gas accounting for all EVM operations
/// - Integration with memory, stack, and opcode gas costs
/// - Support for gas refunds (e.g., storage clearing)
///
/// # Gas Cost Categories
/// - Base costs: Fixed costs per opcode
/// - Memory costs: Dynamic costs based on memory usage
/// - Stack costs: Minimal costs for stack operations
/// - Storage costs: Costs for storage operations (future)
/// - Computation costs: Costs for complex operations
pub struct GasMeter {
    /// Total gas consumed so far.
    gas_used: u64,
    /// Maximum gas allowed for this execution.
    gas_limit: u64,
    /// Gas refunds (e.g., from storage clearing).
    gas_refund: u64,
    /// Memory gas cost tracking.
    memory_gas_cost: u64,
    /// Previous memory size for expansion cost calculation.
    previous_memory_size: usize,
}

impl GasMeter {
    pub fn new(gas_limit: u64) -> Self {
        Self {
            gas_used: 0,
            gas_limit,
            gas_refund: 0,
            memory_gas_cost: 0,
            previous_memory_size: 0,
        }
    }

    /// Consumes the specified amount of gas.
    ///
    /// # Arguments
    /// * `amount` - The amount of gas to consume.
    ///
    /// # Errors
    /// Returns `GasError::OutOfGas` if insufficient gas is available.
    /// Returns `GasError::GasLimitExceeded` if the gas limit would be exceeded.
    pub fn consume_gas(&mut self, amount: u64) -> Result<(), GasError> {
        if self.gas_used + amount > self.gas_limit {
            return Err(GasError::GasLimitExceeded);
        }
        self.gas_used += amount;
        Ok(())
    }

    /// Refunds gas (e.g., from storage clearing).
    ///
    /// # Arguments
    /// * `amount` - The amount of gas to refund.
    pub fn refund_gas(&mut self, amount: u64) -> Result<(), GasError> {
        self.gas_refund = self.gas_refund.saturating_add(amount);
        Ok(())
    }

    /// Returns the remaining gas available for execution.
    pub fn remaining_gas(&self) -> u64 {
        self.gas_limit.saturating_sub(self.gas_used)
    }

    /// Returns the total gas consumed so far.
    pub fn total_gas_used(&self) -> u64 {
        self.gas_used
    }

    /// Returns the effective gas used (gas_used - gas_refund).
    pub fn effective_gas_used(&self) -> u64 {
        self.gas_used.saturating_sub(self.gas_refund)
    }
}
