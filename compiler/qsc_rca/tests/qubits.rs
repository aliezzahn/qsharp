// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

pub mod common;

use common::{
    check_last_statement_compute_propeties, write_compute_properties_to_files,
    write_fir_store_to_files, CompilationContext,
};
use expect_test::expect;

#[test]
fn check_rca_for_static_single_qubit_allcation() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(r#"use q = Qubit();"#);
    let package_store_compute_properties = compilation_context.get_compute_properties();
    write_fir_store_to_files(&compilation_context.fir_store);
    write_compute_properties_to_files(package_store_compute_properties);
    check_last_statement_compute_propeties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsTable:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(0x0)
                    value_kind: Static
                dynamic_param_applications: <empty>"#
        ],
    );
}

#[ignore = "work in progress"] // TODO (cesarzc): Needs regular specialization analysis working exhaustively.
#[test]
fn check_rca_for_static_multi_qubit_allcation() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(r#"use q = Qubit();"#);
    let package_store_compute_properties = compilation_context.get_compute_properties();
    write_fir_store_to_files(&compilation_context.fir_store);
    write_compute_properties_to_files(package_store_compute_properties);
    //check_last_statement_compute_propeties(package_store_compute_properties, &expect![r#""#]);
}