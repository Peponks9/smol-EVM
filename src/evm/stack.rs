//! Stack module for the EVM.
//!
//! This module implements the EVM stack as specified in the Ethereum Yellow Paper (section 9.4.2).
//! The stack holds up to 1024 256-bit words, using [`U256`] from `alloy-primitives` for compatibility
//! with Ethereum's word size. Provides push, pop, peek, and utility methods with proper error handling.
//!
//! # Design
//! - Fixed maximum size (1024 elements)
//! - Each element is a 256-bit unsigned integer ([`U256`])
//! - Overflow and underflow are handled via custom error types

use alloy_primitives::U256;

/// The maximum number of elements allowed on the EVM stack, as per the Yellow Paper.
pub const STACK_MAX_SIZE: usize = 1024;

/// Errors that can occur during stack operations.
#[derive(Debug, PartialEq, Eq)]
pub enum StackError {
    /// Attempted to push onto a full stack.
    Overflow,
    /// Attempted to pop from an empty stack.
    Underflow,
}

/// The EVM stack, holding up to 1024 256-bit words.
///
/// # Invariants
/// - The stack never grows beyond 1024 elements.
/// - All elements are 256-bit unsigned integers ([`U256`]).

pub struct Stack {
    stack: Vec<U256>,
}

impl Stack {
    /// Creates a new, empty EVM stack.
    pub fn new() -> Self {
        Stack {
            stack: Vec::with_capacity(STACK_MAX_SIZE),
        }
    }

    /// Pushes a value onto the stack.
    pub fn push(&mut self, value: U256) -> Result<(), StackError> {
        if self.stack.len() >= STACK_MAX_SIZE {
            Err(StackError::Overflow)
        } else {
            self.stack.push(value);
            Ok(())
        }
    }

    /// Pops the top value off the stack and returns it.
    pub fn pop(&mut self) -> Result<U256, StackError> {
        self.stack.pop().ok_or(StackError::Underflow)
    }

    /// Returns a reference to the top value on the stack, if any.
    pub fn peek(&self) -> Option<&U256> {
        self.stack.last()
    }

    /// Returns `true` if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Returns `true` if the stack is full.
    pub fn is_full(&self) -> bool {
        self.stack.len() == STACK_MAX_SIZE
    }

    /// Returns the current number of elements in the stack.
    pub fn len(&self) -> usize {
        self.stack.len()
    }
}
