use super::liveness::{closure_capture_names, defined_names};
use crate::ast::*;
use crate::collections::{HashMap, HashSet};
use crate::mentions::mention_expr;
use crate::span::Span;

#[derive(Debug, Clone)]
pub(crate) struct CfgNode {
    pub(crate) span: Span,
    pub(crate) defs: HashSet<String>,
    pub(crate) reads: HashSet<String>,
    pub(crate) carriers: Vec<(String, String)>,
    pub(crate) successors: Vec<usize>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Cfg {
    pub(crate) nodes: Vec<CfgNode>,
    #[allow(dead_code)]
    pub(crate) span_to_node: HashMap<Span, usize>,
}

#[derive(Clone, Copy)]
struct LoopContext {
    break_target: usize,
    continue_target: usize,
}

impl Cfg {
    pub(crate) fn from_stmts(stmts: &[Stmt]) -> Self {
        let mut builder = CfgBuilder::new();
        let exit_node = builder.add_node(
            Span::DUMMY,
            HashSet::default(),
            HashSet::default(),
            Vec::new(),
            Vec::new(),
        );

        let mut next = exit_node;
        for stmt in stmts.iter().rev() {
            if let StmtKind::Defer { body, .. } = stmt.kind() {
                next = builder.build_block(body, next, None);
            }
        }
        let defer_exit = next;

        let entry = builder.build_block(stmts, defer_exit, None);
        let _ = entry;

        Cfg {
            nodes: builder.nodes,
            span_to_node: builder.span_to_node,
        }
    }

    pub(crate) fn compute_liveness(&self) -> HashMap<Span, HashSet<String>> {
        let n = self.nodes.len();
        if n == 0 {
            return HashMap::default();
        }

        let mut live_in: Vec<HashSet<String>> = vec![HashSet::default(); n];
        let mut live_out: Vec<HashSet<String>> = vec![HashSet::default(); n];

        let mut changed = true;
        let mut iterations = 0;
        while changed && iterations < 1000 {
            changed = false;
            iterations += 1;
            for i in (0..n).rev() {
                let mut new_out = HashSet::default();
                for &succ in &self.nodes[i].successors {
                    if succ < n {
                        for var in &live_in[succ] {
                            new_out.insert(var.clone());
                        }
                    }
                }

                let mut carrier_changed = true;
                while carrier_changed {
                    carrier_changed = false;
                    for (carrier, source) in &self.nodes[i].carriers {
                        if new_out.contains(carrier) && !new_out.contains(source) {
                            new_out.insert(source.clone());
                            carrier_changed = true;
                        }
                    }
                }

                let mut new_in = new_out.clone();
                for def in &self.nodes[i].defs {
                    new_in.remove(def);
                }
                for read in &self.nodes[i].reads {
                    new_in.insert(read.clone());
                }

                if new_out != live_out[i] {
                    live_out[i] = new_out;
                    changed = true;
                }
                if new_in != live_in[i] {
                    live_in[i] = new_in;
                    changed = true;
                }
            }
        }

        let mut span_map = HashMap::default();
        for (i, node) in self.nodes.iter().enumerate() {
            if node.span != Span::DUMMY {
                span_map.insert(node.span, live_out[i].clone());
            }
        }
        span_map
    }
}

struct CfgBuilder {
    nodes: Vec<CfgNode>,
    span_to_node: HashMap<Span, usize>,
}

impl CfgBuilder {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            span_to_node: HashMap::default(),
        }
    }

    fn add_node(
        &mut self,
        span: Span,
        defs: HashSet<String>,
        reads: HashSet<String>,
        carriers: Vec<(String, String)>,
        successors: Vec<usize>,
    ) -> usize {
        let idx = self.nodes.len();
        if span != Span::DUMMY {
            self.span_to_node.insert(span, idx);
        }
        self.nodes.push(CfgNode {
            span,
            defs,
            reads,
            carriers,
            successors,
        });
        idx
    }

    fn build_block(
        &mut self,
        stmts: &[Stmt],
        exit_target: usize,
        loop_ctx: Option<&LoopContext>,
    ) -> usize {
        let mut next = exit_target;
        for stmt in stmts.iter().rev() {
            next = self.build_stmt(stmt, next, loop_ctx);
        }
        next
    }

    fn build_stmt(&mut self, stmt: &Stmt, next: usize, loop_ctx: Option<&LoopContext>) -> usize {
        match stmt.kind() {
            StmtKind::Return(value) => {
                let mut reads = HashSet::default();
                if let Some(val) = value {
                    mention_expr(val, &mut |name| {
                        reads.insert(name.to_string());
                    });
                }
                self.add_node(stmt.span, HashSet::default(), reads, Vec::new(), vec![0])
            }
            StmtKind::Throw(value) => {
                let mut reads = HashSet::default();
                mention_expr(value, &mut |name| {
                    reads.insert(name.to_string());
                });
                self.add_node(stmt.span, HashSet::default(), reads, Vec::new(), vec![0])
            }
            StmtKind::Break(_) => {
                let target = loop_ctx.map(|ctx| ctx.break_target).unwrap_or(next);
                self.add_node(
                    stmt.span,
                    HashSet::default(),
                    HashSet::default(),
                    Vec::new(),
                    vec![target],
                )
            }
            StmtKind::Continue => {
                let target = loop_ctx.map(|ctx| ctx.continue_target).unwrap_or(next);
                self.add_node(
                    stmt.span,
                    HashSet::default(),
                    HashSet::default(),
                    Vec::new(),
                    vec![target],
                )
            }
            StmtKind::Defer { .. } => self.add_node(
                stmt.span,
                HashSet::default(),
                HashSet::default(),
                Vec::new(),
                vec![next],
            ),
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::If {
                        cond,
                        then_block,
                        else_block,
                    },
                ..
            }) => {
                let then_entry = self.build_block(then_block, next, loop_ctx);
                let else_entry = if let Some(else_block) = else_block {
                    self.build_block(else_block, next, loop_ctx)
                } else {
                    next
                };

                let mut reads = HashSet::default();
                mention_expr(cond, &mut |name| {
                    reads.insert(name.to_string());
                });

                self.add_node(
                    stmt.span,
                    HashSet::default(),
                    reads,
                    Vec::new(),
                    vec![then_entry, else_entry],
                )
            }
            StmtKind::While { cond, body } => {
                let cond_idx = self.nodes.len();
                self.nodes.push(CfgNode {
                    span: stmt.span,
                    defs: HashSet::default(),
                    reads: HashSet::default(),
                    carriers: Vec::new(),
                    successors: Vec::new(),
                });
                if stmt.span != Span::DUMMY {
                    self.span_to_node.insert(stmt.span, cond_idx);
                }

                let loop_context = LoopContext {
                    break_target: next,
                    continue_target: cond_idx,
                };

                let body_entry = self.build_block(body, cond_idx, Some(&loop_context));

                let mut reads = HashSet::default();
                mention_expr(cond, &mut |name| {
                    reads.insert(name.to_string());
                });

                self.nodes[cond_idx].reads = reads;
                self.nodes[cond_idx].successors = vec![body_entry, next];

                cond_idx
            }
            StmtKind::DoWhile { body, cond } => {
                let cond_idx = self.nodes.len();
                self.nodes.push(CfgNode {
                    span: stmt.span,
                    defs: HashSet::default(),
                    reads: HashSet::default(),
                    carriers: Vec::new(),
                    successors: Vec::new(),
                });
                if stmt.span != Span::DUMMY {
                    self.span_to_node.insert(stmt.span, cond_idx);
                }

                let loop_context = LoopContext {
                    break_target: next,
                    continue_target: cond_idx,
                };

                let body_entry = self.build_block(body, cond_idx, Some(&loop_context));

                let mut reads = HashSet::default();
                mention_expr(cond, &mut |name| {
                    reads.insert(name.to_string());
                });

                self.nodes[cond_idx].reads = reads;
                self.nodes[cond_idx].successors = vec![body_entry, next];

                body_entry
            }
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                let update_entry = self.build_stmt(update, 0, loop_ctx);
                let cond_idx = self.nodes.len();
                self.nodes.push(CfgNode {
                    span: stmt.span,
                    defs: HashSet::default(),
                    reads: HashSet::default(),
                    carriers: Vec::new(),
                    successors: Vec::new(),
                });
                if stmt.span != Span::DUMMY {
                    self.span_to_node.insert(stmt.span, cond_idx);
                }

                self.nodes[update_entry].successors = vec![cond_idx];

                let loop_context = LoopContext {
                    break_target: next,
                    continue_target: update_entry,
                };

                let body_entry = self.build_block(body, update_entry, Some(&loop_context));

                let mut reads = HashSet::default();
                mention_expr(cond, &mut |name| {
                    reads.insert(name.to_string());
                });

                self.nodes[cond_idx].reads = reads;
                self.nodes[cond_idx].successors = vec![body_entry, next];

                self.build_stmt(init, cond_idx, loop_ctx)
            }
            StmtKind::ForOf {
                name,
                iterable,
                body,
                ..
            } => {
                let head_idx = self.nodes.len();
                self.nodes.push(CfgNode {
                    span: stmt.span,
                    defs: HashSet::default(),
                    reads: HashSet::default(),
                    carriers: Vec::new(),
                    successors: Vec::new(),
                });
                if stmt.span != Span::DUMMY {
                    self.span_to_node.insert(stmt.span, head_idx);
                }

                let loop_context = LoopContext {
                    break_target: next,
                    continue_target: head_idx,
                };

                let body_entry = self.build_block(body, head_idx, Some(&loop_context));

                let mut reads = HashSet::default();
                mention_expr(iterable, &mut |name| {
                    reads.insert(name.to_string());
                });

                let mut defs = HashSet::default();
                defs.insert(name.clone());

                self.nodes[head_idx].defs = defs;
                self.nodes[head_idx].reads = reads;
                self.nodes[head_idx].successors = vec![body_entry, next];

                head_idx
            }
            StmtKind::ForIn { name, object, body } => {
                let head_idx = self.nodes.len();
                self.nodes.push(CfgNode {
                    span: stmt.span,
                    defs: HashSet::default(),
                    reads: HashSet::default(),
                    carriers: Vec::new(),
                    successors: Vec::new(),
                });
                if stmt.span != Span::DUMMY {
                    self.span_to_node.insert(stmt.span, head_idx);
                }

                let loop_context = LoopContext {
                    break_target: next,
                    continue_target: head_idx,
                };

                let body_entry = self.build_block(body, head_idx, Some(&loop_context));

                let mut reads = HashSet::default();
                mention_expr(object, &mut |name| {
                    reads.insert(name.to_string());
                });

                let mut defs = HashSet::default();
                defs.insert(name.clone());

                self.nodes[head_idx].defs = defs;
                self.nodes[head_idx].reads = reads;
                self.nodes[head_idx].successors = vec![body_entry, next];

                head_idx
            }
            StmtKind::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                let mut arm_entries = Vec::new();
                for case in cases {
                    let entry = self.build_block(&case.body, next, loop_ctx);
                    arm_entries.push(entry);
                }
                if let Some(default_body) = default {
                    let entry = self.build_block(default_body, next, loop_ctx);
                    arm_entries.push(entry);
                } else {
                    arm_entries.push(next);
                }

                let mut reads = HashSet::default();
                mention_expr(expr, &mut |name| {
                    reads.insert(name.to_string());
                });

                self.add_node(
                    stmt.span,
                    HashSet::default(),
                    reads,
                    Vec::new(),
                    arm_entries,
                )
            }
            StmtKind::Match {
                expr,
                arms,
                default,
            } => {
                let mut arm_entries = Vec::new();
                for arm in arms {
                    let entry = self.build_block(&arm.body, next, loop_ctx);
                    arm_entries.push(entry);
                }
                if let Some(default_body) = default {
                    let entry = self.build_block(default_body, next, loop_ctx);
                    arm_entries.push(entry);
                } else {
                    arm_entries.push(next);
                }

                let mut reads = HashSet::default();
                mention_expr(expr, &mut |name| {
                    reads.insert(name.to_string());
                });

                self.add_node(
                    stmt.span,
                    HashSet::default(),
                    reads,
                    Vec::new(),
                    arm_entries,
                )
            }
            StmtKind::TryCatch { .. } => self.build_try_catch(stmt, next, loop_ctx),
            _ => {
                let mut defs = HashSet::default();
                defined_names(stmt, &mut |name| {
                    defs.insert(name.to_string());
                });

                let mut reads = HashSet::default();
                collect_statement_reads(stmt, &mut reads);

                let mut carriers = Vec::new();
                collect_statement_carriers(stmt, &mut carriers);

                self.add_node(stmt.span, defs, reads, carriers, vec![next])
            }
        }
    }

    fn build_try_catch(
        &mut self,
        stmt: &Stmt,
        next: usize,
        loop_ctx: Option<&LoopContext>,
    ) -> usize {
        let StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } = stmt.kind()
        else {
            unreachable!()
        };
        let finally_entry = if let Some(finally_body) = finally_body {
            self.build_block(finally_body, next, loop_ctx)
        } else {
            next
        };
        let catch_entry = self.build_block(catch_body, finally_entry, loop_ctx);
        let try_entry = self.build_block(try_body, finally_entry, loop_ctx);
        self.add_node(
            stmt.span,
            HashSet::default(),
            HashSet::default(),
            Vec::new(),
            vec![try_entry, catch_entry],
        )
    }
}

fn collect_statement_reads(stmt: &Stmt, reads: &mut HashSet<String>) {
    match stmt.kind() {
        StmtKind::VarDecl { value, onfail, .. } => {
            if let Some(value) = value {
                mention_expr(value, &mut |name| {
                    reads.insert(name.to_string());
                });
            }
            if let Some(onfail) = onfail {
                match &onfail.handler {
                    OnFailHandler::Expr(expr) => {
                        mention_expr(expr, &mut |name| {
                            reads.insert(name.to_string());
                        });
                    }
                    OnFailHandler::Block(_) => {}
                }
            }
        }
        StmtKind::Assign { name, op, value } => {
            mention_expr(value, &mut |name| {
                reads.insert(name.to_string());
            });
            if *op != AssignOp::Assign {
                reads.insert(name.clone());
            }
        }
        StmtKind::IndexAssign { name, index, value } => {
            reads.insert(name.clone());
            mention_expr(index, &mut |name| {
                reads.insert(name.to_string());
            });
            mention_expr(value, &mut |name| {
                reads.insert(name.to_string());
            });
        }
        StmtKind::FieldAssign { object, value, .. } => {
            mention_expr(object, &mut |name| {
                reads.insert(name.to_string());
            });
            mention_expr(value, &mut |name| {
                reads.insert(name.to_string());
            });
        }
        StmtKind::ArrayDestructure { value, .. } | StmtKind::ObjectDestructure { value, .. } => {
            mention_expr(value, &mut |name| {
                reads.insert(name.to_string());
            });
        }
        StmtKind::Expr(expr) => {
            mention_expr(expr, &mut |name| {
                reads.insert(name.to_string());
            });
        }
        _ => {}
    }
}

fn collect_statement_carriers(stmt: &Stmt, carriers: &mut Vec<(String, String)>) {
    match stmt.kind() {
        StmtKind::VarDecl {
            name,
            value: Some(value),
            ..
        }
        | StmtKind::Assign { name, value, .. } => {
            collect_expr_carriers(name, value, carriers);
        }
        _ => {}
    }
}

fn collect_expr_carriers(target: &str, expr: &Expr, carriers: &mut Vec<(String, String)>) {
    match expr.kind() {
        ExprKind::Borrow { name, .. } => {
            carriers.push((target.to_string(), name.clone()));
        }
        ExprKind::Ident(src) => {
            carriers.push((target.to_string(), src.clone()));
        }
        ExprKind::ArrowFn { params, body, .. } => {
            let captured = closure_capture_names(params, body);
            for cap in captured {
                carriers.push((target.to_string(), cap));
            }
        }
        _ => {}
    }
}
