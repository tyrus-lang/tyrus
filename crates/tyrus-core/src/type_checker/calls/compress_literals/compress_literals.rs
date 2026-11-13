use super::archive_literals::integer_literal;
use super::*;

impl TypeChecker {
    /// A literal `level` outside the range of a literal `CompressFormat` member
    /// can only fail at run time, so it is reported where it is written, for
    /// `compress` and for `compressionStream`.
    pub(super) fn check_compress_literals(&mut self, args: &[Expr], params: &[Type], ret: &Type) {
        // Compared with the registered signature, the single source of
        // `compress`'s type, so a change to it cannot silently turn this off.
        let signature = Type::Function {
            params: params.to_vec(),
            ret: Box::new(ret.clone()),
        };
        let format_index = [
            (crate::compress::COMPRESS_TARGET, 1),
            (crate::compress::COMPRESSION_STREAM_TARGET, 0),
        ]
        .into_iter()
        .find(|(target, _)| {
            crate::compress::compress_target_type(target).as_ref() == Some(&signature)
        })
        .map(|(_, index)| index);
        let Some(format_index) = format_index else {
            return;
        };
        let Some(ExprKind::FieldAccess { object, field }) = args.get(format_index).map(Expr::kind)
        else {
            return;
        };
        let ExprKind::Ident(enum_name) = object.kind() else {
            return;
        };
        if self.enums.get(enum_name) != Some(&crate::compress::compress_format_enum()) {
            return;
        }
        let Some((_, _, lowest, highest)) = crate::compress::COMPRESS_FORMATS
            .iter()
            .find(|(member, ..)| member == field)
        else {
            return;
        };
        let Some(ExprKind::ObjectLit(fields)) = args.get(format_index + 1).map(Expr::kind) else {
            return;
        };
        let Some((_, level)) = fields.iter().find(|(name, _)| name == "level") else {
            return;
        };
        let Some(value) = integer_literal(level) else {
            return;
        };
        if !(*lowest..=*highest).contains(&value) {
            self.error_at_with_help(
                "E3312",
                format!(
                    "compression level `{value}` is outside the range `{lowest}` to `{highest}` of `{enum_name}.{field}`"
                ),
                level.span,
                vec![format!(
                    "use a `level` from `{lowest}` (fastest) to `{highest}` (smallest output) for `{enum_name}.{field}`"
                )],
            );
        }
    }
}
