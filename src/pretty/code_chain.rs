use pretty::DocAllocator;
use typst_syntax::{ast::*, SyntaxKind, SyntaxNode};

use crate::PrettyPrinter;

use super::{
    chain::{iterate_deep_nodes, ChainStyle, ChainStylist},
    ArenaDoc,
};

impl<'a> PrettyPrinter<'a> {
    pub(super) fn convert_field_access(&'a self, field_access: FieldAccess<'a>) -> ArenaDoc<'a> {
        if self.current_mode().is_code() {
            return self.convert_dot_chain(field_access.to_untyped());
        }
        self.convert_expr(field_access.target())
            + self.arena.text(".")
            + self.convert_ident(field_access.field())
    }

    pub(super) fn convert_dot_chain(&'a self, node: &'a SyntaxNode) -> ArenaDoc<'a> {
        ChainStylist::new(self)
            .process_resolved(
                resolve_dot_chain(node),
                |node| node.kind() == SyntaxKind::FieldAccess,
                |child| child.kind() == SyntaxKind::Dot,
                |child| child.cast().map(|ident| self.convert_ident(ident)),
                |node| {
                    if let Some(func_call) = node.cast::<FuncCall>() {
                        // There is no comment allowed, so we can directly convert args.
                        Some(self.convert_args(func_call.args()))
                    } else {
                        node.cast().map(|expr| self.convert_expr(expr))
                    }
                },
            )
            .print_doc(ChainStyle {
                no_break_single: true,
                ..Default::default()
            })
    }

    pub(super) fn convert_binary_chain(&'a self, binary: Binary<'a>) -> ArenaDoc<'a> {
        let prec = binary.op().precedence();
        ChainStylist::new(self)
            .process_resolved(
                resolve_binary_chain(binary),
                |node| {
                    node.cast::<Binary>()
                        .is_some_and(|binary| binary.op().precedence() == prec)
                },
                |child| BinOp::from_kind(child.kind()).is_some(),
                |child| child.cast().map(|expr| self.convert_expr(expr)),
                |node| node.cast().map(|expr| self.convert_expr(expr)),
            )
            .print_doc(ChainStyle {
                space_around_op: true,
                ..Default::default()
            })
    }
}

pub(super) fn resolve_dot_chain(node: &SyntaxNode) -> impl Iterator<Item = &SyntaxNode> {
    iterate_deep_nodes(node, |current| {
        if let Some(field_access) = current.cast::<FieldAccess>() {
            Some(field_access.target().to_untyped())
        } else if let Some(func_call) = current.cast::<FuncCall>() {
            Some(func_call.callee().to_untyped())
        } else {
            None
        }
    })
}

pub(super) fn resolve_binary_chain(binary: Binary<'_>) -> impl Iterator<Item = &'_ SyntaxNode> {
    let prec = binary.op().precedence();
    iterate_deep_nodes(binary.to_untyped(), move |current| {
        if let Some(binary) = current.cast::<Binary>() {
            if binary.op().precedence() == prec {
                return Some(binary.lhs().to_untyped());
            }
        }
        None
    })
}
