// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#[cfg(test)]
mod tests;

use crate::{
    compilation::{Compilation, CompilationKind},
    protocol::{CodeLens, CodeLensCommand, OperationCircuitParams},
    qsc_utils::{into_range, span_contains},
};
use qsc::{
    hir::{
        ty::{Prim, Ty},
        Attr, ItemKind,
    },
    line_column::Encoding,
};

pub(crate) fn get_code_lenses(
    compilation: &Compilation,
    source_name: &str,
    position_encoding: Encoding,
) -> Vec<CodeLens> {
    if matches!(compilation.kind, CompilationKind::Notebook) {
        // entrypoint actions don't work in notebooks
        return vec![];
    }

    let user_unit = compilation.user_unit();
    let source_span = compilation.package_span_of_source(source_name);

    // Get callables in the current source file with the @EntryPoint() attribute.
    // If there is more than one entrypoint, not our problem, we'll go ahead
    // and return code lenses for all. The duplicate entrypoint diagnostic
    // will be reported from elsewhere.
    let decls = user_unit.package.items.values().filter_map(|item| {
        if span_contains(source_span, item.span.lo) {
            if let ItemKind::Callable(decl) = &item.kind {
                if let Some(ItemKind::Namespace(ns, _)) = item
                    .parent
                    .and_then(|parent_id| user_unit.package.items.get(parent_id))
                    .map(|parent| &parent.kind)
                {
                    if item.attrs.iter().any(|a| a == &Attr::EntryPoint) {
                        return Some((decl, ns.name.to_string(), true));
                    }
                    return Some((decl, ns.name.to_string(), false));
                }
            }
        }
        None
    });

    decls
        .flat_map(|(decl, namespace, is_entry_point)| {
            let range = into_range(position_encoding, decl.span, &user_unit.sources);

            if is_entry_point {
                vec![
                    CodeLens {
                        range,
                        command: CodeLensCommand::Run,
                    },
                    CodeLens {
                        range,
                        command: CodeLensCommand::Histogram,
                    },
                    CodeLens {
                        range,
                        command: CodeLensCommand::Estimate,
                    },
                    CodeLens {
                        range,
                        command: CodeLensCommand::Debug,
                    },
                    CodeLens {
                        range,
                        command: CodeLensCommand::Circuit,
                    },
                ]
            } else {
                let qubit_arg_dimensions = qubit_arg_dimensions(&decl.input.ty);
                if !qubit_arg_dimensions.is_empty() {
                    return vec![CodeLens {
                        range,
                        command: CodeLensCommand::OperationCircuit(OperationCircuitParams {
                            namespace,
                            name: decl.name.name.to_string(),
                            args: qubit_arg_dimensions,
                        }),
                    }];
                }
                vec![]
            }
        })
        .collect()
}

fn qubit_arg_dimensions(input: &Ty) -> Vec<usize> {
    match input {
        Ty::Array(ty) => {
            if let Some(s) = get_array_dimension(ty) {
                return vec![s + 1];
            }
        }
        Ty::Prim(Prim::Qubit) => return vec![0],
        Ty::Tuple(tys) => {
            let params = tys.iter().map(get_array_dimension).collect::<Vec<_>>();

            if params.iter().all(Option::is_some) {
                return params.into_iter().map(Option::unwrap).collect();
            }
        }
        _ => {}
    }
    vec![]
}

fn get_array_dimension(input: &Ty) -> Option<usize> {
    match input {
        Ty::Prim(Prim::Qubit) => Some(0),
        Ty::Array(ty) => get_array_dimension(ty).map(|d| d + 1),
        _ => None,
    }
}
