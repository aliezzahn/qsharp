use crate::{set_indentation, RuntimeCapability};
use qsc_data_structures::index_map::IndexMap;
use qsc_fir::{
    fir::{
        BlockId, CallableDecl, CallableKind, ExprId, ItemId, ItemKind, LocalItemId, Package,
        PackageId, PackageStore, Pat, PatId, PatKind, SpecBody, SpecGen, StmtId,
    },
    ty::{Prim, Ty},
};

use indenter::indented;
use rustc_hash::FxHashSet;

use std::{
    default,
    fmt::{Display, Formatter, Result, Write},
    fs::File,
    io::Write as IoWrite,
    ops::Deref,
    vec::Vec,
};

pub struct StoreRtProps(IndexMap<PackageId, PackageRtProps>);

#[derive(Debug)]
pub struct PackageRtProps {
    pub items: IndexMap<LocalItemId, Option<ItemRtProps>>,
    pub blocks: IndexMap<BlockId, Option<InnerElmtRtProps>>,
    pub stmts: IndexMap<StmtId, Option<InnerElmtRtProps>>,
    pub exprs: IndexMap<ExprId, Option<InnerElmtRtProps>>,
    pub pats: IndexMap<PatId, Option<PatRtProps>>,
}

// TODO (cesarzc): This is probably not needed.
impl Default for PackageRtProps {
    fn default() -> Self {
        Self {
            items: IndexMap::new(),
            blocks: IndexMap::new(),
            stmts: IndexMap::new(),
            exprs: IndexMap::new(),
            pats: IndexMap::new(),
        }
    }
}

impl Display for PackageRtProps {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "Items:")?;
        for (id, item) in self.items.iter() {
            let mut indent = set_indentation(indented(f), 1);
            write!(indent, "\nLocal Item ID {id}: ")?;
            match item {
                None => {
                    _ = write!(f, "None");
                }
                Some(item_rt_props) => {
                    let mut indent = set_indentation(indented(f), 2);
                    write!(indent, "\n{item_rt_props}")?;
                }
            }
        }

        write!(f, "\nBlocks:")?;
        for (id, block) in self.blocks.iter() {
            let mut indent = set_indentation(indented(f), 1);
            write!(indent, "\nBlock ID {id}: ")?;
            match block {
                None => {
                    _ = write!(f, "None");
                }
                Some(inner_elmt_props) => {
                    let mut indent = set_indentation(indented(f), 2);
                    write!(indent, "\n{inner_elmt_props}")?;
                }
            }
        }

        write!(f, "\nStatements:")?;
        for (id, stmt) in self.stmts.iter() {
            let mut indent = set_indentation(indented(f), 1);
            write!(indent, "\nStatement ID {id}: ")?;
            match stmt {
                None => {
                    _ = write!(f, "None");
                }
                Some(inner_elmt_props) => {
                    let mut indent = set_indentation(indented(f), 2);
                    write!(indent, "\n{inner_elmt_props}")?;
                }
            }
        }

        write!(f, "\nExpressions:")?;
        for (id, expr) in self.exprs.iter() {
            let mut indent = set_indentation(indented(f), 1);
            write!(indent, "\nExpression ID {id}: ")?;
            match expr {
                None => {
                    _ = write!(f, "None");
                }
                Some(inner_elmt_props) => {
                    let mut indent = set_indentation(indented(f), 2);
                    write!(indent, "\n{inner_elmt_props}")?;
                }
            }
        }

        write!(f, "\nPatterns:")?;
        for (id, pat) in self.pats.iter() {
            let mut indent = set_indentation(indented(f), 1);
            write!(indent, "\nPattern ID {id}: ")?;
            match pat {
                None => {
                    _ = write!(f, "None");
                }
                Some(inner_elmt_props) => {
                    let mut indent = set_indentation(indented(f), 2);
                    write!(indent, "\n{inner_elmt_props}")?;
                }
            }
        }
        Ok(())
    }
}

impl PackageRtProps {
    fn new(package: &Package) -> Self {
        // Initialize items.
        let mut items = IndexMap::<LocalItemId, Option<ItemRtProps>>::new();
        for (item_id, item) in package.items.iter() {
            // Initialize items depending on whether they are callables or not.
            let item_rt_props = match item.kind {
                ItemKind::Callable(_) => None,
                ItemKind::Namespace(_, _) | ItemKind::Ty(_, _) => Some(ItemRtProps::NonCallable),
            };
            items.insert(item_id, item_rt_props);
        }

        // Initialize blocks.
        let mut blocks = IndexMap::<BlockId, Option<InnerElmtRtProps>>::new();
        for (block_id, _) in package.blocks.iter() {
            blocks.insert(block_id, Option::None);
        }

        // Initialize statements.
        let mut stmts = IndexMap::<StmtId, Option<InnerElmtRtProps>>::new();
        for (stmt_id, _) in package.stmts.iter() {
            stmts.insert(stmt_id, Option::None);
        }

        // Initialize expressions.
        let mut exprs = IndexMap::<ExprId, Option<InnerElmtRtProps>>::new();
        for (expr_id, _) in package.exprs.iter() {
            exprs.insert(expr_id, Option::None);
        }

        // Initialize patterns.
        let mut pats = IndexMap::<PatId, Option<PatRtProps>>::new();
        for (pat_id, _) in package.pats.iter() {
            pats.insert(pat_id, Option::None);
        }

        Self {
            items,
            blocks,
            stmts,
            exprs,
            pats,
        }
    }
}

#[derive(Debug)]
pub enum ItemRtProps {
    NonCallable,
    Callable(CallableRtProps),
}

impl Display for ItemRtProps {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match &self {
            ItemRtProps::NonCallable => write!(f, "Non-Callable")?,
            ItemRtProps::Callable(callable_rt_props) => write!(f, "{callable_rt_props}")?,
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct CallableRtProps {
    pub apps_table: AppsTable,
}

impl Display for CallableRtProps {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "Callable Runtime Properties:")?;
        let mut indent = set_indentation(indented(f), 1);
        write!(indent, "\n{}", self.apps_table)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum InnerElmtRtProps {
    AppDependent(AppsTable),
    AppIndependent(ComputeProps),
}

impl Display for InnerElmtRtProps {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match &self {
            InnerElmtRtProps::AppDependent(apps_table) => {
                write!(f, "Application Dependent: {apps_table}")?
            }
            InnerElmtRtProps::AppIndependent(compute_kind) => {
                write!(f, "Application Independent: {compute_kind}")?
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ParamIdx(usize);

#[derive(Debug)]
pub enum PatRtProps {
    Local,
    CallableParam(ItemId, ParamIdx),
}

impl Display for PatRtProps {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match &self {
            PatRtProps::Local => write!(f, "Local")?,
            PatRtProps::CallableParam(item_id, param_idx) => {
                write!(f, "Callable Parameter:")?;
                let mut indent = set_indentation(indented(f), 1);
                write!(indent, "\nItem ID: {item_id}")?;
                write!(indent, "\nParameter Index: {param_idx:?}")?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AppIdx(usize);

impl AppIdx {
    pub fn map_to_compute_kind_vector(&self, input_param_count: usize) -> Vec<ComputeKind> {
        let mut params_compute_kind = Vec::new();
        for param_idx in 0..input_param_count {
            let mask = 1 << param_idx;
            let compute_kind = if self.0 & mask == 0 {
                ComputeKind::Static
            } else {
                ComputeKind::Dynamic
            };
            params_compute_kind.push(compute_kind);
        }
        params_compute_kind
    }
}

#[derive(Clone, Debug)]
pub struct AppsTable {
    // N.B. (cesarzc): Will probably be only used to assert compatibility when using it.
    pub input_param_count: usize,
    // N.B. (cesarzc): Hide the vector to provide a good interface to access applications (possibly
    // by providing a get that takes a vector of `ComputeKind`).
    apps: Vec<ComputeProps>,
}

impl AppsTable {
    pub fn new(input_param_count: usize) -> Self {
        Self {
            input_param_count,
            apps: Vec::new(),
        }
    }

    // TODO (cesarzc): Implement a get that takes a vector of `ComputeKind` where each element maps
    // to the compute kind of the input parameter.

    pub fn get(&self, index: AppIdx) -> Option<&ComputeProps> {
        self.apps.get(index.0)
    }

    pub fn get_mut(&mut self, index: AppIdx) -> Option<&mut ComputeProps> {
        self.apps.get_mut(index.0)
    }

    pub fn push(&mut self, app: ComputeProps) {
        self.apps.push(app);
    }
}

impl Display for AppsTable {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "Applications Table ({} input parameters):",
            self.input_param_count
        )?;
        let mut indent = set_indentation(indented(f), 1);
        for (idx, app) in self.apps.iter().enumerate() {
            write!(indent, "\n[{idx:#010b}] -> {app}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ComputeProps {
    pub rt_caps: FxHashSet<RuntimeCapability>,
    // N.B. (cesarzc): To get good error messages, maybe quantum source needs expansion and link to compute props.
    pub quantum_sources: Vec<QuantumSource>,
}

impl Display for ComputeProps {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let compute_kind = self.compute_kind();
        write!(f, "Compute Properties ({compute_kind:?}):")?;
        let mut indent = set_indentation(indented(f), 1);
        if self.rt_caps.is_empty() {
            write!(indent, "\nRuntime Capabilities: <empty>")?;
        } else {
            write!(indent, "\nRuntime Capabilities: {{")?;
            for cap in &self.rt_caps {
                indent = set_indentation(indent, 2);
                write!(indent, "\n{cap:?}")?;
            }
            indent = set_indentation(indent, 1);
            write!(indent, "\n}}")?;
        }

        let mut indent = set_indentation(indented(f), 1);
        if self.quantum_sources.is_empty() {
            write!(indent, "\nQuantum Sources: <empty>")?;
        } else {
            write!(indent, "\nQuantum Sources:")?;
            for src in self.quantum_sources.iter() {
                indent = set_indentation(indent, 2);
                write!(indent, "\n{src:?}")?; // TODO (cesarzc): Implement non-debug display, maybe?.
            }
        }

        Ok(())
    }
}

impl ComputeProps {
    pub fn is_quantum_source(&self) -> bool {
        !self.quantum_sources.is_empty()
    }

    pub fn compute_kind(&self) -> ComputeKind {
        if self.rt_caps.is_empty() {
            ComputeKind::Static
        } else {
            ComputeKind::Dynamic
        }
    }
}

#[derive(Debug)]
pub enum ComputeKind {
    Static,
    Dynamic,
}

#[derive(Clone, Debug)]
pub enum QuantumSource {
    Intrinsic,
    ItemId,
    BlockId,
    StmtId,
    ExprId,
    PatId,
}

pub struct Analyzer<'a> {
    package_store: &'a PackageStore,
    store_rt_props: StoreRtProps,
}

impl<'a> Analyzer<'a> {
    pub fn new(package_store: &'a PackageStore) -> Self {
        let mut packages_rt_props = IndexMap::new();
        for (package_id, package) in package_store.0.iter() {
            let package_rt_props = PackageRtProps::new(package);
            packages_rt_props.insert(package_id, package_rt_props);
        }
        Self {
            package_store,
            store_rt_props: StoreRtProps(packages_rt_props),
        }
    }

    pub fn run(&mut self) -> &StoreRtProps {
        self.persist_store_rt_props(0);
        self.initialize_foundational_rt_props();
        self.persist_store_rt_props(1);
        &self.store_rt_props
    }

    fn initialize_foundational_rt_props(&mut self) {
        for (package_id, package) in self.package_store.0.iter() {
            let package_rt_props = self
                .store_rt_props
                .0
                .get_mut(package_id)
                .expect("Package runtime properties should exist");
            FoundationalRtProps::initialize_package_intrinsics(package, package_rt_props);
        }
    }

    fn persist_store_rt_props(&self, phase: u8) {
        for (package_id, package) in self.store_rt_props.0.iter() {
            let filename = format!("dbg/phase{phase}.package{package_id}.txt");
            let mut package_file = File::create(filename).expect("File could be created");
            let package_string = format!("{package}");
            write!(package_file, "{package_string}").expect("Writing to file should succeed.");
        }
    }
}

struct FoundationalRtProps;

impl FoundationalRtProps {
    pub fn initialize_package_intrinsics(package: &Package, package_rt_props: &mut PackageRtProps) {
        for (item_id, item) in package.items.iter() {
            if let ItemKind::Callable(callable) = &item.kind {
                Self::try_initialize_intrinsic(
                    item_id,
                    callable,
                    package,
                    &mut package_rt_props.items,
                );
            }
        }
    }

    fn calculate_intrinsic_function_application(
        input_param_types: &Vec<Ty>,
        output_type: &Ty,
        app_idx: AppIdx,
    ) -> ComputeProps {
        let input_param_count = input_param_types.len();
        assert!((app_idx.0 as i32) < 2i32.pow(input_param_count as u32));

        //
        let params_compute_kind = app_idx.map_to_compute_kind_vector(input_param_count);
        let mut rt_caps = FxHashSet::<RuntimeCapability>::default();
        let params_info_tuple = input_param_types.iter().zip(params_compute_kind.iter());
        for (param_type, param_compute_kind) in params_info_tuple {
            if let ComputeKind::Dynamic = param_compute_kind {
                let param_caps = RtCaps::infer_caps_from_type(param_type);
                rt_caps.extend(param_caps);
            }
        }

        // If this is an all-static application or the function output is `Unit` then this is not a quantum source.
        // Otherwise this is an intrinsic quantum source.
        let is_unit = matches!(*output_type, Ty::UNIT);
        let quantum_sources = if app_idx.0 == 0 || is_unit {
            Vec::new()
        } else {
            vec![QuantumSource::Intrinsic]
        };

        ComputeProps {
            rt_caps,
            quantum_sources,
        }
    }

    fn calculate_intrinsic_operation_application(
        input_param_types: &Vec<Ty>,
        output_type: &Ty,
        app_idx: AppIdx,
    ) -> ComputeProps {
        let input_param_count = input_param_types.len();
        assert!((app_idx.0 as i32) < 2i32.pow(input_param_count as u32));

        //
        let params_compute_kind = app_idx.map_to_compute_kind_vector(input_param_count);
        let mut rt_caps = FxHashSet::<RuntimeCapability>::default();
        let params_info_tuple = input_param_types.iter().zip(params_compute_kind.iter());
        for (param_type, param_compute_kind) in params_info_tuple {
            if let ComputeKind::Dynamic = param_compute_kind {
                let param_caps = RtCaps::infer_caps_from_type(param_type);
                rt_caps.extend(param_caps);
            }
        }

        // If the operation is unit then it is an instrinsic quantum source.
        let is_unit = matches!(*output_type, Ty::UNIT);
        let quantum_sources = if is_unit {
            Vec::new()
        } else {
            vec![QuantumSource::Intrinsic]
        };

        ComputeProps {
            rt_caps,
            quantum_sources,
        }
    }

    fn get_input_params_types(pattern: &Pat) -> Vec<Ty> {
        match pattern.kind {
            PatKind::Bind(_) => match pattern.ty {
                Ty::Array(_) | Ty::Arrow(_) | Ty::Prim(_) | Ty::Tuple(_) | Ty::Udt(_) => {
                    vec![pattern.ty.clone()]
                }
                _ => panic!(
                    "Unexpected pattern type {} for pattern {}",
                    pattern.ty, pattern.id
                ),
            },
            PatKind::Tuple(_) => match &pattern.ty {
                Ty::Tuple(vector) => vector.clone(),
                _ => panic!("Unexpected pattern type"),
            },
            _ => panic!("Only callable parameter patterns are expected"),
        }
    }

    fn initialize_intrinsic_function_rt_props(
        function: &CallableDecl,
        input_pattern: &Pat,
        output_type: &Ty,
    ) -> CallableRtProps {
        assert!(Self::is_callable_intrinsic(function));
        let input_param_types = Self::get_input_params_types(input_pattern);
        let mut apps_table = AppsTable::new(input_param_types.len());
        let apps_count = 2u32.pow(input_param_types.len() as u32);
        for app_idx in 0..apps_count {
            let app = Self::calculate_intrinsic_function_application(
                &input_param_types,
                output_type,
                AppIdx(app_idx as usize),
            );
            apps_table.push(app);
        }
        CallableRtProps { apps_table }
    }

    fn initialize_intrinsic_operation_rt_props(
        operation: &CallableDecl,
        input_pattern: &Pat,
        output_type: &Ty,
    ) -> CallableRtProps {
        assert!(Self::is_callable_intrinsic(operation));
        let input_param_types = Self::get_input_params_types(input_pattern);
        let mut apps_table = AppsTable::new(input_param_types.len());
        let apps_count = 2u32.pow(input_param_types.len() as u32);
        for app_idx in 0..apps_count {
            let app = Self::calculate_intrinsic_operation_application(
                &input_param_types,
                output_type,
                AppIdx(app_idx as usize),
            );
            apps_table.push(app.clone());
        }
        CallableRtProps { apps_table }
    }

    fn is_callable_intrinsic(callable: &CallableDecl) -> bool {
        match callable.body.body {
            SpecBody::Gen(spec_gen) => spec_gen == SpecGen::Intrinsic,
            _ => false,
        }
    }

    fn try_initialize_intrinsic(
        callable_id: LocalItemId,
        callable: &CallableDecl,
        package: &Package,
        callables_rt_props: &mut IndexMap<LocalItemId, Option<ItemRtProps>>,
    ) {
        if Self::is_callable_intrinsic(callable) {
            // Get the input pattern of the callable since that determines properties of intrinsic callables.
            let input_pattern = package
                .pats
                .get(callable.input)
                .expect("Input pattern should exist");
            let callable_rt_props = match callable.kind {
                CallableKind::Function => Self::initialize_intrinsic_function_rt_props(
                    callable,
                    input_pattern,
                    &callable.output,
                ),
                CallableKind::Operation => Self::initialize_intrinsic_operation_rt_props(
                    callable,
                    input_pattern,
                    &callable.output,
                ),
            };

            callables_rt_props.insert(callable_id, Some(ItemRtProps::Callable(callable_rt_props)));
        }
    }
}

struct RtCaps;

impl RtCaps {
    pub fn infer_caps_from_type(ty: &Ty) -> FxHashSet<RuntimeCapability> {
        match ty {
            Ty::Array(_) => FxHashSet::from_iter([RuntimeCapability::HigherLevelConstructs]),
            Ty::Arrow(_) => FxHashSet::from_iter([RuntimeCapability::HigherLevelConstructs]),
            Ty::Prim(prim) => Self::infer_caps_from_primitive_type(prim),
            Ty::Tuple(v) => Self::infer_caps_from_tuple_type(v),
            Ty::Udt(_) => FxHashSet::from_iter([RuntimeCapability::HigherLevelConstructs]),
            _ => panic!("Unexpected type"),
        }
    }

    fn infer_caps_from_tuple_type(tuple: &[Ty]) -> FxHashSet<RuntimeCapability> {
        let mut caps = FxHashSet::<RuntimeCapability>::default();
        for item_type in tuple.iter() {
            let item_caps = Self::infer_caps_from_type(item_type);
            caps.extend(item_caps);
        }
        caps
    }

    fn infer_caps_from_primitive_type(primitive: &Prim) -> FxHashSet<RuntimeCapability> {
        match primitive {
            Prim::BigInt => FxHashSet::from_iter([RuntimeCapability::HigherLevelConstructs]),
            Prim::Bool => FxHashSet::from_iter([RuntimeCapability::ConditionalForwardBranching]),
            Prim::Double => FxHashSet::from_iter([RuntimeCapability::FloatingPointComputation]),
            Prim::Int => FxHashSet::from_iter([RuntimeCapability::IntegerComputations]),
            Prim::Pauli => FxHashSet::from_iter([RuntimeCapability::IntegerComputations]),
            Prim::Qubit => FxHashSet::default(),
            Prim::Range | Prim::RangeFrom | Prim::RangeTo | Prim::RangeFull => {
                FxHashSet::from_iter([RuntimeCapability::IntegerComputations])
            }
            Prim::Result => FxHashSet::from_iter([RuntimeCapability::ConditionalForwardBranching]),
            Prim::String => FxHashSet::from_iter([RuntimeCapability::HigherLevelConstructs]),
        }
    }
}