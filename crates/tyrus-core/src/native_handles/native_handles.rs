use crate::ast::Type;

const OPAQUE_HANDLE_NAMES: &[&str] = &[
    "DigestStream",
    "FSWatcher",
    "HttpClientResponse",
    "HttpClientStreamResponse",
    "HttpRouter",
    "HttpServer",
    "ChildProcess",
    "Semaphore",
    "SemaphorePermit",
    "TcpListener",
    "TcpStream",
    "UdpSocket",
    "WebSocket",
];

const GENERIC_HANDLE_NAMES: &[&str] = &[
    "Mutex",
    "MutexGuard",
    "Receiver",
    "RwLock",
    "RwReadGuard",
    "RwWriteGuard",
    "Sender",
    "ReadableStream",
    "WritableStream",
];

const TWO_ARGUMENT_GENERIC_HANDLE_NAMES: &[&str] = &["DuplexStream", "TransformStream"];

pub fn is_opaque_handle_name(name: &str) -> bool {
    OPAQUE_HANDLE_NAMES.contains(&name)
}

pub fn is_handle_type(ty: &Type) -> bool {
    match ty {
        Type::Optional(inner) => is_handle_type(inner),
        Type::Readonly(inner) | Type::Ref(inner) | Type::MutRef(inner) => is_handle_type(inner),
        Type::Named(name) => is_opaque_handle_name(name),
        Type::Apply { name, args } => {
            (GENERIC_HANDLE_NAMES.contains(&name.as_str()) && args.len() == 1)
                || (TWO_ARGUMENT_GENERIC_HANDLE_NAMES.contains(&name.as_str()) && args.len() == 2)
        }
        _ => false,
    }
}
