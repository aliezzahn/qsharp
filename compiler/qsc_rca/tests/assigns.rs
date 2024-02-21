// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

pub mod common;

use common::{check_last_statement_compute_propeties, CompilationContext};
use expect_test::expect;

#[test]
fn check_rca_for_classical_int_assign_to_local() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        mutable i = 0;
        set i = 1;
        i"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_propeties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsGenerator:
                inherent: Classical
                dynamic_param_applications: <empty>"#
        ],
    );
}

#[test]
fn check_rca_for_dynamic_result_assign_to_local() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        use q = Qubit();
        mutable r = Zero;
        set r = M(q);
        r"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_propeties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsGenerator:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(0x0)
                    value_kind: Dynamic
                dynamic_param_applications: <empty>"#
        ],
    );
}

#[test]
fn check_rca_for_dynamic_bool_assign_to_local() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        open Microsoft.Quantum.Convert;
        use q = Qubit();
        mutable b = false;
        set b = ResultAsBool(M(q));
        b"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_propeties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsGenerator:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(UseOfDynamicBool)
                    value_kind: Dynamic
                dynamic_param_applications: <empty>"#
        ],
    );
}
