use crate::ast::Type;

pub const TEXT_BODY: &str = "TextBody";

pub const BYTES_BODY: &str = "BytesBody";

pub const STREAM_BODY: &str = "StreamBody";

pub const MULTIPART_BODY: &str = "MultipartBody";

pub fn shapeless_body_type(schema: &str) -> Option<Type> {
    match schema {
        TEXT_BODY => Some(Type::Str),
        BYTES_BODY => Some(Type::Array(Box::new(Type::U8))),
        STREAM_BODY => Some(Type::Apply {
            name: "ReadableStream".to_string(),
            args: vec![Type::Array(Box::new(Type::U8))],
        }),
        MULTIPART_BODY => Some(Type::Array(Box::new(Type::Named(
            "MultipartPart".to_string(),
        )))),
        _ => None,
    }
}

pub fn is_request_only_body(schema: &str) -> bool {
    schema == MULTIPART_BODY
}

pub fn is_shapeless_body(schema: &str) -> bool {
    shapeless_body_type(schema).is_some()
}

pub fn request_channel_type(channel: &str, schema: &str) -> Type {
    match shapeless_body_type(schema) {
        Some(ty) if channel == "body" => ty,
        _ => Type::Named(schema.to_string()),
    }
}

pub fn fetch_verb_method(verb: &str) -> Option<&'static str> {
    Some(match verb {
        "get" => "GET",
        "post" => "POST",
        "put" => "PUT",
        "patch" => "PATCH",
        "delete" => "DELETE",
        "head" => "HEAD",
        "options" => "OPTIONS",
        "connect" => "CONNECT",
        "query" => "QUERY",
        "trace" => "TRACE",
        _ => return None,
    })
}
