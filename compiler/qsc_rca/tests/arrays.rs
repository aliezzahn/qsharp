// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

pub mod common;

use common::{check_last_statement_compute_propeties, CompilationContext};
use expect_test::expect;

#[test]
fn check_rca_for_array_with_classical_elements() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(r#"[1.0, 2.0, 3.0, 4.0, 5.0]"#);
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_propeties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsTable:
                inherent: Classical
                dynamic_param_applications: <empty>"#
        ],
    );
}

#[test]
fn check_rca_for_array_with_dynamic_results() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        use (a, b, c) = (Qubit(), Qubit(), Qubit());
        [M(a), M(b), M(c)]"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    // Even though results are dynamic, they do not require any special runtime features to exist.
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

#[test]
fn check_rca_for_array_with_dynamic_bools() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        open Microsoft.Quantum.Convert;
        use (a, b, c) = (Qubit(), Qubit(), Qubit());
        [ResultAsBool(M(a)), ResultAsBool(M(b)), ResultAsBool(M(c))]"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_propeties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsTable:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(UseOfDynamicBool)
                    value_kind: Static
                dynamic_param_applications: <empty>"#
        ],
    );
}

#[test]
fn check_rca_for_array_repeat_with_classical_value_and_classical_size() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(r#"[1L, size = 11]"#);
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_propeties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsTable:
                inherent: Classical
                dynamic_param_applications: <empty>"#
        ],
    );
}

#[test]
fn check_rca_for_array_repeat_with_dynamic_result_value_and_classical_size() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        use q = Qubit();
        [M(q), size = 11]"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
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

#[test]
fn check_rca_for_array_repeat_with_dynamic_bool_value_and_classical_size() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        open Microsoft.Quantum.Convert;
        use q = Qubit();
        [ResultAsBool(M(q)), size = 11]"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_propeties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsTable:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(UseOfDynamicBool)
                    value_kind: Static
                dynamic_param_applications: <empty>"#
        ],
    );
}

#[test]
fn check_rca_for_array_repeat_with_classical_value_and_dynamic_size() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        use q = Qubit();
        let s = M(q) == Zero ? 5 | 10;
        [Zero, size = s]"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_propeties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsTable:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(UseOfDynamicBool | UseOfDynamicInt | UseOfDynamicArray)
                    value_kind: Dynamic
                dynamic_param_applications: <empty>"#
        ],
    );
}

#[test]
fn check_rca_for_array_repeat_with_dynamic_double_value_and_dynamic_size() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        open Microsoft.Quantum.Convert;
        use q = Qubit();
        let r = M(q);
        let s = r == Zero ? 5 | 10;
        let d = IntAsDouble(s);
        [d, size = s]"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_propeties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsTable:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(UseOfDynamicBool | UseOfDynamicInt | UseOfDynamicDouble | UseOfDynamicArray)
                    value_kind: Dynamic
                dynamic_param_applications: <empty>"#
        ],
    );
}