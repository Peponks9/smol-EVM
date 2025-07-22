//! Common EVM types, re-exported from Alloy primitives.
//! All EVM modules should import types from here, not directly from Alloy.

// Re-export core Alloy types for use throughout the EVM.
pub use alloy_primitives::{Address, Bytes, B256, U256};

// Type alias for the EVM "word" (256 bits).
pub type Word = U256;

// Type alias for storage keys (EVM storage is a mapping from 256-bit keys to 256-bit values).
pub type StorageKey = B256;
pub type StorageValue = U256;

// Optionally, define other common types or enums here as your EVM grows.
// For example, you might add an ExecutionResult, Error types, or enums for opcode categories.

// ---
// Note: If you need to add custom serialization, trait impls, or wrappers, do so here
// to keep all EVM-wide type logic in one place.
