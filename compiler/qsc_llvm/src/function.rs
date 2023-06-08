// Portions copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use crate::debugloc::{DebugLoc, HasDebugLoc};
use crate::module::{Comdat, DLLStorageClass, Linkage, Visibility};
use crate::types::{TypeRef, Types};
use crate::{BasicBlock, ConstantRef, Name};
use std::fmt::{Display, Formatter, Result};

/// See [LLVM 14 docs on Functions](https://releases.llvm.org/14.0.0/docs/LangRef.html#functions)
#[derive(PartialEq, Clone, Debug)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub is_var_arg: bool,
    pub return_type: TypeRef,
    pub basic_blocks: Vec<BasicBlock>,
    pub function_attributes: Vec<Attribute>, // llvm-hs-pure has Vec<Either<GroupID, FunctionAttribute>>, but I'm not sure how the GroupID ones come about
    pub return_attributes: Vec<ParameterAttribute>,
    pub linkage: Linkage,
    pub visibility: Visibility,
    pub dll_storage_class: DLLStorageClass, // llvm-hs-pure has Option<DLLStorageClass>, but the llvm_sys api doesn't look like it can fail
    pub calling_convention: CallingConvention,
    pub section: Option<String>,
    pub comdat: Option<Comdat>, // llvm-hs-pure has Option<String>, I'm not sure why
    pub alignment: u32,
    /// See [LLVM 14 docs on Garbage Collector Strategy Names](https://releases.llvm.org/14.0.0/docs/LangRef.html#gc)
    pub garbage_collector_name: Option<String>,
    // pub prefix: Option<ConstantRef>,  // appears to not be exposed in the LLVM C API, only the C++ API
    /// Personalities are used for exception handling. See [LLVM 14 docs on Personality Function](https://releases.llvm.org/14.0.0/docs/LangRef.html#personalityfn)
    pub personality_function: Option<ConstantRef>,
    pub debugloc: Option<DebugLoc>,
}

impl HasDebugLoc for Function {
    fn get_debug_loc(&self) -> &Option<DebugLoc> {
        &self.debugloc
    }
}

impl Function {
    /// Get the `BasicBlock` having the given `Name` (if any).
    #[must_use]
    pub fn get_bb_by_name(&self, name: &Name) -> Option<&BasicBlock> {
        self.basic_blocks.iter().find(|bb| &bb.name == name)
    }

    /// A Function instance as empty as possible, using defaults
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parameters: vec![],
            is_var_arg: false,
            return_type: Types::blank_for_testing().void(),
            basic_blocks: vec![],
            function_attributes: vec![],
            return_attributes: vec![],
            linkage: Linkage::Private,
            visibility: Visibility::Default,
            dll_storage_class: DLLStorageClass::Default,
            calling_convention: CallingConvention::C,
            section: None,
            comdat: None,
            alignment: 4,
            garbage_collector_name: None,
            personality_function: None,
            debugloc: None,
        }
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "define @{}...{{", self.name)?;
        for bb in &self.basic_blocks {
            writeln!(f, "{}:", bb.name)?;
            for i in &bb.instrs {
                writeln!(f, "{i}")?;
            }
        }

        writeln!(f, "}}")
    }
}

/// See [LLVM 14 docs on Functions](https://releases.llvm.org/14.0.0/docs/LangRef.html#functions)
#[derive(PartialEq, Clone, Debug)]
pub struct Declaration {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub is_var_arg: bool,
    pub return_type: TypeRef,
    pub return_attributes: Vec<ParameterAttribute>,
    pub linkage: Linkage,
    pub visibility: Visibility,
    pub dll_storage_class: DLLStorageClass,
    pub calling_convention: CallingConvention,
    pub alignment: u32,
    /// See [LLVM 14 docs on Garbage Collector Strategy Names](https://releases.llvm.org/14.0.0/docs/LangRef.html#gc)
    pub garbage_collector_name: Option<String>,
    pub debugloc: Option<DebugLoc>,
}

impl Display for Declaration {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        /*
         define [linkage] [PreemptionSpecifier] [visibility] [DLLStorageClass]
            [cconv] [ret attrs]
            <ResultType> @<FunctionName> ([argument list])
            [(unnamed_addr|local_unnamed_addr)] [AddrSpace] [fn Attrs]
            [section "name"] [partition "name"] [comdat [($name)]] [align N]
            [gc] [prefix Constant] [prologue Constant] [personality Constant]
            (!name !N)* { ... }
        */
        writeln!(
            f,
            "define {} {} {} @{}...",
            self.linkage, self.visibility, self.return_type, self.name
        )
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct Parameter {
    pub name: Name,
    pub ty: TypeRef,
    pub attributes: Vec<ParameterAttribute>,
}

/// See [LLVM 14 docs on Calling Conventions](https://releases.llvm.org/14.0.0/docs/LangRef.html#callingconv)
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub enum CallingConvention {
    C,
    Fast,
    Cold,
    GHC,
    HiPE,
    WebKit_JS,
    AnyReg,
    PreserveMost,
    PreserveAll,
    Swift,
    CXX_FastTLS,
    X86_StdCall,
    X86_FastCall,
    X86_RegCall,
    X86_ThisCall,
    X86_VectorCall,
    X86_Intr,
    X86_64_SysV,
    ARM_APCS,
    ARM_AAPCS,
    ARM_AAPCS_VFP,
    MSP430_INTR,
    MSP430_Builtin,
    PTX_Kernel,
    PTX_Device,
    SPIR_FUNC,
    SPIR_KERNEL,
    Intel_OCL_BI,
    Win64,
    HHVM,
    HHVM_C,
    AVR_Intr,
    AVR_Signal,
    AVR_Builtin,
    AMDGPU_CS,
    AMDGPU_ES,
    AMDGPU_GS,
    AMDGPU_HS,
    AMDGPU_LS,
    AMDGPU_PS,
    AMDGPU_VS,
    AMDGPU_Kernel,
    /// This is used if LLVM returns a calling convention not in `LLVMCallConv`.
    /// E.g., perhaps a calling convention was added to LLVM and this enum hasn't been updated yet.
    Numbered(u32),
}

/// See [LLVM 14 docs on Function Attributes](https://releases.llvm.org/14.0.0/docs/LangRef.html#fnattrs)
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Attribute {
    AlignStack(u64),
    AllocSize {
        elt_size: u32,
        num_elts: Option<u32>,
    },
    AlwaysInline,
    Builtin,
    Cold,
    Convergent,
    InaccessibleMemOnly,
    InaccessibleMemOrArgMemOnly,
    InlineHint,
    JumpTable,
    MinimizeSize,
    Naked,
    NoBuiltin,
    NoCFCheck,
    NoDuplicate,
    NoFree,
    NoImplicitFloat,
    NoInline,
    NoMerge,
    NonLazyBind,
    NoRedZone,
    NoReturn,
    NoRecurse,
    WillReturn,
    ReturnsTwice,
    NoSync,
    NoUnwind,
    NullPointerIsValid,
    OptForFuzzing,
    OptNone,
    OptSize,
    ReadNone,
    ReadOnly,
    WriteOnly,
    ArgMemOnly,
    SafeStack,
    SanitizeAddress,
    SanitizeMemory,
    SanitizeThread,
    SanitizeHWAddress,
    SanitizeMemTag,
    ShadowCallStack,
    SpeculativeLoadHardening,
    Speculatable,
    StackProtect,
    StackProtectReq,
    StackProtectStrong,
    StrictFP,
    UWTable,
    StringAttribute {
        kind: String,
        value: String, // for no value, use ""
    },
    UnknownAttribute, // this is used if we get a value not in the above list
}

/// `ParameterAttribute`s can apply to function parameters as well as function return types.
/// See [LLVM 14 docs on Parameter Attributes](https://releases.llvm.org/14.0.0/docs/LangRef.html#paramattrs)
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ParameterAttribute {
    ZeroExt,
    SignExt,
    InReg,
    ByVal(TypeRef),
    Preallocated(TypeRef),
    InAlloca(TypeRef),
    SRet(TypeRef),
    Alignment(u64),
    NoAlias,
    NoCapture,
    NoFree,
    Nest,
    Returned,
    NonNull,
    Dereferenceable(u64),
    DereferenceableOrNull(u64),
    SwiftSelf,
    SwiftError,
    ImmArg,
    NoUndef,
    StringAttribute {
        kind: String,
        value: String, // for no value, use ""
    },
    UnknownAttribute, // this is used if we get an EnumAttribute not in the above list; or, for LLVM 11 or lower, also for some TypeAttributes (due to C API limitations)
    UnknownTypeAttribute(TypeRef), // this is used if we get a TypeAttribute not in the above list
}

pub type GroupID = usize;