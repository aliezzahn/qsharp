use crate::{set_indentation, RuntimeCapability};
use qsc_data_structures::index_map::IndexMap;
use qsc_fir::fir::{
    BlockId, CallableDecl, ExprId, Item, ItemId, ItemKind, LocalItemId, Package, PackageId,
    PackageStore, PatId, StmtId,
};

use indenter::indented;
use rustc_hash::FxHashSet;

use std::{
    fmt::{Display, Formatter, Result, Write},
    fs::File,
    io::Write as IoWrite,
    vec::Vec,
};

pub struct StoreComputeProps(IndexMap<PackageId, PackageComputeProps>);

#[derive(Debug)]
pub struct PackageComputeProps {
    pub items: IndexMap<LocalItemId, ItemComputeProps>,
    pub blocks: IndexMap<BlockId, InnerElmtComputeProps>,
    pub stmts: IndexMap<StmtId, InnerElmtComputeProps>,
    pub exprs: IndexMap<ExprId, InnerElmtComputeProps>,
    pub pats: IndexMap<PatId, PatComputeProps>,
}

impl Default for PackageComputeProps {
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

impl Display for PackageComputeProps {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "Items:")?;
        for (id, item) in self.items.iter() {
            let mut indent = set_indentation(indented(f), 1);
            write!(indent, "\nLocal Item ID {id}: ")?;
            let mut indent = set_indentation(indented(f), 2);
            write!(indent, "\n{item}")?;
        }

        write!(f, "\nBlocks:")?;
        for (id, block) in self.blocks.iter() {
            let mut indent = set_indentation(indented(f), 1);
            write!(indent, "\nBlock ID {id}: ")?;
            let mut indent = set_indentation(indented(f), 2);
            write!(indent, "\n{block}")?;
        }

        write!(f, "\nStatements:")?;
        for (id, stmt) in self.stmts.iter() {
            let mut indent = set_indentation(indented(f), 1);
            write!(indent, "\nStatement ID {id}: ")?;
            let mut indent = set_indentation(indented(f), 2);
            write!(indent, "\n{stmt}")?;
        }

        write!(f, "\nExpressions:")?;
        for (id, expr) in self.exprs.iter() {
            let mut indent = set_indentation(indented(f), 1);
            write!(indent, "\nExpression ID {id}: ")?;
            let mut indent = set_indentation(indented(f), 2);
            write!(indent, "\n{expr}")?;
        }

        write!(f, "\nPatterns:")?;
        for (id, pat) in self.pats.iter() {
            let mut indent = set_indentation(indented(f), 1);
            write!(indent, "\nPattern ID {id}: ")?;
            let mut indent = set_indentation(indented(f), 2);
            write!(indent, "\n{pat}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ItemComputeProps {
    NonCallable,
    Callable(CallableComputeProps),
}

impl Display for ItemComputeProps {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match &self {
            ItemComputeProps::NonCallable => write!(f, "Non-Callable")?,
            ItemComputeProps::Callable(callable_rt_props) => write!(f, "{callable_rt_props}")?,
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct CallableComputeProps {
    pub apps: AppsTbl,
}

impl Display for CallableComputeProps {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "Callable Runtime Properties:")?;
        let mut indent = set_indentation(indented(f), 1);
        write!(indent, "\n{}", self.apps)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum InnerElmtComputeProps {
    AppDependent(AppsTbl),
    AppIndependent(ComputeProps),
}

impl Display for InnerElmtComputeProps {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match &self {
            InnerElmtComputeProps::AppDependent(apps_table) => {
                write!(f, "Application Dependent: {apps_table}")?
            }
            InnerElmtComputeProps::AppIndependent(compute_kind) => {
                write!(f, "Application Independent: {compute_kind}")?
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ParamIdx(usize);

#[derive(Debug)]
pub enum PatComputeProps {
    Local,
    CallableParam(ItemId, ParamIdx),
}

impl Display for PatComputeProps {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match &self {
            PatComputeProps::Local => write!(f, "Local")?,
            PatComputeProps::CallableParam(item_id, param_idx) => {
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
pub struct AppsTbl {
    // N.B. (cesarzc): Will probably be only used to assert compatibility when using it.
    pub input_param_count: usize,
    // N.B. (cesarzc): Hide the vector to provide a good interface to access applications (possibly
    // by providing a get that takes a vector of `ComputeKind`).
    apps: Vec<ComputeProps>,
}

impl AppsTbl {
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

impl Display for AppsTbl {
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

pub struct SinglePassAnalyzer {
    store_compute_props: StoreComputeProps,
}

impl Default for SinglePassAnalyzer {
    fn default() -> Self {
        Self {
            store_compute_props: StoreComputeProps(IndexMap::new()),
        }
    }
}

impl SinglePassAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(&mut self, package_store: &PackageStore) -> &StoreComputeProps {
        // Create package runtime properties for each package.
        for (package_id, package) in package_store.0.iter() {
            let package_compute_props = PackageComputeProps::default();
            self.store_compute_props
                .0
                .insert(package_id, package_compute_props);
        }

        // Propagate package runtime properties for each package.
        for (package_id, package) in package_store.0.iter() {
            self.propagate_package_rt_props(package_id, package);
        }

        &self.store_compute_props
    }

    fn compute_callable_rt_props(
        &mut self,
        package_id: PackageId,
        item_id: LocalItemId,
        callable: &CallableDecl,
    ) {
        let callabale_compute_props = CallableComputeProps {
            apps: AppsTbl::new(0),
        };

        let package_compute_props = self
            .store_compute_props
            .0
            .get_mut(package_id)
            .expect("Package runtime properties should exist");

        package_compute_props
            .items
            .insert(item_id, ItemComputeProps::Callable(callabale_compute_props));
    }

    fn propagate_package_rt_props(&mut self, package_id: PackageId, package: &Package) {
        for (item_id, item) in &package.items {
            self.propagate_item_rt_props(package_id, item_id, item);
        }
    }

    fn propagate_item_rt_props(
        &mut self,
        package_id: PackageId,
        item_id: LocalItemId,
        item: &Item,
    ) {
        let package_compute_props = self
            .store_compute_props
            .0
            .get_mut(package_id)
            .expect("Package runtime properties should exist");

        let item_props = package_compute_props.items.get(item_id);
        if item_props.is_none() {
            match &item.kind {
                ItemKind::Callable(callable) => {
                    self.compute_callable_rt_props(package_id, item_id, &callable)
                }
                _ => package_compute_props
                    .items
                    .insert(item_id, ItemComputeProps::NonCallable),
            };
        }
    }
}
