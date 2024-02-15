// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

pub mod common;

use common::{
    check_callable_compute_properties, check_last_statement_compute_propeties, CompilationContext,
};
use expect_test::expect;

#[test]
fn check_rca_for_if_stmt_with_classic_condition_and_classic_if_true_block() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        open Microsoft.Quantum.Math;
        operation Foo() : Unit {
            if true {
                let s = Sqrt(4.0);
            }
        }"#,
    );
    check_callable_compute_properties(
        &compilation_context.fir_store,
        compilation_context.get_compute_properties(),
        "Foo",
        &expect![
            r#"
            Callable: CallableComputeProperties:
                body: ApplicationsTable:
                    inherent: Classical
                    dynamic_param_applications: <empty>
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_if_stmt_with_dynamic_condition_and_classic_if_true_block() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        open Microsoft.Quantum.Math;
        operation Foo() : Unit {
            use q = Qubit();
            if M(q) == Zero {
                let s = Sqrt(4.0);
            }
        }"#,
    );
    check_callable_compute_properties(
        &compilation_context.fir_store,
        compilation_context.get_compute_properties(),
        "Foo",
        &expect![
            r#"
            Callable: CallableComputeProperties:
                body: ApplicationsTable:
                    inherent: Quantum: QuantumProperties:
                        runtime_features: RuntimeFeatureFlags(UseOfDynamicBool | ForwardBranchingOnDynamicValue)
                        value_kind: Static
                    dynamic_param_applications: <empty>
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_if_else_expr_with_classic_condition_and_classic_branch_blocks() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        let i = if true {
            1
        } else {
            0
        };
        i"#,
    );
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
fn check_rca_for_if_else_expr_with_dynamic_condition_and_classic_branch_blocks() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        use q = Qubit();
        let i = if M(q) == One {
            1
        } else {
            0
        };
        i"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_propeties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsTable:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(UseOfDynamicBool | UseOfDynamicInt)
                    value_kind: Dynamic
                dynamic_param_applications: <empty>"#
        ],
    );
}
