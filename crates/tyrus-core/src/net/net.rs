use crate::ast::{Type, Type::*};
use crate::builtin_interfaces;
use crate::collections::HashMap;
use crate::module_loader::ExportInfo;

fn net_result(value: Type) -> Type {
    Type::Result {
        success: Box::new(value),
        error: Box::new(Named("NetError".to_string())),
    }
}

fn net_promise(value: Type) -> Type {
    Apply {
        name: "Promise".to_string(),
        args: vec![net_result(value)],
    }
}

pub(crate) fn net_builtins() -> HashMap<String, ExportInfo> {
    [
        (
            "udpBind".to_string(),
            ExportInfo {
                target: "__tyrus_net_udpBind".to_string(),
                ty: Function {
                    params: vec![Str],
                    ret: Box::new(net_promise(Named("UdpSocket".to_string()))),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "UdpSocket".to_string(),
            ExportInfo {
                target: "UdpSocket".to_string(),
                ty: Named("UdpSocket".to_string()),
                interface_fields: Some(Vec::new()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "UdpPacket".to_string(),
            ExportInfo {
                target: "UdpPacket".to_string(),
                ty: Named("UdpPacket".to_string()),
                interface_fields: Some(builtin_interfaces::udp_packet_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "ping".to_string(),
            ExportInfo {
                target: "__tyrus_net_ping".to_string(),
                ty: Function {
                    params: vec![Str, Optional(Box::new(U64))],
                    ret: Box::new(net_promise(U64)),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "traceroute".to_string(),
            ExportInfo {
                target: "__tyrus_net_traceroute".to_string(),
                ty: Function {
                    params: vec![Str, Optional(Box::new(U64))],
                    ret: Box::new(net_promise(Array(Box::new(Str)))),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "supportsIcmp".to_string(),
            ExportInfo {
                target: "__tyrus_net_supportsIcmp".to_string(),
                ty: Function {
                    params: Vec::new(),
                    ret: Box::new(Bool),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "tcpListen".to_string(),
            ExportInfo {
                target: "__tyrus_net_tcpListen".to_string(),
                ty: Function {
                    params: vec![Str, Optional(Box::new(U64))],
                    ret: Box::new(net_promise(Named("TcpListener".to_string()))),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "TcpListener".to_string(),
            ExportInfo {
                target: "TcpListener".to_string(),
                ty: Named("TcpListener".to_string()),
                interface_fields: Some(Vec::new()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "tcpConnect".to_string(),
            ExportInfo {
                target: "__tyrus_net_tcpConnect".to_string(),
                ty: Function {
                    params: vec![Str, Optional(Box::new(U64))],
                    ret: Box::new(net_promise(Named("TcpStream".to_string()))),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "TcpStream".to_string(),
            ExportInfo {
                target: "TcpStream".to_string(),
                ty: Named("TcpStream".to_string()),
                interface_fields: Some(Vec::new()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "resolve".to_string(),
            ExportInfo {
                target: "__tyrus_net_resolve".to_string(),
                ty: Function {
                    params: vec![Str, Optional(Box::new(U64))],
                    ret: Box::new(net_promise(Array(Box::new(Str)))),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "reverseLookup".to_string(),
            ExportInfo {
                target: "__tyrus_net_reverseLookup".to_string(),
                ty: Function {
                    params: vec![Str, Optional(Box::new(U64))],
                    ret: Box::new(net_promise(Str)),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "parseIpAddress".to_string(),
            ExportInfo {
                target: "__tyrus_net_parseIpAddress".to_string(),
                ty: Function {
                    params: vec![Str],
                    ret: Box::new(Optional(Box::new(Named("IpAddress".to_string())))),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "parseSocketAddress".to_string(),
            ExportInfo {
                target: "__tyrus_net_parseSocketAddress".to_string(),
                ty: Function {
                    params: vec![Str],
                    ret: Box::new(Optional(Box::new(Named("SocketAddress".to_string())))),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "NetError".to_string(),
            ExportInfo {
                target: "NetError".to_string(),
                ty: Type::Named("NetError".to_string()),
                interface_fields: Some(builtin_interfaces::net_error_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "IpAddress".to_string(),
            ExportInfo {
                target: "IpAddress".to_string(),
                ty: Type::Named("IpAddress".to_string()),
                interface_fields: Some(builtin_interfaces::ip_address_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "SocketAddress".to_string(),
            ExportInfo {
                target: "SocketAddress".to_string(),
                ty: Type::Named("SocketAddress".to_string()),
                interface_fields: Some(builtin_interfaces::socket_address_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
    ]
    .into_iter()
    .collect()
}
