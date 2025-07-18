use alloy_primitives::U256;

const STACK_MAX_SIZE: usize = 1024;

#[derive(Debug)]
pub enum StackError {
    Overflow,
    Underflow,
}

pub struct Stack {
    stack: Vec<U256>,
}

impl Stack {
    pub fn new() -> Self {
        Stack {
            stack: Vec::with_capacity(STACK_MAX_SIZE),
        }
    }

    pub fn push(&mut self, value: U256) -> Result<(), StackError> {
        if self.stack.len() >= STACK_MAX_SIZE {
            Err(StackError::Overflow)
        } else {
            self.stack.push(value);
            Ok(())
        }
    }

    pub fn pop(&mut self) -> Result<U256, StackError> {
        self.stack.pop().ok_or(StackError::Underflow)
    }

    pub fn peek(&self) -> Option<&U256> {
        self.stack.last()
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.stack.len() == STACK_MAX_SIZE
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }
}
