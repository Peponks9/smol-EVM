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

/// Parameters for calculating dynamic gas costs.
/// Contains the contextual information needed for operations with variable costs.
#[derive(Debug, Clone)]
pub struct DynamicGasParams {
    /// Size of data being processed (bytes)
    pub size: usize,
    /// Exponent value for EXP operations
    pub exponent: U256,
    /// Current storage value for SSTORE
    pub current_value: U256,
    /// Original storage value for SSTORE
    pub original_value: U256,
    /// New storage value for SSTORE
    pub new_value: U256,
    /// Value being transferred in calls
    pub value: U256,
    /// Account balance for SELFDESTRUCT
    pub balance: U256,
    /// Whether the target account is empty
    pub is_account_empty: bool,
}

impl DynamicGasParams {
    /// Creates a new `DynamicGasParams` with default values.
    pub fn new() -> Self {
        Self {
            size: 0,
            exponent: U256::ZERO,
            current_value: U256::ZERO,
            original_value: U256::ZERO,
            new_value: U256::ZERO,
            value: U256::ZERO,
            balance: U256::ZERO,
            is_account_empty: false,
        }
    }

    /// Sets the size parameter for data operations.
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Sets the exponent for EXP operations.
    pub fn with_exponent(mut self, exponent: U256) -> Self {
        self.exponent = exponent;
        self
    }

    /// Sets storage values for SSTORE operations.
    pub fn with_storage_values(mut self, current: U256, original: U256, new: U256) -> Self {
        self.current_value = current;
        self.original_value = original;
        self.new_value = new;
        self
    }

    /// Sets call parameters.
    pub fn with_call_params(mut self, value: U256, is_account_empty: bool) -> Self {
        self.value = value;
        self.is_account_empty = is_account_empty;
        self
    }

    /// Sets balance for SELFDESTRUCT operations.
    pub fn with_balance(mut self, balance: U256) -> Self {
        self.balance = balance;
        self
    }
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
    pub fn dynamic_gas_cost(&self, opcode: Opcode, params: &DynamicGasParams) -> u64 {
        match opcode {
            // Data copying operations
            Opcode::Calldatacopy | Opcode::Codecopy | Opcode::Returndatacopy => {
                // 3 gas per word copied
                let words = (params.size + 31) / 32;
                3 * words as u64
            }

            // External code operations
            Opcode::Extcodecopy => {
                // Base cost (2600) + copying cost
                let words = (params.size + 31) / 32;
                3 * words as u64
            }

            // Memory copy operation
            Opcode::Mcopy => {
                // 3 gas per word copied
                let words = (params.size + 31) / 32;
                3 * words as u64
            }

            // Cryptographic operations
            Opcode::Keccak256 => {
                // 6 gas per word hashed
                let words = (params.size + 31) / 32;
                6 * words as u64
            }

            // Exponentiation
            Opcode::Exp => {
                // Additional cost based on exponent byte length
                if params.exponent.is_zero() {
                    0
                } else {
                    let byte_length = (params.exponent.bit_len() + 7) / 8;
                    50 * byte_length as u64
                }
            }

            // Logging operations
            Opcode::Log0 | Opcode::Log1 | Opcode::Log2 | Opcode::Log3 | Opcode::Log4 => {
                // 8 gas per byte logged
                8 * params.size as u64
            }

            // Storage operations
            Opcode::Sstore => {
                // Complex storage cost calculation based on current/original values
                self.calculate_sstore_cost(
                    params.current_value,
                    params.original_value,
                    params.new_value,
                )
            }

            // Contract creation
            Opcode::Create | Opcode::Create2 => {
                // 2 gas per byte of init code
                let init_code_cost = 2 * params.size as u64;

                // CREATE2 has additional cost for address calculation
                if opcode == Opcode::Create2 {
                    let hash_cost = 6 * ((params.size + 31) / 32) as u64;
                    init_code_cost + hash_cost
                } else {
                    init_code_cost
                }
            }

            // Call operations
            Opcode::Call | Opcode::Callcode | Opcode::Delegatecall | Opcode::Staticcall => {
                let mut cost = 0u64;

                // Value transfer cost
                if opcode == Opcode::Call && !params.value.is_zero() {
                    cost += 9000;

                    // New account creation cost
                    if params.is_account_empty {
                        cost += 25000;
                    }
                }

                // Memory expansion cost for call data and return data
                if params.size > 0 {
                    let words = (params.size + 31) / 32;
                    cost += words as u64;
                }

                cost
            }

            // Self-destruct
            Opcode::Selfdestruct => {
                let mut cost = 0u64;

                // Transfer to new account
                if params.is_account_empty && !params.balance.is_zero() {
                    cost += 25000;
                }

                cost
            }

            // Operations without dynamic costs
            _ => 0,
        }
    }

    /// Calculates the gas cost for SSTORE operations based on EIP-2200.
    /// This implements the complex gas pricing for storage operations.
    fn calculate_sstore_cost(
        &self,
        current_value: U256,
        original_value: U256,
        new_value: U256,
    ) -> u64 {
        // Gas costs as per EIP-2200
        const SLOAD_GAS: u64 = 800;
        const SSTORE_SET_GAS: u64 = 20000;
        const SSTORE_RESET_GAS: u64 = 5000;
        const _SSTORE_CLEAR_REFUND: u64 = 15000;

        if new_value == current_value {
            // No change
            SLOAD_GAS
        } else if original_value == current_value {
            // First change in transaction
            if original_value.is_zero() {
                // Setting from zero
                SSTORE_SET_GAS
            } else {
                // Modifying existing value
                SSTORE_RESET_GAS
            }
        } else {
            // Subsequent change in transaction
            SLOAD_GAS
        }
    }

    /// Resets the gas meter for a new execution context.
    pub fn reset(&mut self, gas_limit: u64) {
        self.gas_used = 0;
        self.gas_limit = gas_limit;
        self.gas_refund = 0;
        self.memory_gas_cost = 0;
        self.previous_memory_size = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::U256;

    #[test]
    fn test_dynamic_gas_cost_data_copy() {
        let gas_meter = GasMeter::new(1000000);

        // Test CALLDATACOPY with 64 bytes (2 words)
        let params = DynamicGasParams::new().with_size(64);
        let cost = gas_meter.dynamic_gas_cost(Opcode::Calldatacopy, &params);
        assert_eq!(cost, 6); // 3 gas per word * 2 words

        // Test with partial word
        let params = DynamicGasParams::new().with_size(33);
        let cost = gas_meter.dynamic_gas_cost(Opcode::Calldatacopy, &params);
        assert_eq!(cost, 6); // Still 2 words (33 bytes rounds up)
    }

    #[test]
    fn test_dynamic_gas_cost_keccak256() {
        let gas_meter = GasMeter::new(1000000);

        // Test KECCAK256 with 32 bytes (1 word)
        let params = DynamicGasParams::new().with_size(32);
        let cost = gas_meter.dynamic_gas_cost(Opcode::Keccak256, &params);
        assert_eq!(cost, 6); // 6 gas per word

        // Test with larger data
        let params = DynamicGasParams::new().with_size(128);
        let cost = gas_meter.dynamic_gas_cost(Opcode::Keccak256, &params);
        assert_eq!(cost, 24); // 6 gas per word * 4 words
    }

    #[test]
    fn test_dynamic_gas_cost_exp() {
        let gas_meter = GasMeter::new(1000000);

        // Test EXP with zero exponent
        let params = DynamicGasParams::new().with_exponent(U256::ZERO);
        let cost = gas_meter.dynamic_gas_cost(Opcode::Exp, &params);
        assert_eq!(cost, 0);

        // Test EXP with small exponent (1 byte)
        let params = DynamicGasParams::new().with_exponent(U256::from(255));
        let cost = gas_meter.dynamic_gas_cost(Opcode::Exp, &params);
        assert_eq!(cost, 50); // 50 gas per byte * 1 byte

        // Test EXP with larger exponent (2 bytes)
        let params = DynamicGasParams::new().with_exponent(U256::from(256));
        let cost = gas_meter.dynamic_gas_cost(Opcode::Exp, &params);
        assert_eq!(cost, 100); // 50 gas per byte * 2 bytes
    }

    #[test]
    fn test_dynamic_gas_cost_logging() {
        let gas_meter = GasMeter::new(1000000);

        // Test LOG0 with data
        let params = DynamicGasParams::new().with_size(100);
        let cost = gas_meter.dynamic_gas_cost(Opcode::Log0, &params);
        assert_eq!(cost, 800); // 8 gas per byte * 100 bytes

        // Test LOG2 with same data (base cost is different but dynamic cost is same)
        let cost = gas_meter.dynamic_gas_cost(Opcode::Log2, &params);
        assert_eq!(cost, 800); // 8 gas per byte * 100 bytes
    }

    #[test]
    fn test_sstore_gas_calculation() {
        let gas_meter = GasMeter::new(1000000);

        // Setting a new value (from zero)
        let cost = gas_meter.calculate_sstore_cost(
            U256::ZERO,     // current
            U256::ZERO,     // original
            U256::from(42), // new
        );
        assert_eq!(cost, 20000); // SSTORE_SET_GAS

        // Modifying existing value
        let cost = gas_meter.calculate_sstore_cost(
            U256::from(42), // current
            U256::from(42), // original (same as current)
            U256::from(24), // new
        );
        assert_eq!(cost, 5000); // SSTORE_RESET_GAS

        // No change
        let cost = gas_meter.calculate_sstore_cost(
            U256::from(42), // current
            U256::from(42), // original
            U256::from(42), // new (same as current)
        );
        assert_eq!(cost, 800); // SLOAD_GAS
    }

    #[test]
    fn test_dynamic_gas_params_builder() {
        let params = DynamicGasParams::new()
            .with_size(64)
            .with_exponent(U256::from(256))
            .with_storage_values(U256::ZERO, U256::ZERO, U256::from(42))
            .with_call_params(U256::from(1000), true)
            .with_balance(U256::from(5000));

        assert_eq!(params.size, 64);
        assert_eq!(params.exponent, U256::from(256));
        assert_eq!(params.new_value, U256::from(42));
        assert_eq!(params.value, U256::from(1000));
        assert_eq!(params.balance, U256::from(5000));
        assert!(params.is_account_empty);
    }
}
