//! EVM Gas Metering System
//!
//! Implements the EVM's gas accounting system as specified in the Ethereum Yellow Paper.
//! This module provides gas tracking, consumption, and refund mechanisms.
//! It coordinates with other EVM components (memory, stack, opcodes) to ensure
//! accurate gas accounting throughout execution.

use super::memory::Memory;
use super::opcodes::Opcode;
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

    /// Updates memory gas cost based on current memory state.
    pub fn update_memory_cost(&mut self, memory: &Memory) -> Result<(), GasError> {
        let current_memory_size = memory.size();
        let expansion_cost =
            self.memory_expansion_cost(self.previous_memory_size, current_memory_size);

        if expansion_cost > 0 {
            self.consume_gas(expansion_cost)?;
            self.memory_gas_cost += expansion_cost;
        }

        self.previous_memory_size = current_memory_size;
        Ok(())
    }

    /// Calculates the gas cost for memory expansion.
    pub fn memory_expansion_cost(&self, old_size: usize, new_size: usize) -> u64 {
        if new_size <= old_size {
            return 0;
        }

        let g_memory: u64 = 3;
        let old_words = (old_size + 31) / 32;
        let new_words = (new_size + 31) / 32;

        let old_cost = g_memory * old_words as u64 + (old_words * old_words) as u64 / 512;
        let new_cost = g_memory * new_words as u64 + (new_words * new_words) as u64 / 512;

        new_cost.saturating_sub(old_cost)
    }

    /// Returns the gas cost for a specific opcode.
    pub fn opcode_cost(&self, opcode: Opcode) -> u64 {
        match opcode {
            // Stop and arithmetic operations
            Opcode::Stop => 0,
            Opcode::Add => 3,
            Opcode::Mul => 5,
            Opcode::Sub => 3,
            Opcode::Div => 5,
            Opcode::Sdiv => 5,
            Opcode::Mod => 5,
            Opcode::Smod => 5,
            Opcode::Addmod => 8,
            Opcode::Mulmod => 8,
            Opcode::Exp => 10, // Base cost, actual cost depends on exponent
            Opcode::Signextend => 5,

            // Comparison operations
            Opcode::Lt => 3,
            Opcode::Gt => 3,
            Opcode::Slt => 3,
            Opcode::Sgt => 3,
            Opcode::Eq => 3,
            Opcode::Iszero => 3,

            // Bitwise operations
            Opcode::And => 3,
            Opcode::Or => 3,
            Opcode::Xor => 3,
            Opcode::Not => 3,
            Opcode::Byte => 3,
            Opcode::Shl => 3,
            Opcode::Shr => 3,
            Opcode::Sar => 3,

            // Cryptographic operations
            Opcode::Keccak256 => 30, // Base cost, actual cost depends on data size

            // Environment information
            Opcode::Address => 2,
            Opcode::Balance => 2600, // Cold storage access cost
            Opcode::Origin => 2,
            Opcode::Caller => 2,
            Opcode::Callvalue => 2,
            Opcode::Calldataload => 3,
            Opcode::Calldatasize => 2,
            Opcode::Calldatacopy => 3, // Base cost, actual cost depends on data size
            Opcode::Codesize => 2,
            Opcode::Codecopy => 3, // Base cost, actual cost depends on data size
            Opcode::Gasprice => 2,
            Opcode::Extcodecopy => 2600, // Cold storage access cost + copy cost
            Opcode::Extcodesize => 2600, // Cold storage access cost
            Opcode::Extcodehash => 2600, // Cold storage access cost
            Opcode::Returndatasize => 2,
            Opcode::Returndatacopy => 3, // Base cost, actual cost depends on data size
            Opcode::Blockhash => 20,
            Opcode::Coinbase => 2,
            Opcode::Timestamp => 2,
            Opcode::Number => 2,
            Opcode::Difficulty => 2,
            Opcode::Gaslimit => 2,
            Opcode::Chainid => 2,
            Opcode::Selfbalance => 2,
            Opcode::Basefee => 2,
            Opcode::Blobhash => 2,
            Opcode::Blobbasefee => 2,

            // Stack operations
            Opcode::Pop => 2,
            Opcode::Mload => 3,
            Opcode::Mstore => 3,
            Opcode::Mstore8 => 3,
            Opcode::Sload => 2100,   // Cold storage access cost
            Opcode::Sstore => 22100, // Cold storage write cost (base)
            Opcode::Jump => 8,
            Opcode::Jumpi => 10,
            Opcode::Pc => 2,
            Opcode::Msize => 2,
            Opcode::Gas => 2,
            Opcode::Jumpdest => 1,
            Opcode::Tload => 100,  // Warm storage access
            Opcode::Tstore => 100, // Warm storage write
            Opcode::Mcopy => 3,

            // Push operations (all have same cost)
            Opcode::Push0 => 2,
            Opcode::Push1 => 2,
            Opcode::Push2 => 2,
            Opcode::Push3 => 2,
            Opcode::Push4 => 2,
            Opcode::Push5 => 2,
            Opcode::Push6 => 2,
            Opcode::Push7 => 2,
            Opcode::Push8 => 2,
            Opcode::Push9 => 2,
            Opcode::Push10 => 2,
            Opcode::Push11 => 2,
            Opcode::Push12 => 2,
            Opcode::Push13 => 2,
            Opcode::Push14 => 2,
            Opcode::Push15 => 2,
            Opcode::Push16 => 2,
            Opcode::Push17 => 2,
            Opcode::Push18 => 2,
            Opcode::Push19 => 2,
            Opcode::Push20 => 2,
            Opcode::Push21 => 2,
            Opcode::Push22 => 2,
            Opcode::Push23 => 2,
            Opcode::Push24 => 2,
            Opcode::Push25 => 2,
            Opcode::Push26 => 2,
            Opcode::Push27 => 2,
            Opcode::Push28 => 2,
            Opcode::Push29 => 2,
            Opcode::Push30 => 2,
            Opcode::Push31 => 2,
            Opcode::Push32 => 2,

            // Duplicate operations
            Opcode::Dup1 => 3,
            Opcode::Dup2 => 3,
            Opcode::Dup3 => 3,
            Opcode::Dup4 => 3,
            Opcode::Dup5 => 3,
            Opcode::Dup6 => 3,
            Opcode::Dup7 => 3,
            Opcode::Dup8 => 3,
            Opcode::Dup9 => 3,
            Opcode::Dup10 => 3,
            Opcode::Dup11 => 3,
            Opcode::Dup12 => 3,
            Opcode::Dup13 => 3,
            Opcode::Dup14 => 3,
            Opcode::Dup15 => 3,
            Opcode::Dup16 => 3,

            // Swap operations
            Opcode::Swap1 => 3,
            Opcode::Swap2 => 3,
            Opcode::Swap3 => 3,
            Opcode::Swap4 => 3,
            Opcode::Swap5 => 3,
            Opcode::Swap6 => 3,
            Opcode::Swap7 => 3,
            Opcode::Swap8 => 3,
            Opcode::Swap9 => 3,
            Opcode::Swap10 => 3,
            Opcode::Swap11 => 3,
            Opcode::Swap12 => 3,
            Opcode::Swap13 => 3,
            Opcode::Swap14 => 3,
            Opcode::Swap15 => 3,
            Opcode::Swap16 => 3,

            // Logging operations
            Opcode::Log0 => 375,  // Base cost, actual cost depends on data size
            Opcode::Log1 => 750,  // Base cost, actual cost depends on data size
            Opcode::Log2 => 1125, // Base cost, actual cost depends on data size
            Opcode::Log3 => 1500, // Base cost, actual cost depends on data size
            Opcode::Log4 => 1875, // Base cost, actual cost depends on data size

            // Contract creation and calls
            Opcode::Create => 32000,  // Base cost for contract creation
            Opcode::Call => 2600,     // Base cost for calls
            Opcode::Callcode => 2600, // Base cost for callcode
            Opcode::Return => 0,
            Opcode::Delegatecall => 2600, // Base cost for delegatecall
            Opcode::Create2 => 32000,     // Base cost for contract creation
            Opcode::Staticcall => 2600,   // Base cost for staticcall
            Opcode::Revert => 0,
            Opcode::Invalid => 0,
            Opcode::Selfdestruct => 5000, // Base cost for selfdestruct
        }
    }

    /// Calculates the dynamic gas cost for operations that depend on parameters.
    /// This should be called in addition to the base opcode cost.
    todo!()

    /// Resets the gas meter for a new execution context.
    pub fn reset(&mut self, gas_limit: u64) {
        self.gas_used = 0;
        self.gas_limit = gas_limit;
        self.gas_refund = 0;
        self.memory_gas_cost = 0;
        self.previous_memory_size = 0;
    }
}
