// Copyright 2018 Kodebox, Inc.
// This file is part of CodeChain.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use ckeys::{verify_ecdsa, ECDSASignature};
use ctypes::{H256, H520, Public};

use instruction::Instruction;

const DEFAULT_MAX_MEMORY: usize = 1024;

pub struct Config {
    pub max_memory: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_memory: DEFAULT_MAX_MEMORY,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ScriptResult {
    Fail,
    Unlocked,
    Burnt,
}

#[derive(Debug, PartialEq)]
pub enum RuntimeError {
    OutOfMemory,
    IndexOutOfBound,
    StackUnderflow,
    TypeMismatch,
    InvalidResult,
}

#[derive(Clone)]
struct Item(Vec<u8>);

impl Item {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn assert_len(self, len: usize) -> Result<Self, RuntimeError> {
        if self.len() == len {
            Ok(self)
        } else {
            Err(RuntimeError::TypeMismatch)
        }
    }
}

impl AsRef<[u8]> for Item {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<bool> for Item {
    fn from(val: bool) -> Item {
        if val {
            Item(vec![1])
        } else {
            Item(vec![])
        }
    }
}

impl Into<bool> for Item {
    fn into(self) -> bool {
        self.as_ref().iter().find(|b| **b != 0).is_some()
    }
}

struct Stack {
    stack: Vec<Item>,
    memory_usage: usize,
    config: Config,
}

impl Stack {
    fn new(config: Config) -> Self {
        Self {
            stack: Vec::new(),
            memory_usage: 0,
            config,
        }
    }

    /// Returns true if value is successfully pushed
    fn push(&mut self, val: Item) -> Result<(), RuntimeError> {
        if self.memory_usage + val.len() > self.config.max_memory {
            Err(RuntimeError::OutOfMemory)
        } else {
            self.memory_usage += val.len();
            self.stack.push(val);
            Ok(())
        }
    }

    fn pop(&mut self) -> Result<Item, RuntimeError> {
        let item = self.stack.pop();
        self.memory_usage -= item.as_ref().map_or(0, |i| i.len());
        item.ok_or(RuntimeError::StackUnderflow)
    }

    fn len(&self) -> usize {
        self.stack.len()
    }

    fn get(&self, index: usize) -> Result<Item, RuntimeError> {
        self.stack.get(index).cloned().ok_or(RuntimeError::IndexOutOfBound)
    }

    fn remove(&mut self, index: usize) -> Result<Item, RuntimeError> {
        if index < self.stack.len() {
            let item = self.stack.remove(index);
            self.memory_usage -= item.len();
            Ok(item)
        } else {
            Err(RuntimeError::IndexOutOfBound)
        }
    }
}

pub fn execute(
    unlock: &[Instruction],
    params: &[Vec<u8>],
    lock: &[Instruction],
    tx_hash: H256,
    config: Config,
) -> Result<ScriptResult, RuntimeError> {
    // FIXME: don't merge scripts
    let param_scripts: Vec<_> = params.iter().map(|p| Instruction::PushB(p.clone())).collect();
    let script = [unlock, &param_scripts, lock].concat();

    let mut stack = Stack::new(config);
    let mut pc = 0;
    while pc < script.len() {
        match &script[pc] {
            Instruction::Nop => {}
            Instruction::Not => {
                let value: bool = stack.pop()?.into();
                stack.push(Item::from(!value))?;
            }
            Instruction::Eq => {
                let first = stack.pop()?;
                let second = stack.pop()?;
                stack.push(Item::from(first.as_ref() == second.as_ref()))?;
            }
            Instruction::Jmp(val) => {
                pc += *val as usize;
            }
            Instruction::Jnz(val) => {
                if stack.pop()?.into() {
                    pc += *val as usize;
                }
            }
            Instruction::Jz(val) => {
                let condition: bool = stack.pop()?.into();
                if !condition {
                    pc += *val as usize;
                }
            }
            Instruction::Push(val) => stack.push(Item(vec![*val]))?,
            Instruction::Pop => {
                stack.pop()?;
            }
            Instruction::PushB(blob) => stack.push(Item(blob.clone()))?,
            Instruction::Dup => {
                let top = stack.pop()?;
                stack.push(top.clone())?;
                stack.push(top)?;
            }
            Instruction::Swap => {
                let first = stack.pop()?;
                let second = stack.pop()?;
                stack.push(first)?;
                stack.push(second)?;
            }
            Instruction::Copy(index) => {
                let item = stack.get(*index as usize)?;
                stack.push(item)?
            }
            Instruction::Drop(index) => {
                stack.remove(*index as usize)?;
            }
            Instruction::ChkSig => {
                let pubkey = Public::from_slice(stack.pop()?.assert_len(64)?.as_ref());
                let signature = ECDSASignature::from(H520::from(stack.pop()?.assert_len(65)?.as_ref()));
                let result = match verify_ecdsa(&pubkey, &signature, &tx_hash) {
                    Ok(true) => 1,
                    _ => 0,
                };
                stack.push(Item(vec![result]))?;
            }
            Instruction::Blake256 => unimplemented!(),
            Instruction::Sha256 => unimplemented!(),
            Instruction::Ripemd160 => unimplemented!(),
        }
        pc += 1;
    }

    let result = stack.pop()?;
    // FIXME: convert stack top value to integer value
    if result.as_ref() != [0] && stack.len() == 0 {
        Ok(ScriptResult::Unlocked)
    } else {
        Ok(ScriptResult::Fail)
    }
}
