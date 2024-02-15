// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

pub mod common;

use common::{check_callable_compute_properties, CompilationContext};
use expect_test::expect;

#[test]
fn check_rca_for_one_function_cycle() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i : Int) : Int {
            Foo(i)
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_two_functions_cycle() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i : Int) : Int {
            Bar(i)
        }
        function Bar(i : Int) : Int {
            Foo(i)
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );

    check_callable_compute_properties(
        &compilation_context.fir_store,
        compilation_context.get_compute_properties(),
        "Bar",
        &expect![
            r#"
            Callable: CallableComputeProperties:
                body: ApplicationsTable:
                    inherent: Classical
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_three_functions_cycle() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i : Int) : Int {
            Bar(i)
        }
        function Bar(i : Int) : Int {
            Baz(i)
        }
        function Baz(i : Int) : Int {
            Foo(i)
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
    check_callable_compute_properties(
        &compilation_context.fir_store,
        compilation_context.get_compute_properties(),
        "Bar",
        &expect![
            r#"
            Callable: CallableComputeProperties:
                body: ApplicationsTable:
                    inherent: Classical
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
    check_callable_compute_properties(
        &compilation_context.fir_store,
        compilation_context.get_compute_properties(),
        "Baz",
        &expect![
            r#"
            Callable: CallableComputeProperties:
                body: ApplicationsTable:
                    inherent: Classical
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_indirect_function_cycle() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i : Int) : Int {
            let f = Foo;
            f(i)
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_indirect_chain_function_cycle() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i : Int) : Int {
            let a = Foo;
            let b = a;
            let c = b;
            c(i)
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_indirect_tuple_function_cycle() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i : Int) : Int {
            let (f, _) = (Foo, 0);
            f(i)
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[ignore = "insufficient closure resolution support"]
#[test]
fn check_rca_for_indirect_closure_function_cycle() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i : Int) : Int {
            let f = () -> Foo(0);
            f()
        }"#,
    );
    check_callable_compute_properties(
        &compilation_context.fir_store,
        compilation_context.get_compute_properties(),
        "Foo",
        &expect![r#""#],
    );
}

#[ignore = "insufficient partial application resolution support"]
#[test]
fn check_rca_for_indirect_partial_appplication_function_cycle() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(b : Bool, i : Int) : Int {
            let f = Foo(false, _);
            f(0)
        }"#,
    );
    check_callable_compute_properties(
        &compilation_context.fir_store,
        compilation_context.get_compute_properties(),
        "Foo",
        &expect![r#""#],
    );
}

#[test]
fn check_rca_for_function_cycle_within_binding() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i : Int) : Int {
            let out = Foo(i);
            return out;
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_function_cycle_within_assignment() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i : Int) : Int {
            mutable out = 0;
            set out = Foo(i);
            return out;
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_function_cycle_within_return() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i : Int) : Int {
            return Foo(i);
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_function_cycle_within_tuple() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i : Int) : Int {
            let (a, b) = (Foo(0), Foo(1));
            return a + b;
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[ignore = "work in progress"] // TODO (cesarzc): Needs regular specialization analysis working exhaustively.
#[test]
fn check_rca_for_function_cycle_within_call_input() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        open Microsoft.Quantum.Arrays;
        function MySorted<'T>(comparison : (('T, 'T) -> Bool), array : 'T[]) : 'T[] {
            if Length(array) <= 1 {
                return array;
            }
            let pivotIndex = Length(array) / 2;
            let left = array[...pivotIndex - 1];
            let right = array[pivotIndex...];
            MySortedMerged(
                comparison,
                MySorted(comparison, left),
                MySorted(comparison, right)
            )
        }
        internal function MySortedMerged<'T>(comparison : (('T, 'T) -> Bool), left : 'T[], right : 'T[]) : 'T[] {
            mutable output = [];
            mutable remainingLeft = left;
            mutable remainingRight = right;
            while (not IsEmpty(remainingLeft)) and (not IsEmpty(remainingRight)) {
                if comparison(Head(remainingLeft), Head(remainingRight)) {
                    set output += [Head(remainingLeft)];
                    set remainingLeft = Rest(remainingLeft);
                } else {
                    set output += [Head(remainingRight)];
                    set remainingRight = Rest(remainingRight);
                }
            }
            output + remainingLeft + remainingRight
        }"#,
    );
    check_callable_compute_properties(
        &compilation_context.fir_store,
        compilation_context.get_compute_properties(),
        "MySorted",
        &expect![
            r#"
            Callable: CallableComputeProperties:
                body: ApplicationsTable:
                    inherent: Classical
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                        [1]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_function_cycle_within_if_block() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i: Int) : Int {
            if (i > 0) {
                Foo(i - 1)
            } else {
                0
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_function_cycle_within_if_condition() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i: Int) : Int {
            if (Foo(i) > 0) {
                1
            } else {
                0
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_function_cycle_within_for_block() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i: Int) : Int {
            for _ in 0..10 {
                Foo(i);
            }
            0
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_function_cycle_within_while_block() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i: Int) : Int {
            while true {
                Foo(i);
            }
            0
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_function_cycle_within_while_condition() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(i: Int) : Int {
            while Foo(i) > 0{
            }
            0
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_multi_param_recursive_bool_function() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(r : Result, i : Int, d: Double) : Bool {
            Foo(r, i, d)
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                        [1]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                        [2]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_multi_param_recursive_unit_function() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        function Foo(p : Pauli, s: String[], t: (Range, BigInt)) : Unit {
            Foo(p, s, t);
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
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Static
                        [1]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Static
                        [2]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledFunctionUsesDynamicArg)
                            value_kind: Static
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_result_recursive_operation() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        operation Foo(q : Qubit) : Result {
            Foo(q)
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
                        runtime_features: RuntimeFeatureFlags(CycledOperation)
                        value_kind: Dynamic
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledOperation)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_multi_param_result_recursive_operation() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        operation Foo(q : Qubit, b : Bool, i : Int, d : Double) : Result {
            Foo(q, b, i, d)
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
                        runtime_features: RuntimeFeatureFlags(CycledOperation)
                        value_kind: Dynamic
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledOperation)
                            value_kind: Dynamic
                        [1]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledOperation)
                            value_kind: Dynamic
                        [2]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledOperation)
                            value_kind: Dynamic
                        [3]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledOperation)
                            value_kind: Dynamic
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_operation_body_recursion() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        operation Foo(q : Qubit) : Unit {
            Foo(q);
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
                        runtime_features: RuntimeFeatureFlags(CycledOperation)
                        value_kind: Static
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledOperation)
                            value_kind: Static
                adj: <none>
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_operation_body_adj_recursion() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        operation Foo(q : Qubit) : Unit is Adj {
            body ... {
                Adjoint Foo(q);
            }
            adjoint ... { 
                Foo(q);
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
                        runtime_features: RuntimeFeatureFlags(CycledOperation)
                        value_kind: Static
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledOperation)
                            value_kind: Static
                adj: ApplicationsTable:
                    inherent: Quantum: QuantumProperties:
                        runtime_features: RuntimeFeatureFlags(CycledOperation)
                        value_kind: Static
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledOperation)
                            value_kind: Static
                ctl: <none>
                ctl-adj: <none>"#
        ],
    );
}

#[test]
fn check_rca_for_operation_body_ctl_recursion() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        operation Foo(q : Qubit) : Unit is Ctl {
            body ... {
                Controlled Foo([], q);
            }
            controlled (_, ...) { 
                Foo(q);
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
                        runtime_features: RuntimeFeatureFlags(CycledOperation)
                        value_kind: Static
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledOperation)
                            value_kind: Static
                adj: <none>
                ctl: ApplicationsTable:
                    inherent: Quantum: QuantumProperties:
                        runtime_features: RuntimeFeatureFlags(CycledOperation)
                        value_kind: Static
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledOperation)
                            value_kind: Static
                ctl-adj: <none>"#
        ],
    );
}

#[ignore = "work in progress"] // TODO (cesarzc): Needs regular specialization analysis working exhaustively.
#[test]
fn check_rca_for_operation_body_adj_ctl_recursion() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        operation Foo(q : Qubit) : Unit is Adj + Ctl {
            body ... {
                Adjoint Foo(q);
            }
            adjoint ... { 
                Controlled Foo([], q);
            }
            controlled (_, ...) { 
                Foo(q);
            }
        }"#,
    );
    check_callable_compute_properties(
        &compilation_context.fir_store,
        compilation_context.get_compute_properties(),
        "Foo",
        &expect![r#""#],
    );
}

#[ignore = "work in progress"] // TODO (cesarzc): Needs regular specialization analysis working exhaustively.
#[test]
fn check_rca_for_operation_adj_recursion() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        operation Foo(q : Qubit) : Unit is Adj {
            body ... {}
            adjoint ... { 
                Adjoint Foo(q);
            }
        }"#,
    );
    check_callable_compute_properties(
        &compilation_context.fir_store,
        compilation_context.get_compute_properties(),
        "Foo",
        &expect![r#""#],
    );
}

#[ignore = "work in progress"] // TODO (cesarzc): Needs regular specialization analysis working exhaustively.
#[test]
fn check_rca_for_operation_ctl_recursion() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        operation Foo(q : Qubit) : Unit is Ctl {
            body ... {}
            controlled (cs, ...) { 
                Controlled Foo(cs, q);
            }
        }"#,
    );
    check_callable_compute_properties(
        &compilation_context.fir_store,
        compilation_context.get_compute_properties(),
        "Foo",
        &expect![r#""#],
    );
}

#[ignore = "work in progress"] // TODO (cesarzc): Needs regular specialization analysis working exhaustively.
#[test]
fn check_rca_for_operation_multi_adjoint_functor_recursion() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        operation Foo(q : Qubit) : Unit is Adj {
            body ... {
                Adjoint Adjoint Foo(q);
            }
            adjoint ... {}
        }"#,
    );
    check_callable_compute_properties(
        &compilation_context.fir_store,
        compilation_context.get_compute_properties(),
        "Foo",
        &expect![r#""#],
    );
}

#[test]
fn check_rca_for_operation_multi_controlled_functor_recursion() {
    let mut compilation_context = CompilationContext::new();
    compilation_context.update(
        r#"
        operation Foo(q : Qubit) : Unit is Ctl {
            body ... {
                Controlled Controlled Foo([], ([], q));
            }
            controlled (_, ...) { 
                Foo(q);
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
                        runtime_features: RuntimeFeatureFlags(CycledOperation)
                        value_kind: Static
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledOperation)
                            value_kind: Static
                adj: <none>
                ctl: ApplicationsTable:
                    inherent: Quantum: QuantumProperties:
                        runtime_features: RuntimeFeatureFlags(CycledOperation)
                        value_kind: Static
                    dynamic_param_applications:
                        [0]: Quantum: QuantumProperties:
                            runtime_features: RuntimeFeatureFlags(CycledOperation)
                            value_kind: Static
                ctl-adj: <none>"#
        ],
    );
}