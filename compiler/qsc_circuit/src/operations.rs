// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use qsc_hir::{
    hir::CallableDecl,
    ty::{Prim, Ty},
};
use std::cell::RefCell;

#[derive(Debug)]
pub struct OperationInfo {
    pub namespace: String,
    pub name: String,
    pub internal: bool,
    pub qubit_param_dimensions: Vec<u32>,
    pub total_num_qubits: u32,
}

/// For a given operation declaration, returns the information that would
/// be needed to generate a circuit for it. This information can be contained
/// in the code lens associated with the operation, so that we can use
/// it to generate the circuit when the command is invoked.
pub fn operation_circuit_info(
    decl: &CallableDecl,
    namespace: impl Into<String>,
    internal: bool,
) -> Option<OperationInfo> {
    let (qubit_param_dimensions, total_num_qubits) = qubit_param_info(&decl.input.ty);
    if !qubit_param_dimensions.is_empty() {
        return Some(OperationInfo {
            namespace: namespace.into(),
            name: decl.name.name.to_string(),
            internal,
            qubit_param_dimensions,
            total_num_qubits,
        });
    }
    None
}

/// Generates the entry expression to call the operation described by `params`.
/// The expression allocates qubits and invokes the operation.
#[must_use]
pub fn operation_circuit_entry_expr(info: &OperationInfo) -> String {
    let alloc_qubits = format!("use qs = Qubit[{}];", info.total_num_qubits);

    let mut qs_start = 0;
    let mut call_args = vec![];
    for dim in &info.qubit_param_dimensions {
        let dim = *dim;
        let qs_len = NUM_QUBITS.pow(dim);
        // Q# ranges are end-inclusive
        let qs_end = qs_start + qs_len - 1;
        if dim == 0 {
            call_args.push(format!("qs[{qs_start}]"));
            //let _ = write!(&mut call_params, "qs[{qs_start}]");
        } else {
            // Array argument - use a range to index
            let mut call_arg = format!("qs[{qs_start}..{qs_end}]");
            for _ in 1..dim {
                // Chunk the array for multi-dimensional array arguments
                call_arg = format!("Microsoft.Quantum.Arrays.Chunks({NUM_QUBITS}, {call_arg})");
            }
            call_args.push(call_arg);
        }
        qs_start = qs_end + 1;
    }

    let call_args = call_args.join(", ");
    let namespace = &info.namespace;
    let name = &info.name;

    // We don't reset the qubits since we don't want reset gates
    // included in circuit output.
    // We also don't measure the qubits but we have to return a result
    // array to satisfy Base Profile.
    format!(
        r#"{{
            {alloc_qubits}
            {namespace}.{name}({call_args});
            let r: Result[] = [];
            r
        }}"#
    )
}

/// The number of qubits to allocate for each qubit array
/// in the operation arguments.
static NUM_QUBITS: u32 = 2;

thread_local! {
     static DISAMBIGUATOR: RefCell<u32> = RefCell::new(1);
}

/// If `Ty` consists of all qubit parameters (including qubit arrays):
///
/// The first element of the return tuple is a vector,
/// where each element corresponds to a parameter, and the
/// value is the number of dimensions of the parameter.
///
/// For example, for input parameters
/// `(Qubit, Qubit[][], Qubit[])` returns `vec![0, 2, 1]`.
///
/// The second element of the return tuple is the total number of qubits that would
/// need be allocated to run this operation for the purposes of circuit generation.
///
/// If `Ty` contains any non-qubit parameters, returns an empty vector.
fn qubit_param_info(input: &Ty) -> (Vec<u32>, u32) {
    match input {
        Ty::Prim(Prim::Qubit) => return (vec![0], 1),
        Ty::Array(ty) => {
            if let Some(element_dim) = get_array_dimension(ty) {
                let dim = element_dim + 1;
                return (vec![dim], NUM_QUBITS.pow(dim));
            }
        }
        Ty::Tuple(tys) => {
            let params = tys.iter().map(get_array_dimension).collect::<Vec<_>>();

            if params.iter().all(Option::is_some) {
                return params.into_iter().map(Option::unwrap).fold(
                    (vec![], 0),
                    |(mut dims, mut total_qubits), dim| {
                        dims.push(dim);
                        total_qubits += NUM_QUBITS.pow(dim);
                        (dims, total_qubits)
                    },
                );
            }
        }
        _ => {}
    }
    (vec![], 0)
}

/// If `Ty` is a qubit or a qubit array, returns the number of dimensions of the array.
/// A qubit is considered to be a 0-dimensional array.
/// For example, for a `Qubit` it returns `Some(0)`, for a `Qubit[][]` it returns `Some(2)`.
/// For a non-qubit type, returns `None`.
fn get_array_dimension(input: &Ty) -> Option<u32> {
    match input {
        Ty::Prim(Prim::Qubit) => Some(0),
        Ty::Array(ty) => get_array_dimension(ty).map(|d| d + 1),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;
    use qsc_data_structures::language_features::LanguageFeatures;
    use qsc_frontend::compile::{
        compile, core, std, PackageStore, RuntimeCapabilityFlags, SourceMap,
    };
    use qsc_hir::hir::ItemKind;
    use std::rc::Rc;

    fn compile_one_operation(code: &str) -> (CallableDecl, Rc<str>) {
        let core_pkg = core();
        let mut store = PackageStore::new(core_pkg);
        let std = std(&store, RuntimeCapabilityFlags::empty());
        let std = store.insert(std);

        let sources = SourceMap::new([("test".into(), code.into())], None);
        let unit = compile(
            &store,
            &[std],
            sources,
            RuntimeCapabilityFlags::empty(),
            LanguageFeatures::default(),
        );
        let mut callables = unit.package.items.values().filter_map(|i| {
            if let ItemKind::Callable(decl) = &i.kind {
                Some(decl)
            } else {
                None
            }
        });
        let mut namespaces = unit.package.items.values().filter_map(|i| {
            if let ItemKind::Namespace(ident, _) = &i.kind {
                Some(ident.name.clone())
            } else {
                None
            }
        });
        let only_callable = callables.next().expect("Expected exactly one callable");
        assert!(callables.next().is_none(), "Expected exactly one callable");
        let only_namespace = namespaces.next().expect("Expected exactly one namespace");
        assert!(
            namespaces.next().is_none(),
            "Expected exactly one namespace"
        );
        (only_callable.clone(), only_namespace)
    }

    #[test]
    fn no_params() {
        let (decl, namespace) = compile_one_operation(
            r"
            namespace Test {
                operation Test() : Result[] {
                }
            }
        ",
        );
        let params = operation_circuit_info(&decl, namespace.as_ref(), false);
        expect![[r"
            None
        "]]
        .assert_debug_eq(&params);
    }

    #[test]
    fn non_qubit_params() {
        let (decl, namespace) = compile_one_operation(
            r"
            namespace Test {
                operation Test(q1: Qubit, q2: Qubit, i: Int) : Result[] {
                }
            }
        ",
        );
        let params = operation_circuit_info(&decl, namespace.as_ref(), false);
        expect![[r"
            None
        "]]
        .assert_debug_eq(&params);
    }

    #[test]
    fn non_qubit_array_param() {
        let (decl, namespace) = compile_one_operation(
            r"
            namespace Test {
                operation Test(q1: Qubit[], q2: Qubit[][], i: Int[]) : Result[] {
                }
            }
        ",
        );
        let params = operation_circuit_info(&decl, namespace.as_ref(), false);
        expect![[r"
            None
        "]]
        .assert_debug_eq(&params);
    }

    #[test]
    fn qubit_params() {
        let (decl, namespace) = compile_one_operation(
            r"
            namespace Test {
                operation Test(q1: Qubit, q2: Qubit) : Result[] {
                }
            }
        ",
        );
        let params = operation_circuit_info(&decl, namespace.as_ref(), false);
        expect![[r#"
            Some(
                OperationInfo {
                    namespace: "Test",
                    name: "Test",
                    internal: false,
                    qubit_param_dimensions: [
                        0,
                        0,
                    ],
                    total_num_qubits: 2,
                },
            )
        "#]]
        .assert_debug_eq(&params);

        let expr = operation_circuit_entry_expr(params.as_ref().expect("expected params"));
        expect![[r"
            {
                        use qs = Qubit[2];
                        Test.Test(qs[0], qs[1]);
                        let r: Result[] = [];
                        r
                    }"]]
        .assert_eq(&expr);
    }

    #[test]
    fn qubit_array_params() {
        let (decl, namespace) = compile_one_operation(
            r"
            namespace Test {
                operation Test(q1: Qubit[], q2: Qubit[][], q3: Qubit[][][], q: Qubit) : Result[] {
                }
            }
        ",
        );
        let params = operation_circuit_info(&decl, namespace.as_ref(), false);
        expect![[r#"
            Some(
                OperationInfo {
                    namespace: "Test",
                    name: "Test",
                    internal: false,
                    qubit_param_dimensions: [
                        1,
                        2,
                        3,
                        0,
                    ],
                    total_num_qubits: 15,
                },
            )
        "#]]
        .assert_debug_eq(&params);

        let expr = operation_circuit_entry_expr(params.as_ref().expect("expected params"));
        expect![[r"
            {
                        use qs = Qubit[15];
                        Test.Test(qs[0..1], Microsoft.Quantum.Arrays.Chunks(2, qs[2..5]), Microsoft.Quantum.Arrays.Chunks(2, Microsoft.Quantum.Arrays.Chunks(2, qs[6..13])), qs[14]);
                        let r: Result[] = [];
                        r
                    }"]].assert_eq(&expr);
    }
}
