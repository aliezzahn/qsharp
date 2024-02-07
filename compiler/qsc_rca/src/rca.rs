// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use crate::{
    common::{
        derive_callable_input_params, derive_specialization_input_params, initalize_locals_map,
        GlobalSpecializationId, InputParam, InputParamIndex, Local, LocalKind, SpecializationKind,
    },
    scaffolding::{ItemScaffolding, PackageScaffolding, PackageStoreScaffolding},
    ApplicationsTable, ComputeProperties, ComputePropertiesLookup, DynamismSource,
    RuntimeFeatureFlags,
};
use qsc_fir::fir::{BlockId, ExprId, NodeId, StmtId, StmtKind, StoreBlockId};
use qsc_fir::{
    fir::{
        CallableDecl, CallableImpl, CallableKind, Global, PackageId, PackageStore,
        PackageStoreLookup, SpecDecl, StoreItemId, StoreStmtId,
    },
    ty::{Prim, Ty},
};
use rustc_hash::{FxHashMap, FxHashSet};

/// An instance of a callable application.
#[derive(Debug, Default)]
struct ApplicationInstance {
    /// A map of locals with their associated compute properties.
    pub locals_map: FxHashMap<NodeId, LocalComputeProperties>,
    /// The currently active dynamic scopes in the application instance.
    pub active_dynamic_scopes: Vec<ExprId>,
    /// The return expressions througout the application instance.
    pub return_expressions: Vec<ExprId>,
    /// The compute properties of the blocks related to the application instance.
    pub blocks: FxHashMap<BlockId, ComputeProperties>,
    /// The compute properties of the statements related to the application instance.
    pub stmts: FxHashMap<StmtId, ComputeProperties>,
    /// The compute properties of the expressions related to the application instance.
    pub exprs: FxHashMap<ExprId, ComputeProperties>,
    /// Whether the application instance analysis has been completed.
    /// This is used to verify that its contents are not used in a partial state.
    is_settled: bool,
    /// Whether the application instance's compute properties has been flushed.
    /// This is used to verify that its contents are not used in a partial state.
    was_flushed: bool,
}

impl ApplicationInstance {
    fn new(input_params: &Vec<InputParam>, dynamic_param_index: Option<InputParamIndex>) -> Self {
        let mut unprocessed_locals_map = initalize_locals_map(input_params);
        let mut locals_map = FxHashMap::default();
        for (node_id, local) in unprocessed_locals_map.drain() {
            let LocalKind::InputParam(input_param_index) = local.kind else {
                panic!("only input parameters are expected");
            };

            // If a dynamic parameter index is provided, set the local compute properties as dynamic.
            let dynamism_sources = if let Some(dynamic_param_index) = dynamic_param_index {
                if input_param_index == dynamic_param_index {
                    FxHashSet::from_iter(vec![DynamismSource::Assumed])
                } else {
                    FxHashSet::default()
                }
            } else {
                FxHashSet::default()
            };
            let local_compute_properties = LocalComputeProperties {
                local,
                compute_properties: ComputeProperties {
                    runtime_features: RuntimeFeatureFlags::empty(),
                    dynamism_sources,
                },
            };
            locals_map.insert(node_id, local_compute_properties);
        }
        Self {
            locals_map,
            active_dynamic_scopes: Vec::new(),
            return_expressions: Vec::new(),
            blocks: FxHashMap::default(),
            stmts: FxHashMap::default(),
            exprs: FxHashMap::default(),
            is_settled: false,
            was_flushed: false,
        }
    }

    fn aggregate_return_expressions(&mut self) -> FxHashSet<DynamismSource> {
        // Cannot aggregate return expressions until the application instance has been settled, but not yet flushed.
        assert!(self.is_settled);
        assert!(!self.was_flushed);
        let mut dynamism_sources = FxHashSet::default();
        for expr_id in self.return_expressions.drain(..) {
            let expr_compute_properties = self
                .exprs
                .get(&expr_id)
                .expect("expression compute properties should exist");
            if !expr_compute_properties.dynamism_sources.is_empty() {
                dynamism_sources.insert(DynamismSource::Expr(expr_id));
            }
        }
        dynamism_sources
    }

    fn clear_locals(&mut self) {
        // Cannot clear locals until the application instance has been settled.
        assert!(self.is_settled);
        self.locals_map.clear();
    }

    fn mark_flushed(&mut self) {
        // Cannot mark as flushed until the application instance has been settled, no return expressions remain and all
        // compute properties maps are empty.
        assert!(self.is_settled);
        assert!(self.return_expressions.is_empty());
        assert!(self.blocks.is_empty());
        assert!(self.stmts.is_empty());
        assert!(self.exprs.is_empty());
        self.was_flushed = true;
    }

    fn settle(&mut self) {
        // Cannot settle an application instance while there are active dynamic scopes.
        assert!(self.active_dynamic_scopes.is_empty());
        self.is_settled = true;
    }
}

#[derive(Debug)]
struct SpecApplicationInstances {
    pub inherent: ApplicationInstance,
    pub dynamic_params: Vec<ApplicationInstance>,
    is_settled: bool,
}

impl SpecApplicationInstances {
    pub fn new(input_params: &Vec<InputParam>) -> Self {
        let inherent = ApplicationInstance::new(input_params, None);
        let mut dynamic_params = Vec::<ApplicationInstance>::with_capacity(input_params.len());
        for input_param in input_params {
            let application_instance =
                ApplicationInstance::new(input_params, Some(input_param.index));
            dynamic_params.push(application_instance);
        }

        Self {
            inherent,
            dynamic_params,
            is_settled: false,
        }
    }

    pub fn close(
        &mut self,
        main_block_id: BlockId,
        package_scaffolding: &mut PackageScaffolding,
    ) -> ApplicationsTable {
        // We can close only if this structure is not yet settled and if all the internal application instances are
        // already settled.
        assert!(!self.is_settled);
        assert!(self.inherent.is_settled);
        self.dynamic_params
            .iter()
            .for_each(|application_instance| assert!(application_instance.is_settled));

        // Clear the locals since they are no longer needed.
        self.clear_locals();

        // Initialize the applications table and aggregate the return expressions to it.
        let mut applications_table = ApplicationsTable::new(self.dynamic_params.len());
        self.aggregate_return_expressions(&mut applications_table);

        // Flush the compute properties to the package scaffolding
        self.flush_compute_properties(package_scaffolding);

        // Get the applications table of the main block and aggregate its runtime features.
        let main_block_applications_table = package_scaffolding
            .blocks
            .get(main_block_id)
            .expect("block applications table should exist");
        applications_table.aggregate_runtime_features(main_block_applications_table);

        // Mark the struct as settled and return the applications table that represents it.
        self.is_settled = true;
        applications_table
    }

    fn aggregate_return_expressions(&mut self, applications_table: &mut ApplicationsTable) {
        assert!(self.dynamic_params.len() == applications_table.dynamic_params_properties.len());
        let inherent_dynamism_sources = self.inherent.aggregate_return_expressions();
        applications_table
            .inherent_properties
            .dynamism_sources
            .extend(inherent_dynamism_sources);
        for (param_compute_properties, application_instance) in applications_table
            .dynamic_params_properties
            .iter_mut()
            .zip(self.dynamic_params.iter_mut())
        {
            let dynamism_sources = application_instance.aggregate_return_expressions();
            param_compute_properties
                .dynamism_sources
                .extend(dynamism_sources);
        }
    }

    fn clear_locals(&mut self) {
        self.inherent.clear_locals();
        self.dynamic_params
            .iter_mut()
            .for_each(|application_instance| application_instance.clear_locals());
    }

    fn flush_compute_properties(&mut self, package_scaffolding: &mut PackageScaffolding) {
        let input_params_count = self.dynamic_params.len();

        // Flush blocks.
        for (block_id, inherent_properties) in self.inherent.blocks.drain() {
            let mut dynamic_params_properties =
                Vec::<ComputeProperties>::with_capacity(input_params_count);
            for application_instance in self.dynamic_params.iter_mut() {
                let block_compute_properties = application_instance
                    .blocks
                    .remove(&block_id)
                    .expect("block should exist in application instance");
                dynamic_params_properties.push(block_compute_properties);
            }
            let block_applications_table = ApplicationsTable {
                inherent_properties,
                dynamic_params_properties,
            };
            package_scaffolding
                .blocks
                .insert(block_id, block_applications_table);
        }

        // Flush statements.
        for (stmt_id, inherent_properties) in self.inherent.stmts.drain() {
            let mut dynamic_params_properties =
                Vec::<ComputeProperties>::with_capacity(input_params_count);
            for application_instance in self.dynamic_params.iter_mut() {
                let stmt_compute_properties = application_instance
                    .stmts
                    .remove(&stmt_id)
                    .expect("statement should exist in application instance");
                dynamic_params_properties.push(stmt_compute_properties);
            }
            let stmt_applications_table = ApplicationsTable {
                inherent_properties,
                dynamic_params_properties,
            };
            package_scaffolding
                .stmts
                .insert(stmt_id, stmt_applications_table);
        }

        // Flush expressions.
        for (expr_id, inherent_properties) in self.inherent.exprs.drain() {
            let mut dynamic_params_properties =
                Vec::<ComputeProperties>::with_capacity(input_params_count);
            for application_instance in self.dynamic_params.iter_mut() {
                let expr_compute_properties = application_instance
                    .exprs
                    .remove(&expr_id)
                    .expect("statement should exist in application instance");
                dynamic_params_properties.push(expr_compute_properties);
            }
            let expr_applications_table = ApplicationsTable {
                inherent_properties,
                dynamic_params_properties,
            };
            package_scaffolding
                .exprs
                .insert(expr_id, expr_applications_table);
        }

        // Mark individual application instances as flushed.
        self.inherent.mark_flushed();
        self.dynamic_params
            .iter_mut()
            .for_each(|application_instance| application_instance.mark_flushed());
    }
}

#[derive(Debug)]
struct LocalComputeProperties {
    pub local: Local,
    pub compute_properties: ComputeProperties,
}

/// Performs runtime capabilities analysis (RCA) on a package.
/// N.B. This function assumes specializations that are part of call cycles have already been analyzed. Otherwise, this
/// function will get stuck in an infinite analysis loop.
pub fn analyze_package(
    id: PackageId,
    package_store: &PackageStore,
    package_store_scaffolding: &mut PackageStoreScaffolding,
) {
    let package = package_store.get(id).expect("package should exist");

    // Analyze all top-level items.
    for (item_id, _) in &package.items {
        analyze_item(
            (id, item_id).into(),
            package_store,
            package_store_scaffolding,
        );
    }

    // By this point, only top-level statements remain unanalyzed.
    for (stmt_id, _) in &package.stmts {
        analyze_statement(
            (id, stmt_id).into(),
            package_store,
            package_store_scaffolding,
        );
    }
}

/// Performs runtime capabilities analysis (RCA) on a specialization that is part of a callable cycle.
pub fn analyze_specialization_with_cyles(
    specialization_id: GlobalSpecializationId,
    package_store: &PackageStore,
    package_store_scaffolding: &mut PackageStoreScaffolding,
) {
    // This function is only called when a specialization has not already been analyzed.
    assert!(package_store_scaffolding
        .find_specialization(specialization_id)
        .is_none());
    let Some(Global::Callable(callable)) = package_store.get_global(specialization_id.callable)
    else {
        panic!("global item should exist and it should be a global");
    };

    let CallableImpl::Spec(spec_impl) = &callable.implementation else {
        panic!("callable implementation should not be intrinsic");
    };

    // Use the correct specialization declaration.
    let spec_decl = match specialization_id.specialization {
        SpecializationKind::Body => &spec_impl.body,
        SpecializationKind::Adj => spec_impl
            .adj
            .as_ref()
            .expect("adj specialization should exist"),
        SpecializationKind::Ctl => spec_impl
            .ctl
            .as_ref()
            .expect("ctl specialization should exist"),
        SpecializationKind::CtlAdj => spec_impl
            .ctl_adj
            .as_ref()
            .expect("ctl_adj specializatiob should exist"),
    };

    let input_params = derive_callable_input_params(
        callable,
        &package_store
            .get(specialization_id.callable.package)
            .expect("package should exist")
            .pats,
    );

    // Create compute properties differently depending on whether the callable is a function or an operation.
    let applications_table = match callable.kind {
        CallableKind::Function => create_cycled_function_specialization_applications_table(
            spec_decl,
            input_params.len(),
            &callable.output,
        ),
        CallableKind::Operation => create_cycled_operation_specialization_applications_table(
            spec_decl,
            input_params.len(),
            &callable.output,
        ),
    };

    // Finally, update the package store scaffolding.
    package_store_scaffolding.insert_spec(specialization_id, applications_table);
}

fn analyze_callable(
    id: StoreItemId,
    callable: &CallableDecl,
    package_store: &PackageStore,
    package_store_scaffolding: &mut PackageStoreScaffolding,
) {
    // Analyze the callable depending on its type.
    let input_params = derive_callable_input_params(
        callable,
        &package_store
            .get(id.package)
            .expect("package should exist")
            .pats,
    );
    match callable.implementation {
        CallableImpl::Intrinsic => {
            analyze_intrinsic_callable(id, callable, &input_params, package_store_scaffolding)
        }
        CallableImpl::Spec(_) => analyze_non_intrinsic_callable(
            id,
            callable,
            &input_params,
            package_store,
            package_store_scaffolding,
        ),
    }
}

fn analyze_intrinsic_callable(
    id: StoreItemId,
    callable: &CallableDecl,
    input_params: &Vec<InputParam>,
    package_store_scaffolding: &mut PackageStoreScaffolding,
) {
    // If an entry for the specialization already exists, there is nothing left to do. Note that intrinsic callables
    // only have a body specialization.
    let body_specialization_id = GlobalSpecializationId::from((id, SpecializationKind::Body));
    if package_store_scaffolding
        .find_specialization(body_specialization_id)
        .is_some()
    {
        return;
    }

    // This function is meant for instrinsic callables only.
    assert!(matches!(callable.implementation, CallableImpl::Intrinsic));

    // Create an applications table depending on whether the callable is a function or an operation.
    let applications_table = match callable.kind {
        CallableKind::Function => {
            create_intrinsic_function_applications_table(callable, input_params)
        }
        CallableKind::Operation => {
            create_instrinsic_operation_applications_table(callable, input_params)
        }
    };
    package_store_scaffolding.insert_spec(body_specialization_id, applications_table);
}

fn analyze_item(
    id: StoreItemId,
    package_store: &PackageStore,
    package_store_scaffolding: &mut PackageStoreScaffolding,
) {
    if let Some(Global::Callable(callable)) = package_store.get_global(id) {
        analyze_callable(id, callable, package_store, package_store_scaffolding);
    } else {
        package_store_scaffolding.insert_item(id, ItemScaffolding::NonCallable);
    }
}

fn analyze_non_intrinsic_callable(
    id: StoreItemId,
    callable: &CallableDecl,
    input_params: &Vec<InputParam>,
    package_store: &PackageStore,
    package_store_scaffolding: &mut PackageStoreScaffolding,
) {
    // This function is not meant for instrinsics.
    let CallableImpl::Spec(implementation) = &callable.implementation else {
        panic!("callable is assumed to have a specialized implementation");
    };

    // Analyze each one of the specializations.
    analyze_specialization(
        id,
        SpecializationKind::Body,
        &implementation.body,
        input_params,
        package_store,
        package_store_scaffolding,
    );

    if let Some(adj_spec) = &implementation.adj {
        analyze_specialization(
            id,
            SpecializationKind::Adj,
            adj_spec,
            input_params,
            package_store,
            package_store_scaffolding,
        );
    }

    if let Some(ctl_spec) = &implementation.ctl {
        analyze_specialization(
            id,
            SpecializationKind::Ctl,
            ctl_spec,
            input_params,
            package_store,
            package_store_scaffolding,
        );
    }

    if let Some(ctl_adj_spec) = &implementation.ctl_adj {
        analyze_specialization(
            id,
            SpecializationKind::CtlAdj,
            ctl_adj_spec,
            input_params,
            package_store,
            package_store_scaffolding,
        );
    }
}

fn analyze_specialization(
    callable_id: StoreItemId,
    spec_kind: SpecializationKind,
    spec_decl: &SpecDecl,
    callable_input_params: &Vec<InputParam>,
    package_store: &PackageStore,
    package_store_scaffolding: &mut PackageStoreScaffolding,
) {
    // If an entry for the specialization already exists, there is nothing left to do.
    let specialization_id = GlobalSpecializationId::from((callable_id, spec_kind));
    if package_store_scaffolding
        .find_specialization(specialization_id)
        .is_some()
    {
        return;
    }

    // We expand the input map for controlled specializations, which have its own additional input (the control qubit
    // register).
    let package_patterns = &package_store
        .get(callable_id.package)
        .expect("package should exist")
        .pats;

    // Derive the input parameters for the specialization, which can be different from the callable input parameters
    // if the specialization has its own input.
    let specialization_input_params =
        derive_specialization_input_params(spec_decl, callable_input_params, package_patterns);

    // Then we analyze the block which implements the specialization by simulating callable applications.
    let block_id = (callable_id.package, spec_decl.block).into();
    let mut spec_application_instances =
        SpecApplicationInstances::new(&specialization_input_params);

    // First, we simulate the inherent application, in which all arguments are static.
    simulate_block_application_instance(
        block_id,
        &mut spec_application_instances.inherent,
        package_store,
        package_store_scaffolding,
    );
    spec_application_instances.inherent.settle();

    // Then, we simulate an application for each imput parameter, in which we consider it dynamic.
    for application_instance in spec_application_instances.dynamic_params.iter_mut() {
        simulate_block_application_instance(
            block_id,
            application_instance,
            package_store,
            package_store_scaffolding,
        );
        application_instance.settle();
    }

    // Now that we have all the application instances for the block that implements the specialization, we can close the
    // application instances for the specialization, which will save all the analysis to the package store scaffolding
    // and will return the applications table corresponding to the specialization.
    let package_scaffolding = package_store_scaffolding
        .get_mut(callable_id.package)
        .expect("package scaffolding should exist");
    let specialization_applications_table =
        spec_application_instances.close(block_id.block, package_scaffolding);

    // Finally, we insert the applications table to the scaffolding data structure.
    package_store_scaffolding.insert_spec(specialization_id, specialization_applications_table);
}

fn analyze_statement(
    id: StoreStmtId,
    package_store: &PackageStore,
    package_store_scaffolding: &mut PackageStoreScaffolding,
) {
    // If the item has already been analyzed, there's nothing left to do.
    if package_store_scaffolding.find_stmt(id).is_some() {
        return;
    }

    let _stmt = package_store.get_stmt(id);

    // TODO (cesarzc): Implement.
}

fn create_cycled_function_specialization_applications_table(
    spec_decl: &SpecDecl,
    callable_input_params_count: usize,
    output_type: &Ty,
) -> ApplicationsTable {
    // Functions can only have a body specialization, which does not have its input.
    assert!(spec_decl.input.is_none());

    // Since functions are classically pure, they inherently do not use any runtime feature nor represent a source of
    // dynamism.
    let inherent_properties = ComputeProperties::empty();

    // Create compute properties for each dynamic parameter.
    let mut dynamic_params_properties = Vec::new();
    for _ in 0..callable_input_params_count {
        // If any parameter is dynamic, we assume a function with cycles is a a source of dynamism if its output type
        // is non-unit.
        let dynamism_sources = if *output_type == Ty::UNIT {
            FxHashSet::default()
        } else {
            FxHashSet::from_iter(vec![DynamismSource::Assumed])
        };

        // Since convert functions can be called with dynamic parameters, we assume that all capabilities are required
        // for any dynamic parameter. The `CycledFunctionWithDynamicArg` feature conveys this assumption.
        let compute_properties = ComputeProperties {
            runtime_features: RuntimeFeatureFlags::CycledFunctionApplicationUsesDynamicArg,
            dynamism_sources,
        };
        dynamic_params_properties.push(compute_properties);
    }

    ApplicationsTable {
        inherent_properties,
        dynamic_params_properties,
    }
}

fn create_cycled_operation_specialization_applications_table(
    spec_decl: &SpecDecl,
    callable_input_params_count: usize,
    output_type: &Ty,
) -> ApplicationsTable {
    // Since operations can allocate and measure qubits freely, we assume it requires all capabilities (encompassed by
    // the `CycledOperationSpecialization` runtime feature) and that they are a source of dynamism if they have a
    // non-unit output.
    let dynamism_sources = if *output_type == Ty::UNIT {
        FxHashSet::default()
    } else {
        FxHashSet::from_iter(vec![DynamismSource::Assumed])
    };
    let compute_properties = ComputeProperties {
        runtime_features: RuntimeFeatureFlags::CycledOperationSpecializationApplication,
        dynamism_sources,
    };

    // If the specialization has its own input, then the number of input params needs to be increased by one.
    let specialization_input_params_count = if spec_decl.input.is_some() {
        callable_input_params_count + 1
    } else {
        callable_input_params_count
    };

    // Create compute properties for each dynamic parameter. These compute properties are the same than the inherent
    // properties.
    let mut dynamic_params_properties = Vec::new();
    for _ in 0..specialization_input_params_count {
        dynamic_params_properties.push(compute_properties.clone());
    }

    // Finally, create the applications table.
    ApplicationsTable {
        inherent_properties: compute_properties,
        dynamic_params_properties,
    }
}

fn create_intrinsic_function_applications_table(
    callable_decl: &CallableDecl,
    input_params: &Vec<InputParam>,
) -> ApplicationsTable {
    assert!(matches!(callable_decl.kind, CallableKind::Function));

    // Functions are purely classical, so no runtime features are needed and cannot be an inherent dynamism source.
    let inherent_properties = ComputeProperties::empty();

    // Calculate the properties for all parameters.
    let mut dynamic_params_properties = Vec::new();
    for param in input_params {
        // For intrinsic functions, we assume any parameter can contribute to the output, so if any parameter is dynamic
        // the output of the function is dynamic. Therefore, for all dynamic parameters, if the function's output is
        // non-unit:
        // - It becomes a source of dynamism.
        // - The output type contributes to the runtime features used by the function.
        let (dynamism_sources, mut runtime_features) = if callable_decl.output == Ty::UNIT {
            (FxHashSet::default(), RuntimeFeatureFlags::empty())
        } else {
            (
                FxHashSet::from_iter(vec![DynamismSource::Intrinsic]),
                derive_intrinsic_runtime_features_from_type(&callable_decl.output),
            )
        };

        // When a parameter is binded to a dynamic value, its type contributes to the runtime features used by the
        // function.
        runtime_features |= derive_intrinsic_runtime_features_from_type(&param.ty);
        let param_compute_properties = ComputeProperties {
            runtime_features,
            dynamism_sources,
        };
        dynamic_params_properties.push(param_compute_properties);
    }

    ApplicationsTable {
        inherent_properties,
        dynamic_params_properties,
    }
}

fn create_instrinsic_operation_applications_table(
    callable_decl: &CallableDecl,
    input_params: &Vec<InputParam>,
) -> ApplicationsTable {
    assert!(matches!(callable_decl.kind, CallableKind::Operation));

    // Intrinsic operations inherently use runtime features if their output is not `Unit`, `Qubit` or `Result`, and
    // these runtime features are derived from the output type.
    let runtime_features = if callable_decl.output == Ty::UNIT
        || callable_decl.output == Ty::Prim(Prim::Qubit)
        || callable_decl.output == Ty::Prim(Prim::Result)
    {
        RuntimeFeatureFlags::empty()
    } else {
        derive_intrinsic_runtime_features_from_type(&callable_decl.output)
    };

    // Intrinsic are an inherent source of dynamism if their output is not `Unit` or `Qubit`.
    let dynamism_sources =
        if callable_decl.output == Ty::UNIT || callable_decl.output == Ty::Prim(Prim::Qubit) {
            FxHashSet::default()
        } else {
            FxHashSet::from_iter(vec![DynamismSource::Intrinsic])
        };

    // Build the inherent properties.
    let inherent_properties = ComputeProperties {
        runtime_features,
        dynamism_sources,
    };

    // Calculate the properties for all dynamic parameters.
    let mut dynamic_params_properties = Vec::new();
    for param in input_params {
        // For intrinsic operations, we assume any parameter can contribute to the output, so if any parameter is
        // dynamic the output of the operation is dynamic. Therefore, this operation becomes a source of dynamism for
        // all dynamic params if its output is not `Unit`.
        let dynamism_sources = if callable_decl.output == Ty::UNIT {
            FxHashSet::default()
        } else {
            FxHashSet::from_iter(vec![DynamismSource::Intrinsic])
        };

        // When a parameter is binded to a dynamic value, its runtime features depend on the parameter type.
        let param_compute_properties = ComputeProperties {
            runtime_features: derive_intrinsic_runtime_features_from_type(&param.ty),
            dynamism_sources,
        };
        dynamic_params_properties.push(param_compute_properties);
    }

    ApplicationsTable {
        inherent_properties,
        dynamic_params_properties,
    }
}

fn derive_intrinsic_runtime_features_from_type(ty: &Ty) -> RuntimeFeatureFlags {
    fn intrinsic_runtime_features_from_primitive_type(prim: &Prim) -> RuntimeFeatureFlags {
        match prim {
            Prim::BigInt => RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicBigInt,
            Prim::Bool => RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicBool,
            Prim::Double => RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicDouble,
            Prim::Int => RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicInt,
            Prim::Pauli => RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicPauli,
            Prim::Qubit => RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicQubit,
            Prim::Range | Prim::RangeFrom | Prim::RangeTo | Prim::RangeFull => {
                RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicRange
            }
            Prim::Result => RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicResult,
            Prim::String => RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicString,
        }
    }

    fn intrinsic_runtime_features_from_tuple(tuple: &Vec<Ty>) -> RuntimeFeatureFlags {
        let mut runtime_features = if tuple.is_empty() {
            RuntimeFeatureFlags::empty()
        } else {
            RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicTuple
        };
        for item_type in tuple {
            runtime_features |= derive_intrinsic_runtime_features_from_type(item_type);
        }
        runtime_features
    }

    match ty {
        Ty::Array(_) => RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicArray,
        Ty::Arrow(arrow) => match arrow.kind {
            CallableKind::Function => {
                RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicArrowFunction
            }
            CallableKind::Operation => {
                RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicArrowOperation
            }
        },
        Ty::Prim(prim) => intrinsic_runtime_features_from_primitive_type(prim),
        Ty::Tuple(tuple) => intrinsic_runtime_features_from_tuple(tuple),
        Ty::Udt(_) => RuntimeFeatureFlags::IntrinsicApplicationUsesDynamicUdt,
        _ => panic!("unexpected type"),
    }
}

fn simulate_block_application_instance(
    id: StoreBlockId,
    application_instance: &mut ApplicationInstance,
    package_store: &PackageStore,
    package_store_scaffolding: &mut PackageStoreScaffolding,
) {
    // This function is only called when a block has not already been analyzed.
    if package_store_scaffolding.find_block(id).is_some() {
        panic!("block is already analyzed");
    }

    // Initialize the compute properties of the block.
    let block = package_store.get_block(id);
    let mut block_compute_properties = ComputeProperties::empty();

    // Iterate through the block statements and aggregate the runtime features of each into the block compute properties.
    for (stmt_index, stmt_id) in block.stmts.iter().enumerate() {
        let store_stmt_id = StoreStmtId::from((id.package, *stmt_id));
        simulate_stmt_application_instance(
            store_stmt_id,
            application_instance,
            package_store,
            package_store_scaffolding,
        );
        let stmt_compute_properties = application_instance
            .stmts
            .get(stmt_id)
            .expect("statement compute properties should exist");
        block_compute_properties.runtime_features |= stmt_compute_properties.runtime_features;

        // If this is the last statement and it is a non-unit expression without a trailing semicolon, aggregate it to
        // the block dynamism sources since the statement represents the block "return" value.
        if stmt_index == block.stmts.len() - 1 {
            let stmt = package_store.get_stmt((id.package, *stmt_id).into());
            if let StmtKind::Expr(expr_id) = stmt.kind {
                let expr = package_store.get_expr((id.package, expr_id).into());
                if expr.ty != Ty::UNIT {
                    block_compute_properties
                        .dynamism_sources
                        .insert(DynamismSource::Expr(expr_id));
                }
            }
        }
    }

    // Finally, we insert the compute properties of the block to the application instance.
    application_instance
        .blocks
        .insert(id.block, block_compute_properties);
}

fn simulate_stmt_application_instance(
    id: StoreStmtId,
    application_instance: &mut ApplicationInstance,
    _package_store: &PackageStore,
    _package_store_scaffolding: &mut PackageStoreScaffolding,
) {
    let stmt_compute_properties = ComputeProperties::default();
    // TODO (cesarzc): implement properly.
    application_instance
        .stmts
        .insert(id.stmt, stmt_compute_properties);
}
