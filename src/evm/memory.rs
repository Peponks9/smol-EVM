//! EVM Memory Module
//!
//! Implements the EVM's volatile memory as specified in the Ethereum Yellow Paper (section 9.4.2).
//! Memory is organized internally as 256-bit words ([`U256`]) for compatibility with the EVM's word size.
//! This module provides methods for reading and writing both bytes and words, as well as memory expansion and
//! gas cost calculation.
//!
//! # Design
//! - Dynamic size (up to 2^20 words, or 32 MiB)
//! - Byte-addressable, but stored as 256-bit words
//! - Memory is cleared after each execution context
//!
//! # References
//! - [Ethereum Yellow Paper, Section 9.4.2]
//! - [Ethereum EVM Illustrated]

use alloy_primitives::U256;
/// The maximum number of words allowed in the EVM memory, as per the Yellow Paper.
pub const MEMORY_MAX_SIZE: usize = 1024 * 1024;

/// Errors that can occur during memory operations.
#[derive(Debug, PartialEq, Eq)]
pub enum MemoryError {
    /// Attempted to read or write beyond the current allocated memory.
    OutOfBounds,
    /// Memory expansion would exceed the maximum allowed size.
    ExpansionLimit,
    /// The provided memory address is invalid (e.g., not word-aligned for word operations).
    InvalidAddress,
}

/// The EVM memory, holding a dynamic array of 256-bit words.
///
/// # Invariants
/// - Memory is dynamically sized and grows as needed, up to `MEMORY_MAX_SIZE` words.
/// - All elements are 256-bit unsigned integers ([`U256`]).
/// - Provides byte-level and word-level access methods.
pub struct Memory {
    /// The underlying memory storage, organized as 256-bit words.
    memory: Vec<U256>,
    /// The current size of memory in bytes.
    size: usize, // Current size in bytes
}

impl Memory {
    /// Creates a new, empty EVM memory.
    pub fn new() -> Self {
        Self {
            memory: Vec::new(),
            size: 0,
        }
    }

    /// Reads a single byte from the given address in memory.
    ///
    /// # Arguments
    /// * `address` - The byte address to read from.
    ///
    /// # Errors
    /// Returns `MemoryError::OutOfBounds` if the address is beyond the current memory size.
    pub fn read_byte(&self, address: usize) -> Result<u8, MemoryError> {
        if address >= self.size {
            return Err(MemoryError::OutOfBounds);
        }
        let word_index = address / 32;
        let byte_offset = address % 32;
        if word_index >= self.memory.len() {
            return Err(MemoryError::OutOfBounds);
        }
        let word = self.memory[word_index];
        let byte = (word >> (8 * byte_offset)) & U256::from(0xff);
        Ok(byte.as_limbs()[0] as u8)
    }

    /// Writes a single byte to the given address in memory.
    ///
    /// # Arguments
    /// * `address` - The byte address to write to.
    /// * `value` - The byte value to write.
    ///
    /// # Errors
    /// Returns `MemoryError::OutOfBounds` if the address is beyond the maximum allowed memory size.
    pub fn write_byte(&mut self, address: usize, value: u8) -> Result<(), MemoryError> {
        if address >= MEMORY_MAX_SIZE {
            return Err(MemoryError::OutOfBounds);
        }
        let word_index = address / 32;
        let byte_offset = address % 32;
        if word_index >= self.memory.len() {
            self.memory.resize(word_index + 1, U256::ZERO);
        }
        let mut word = self.memory[word_index];
        // Clear the byte at the offset, then set it
        let mask = !(U256::from(0xff) << (8 * byte_offset));
        word = (word & mask) | (U256::from(value) << (8 * byte_offset));
        self.memory[word_index] = word;
        // Update size if needed
        let new_size = address + 1;
        if new_size > self.size {
            self.size = new_size;
        }
        Ok(())
    }

    /// Reads a 256-bit word from the given address in memory.
    ///
    /// # Arguments
    /// * `address` - The byte address to read from (should be word-aligned).
    ///
    /// # Errors
    /// Returns `MemoryError::OutOfBounds` if the address is beyond the current memory size.
    /// Returns `MemoryError::InvalidAddress` if the address is not word-aligned.
    pub fn read_word(&self, address: usize) -> Result<U256, MemoryError> {
        if address >= self.size {
            return Err(MemoryError::OutOfBounds);
        }
        if address % 32 != 0 {
            return Err(MemoryError::InvalidAddress);
        }
        let word_index = address / 32;
        if word_index >= self.memory.len() {
            return Err(MemoryError::OutOfBounds);
        }
        let word = self.memory[word_index];
        Ok(word)
    }

    /// Writes a 256-bit word to the given address in memory.
    ///
    /// # Arguments
    /// * `address` - The byte address to write to (should be word-aligned).
    /// * `value` - The 256-bit word to write.
    ///
    /// # Errors
    /// Returns `MemoryError::OutOfBounds` if the address is beyond the maximum allowed memory size.
    /// Returns `MemoryError::InvalidAddress` if the address is not word-aligned.
    pub fn write_word(&mut self, address: usize, value: U256) -> Result<(), MemoryError> {
        if address >= MEMORY_MAX_SIZE {
            return Err(MemoryError::OutOfBounds);
        }
        if address % 32 != 0 {
            return Err(MemoryError::InvalidAddress);
        }
        let word_index = address / 32;
        if word_index >= self.memory.len() {
            self.memory.resize(word_index + 1, U256::ZERO);
        }
        self.memory[word_index] = value;
        // Update size if needed
        let new_size = address + 32;
        if new_size > self.size {
            self.size = new_size;
        }
        Ok(())
    }

    /// Expands the memory to at least `new_size` bytes, zero-initializing new memory.
    ///
    /// # Arguments
    /// * `new_size` - The new desired memory size in bytes.
    ///
    /// # Errors
    /// Returns `MemoryError::ExpansionLimit` if the new size exceeds `MEMORY_MAX_SIZE`.
    pub fn expand(&mut self, new_size: usize) -> Result<(), MemoryError> {
        if new_size > MEMORY_MAX_SIZE {
            return Err(MemoryError::ExpansionLimit);
        }
        let new_word_len = (new_size + 31) / 32;
        if new_word_len > self.memory.len() {
            self.memory.resize(new_word_len, U256::ZERO);
        }
        if new_size > self.size {
            self.size = new_size;
        }
        Ok(())
    }

    /// Returns the current size of memory in bytes.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Calculates the gas cost for the current memory size, as per the Yellow Paper.
    ///
    /// # Formula
    /// Per Yellow Paper: C_mem(a) = G_memory * a + (a^2) / 512, where a = ceil(size/32)
    /// G_memory is typically 3
    pub fn gas_cost(&self) -> u64 {
        let g_memory: u64 = 3;
        let a = ((self.size + 31) / 32) as u64;
        g_memory * a + (a * a) / 512
    }
}

/// Tests for the EVM Memory module.
///
/// These tests verify the correctness of memory operations according to the Ethereum Yellow Paper.
/// Tests are organized into logical groups to ensure comprehensive coverage of all functionality.
#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::U256;

    /// Tests for basic memory operations including initialization, byte-level, and word-level access.
    mod basic_operations {
        use super::*;

        /// Verifies that a newly created memory instance is empty with zero size.
        #[test]
        fn test_new_memory_is_empty() {
            let memory = Memory::new();
            assert_eq!(memory.size(), 0);
        }

        /// Tests byte-level read and write operations at address 0.
        /// Verifies that written bytes can be read back correctly.
        #[test]
        fn test_write_and_read_byte() {
            let mut memory = Memory::new();
            memory.write_byte(0, 0x42).unwrap();
            assert_eq!(memory.read_byte(0).unwrap(), 0x42);
        }

        /// Tests word-level read and write operations at word-aligned address 0.
        /// Verifies that 256-bit words can be written and read back correctly.
        #[test]
        fn test_write_and_read_word() {
            let mut memory = Memory::new();
            let value = U256::from(0x12345678);
            memory.write_word(0, value).unwrap();
            assert_eq!(memory.read_word(0).unwrap(), value);
        }
    }

    /// Tests for edge cases and error conditions to ensure robust error handling.
    mod edge_cases {
        use super::*;

        /// Verifies that reading from an unallocated memory address returns OutOfBounds error.
        #[test]
        fn test_out_of_bounds_read() {
            let memory = Memory::new();
            assert_eq!(memory.read_byte(100), Err(MemoryError::OutOfBounds));
        }

        /// Verifies that writing a word to a non-word-aligned address returns InvalidAddress error.
        #[test]
        fn test_invalid_word_address() {
            let mut memory = Memory::new();
            assert_eq!(
                memory.write_word(1, U256::ZERO),
                Err(MemoryError::InvalidAddress)
            );
        }
    }

    /// Tests for gas cost calculation according to the Ethereum Yellow Paper formula.
    mod gas_calculation {
        use super::*;

        /// Verifies that empty memory has zero gas cost.
        #[test]
        fn test_gas_for_empty_memory() {
            let memory = Memory::new();
            assert_eq!(memory.gas_cost(), 0);
        }

        /// Verifies that gas cost increases as memory size grows.
        /// Tests the Yellow Paper formula: C_mem(a) = G_memory * a + (a^2) / 512
        #[test]
        fn test_gas_cost_increases_with_size() {
            let mut memory = Memory::new();
            let initial_cost = memory.gas_cost();

            // Write to first word boundary
            memory.write_byte(32, 0x42).unwrap();
            let new_cost = memory.gas_cost();
            assert!(new_cost > initial_cost);

            // Write to second word boundary
            memory.write_byte(64, 0x42).unwrap();
            let new_cost = memory.gas_cost();
            assert!(new_cost > initial_cost);
        }
    }
}
