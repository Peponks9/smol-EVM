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
        let byte = (word >> (8 * byte_offset)) & 0xff;
        Ok(byte as u8)
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
            self.memory.resize(word_index + 1, U256::zero());
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
            self.memory.resize(word_index + 1, U256::zero());
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
            self.memory.resize(new_word_len, U256::zero());
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
