use super::*;

/// What `pack` would write for a destination spelled as a literal. `None`
/// means the extension is not one `pack` accepts, lowercase only.
fn archive_extension(destination: &str) -> Option<ArchiveExtension> {
    let name = destination
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(destination);
    let has = |extension: &str| name.len() > extension.len() && name.ends_with(extension);
    if has(".tar.gz") || has(".tgz") {
        Some(ArchiveExtension::Compressed)
    } else if has(".tar") {
        Some(ArchiveExtension::Tar)
    } else if has(".zip") {
        Some(ArchiveExtension::Compressed)
    } else {
        None
    }
}

#[derive(Clone, Copy)]
enum ArchiveExtension {
    Compressed,
    Tar,
}

pub(super) fn integer_literal(expr: &Expr) -> Option<i128> {
    match expr.kind() {
        ExprKind::IntLit(value) => Some(*value),
        ExprKind::UIntLit(value) => i128::try_from(*value).ok(),
        ExprKind::Neg(inner) => integer_literal(inner).map(|value| -value),
        _ => None,
    }
}

impl TypeChecker {
    /// `pack` decides the format from the destination's extension and the
    /// runtime rejects anything else, so a literal destination or level that
    /// can only fail is reported here, where the mistake is written.
    pub(super) fn check_archive_pack_literals(
        &mut self,
        args: &[Expr],
        params: &[Type],
        ret: &Type,
    ) {
        // Compared with the registered signature, the single source of `pack`'s
        // type, so a change to that signature cannot silently turn this off.
        let signature = Type::Function {
            params: params.to_vec(),
            ret: Box::new(ret.clone()),
        };
        if crate::archive::archive_target_type(crate::archive::PACK_STREAM_TARGET).as_ref()
            == Some(&signature)
        {
            self.check_archive_pack_stream_literals(args);
            return;
        }
        if crate::archive::archive_target_type(crate::archive::PACK_TARGET).as_ref()
            != Some(&signature)
        {
            return;
        }
        let extension = match args.get(1).map(Expr::kind) {
            Some(ExprKind::StrLit(destination)) => match archive_extension(destination) {
                Some(extension) => Some(extension),
                None => {
                    self.error_at_with_help(
                        "E3311",
                        format!(
                            "`pack` cannot tell the archive format of `{destination}`: the destination must end with `.zip`, `.tar`, `.tar.gz` or `.tgz`"
                        ),
                        args[1].span,
                        vec![format!(
                            "end `{destination}` with `.zip`, `.tar`, `.tar.gz` or `.tgz`; the extension is lowercase and decides the format"
                        )],
                    );
                    return;
                }
            },
            _ => None,
        };
        self.check_archive_level(args, extension, "write a `.tar.gz` or `.zip` destination");
    }

    /// `packStream` names its format with an `ArchiveFormat` member instead of
    /// an extension; a literal member is checked the same way.
    fn check_archive_pack_stream_literals(&mut self, args: &[Expr]) {
        let Some(ExprKind::FieldAccess { object, field }) = args.get(1).map(Expr::kind) else {
            return;
        };
        let ExprKind::Ident(enum_name) = object.kind() else {
            return;
        };
        if self.enums.get(enum_name) != Some(&crate::archive::archive_format_enum()) {
            return;
        }
        let extension = match field.as_str() {
            "Tar" => ArchiveExtension::Tar,
            "Zip" | "TarGz" => ArchiveExtension::Compressed,
            _ => return,
        };
        let compressed = format!("use `{enum_name}.TarGz` or `{enum_name}.Zip`");
        self.check_archive_level(args, Some(extension), &compressed);
    }

    fn check_archive_level(
        &mut self,
        args: &[Expr],
        extension: Option<ArchiveExtension>,
        compressed: &str,
    ) {
        let Some(ExprKind::ObjectLit(fields)) = args.get(2).map(Expr::kind) else {
            return;
        };
        let Some((_, level)) = fields.iter().find(|(name, _)| name == "level") else {
            return;
        };
        let Some(value) = integer_literal(level) else {
            return;
        };
        match extension {
            Some(ArchiveExtension::Tar) => self.error_at_with_help(
                "E3312",
                "`level` has no effect on a `.tar` archive, which is never compressed".to_string(),
                level.span,
                vec![format!(
                    "remove `level`, or {compressed} to compress the archive"
                )],
            ),
            _ if !(0..=9).contains(&value) => self.error_at_with_help(
                "E3312",
                format!("compression level `{value}` is outside the range `0` to `9`"),
                level.span,
                vec!["use a `level` from `0` (store only) to `9` (smallest output)".to_string()],
            ),
            _ => {}
        }
    }
}
