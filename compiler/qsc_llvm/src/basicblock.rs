// Portions copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use crate::instruction::Instruction;
use crate::name::Name;
use crate::terminator::Terminator;

/// A `BasicBlock` is a sequence of zero or more non-terminator instructions
/// followed by a single terminator instruction which ends the block.
/// Basic blocks are discussed in the [LLVM 14 docs on Functions](https://releases.llvm.org/14.0.0/docs/LangRef.html#functionstructure)
#[derive(PartialEq, Clone, Debug)]
pub struct BasicBlock {
    pub name: Name,
    pub instrs: Vec<Instruction>,
    pub term: Terminator,
}

impl BasicBlock {
    /// A `BasicBlock` instance with no instructions and an `Unreachable` terminator
    #[must_use]
    pub fn new(name: Name) -> Self {
        use crate::terminator::Unreachable;
        Self {
            name,
            instrs: vec![],
            term: Terminator::Unreachable(Unreachable { debugloc: None }),
        }
    }
}