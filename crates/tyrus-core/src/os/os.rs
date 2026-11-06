use crate::ast::*;
use crate::collections::HashMap;
use crate::module_loader::ExportInfo;

#[path = "os_builtins/os_builtins.rs"]
mod os_builtins;

pub(crate) use os_builtins::os_builtins;

pub fn os_platform_enum() -> ResolvedEnum {
    ResolvedEnum {
        repr: Type::I32,
        nominal: false,
        members: vec![
            ("Linux".to_string(), Expr::synthetic(ExprKind::IntLit(0))),
            ("MacOS".to_string(), Expr::synthetic(ExprKind::IntLit(1))),
            ("Windows".to_string(), Expr::synthetic(ExprKind::IntLit(2))),
            ("Other".to_string(), Expr::synthetic(ExprKind::IntLit(3))),
        ],
        member_kinds: vec![EnumPayload::Unit; 4],
        is_algebraic: false,
    }
}

pub fn os_arch_enum() -> ResolvedEnum {
    ResolvedEnum {
        repr: Type::I32,
        nominal: false,
        members: vec![
            ("X86_64".to_string(), Expr::synthetic(ExprKind::IntLit(0))),
            ("Aarch64".to_string(), Expr::synthetic(ExprKind::IntLit(1))),
            ("X86".to_string(), Expr::synthetic(ExprKind::IntLit(2))),
            ("Arm".to_string(), Expr::synthetic(ExprKind::IntLit(3))),
            ("Other".to_string(), Expr::synthetic(ExprKind::IntLit(4))),
        ],
        member_kinds: vec![EnumPayload::Unit; 5],
        is_algebraic: false,
    }
}

pub fn notify_urgency_enum() -> ResolvedEnum {
    ResolvedEnum {
        repr: Type::I32,
        nominal: false,
        members: vec![
            ("Low".to_string(), Expr::synthetic(ExprKind::IntLit(0))),
            ("Normal".to_string(), Expr::synthetic(ExprKind::IntLit(1))),
            ("Critical".to_string(), Expr::synthetic(ExprKind::IntLit(2))),
        ],
        member_kinds: vec![EnumPayload::Unit; 3],
        is_algebraic: false,
    }
}

pub fn system_capability_enum() -> ResolvedEnum {
    ResolvedEnum {
        repr: Type::I32,
        nominal: false,
        members: vec![
            (
                "Notification".to_string(),
                Expr::synthetic(ExprKind::IntLit(0)),
            ),
            (
                "Clipboard".to_string(),
                Expr::synthetic(ExprKind::IntLit(1)),
            ),
            ("Battery".to_string(), Expr::synthetic(ExprKind::IntLit(2))),
            ("DarkMode".to_string(), Expr::synthetic(ExprKind::IntLit(3))),
            (
                "ScreenLock".to_string(),
                Expr::synthetic(ExprKind::IntLit(4)),
            ),
            (
                "SessionLock".to_string(),
                Expr::synthetic(ExprKind::IntLit(5)),
            ),
            (
                "SleepPrevention".to_string(),
                Expr::synthetic(ExprKind::IntLit(6)),
            ),
            ("Fonts".to_string(), Expr::synthetic(ExprKind::IntLit(7))),
        ],
        member_kinds: vec![EnumPayload::Unit; 8],
        is_algebraic: false,
    }
}
