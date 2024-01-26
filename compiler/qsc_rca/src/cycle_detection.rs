// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use crate::data_structures::{
    derive_callable_input_elements, derive_callable_input_map, derive_callable_input_params,
    CallableVariable, CallableVariableKind,
};

use qsc_fir::{
    fir::{
        Block, BlockId, CallableDecl, CallableImpl, Expr, ExprId, ExprKind, Functor, Item, ItemId,
        ItemKind, LocalItemId, NodeId, Package, PackageId, PackageLookup, Pat, PatId, PatKind, Res,
        SpecDecl, Stmt, StmtId, StmtKind, UnOp,
    },
    visit::Visitor,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::Entry;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct FunctorApplication {
    pub adjoint: bool,
    // N.B. For the purposes of cycle detection, we only care about the controlled functor being applied, but not the
    // number of times it was applied.
    pub controlled: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct CallableSpecializationId {
    callable: LocalItemId,
    functor_application: FunctorApplication,
}

#[derive(Default)]
struct CallStack {
    set: FxHashSet<CallableSpecializationId>,
    stack: Vec<CallableSpecializationId>,
}

impl CallStack {
    fn contains(&self, value: &CallableSpecializationId) -> bool {
        self.set.contains(value)
    }

    fn peak(&self) -> &CallableSpecializationId {
        self.stack.last().expect("stack should not be empty")
    }

    fn pop(&mut self) -> CallableSpecializationId {
        let popped = self.stack.pop().expect("stack should not be empty");
        self.set.remove(&popped);
        popped
    }

    fn push(&mut self, value: CallableSpecializationId) {
        self.set.insert(value);
        self.stack.push(value);
    }
}

struct CycleDetector<'a> {
    package_id: PackageId,
    package: &'a Package,
    stack: CallStack,
    node_maps: FxHashMap<CallableSpecializationId, FxHashMap<NodeId, CallableVariable>>,
    callables_with_cycles: FxHashSet<CallableSpecializationId>,
}

impl<'a> CycleDetector<'a> {
    fn new(package_id: PackageId, package: &'a Package) -> Self {
        Self {
            package_id,
            package,
            stack: CallStack::default(),
            node_maps: FxHashMap::default(),
            callables_with_cycles: FxHashSet::<CallableSpecializationId>::default(),
        }
    }

    fn detect_callables_with_cycles(&mut self) {
        self.visit_package(self.package);
    }

    fn get_callables_with_cycles(&self) -> &FxHashSet<CallableSpecializationId> {
        &self.callables_with_cycles
    }

    fn map_pat_to_expr(&mut self, pat_id: PatId, expr_id: ExprId) {
        let pat = self.get_pat(pat_id);
        match &pat.kind {
            PatKind::Bind(ident) => {
                let callable_specialization_id = self.stack.peak();
                let node_map = self
                    .node_maps
                    .get_mut(callable_specialization_id)
                    .expect("node map should exist");
                node_map.insert(
                    ident.id,
                    CallableVariable {
                        node: ident.id,
                        pat: pat_id,
                        ty: pat.ty.clone(),
                        kind: CallableVariableKind::Local(expr_id),
                    },
                );
            }
            PatKind::Tuple(_) => {
                // TODO (cesarzc): implement correctly.
            }
            PatKind::Discard => {}
        }
    }

    /// Uniquely resolves the callable specialization referenced in a callee expression.
    fn resolve_callee(&self, expr_id: ExprId) -> Option<CallableSpecializationId> {
        // Resolves a closure callee.
        let resolve_closure = |local_item_id: LocalItemId| -> Option<CallableSpecializationId> {
            Some(CallableSpecializationId {
                callable: local_item_id,
                functor_application: FunctorApplication::default(),
            })
        };

        // Resolves a unary operator callee.
        let resolve_un_op =
            |operator: &UnOp, expr_id: ExprId| -> Option<CallableSpecializationId> {
                let UnOp::Functor(functor) = operator else {
                    panic!("unary operator is expected to be a functor for a callee expression")
                };

                let resolved_callee = self.resolve_callee(expr_id);
                if let Some(callable_specialization_id) = resolved_callee {
                    let functor_application = match functor {
                        Functor::Adj => FunctorApplication {
                            adjoint: !callable_specialization_id.functor_application.adjoint,
                            controlled: callable_specialization_id.functor_application.controlled,
                        },
                        Functor::Ctl => FunctorApplication {
                            adjoint: callable_specialization_id.functor_application.adjoint,
                            // Once set to `true`, it remains as `true`.
                            controlled: true,
                        },
                    };
                    Some(CallableSpecializationId {
                        callable: callable_specialization_id.callable,
                        functor_application,
                    })
                } else {
                    None
                }
            };

        // Resolves an item callee.
        let resolve_item = |item_id: ItemId| -> Option<CallableSpecializationId> {
            match item_id.package {
                Some(package_id) => {
                    if package_id == self.package_id {
                        Some(CallableSpecializationId {
                            callable: item_id.item,
                            functor_application: FunctorApplication::default(),
                        })
                    } else {
                        None
                    }
                }
                // No package ID assumes the callee is in the same package than the caller.
                None => Some(CallableSpecializationId {
                    callable: item_id.item,
                    functor_application: FunctorApplication::default(),
                }),
            }
        };

        // Resolves a local callee.
        let resolve_local = |node_id: NodeId| -> Option<CallableSpecializationId> {
            let callable_specialization_id = self.stack.peak();
            let node_map = self
                .node_maps
                .get(callable_specialization_id)
                .expect("node map should exist");
            if let Some(callable_variable) = node_map.get(&node_id) {
                match &callable_variable.kind {
                    CallableVariableKind::InputParam(_) => None,
                    CallableVariableKind::Local(expr_id) => self.resolve_callee(*expr_id),
                }
            } else {
                panic!("cannot determine callee from resolution")
            }
        };

        let expr = self.get_expr(expr_id);
        match &expr.kind {
            ExprKind::Closure(_, local_item_id) => resolve_closure(*local_item_id),
            ExprKind::UnOp(operator, expr_id) => resolve_un_op(operator, *expr_id),
            ExprKind::Var(res, _) => match res {
                Res::Item(item_id) => resolve_item(*item_id),
                Res::Local(node_id) => resolve_local(*node_id),
                Res::Err => panic!("resolution should not be error"),
            },
            _ => panic!("cannot determine callee from expression"),
        }
    }

    fn walk_callable_decl(
        &mut self,
        callable_specialization_id: CallableSpecializationId,
        callable_decl: &'a CallableDecl,
    ) {
        // We only need to go deeper for non-intrinsic callables.
        let CallableImpl::Spec(spec_impl) = &callable_decl.implementation else {
            return;
        };

        let functor_application = callable_specialization_id.functor_application;
        let spec_decl = if !functor_application.adjoint && !functor_application.controlled {
            &spec_impl.body
        } else if functor_application.adjoint && !functor_application.controlled {
            spec_impl
                .adj
                .as_ref()
                .expect("adj specialization must exist")
        } else if !functor_application.adjoint && functor_application.controlled {
            spec_impl
                .ctl
                .as_ref()
                .expect("ctl specialization must exist")
        } else {
            spec_impl
                .ctl_adj
                .as_ref()
                .expect("ctl_adj specialization must exist")
        };

        self.walk_spec_decl(callable_specialization_id, spec_decl);
    }

    fn walk_spec_decl(
        &mut self,
        callable_specialization_id: CallableSpecializationId,
        spec_decl: &'a SpecDecl,
    ) {
        // If the specialization is already in the stack, it means the callable has a cycle.
        if self.stack.contains(&callable_specialization_id) {
            self.callables_with_cycles
                .insert(callable_specialization_id);
            return;
        }

        // If this is the first time we are walking this specialization, create a node map for it.
        if let Entry::Vacant(entry) = self.node_maps.entry(callable_specialization_id) {
            let ItemKind::Callable(callable_decl) = &self
                .package
                .get_item(callable_specialization_id.callable)
                .kind
            else {
                panic!("item must be a callable");
            };

            let input_elements = derive_callable_input_elements(callable_decl, &self.package.pats);
            let input_params = derive_callable_input_params(input_elements.iter());
            let input_map = derive_callable_input_map(input_params.iter());
            entry.insert(input_map);
        }

        // Push the callable specialization to the stack, visit it and then pop it.
        self.stack.push(callable_specialization_id);
        self.visit_spec_decl(spec_decl);
        _ = self.stack.pop();
    }
}

impl<'a> Visitor<'a> for CycleDetector<'a> {
    fn get_block(&self, id: BlockId) -> &'a Block {
        self.package
            .blocks
            .get(id)
            .expect("couldn't find block in FIR")
    }

    fn get_expr(&self, id: ExprId) -> &'a Expr {
        self.package
            .exprs
            .get(id)
            .expect("couldn't find expr in FIR")
    }

    fn get_pat(&self, id: PatId) -> &'a Pat {
        self.package.pats.get(id).expect("couldn't find pat in FIR")
    }

    fn get_stmt(&self, id: StmtId) -> &'a Stmt {
        self.package
            .stmts
            .get(id)
            .expect("couldn't find stmt in FIR")
    }

    fn visit_callable_decl(&mut self, _: &'a CallableDecl) {
        panic!("visiting a callable declaration through this method is unexpected");
    }

    fn visit_expr(&mut self, expr_id: ExprId) {
        let expr = self.get_expr(expr_id);
        // We are only interested in call expressions.
        if let ExprKind::Call(callee, _) = expr.kind {
            // TODO (cesarzc): if passing any function, it needs to check whether the function being passed is already
            // in the stack.
            // Example of this behavior: Microsoft.Quantum.Arrays.Sorted.

            // Visit the callee only if it resolves to a local specialization.
            if let Some(callable_specialization_id) = self.resolve_callee(callee) {
                let item = self.package.get_item(callable_specialization_id.callable);
                match &item.kind {
                    ItemKind::Callable(callable_decl) => {
                        self.walk_callable_decl(callable_specialization_id, callable_decl)
                    }
                    ItemKind::Namespace(_, _) => panic!("calls to namespaces are invalid"),
                    ItemKind::Ty(_, _) => {
                        // Ignore "calls" to types.
                    }
                }
            }
        }
    }

    fn visit_item(&mut self, item: &'a Item) {
        // We are only interested in visiting callables.
        let ItemKind::Callable(callable_decl) = &item.kind else {
            return;
        };

        // We are only interested in non-intrinsic callables.
        let CallableImpl::Spec(spec_impl) = &callable_decl.implementation else {
            return;
        };

        // Visit the body specialization.
        self.walk_spec_decl(
            CallableSpecializationId {
                callable: item.id,
                functor_application: FunctorApplication {
                    adjoint: false,
                    controlled: false,
                },
            },
            &spec_impl.body,
        );

        // Visit the adj specialization.
        if let Some(adj_decl) = &spec_impl.adj {
            self.walk_spec_decl(
                CallableSpecializationId {
                    callable: item.id,
                    functor_application: FunctorApplication {
                        adjoint: true,
                        controlled: false,
                    },
                },
                adj_decl,
            );
        }

        // Visit the ctl specialization.
        if let Some(ctl_decl) = &spec_impl.ctl {
            self.walk_spec_decl(
                CallableSpecializationId {
                    callable: item.id,
                    functor_application: FunctorApplication {
                        adjoint: false,
                        controlled: true,
                    },
                },
                ctl_decl,
            );
        }

        // Visit the ctl_adj specialization.
        if let Some(ctl_adj_decl) = &spec_impl.ctl {
            self.walk_spec_decl(
                CallableSpecializationId {
                    callable: item.id,
                    functor_application: FunctorApplication {
                        adjoint: true,
                        controlled: true,
                    },
                },
                ctl_adj_decl,
            );
        }
    }

    fn visit_package(&mut self, package: &'a Package) {
        // We are only interested in visiting items.
        package.items.values().for_each(|i| self.visit_item(i));
    }

    fn visit_spec_decl(&mut self, decl: &'a SpecDecl) {
        // For cycle detection we only need to visit the specialization block.
        self.visit_block(decl.block);
    }

    fn visit_stmt(&mut self, stmt_id: StmtId) {
        let stmt = self.get_stmt(stmt_id);
        match &stmt.kind {
            StmtKind::Item(_) => {}
            StmtKind::Expr(expr_id) | StmtKind::Semi(expr_id) => self.visit_expr(*expr_id),
            StmtKind::Local(_, pat_id, expr_id) => self.map_pat_to_expr(*pat_id, *expr_id),
        }
    }
}

pub fn detect_callables_with_cycles(
    package_id: PackageId,
    package: &Package,
) -> FxHashSet<CallableSpecializationId> {
    let mut cycle_detector = CycleDetector::new(package_id, package);
    cycle_detector.detect_callables_with_cycles();
    cycle_detector.get_callables_with_cycles().clone()
}