use super::*;

fn string_array(value: &TyrusStringArray, name: &str) -> Result<Vec<String>, String> {
    let length = value.length;
    let capacity = value.capacity;
    if length > capacity || (length > 0 && value.data.is_null()) {
        return Err(format!("CORS {name} array has an invalid native layout"));
    }
    let values = if length == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(value.data, length) }
    };
    values
        .iter()
        .map(|value| text(value).map_err(|()| format!("CORS {name} contains invalid UTF-8")))
        .collect()
}

fn cors_layer(options: &CorsOptions) -> Result<CorsLayer, String> {
    let origins = string_array(&options.origin, "origin")?;
    let methods = string_array(&options.methods, "methods")?;
    let allowed_headers = string_array(&options.allowed_headers, "allowedHeaders")?;
    let exposed_headers = string_array(&options.exposed_headers, "exposedHeaders")?;
    let uses_wildcard = [&origins, &methods, &allowed_headers]
        .iter()
        .any(|values| values.iter().any(|value| value == "*"));
    if options.credentials && uses_wildcard {
        return Err(
            "CORS credentials cannot be combined with wildcard origin, methods, or headers"
                .to_string(),
        );
    }

    let mut layer = CorsLayer::new().allow_credentials(options.credentials);
    layer = if origins.as_slice() == ["*"] {
        layer.allow_origin(Any)
    } else {
        let origins = origins
            .iter()
            .map(|value| {
                value
                    .parse::<HeaderValue>()
                    .map_err(|_| format!("invalid CORS origin `{value}`"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if origins.is_empty() {
            return Err("CORS origin cannot be empty".to_string());
        }
        layer.allow_origin(AllowOrigin::list(origins))
    };
    layer = if methods.as_slice() == ["*"] {
        layer.allow_methods(Any)
    } else {
        let methods = methods
            .iter()
            .map(|value| {
                Method::from_bytes(value.as_bytes())
                    .map_err(|_| format!("invalid CORS method `{value}`"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        layer.allow_methods(AllowMethods::list(methods))
    };
    layer = if allowed_headers.as_slice() == ["*"] {
        layer.allow_headers(Any)
    } else {
        let headers = allowed_headers
            .iter()
            .map(|value| {
                value
                    .parse::<HeaderName>()
                    .map_err(|_| format!("invalid CORS allowed header `{value}`"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        layer.allow_headers(AllowHeaders::list(headers))
    };
    if !exposed_headers.is_empty() {
        let headers = exposed_headers
            .iter()
            .map(|value| {
                value
                    .parse::<HeaderName>()
                    .map_err(|_| format!("invalid CORS exposed header `{value}`"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        layer = layer.expose_headers(ExposeHeaders::list(headers));
    }
    if options.max_age > 0 {
        layer = layer.max_age(Duration::from_secs(options.max_age as u64));
    }
    Ok(layer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_cors(
    router: *mut HttpRouter,
    options: *const CorsOptions,
) -> i32 {
    unsafe {
        let (Some(router), Some(options)) = (router.as_mut(), options.as_ref()) else {
            crate::http::set_http_error(
                "ERR_HTTP_CORS_CONFIG",
                "router or CORS options are invalid",
            );
            return -1;
        };
        match cors_layer(options) {
            Ok(layer) => {
                router.cors = Some(layer);
                0
            }
            Err(_) => {
                crate::http::set_http_error(
                    "ERR_HTTP_CORS_CONFIG",
                    "CORS configuration is invalid",
                );
                -1
            }
        }
    }
}
