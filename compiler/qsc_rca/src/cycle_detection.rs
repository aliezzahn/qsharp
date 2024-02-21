// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use crate::common::{
    derive_callable_input_params, initalize_locals_map, try_resolve_callee, GlobalSpecId, Local,
    LocalKind, SpecKind,
};
use qsc_fir::{
    fir::{
        Block, BlockId, CallableDecl, CallableImpl, Expr, ExprId, ExprKind, Item, ItemKind,
        LocalItemId, LocalVarId, Mutability, Package, PackageId, PackageLookup, Pat, PatId,
        PatKind, SpecDecl, Stmt, StmtId, StmtKind, StoreItemId, StringComponent,
    },
    visit::Visitor,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::Entry;

#[derive(Default)]
struct CallStack {
    set: FxHashSet<LocalSpecId>,
    stack: Vec<LocalSpecId>,
}

impl CallStack {
    fn contains(&self, value: &LocalSpecId) -> bool {
        self.set.contains(value)
    }

    fn peak(&self) -> &LocalSpecId {
        self.stack.last().expect("stack should not be empty")
    }

    fn pop(&mut self) -> LocalSpecId {
        let popped = self.stack.pop().expect("stack should not be empty");
        self.set.remove(&popped);
        popped
    }

    fn push(&mut self, value: LocalSpecId) {
        self.set.insert(value);
        self.stack.push(value);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct LocalSpecId {
    pub callable: LocalItemId,
    pub spec_kind: SpecKind,
}

struct CycleDetector<'a> {
    package_id: PackageId,
    package: &'a Package,
    stack: CallStack,
    specializations_locals: FxHashMap<LocalSpecId, FxHashMap<LocalVarId, Local>>,
    specializations_with_cycles: FxHashSet<LocalSpecId>,
}

impl<'a> CycleDetector<'a> {
    fn new(package_id: PackageId, package: &'a Package) -> Self {
        Self {
            package_id,
            package,
            stack: CallStack::default(),
            specializations_locals: FxHashMap::default(),
            specializations_with_cycles: FxHashSet::<LocalSpecId>::default(),
        }
    }

    fn detect_specializations_with_cycles(&mut self) -> Vec<LocalSpecId> {
        self.visit_package(self.package);
        self.specializations_with_cycles.drain().collect()
    }

    fn map_pat_to_expr(&mut self, mutability: Mutability, pat_id: PatId, expr_id: ExprId) {
        let pat = self.get_pat(pat_id);
        match &pat.kind {
            PatKind::Bind(ident) => {
                let local_spec_id = self.stack.peak();
                let locals_map = self
                    .specializations_locals
                    .get_mut(local_spec_id)
                    .expect("node map should exist");
                let kind = match mutability {
                    Mutability::Immutable => LocalKind::Immutable(expr_id),
                    Mutability::Mutable => LocalKind::Mutable,
                };
                locals_map.insert(
                    ident.id,
                    Local {
                        pat: pat_id,
                        var: ident.id,
                        ty: pat.ty.clone(),
                        kind,
                    },
                );
            }
            PatKind::Tuple(pats) => {
                let expr = self.get_expr(expr_id);
                if let ExprKind::Tuple(exprs) = &expr.kind {
                    for (pat_id, expr_id) in pats.iter().zip(exprs.iter()) {
                        self.map_pat_to_expr(mutability, *pat_id, *expr_id);
                    }
                }
            }
            PatKind::Discard => {}
        }
    }

    fn walk_callable_decl(&mut self, local_spec_id: LocalSpecId, callable_decl: &'a CallableDecl) {
        // We only need to go deeper for non-intrinsic callables.
        let CallableImpl::Spec(spec_impl) = &callable_decl.implementation else {
            return;
        };

        let spec_decl = match local_spec_id.spec_kind {
            SpecKind::Body => &spec_impl.body,
            SpecKind::Adj => spec_impl
                .adj
                .as_ref()
                .expect("adj specialization should exist"),
            SpecKind::Ctl => spec_impl
                .ctl
                .as_ref()
                .expect("ctl specialization should exist"),
            SpecKind::CtlAdj => spec_impl
                .ctl_adj
                .as_ref()
                .expect("ctl_adj specialization should exist"),
        };
        self.walk_spec_decl(local_spec_id, spec_decl);
    }

    fn walk_call_expr(&mut self, callee: ExprId, args: ExprId) {
        // Visit the arguments expression in case it triggers a call already in the stack.
        self.visit_expr(args);

        // Visit the callee if it resolves to something.
        let local_spec_id = self.stack.peak();
        let locals_map = self
            .specializations_locals
            .get_mut(local_spec_id)
            .expect("node map should exist");
        let maybe_callee = try_resolve_callee(callee, self.package_id, self.package, locals_map);
        if let Some(callee) = maybe_callee {
            // We are not interested in visiting callables outside this package.
            if callee.item.package != self.package_id {
                return;
            }
            let item = self.package.get_item(callee.item.item);
            match &item.kind {
                ItemKind::Callable(callable_decl) => self.walk_callable_decl(
                    LocalSpecId {
                        callable: callee.item.item,
                        spec_kind: SpecKind::from(&callee.spec_functor),
                    },
                    callable_decl,
                ),
                ItemKind::Namespace(_, _) => panic!("calls to namespaces are invalid"),
                ItemKind::Ty(_, _) => {
                    // Ignore "calls" to types.
                }
            }
        }
    }

    fn walk_spec_decl(&mut self, local_spec_id: LocalSpecId, spec_decl: &'a SpecDecl) {
        // If the specialization is already in the stack, it means the callable has a cycle.
        if self.stack.contains(&local_spec_id) {
            self.specializations_with_cycles.insert(local_spec_id);
            return;
        }

        // If this is the first time we are walking this specialization, create a node map for it.
        if let Entry::Vacant(entry) = self.specializations_locals.entry(local_spec_id) {
            let ItemKind::Callable(callable_decl) =
                &self.package.get_item(local_spec_id.callable).kind
            else {
                panic!("item must be a callable");
            };

            let input_params = derive_callable_input_params(callable_decl, &self.package.pats);
            let locals_map = initalize_locals_map(&input_params);
            entry.insert(locals_map);
        }

        // Push the callable specialization to the stack, visit it and then pop it.
        self.stack.push(local_spec_id);
        self.visit_spec_decl(spec_decl);
        _ = self.stack.pop();
    }

    fn walk_local_stmt(&mut self, mutability: Mutability, pat_id: PatId, expr_id: ExprId) {
        self.map_pat_to_expr(mutability, pat_id, expr_id);
        self.visit_expr(expr_id);
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
        match &expr.kind {
            ExprKind::Array(exprs) => exprs.iter().for_each(|e| self.visit_expr(*e)),
            ExprKind::ArrayRepeat(item, size) => {
                self.visit_expr(*item);
                self.visit_expr(*size);
            }
            ExprKind::Assign(lhs, rhs)
            | ExprKind::AssignOp(_, lhs, rhs)
            | ExprKind::BinOp(_, lhs, rhs) => {
                self.visit_expr(*lhs);
                self.visit_expr(*rhs);
            }
            ExprKind::AssignField(record, _, replace)
            | ExprKind::UpdateField(record, _, replace) => {
                self.visit_expr(*record);
                self.visit_expr(*replace);
            }
            ExprKind::AssignIndex(array, index, replace) => {
                self.visit_expr(*array);
                self.visit_expr(*index);
                self.visit_expr(*replace);
            }
            ExprKind::Block(block) => self.visit_block(*block),
            ExprKind::Call(callee, args) => self.walk_call_expr(*callee, *args),
            ExprKind::Fail(msg) => self.visit_expr(*msg),
            ExprKind::Field(record, _) => self.visit_expr(*record),
            ExprKind::If(cond, body, otherwise) => {
                self.visit_expr(*cond);
                self.visit_expr(*body);
                otherwise.iter().for_each(|e| self.visit_expr(*e));
            }
            ExprKind::Index(array, index) => {
                self.visit_expr(*array);
                self.visit_expr(*index);
            }
            ExprKind::Return(expr) | ExprKind::UnOp(_, expr) => {
                self.visit_expr(*expr);
            }
            ExprKind::Range(start, step, end) => {
                start.iter().for_each(|s| self.visit_expr(*s));
                step.iter().for_each(|s| self.visit_expr(*s));
                end.iter().for_each(|e| self.visit_expr(*e));
            }
            ExprKind::String(components) => {
                for component in components {
                    match component {
                        StringComponent::Expr(expr) => self.visit_expr(*expr),
                        StringComponent::Lit(_) => {}
                    }
                }
            }
            ExprKind::UpdateIndex(e1, e2, e3) => {
                self.visit_expr(*e1);
                self.visit_expr(*e2);
                self.visit_expr(*e3);
            }
            ExprKind::Tuple(exprs) => exprs.iter().for_each(|e| self.visit_expr(*e)),
            ExprKind::While(cond, block) => {
                self.visit_expr(*cond);
                self.visit_block(*block);
            }
            ExprKind::Closure(_, _) | ExprKind::Hole | ExprKind::Lit(_) | ExprKind::Var(_, _) => {}
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
            LocalSpecId {
                callable: item.id,
                spec_kind: SpecKind::Body,
            },
            &spec_impl.body,
        );

        // Visit the adj specialization.
        if let Some(adj_decl) = &spec_impl.adj {
            self.walk_spec_decl(
                LocalSpecId {
                    callable: item.id,
                    spec_kind: SpecKind::Adj,
                },
                adj_decl,
            );
        }

        // Visit the ctl specialization.
        if let Some(ctl_decl) = &spec_impl.ctl {
            self.walk_spec_decl(
                LocalSpecId {
                    callable: item.id,
                    spec_kind: SpecKind::Ctl,
                },
                ctl_decl,
            );
        }

        // Visit the ctl_adj specialization.
        if let Some(ctl_adj_decl) = &spec_impl.ctl_adj {
            self.walk_spec_decl(
                LocalSpecId {
                    callable: item.id,
                    spec_kind: SpecKind::CtlAdj,
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
            StmtKind::Local(mutability, pat_id, expr_id) => {
                self.walk_local_stmt(*mutability, *pat_id, *expr_id)
            }
        }
    }
}

pub fn detect_specializations_with_cycles(
    package_id: PackageId,
    package: &Package,
) -> Vec<GlobalSpecId> {
    // First, detect the specializations that have cycles.
    let mut cycle_detector = CycleDetector::new(package_id, package);
    let specializations_with_cycles = cycle_detector.detect_specializations_with_cycles();

    // Convert the package specialization IDs to global specialization IDs.
    specializations_with_cycles
        .iter()
        .map(|local_spec_id| GlobalSpecId {
            callable: StoreItemId {
                package: package_id,
                item: local_spec_id.callable,
            },
            spec_kind: local_spec_id.spec_kind,
        })
        .collect()
}
