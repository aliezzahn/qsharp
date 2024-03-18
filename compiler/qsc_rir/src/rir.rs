// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use qsc_data_structures::index_map::IndexMap;

/// The root of the RIR.
pub struct Program {
    pub blocks: IndexMap<BlockId, Block>,
    pub callables: IndexMap<CallableId, Callable>,
    pub entry: CallableId,
}

/// A unique identifier for a callable in a RIR program.
#[derive(Clone, Copy, Debug, Default)]
pub struct CallableId(u32);

/// A callable.
#[derive(Clone, Debug)]
pub struct Callable {
    /// The ID of the callable.
    pub id: CallableId,
    /// The name of the callable.
    pub name: String,
    /// The input type of the callable.
    pub input_type: Vec<Ty>,
    /// The output type of the callable.
    pub output_type: Option<Ty>,
    /// The callable body.
    /// N.B. `None` bodys represent an intrinsic.
    pub body: Option<Block>,
}

#[derive(Clone, Debug)]
pub struct Block(pub Vec<Stmt>);

#[derive(Clone, Debug)]
pub enum Stmt {
    Binding(Variable, Instruction),
    Instruction(Instruction),
}

#[derive(Clone, Debug)]
pub enum Instruction {
    Store(Variable, Value),
    Call(CallableId, Vec<Value>),
    Jump(Block),
    Branch(Value, Block, Block),
    Add(Value, Value),
    Sub(Value, Value),
    Mul(Value, Value),
    Div(Value, Value),
    LogicalNot(Value),
    LogicalAnd(Value, Value),
    LogicalOr(Value, Value),
    BitwiseNot(Value),
    BitwiseAnd(Value, Value),
    BitwiseOr(Value, Value),
    BitwiseXor(Value, Value),
}

#[derive(Clone, Debug)]
pub struct VariableId(u32);

#[derive(Clone, Debug)]
pub struct Variable {
    pub variable_id: VariableId,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub enum Ty {
    Qubit,
    Result,
    Boolean,
    Integer,
    Double,
}

#[derive(Clone, Debug)]
pub enum Value {
    Literal(Literal),
    Variable(Variable),
}

#[derive(Clone, Debug)]
pub enum Literal {
    Qubit(u32),
    Result(u32),
    Bool(bool),
    Integer(i64),
    Double(f64),
}
