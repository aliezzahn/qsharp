// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use crate::{
    common::{
        aggregate_compute_kind, aggregate_value_kind, initialize_locals_map, InputParam,
        InputParamIndex, Local, LocalKind, LocalsLookup,
    },
    scaffolding::PackageComputeProperties,
    ApplicationGeneratorSet, ComputeKind, QuantumProperties, RuntimeFeatureFlags, ValueKind,
};
use qsc_data_structures::index_map::IndexMap;
use qsc_fir::fir::{BlockId, ExprId, LocalVarId, Pat, PatId, PatKind, SpecDecl, StmtId};
use rustc_hash::FxHashMap;
use std::convert::{From, TryFrom};

/// Auxiliary data structure used to build multiple related application generator sets from individual application
/// instances.
#[derive(Debug)]
pub struct GeneratorSetsBuilder {
    pub inherent: ApplicationInstance,
    pub dynamic_param_applications: Vec<ApplicationInstance>,
}

impl GeneratorSetsBuilder {
    /// Creates a new builder.
    pub fn new(input_params: &Vec<InputParam>, controls: Option<&Local>) -> Self {
        let inherent = ApplicationInstance::new(input_params, controls, None);
        let mut dynamic_param_applications =
            Vec::<ApplicationInstance>::with_capacity(input_params.len());
        for input_param in input_params {
            let application_instance =
                ApplicationInstance::new(input_params, controls, Some(input_param.index));
            dynamic_param_applications.push(application_instance);
        }

        Self {
            inherent,
            dynamic_param_applications,
        }
    }

    /// Creates a new builder from a specialization.
    // TODO (cesarzc): Remove.
    pub fn from_spec(
        spec_decl: &SpecDecl,
        input_params: &Vec<InputParam>,
        pats: &IndexMap<PatId, Pat>,
    ) -> Self {
        let spec_input = derive_spec_input(spec_decl, pats);
        let inherent = ApplicationInstance::new(input_params, spec_input.as_ref(), None);
        let mut dynamic_params = Vec::<ApplicationInstance>::with_capacity(input_params.len());
        for input_param in input_params {
            let application_instance = ApplicationInstance::new(
                input_params,
                spec_input.as_ref(),
                Some(input_param.index),
            );
            dynamic_params.push(application_instance);
        }

        Self {
            inherent,
            dynamic_param_applications: dynamic_params,
        }
    }

    /// Creates a new builder with no dynamic parameter applications.
    // TODO (cesarzc): Remove.
    pub fn with_no_dynamic_param_applications() -> Self {
        let inherent = ApplicationInstance::new(&Vec::new(), None, None);
        Self {
            inherent,
            dynamic_param_applications: Vec::new(),
        }
    }

    pub fn get_application_instance(
        &self,
        index: ApplicationInstanceIndex,
    ) -> &ApplicationInstance {
        let index_as_int = i32::from(index);
        if index_as_int < 0 {
            assert!(index_as_int == -1);
            return &self.inherent;
        }

        self.dynamic_param_applications
            .get(usize::try_from(index_as_int).expect("index should be valid"))
            .expect("application instance at index does not exist")
    }

    pub fn get_application_instance_mut(
        &mut self,
        index: ApplicationInstanceIndex,
    ) -> &mut ApplicationInstance {
        let index_as_int = i32::from(index);
        if index_as_int < 0 {
            assert!(index_as_int == -1);
            return &mut self.inherent;
        }

        self.dynamic_param_applications
            .get_mut(usize::try_from(index_as_int).expect("index should be valid"))
            .expect("application instance at index does not exist")
    }

    /// Saves the contents of the builder to the package compute properties data structure.
    /// If a main block ID is provided, it returns the applications generator set representing the block.
    pub fn save_to_package_compute_properties(
        self,
        package_compute_properties: &mut PackageComputeProperties,
        main_block: Option<BlockId>,
    ) -> Option<ApplicationGeneratorSet> {
        // Get the compute properties of the inherent application instance and the dynamic parameter applications.
        let input_params_count = self.dynamic_param_applications.len();
        let mut inherent_application_compute_properties = self.inherent.close();
        let mut dynamic_param_applications_compute_properties =
            Vec::<ApplicationInstanceComputeProperties>::with_capacity(input_params_count);
        for application_instance in self.dynamic_param_applications {
            let application_instance_compute_properties = application_instance.close();
            dynamic_param_applications_compute_properties
                .push(application_instance_compute_properties);
        }

        // Save the compute properties to the package.
        Self::save_application_generator_sets(
            &mut inherent_application_compute_properties,
            &mut dynamic_param_applications_compute_properties,
            package_compute_properties,
        );

        // If a main block was provided, create an applications generator that represents the specialization based on
        // the applications generator of the main block.
        let close_output = main_block.map(|main_block_id| {
            let mut applications_generator = package_compute_properties
                .blocks
                .get(main_block_id)
                .expect("block applications generator should exist")
                .clone();
            assert!(
                applications_generator.dynamic_param_applications.len()
                    == dynamic_param_applications_compute_properties.len()
            );
            if let Some(inherent_value_kind) = inherent_application_compute_properties.value_kind {
                applications_generator
                    .inherent
                    .aggregate_value_kind(inherent_value_kind);
            }
            for (application_compute_kind, application_instance_compute_properties) in
                applications_generator
                    .dynamic_param_applications
                    .iter_mut()
                    .zip(dynamic_param_applications_compute_properties)
            {
                if let Some(value_kind) = application_instance_compute_properties.value_kind {
                    application_compute_kind.aggregate_value_kind(value_kind);
                }
            }

            // Return the applications table with the updated dynamism sources.
            applications_generator
        });

        close_output
    }

    fn save_application_generator_sets(
        inherent_application_compute_properties: &mut ApplicationInstanceComputeProperties,
        dynamic_param_applications_compute_properties: &mut Vec<
            ApplicationInstanceComputeProperties,
        >,
        package_compute_properties: &mut PackageComputeProperties,
    ) {
        let input_params_count = dynamic_param_applications_compute_properties.len();

        // Save an applications generator set for each block using their compute properties.
        for (block_id, block_inherent_compute_kind) in
            inherent_application_compute_properties.blocks.drain()
        {
            let mut block_dynamic_param_applications =
                Vec::<ComputeKind>::with_capacity(input_params_count);
            for application_instance_compute_properties in
                dynamic_param_applications_compute_properties.iter_mut()
            {
                let compute_kind = application_instance_compute_properties.remove_block(block_id);
                block_dynamic_param_applications.push(compute_kind);
            }
            let application_generator_set = ApplicationGeneratorSet {
                inherent: block_inherent_compute_kind,
                dynamic_param_applications: block_dynamic_param_applications,
            };
            package_compute_properties
                .blocks
                .insert(block_id, application_generator_set);
        }

        // Save an applications generator set for each statement using their compute properties.
        for (stmt_id, stmt_inherent_compute_kind) in
            inherent_application_compute_properties.stmts.drain()
        {
            let mut stmt_dynamic_param_applications =
                Vec::<ComputeKind>::with_capacity(input_params_count);
            for application_instance_compute_properties in
                dynamic_param_applications_compute_properties.iter_mut()
            {
                let compute_kind = application_instance_compute_properties.remove_stmt(stmt_id);
                stmt_dynamic_param_applications.push(compute_kind);
            }
            let application_generator_set = ApplicationGeneratorSet {
                inherent: stmt_inherent_compute_kind,
                dynamic_param_applications: stmt_dynamic_param_applications,
            };
            package_compute_properties
                .stmts
                .insert(stmt_id, application_generator_set);
        }

        // Save an applications generator set for each expression using their compute properties.
        for (expr_id, expr_inherent_compute_kind) in
            inherent_application_compute_properties.exprs.drain()
        {
            let mut expr_dynamic_param_applications =
                Vec::<ComputeKind>::with_capacity(input_params_count);
            for application_instance_compute_properties in
                dynamic_param_applications_compute_properties.iter_mut()
            {
                let compute_kind = application_instance_compute_properties.remove_expr(expr_id);
                expr_dynamic_param_applications.push(compute_kind);
            }
            let application_generator_set = ApplicationGeneratorSet {
                inherent: expr_inherent_compute_kind,
                dynamic_param_applications: expr_dynamic_param_applications,
            };
            package_compute_properties
                .exprs
                .insert(expr_id, application_generator_set);
        }
    }
}

fn derive_spec_input(spec_decl: &SpecDecl, pats: &IndexMap<PatId, Pat>) -> Option<Local> {
    spec_decl.input.and_then(|pat_id| {
        let pat = pats.get(pat_id).expect("pat should exist");
        match &pat.kind {
            PatKind::Bind(ident) => Some(Local {
                var: ident.id,
                pat: pat_id,
                ty: pat.ty.clone(),
                kind: LocalKind::SpecInput,
            }),
            PatKind::Discard => None, // Nothing to bind to.
            PatKind::Tuple(_) => panic!("expected specialization input pattern"),
        }
    })
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ApplicationInstanceIndex(i32);

impl From<ApplicationInstanceIndex> for i32 {
    fn from(value: ApplicationInstanceIndex) -> Self {
        value.0
    }
}

impl From<i32> for ApplicationInstanceIndex {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

/// An instance of a callable application.
#[derive(Debug, Default)]
pub struct ApplicationInstance {
    /// A map of locals with their associated compute kind.
    pub locals_map: LocalsComputeKindMap,
    /// The currently active dynamic scopes in the application instance.
    pub active_dynamic_scopes: Vec<ExprId>,
    /// The return expressions throughout the application instance.
    pub return_expressions: Vec<ExprId>,
    /// The compute kind of the blocks related to the application instance.
    blocks: FxHashMap<BlockId, ComputeKind>,
    /// The compute kind of the statements related to the application instance.
    stmts: FxHashMap<StmtId, ComputeKind>,
    /// The compute kind of the expressions related to the application instance.
    exprs: FxHashMap<ExprId, ComputeKind>,
}

impl ApplicationInstance {
    pub fn find_block_compute_kind(&self, id: BlockId) -> Option<&ComputeKind> {
        self.blocks.get(&id)
    }

    pub fn find_expr_compute_kind(&self, id: ExprId) -> Option<&ComputeKind> {
        self.exprs.get(&id)
    }

    pub fn find_stmt_compute_kind(&self, id: StmtId) -> Option<&ComputeKind> {
        self.stmts.get(&id)
    }

    pub fn get_block_compute_kind(&self, id: BlockId) -> &ComputeKind {
        self.find_block_compute_kind(id)
            .expect("block compute kind should exist in application instance")
    }

    pub fn get_expr_compute_kind(&self, id: ExprId) -> &ComputeKind {
        self.find_expr_compute_kind(id)
            .expect("expression compute kind should exist in application instance")
    }

    pub fn get_stmt_compute_kind(&self, id: StmtId) -> &ComputeKind {
        self.find_stmt_compute_kind(id)
            .expect("expression compute kind should exist in application instance")
    }

    pub fn insert_block_compute_kind(&mut self, id: BlockId, value: ComputeKind) {
        self.blocks.insert(id, value);
    }

    pub fn insert_expr_compute_kind(&mut self, id: ExprId, value: ComputeKind) {
        self.exprs.insert(id, value);
    }

    pub fn insert_stmt_compute_kind(&mut self, id: StmtId, value: ComputeKind) {
        self.stmts.insert(id, value);
    }

    fn new(
        input_params: &Vec<InputParam>,
        controls: Option<&Local>,
        dynamic_param_index: Option<InputParamIndex>,
    ) -> Self {
        // Initialize the locals map with the specialization controls (if any).
        let mut locals_map = LocalsComputeKindMap::default();
        if let Some(controls) = controls {
            // Controls compute properties are handled at the call expression, so just use quantum compute kind with
            // no runtime features here.
            locals_map.insert(
                controls.var,
                LocalComputeKind {
                    local: controls.clone(),
                    compute_kind: ComputeKind::Quantum(QuantumProperties {
                        runtime_features: RuntimeFeatureFlags::empty(),
                        value_kind: ValueKind::Static,
                    }),
                },
            );
        }

        let mut unprocessed_locals_map = initialize_locals_map(input_params);
        for (node_id, local) in unprocessed_locals_map.drain() {
            let LocalKind::InputParam(input_param_index) = local.kind else {
                panic!("only input parameters are expected");
            };

            // If a dynamic parameter index is provided, set the local compute kind as dynamic.
            let compute_kind = if let Some(dynamic_param_index) = dynamic_param_index {
                if input_param_index == dynamic_param_index {
                    ComputeKind::Quantum(QuantumProperties {
                        runtime_features: RuntimeFeatureFlags::empty(),
                        value_kind: ValueKind::Dynamic,
                    })
                } else {
                    ComputeKind::Classical
                }
            } else {
                ComputeKind::Classical
            };

            locals_map.insert(
                node_id,
                LocalComputeKind {
                    local,
                    compute_kind,
                },
            );
        }
        Self {
            locals_map,
            active_dynamic_scopes: Vec::new(),
            return_expressions: Vec::new(),
            blocks: FxHashMap::default(),
            stmts: FxHashMap::default(),
            exprs: FxHashMap::default(),
        }
    }

    fn close(self) -> ApplicationInstanceComputeProperties {
        // Determine the value kind of the application instance by going through each return expression aggregating
        // their value kind (if any).
        let mut value_kinds = Vec::<ValueKind>::new();
        for expr_id in self.return_expressions {
            let expr_compute_kind = self
                .exprs
                .get(&expr_id)
                .expect("expression compute kind should exist");
            if let ComputeKind::Quantum(quantum_properties) = expr_compute_kind {
                value_kinds.push(quantum_properties.value_kind);
            }
        }

        // The application instance has a value kind only if one of its return expressions was quantum.
        let value_kind = if value_kinds.is_empty() {
            None
        } else {
            let value_kind = value_kinds.iter().fold(
                ValueKind::Static,
                |aggregated_value_kind, current_value_kind| {
                    aggregate_value_kind(aggregated_value_kind, *current_value_kind)
                },
            );
            Some(value_kind)
        };

        ApplicationInstanceComputeProperties {
            blocks: self.blocks,
            stmts: self.stmts,
            exprs: self.exprs,
            value_kind,
        }
    }
}

struct ApplicationInstanceComputeProperties {
    blocks: FxHashMap<BlockId, ComputeKind>,
    stmts: FxHashMap<StmtId, ComputeKind>,
    exprs: FxHashMap<ExprId, ComputeKind>,
    value_kind: Option<ValueKind>,
}

impl ApplicationInstanceComputeProperties {
    fn remove_block(&mut self, id: BlockId) -> ComputeKind {
        self.blocks.remove(&id).expect(
            "block to be removed should exist in the compute properties of the application instance",
        )
    }

    fn remove_stmt(&mut self, id: StmtId) -> ComputeKind {
        self.stmts.remove(&id).expect(
            "statement to be removed should exist in the compute properties of the application instance",
        )
    }

    fn remove_expr(&mut self, id: ExprId) -> ComputeKind {
        self.exprs.remove(&id).expect(
            "expression to be removed should exist in the compute properties of the application instance",
        )
    }
}

#[derive(Debug, Default)]
pub struct LocalsComputeKindMap(FxHashMap<LocalVarId, LocalComputeKind>);

impl LocalsLookup for LocalsComputeKindMap {
    fn find(&self, local_var_id: LocalVarId) -> Option<&Local> {
        self.0
            .get(&local_var_id)
            .map(|local_compute_kind| &local_compute_kind.local)
    }
}

impl LocalsComputeKindMap {
    pub fn aggregate_compute_kind(&mut self, local_var_id: LocalVarId, delta: ComputeKind) {
        let local_compute_kind = self
            .0
            .get_mut(&local_var_id)
            .expect("compute kind for local should exist");
        local_compute_kind.compute_kind =
            aggregate_compute_kind(local_compute_kind.compute_kind, delta);
    }

    pub fn find_compute_kind(&self, local_var_id: LocalVarId) -> Option<&ComputeKind> {
        self.0
            .get(&local_var_id)
            .map(|local_compute_kind| &local_compute_kind.compute_kind)
    }

    pub fn get_compute_kind(&self, local_var_id: LocalVarId) -> &ComputeKind {
        self.find_compute_kind(local_var_id)
            .expect("compute kind for local should exist")
    }

    pub fn insert(&mut self, local_var_id: LocalVarId, value: LocalComputeKind) {
        self.0.insert(local_var_id, value);
    }
}

#[derive(Debug)]
pub struct LocalComputeKind {
    pub local: Local,
    pub compute_kind: ComputeKind,
}
