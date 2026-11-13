use super::*;
use crate::temporal::Instant;
use axum::body::to_bytes;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tower::ServiceExt;
use tyrus_string::TyrusString;

static HEALTH_BODY_TEXT: &str = "{ \"status\": \"ok\" }";
static NOT_FOUND_BODY_TEXT: &str = "{ \"error\": \"not found\" }";
static RESPONSE_HOOK_CALLS: AtomicUsize = AtomicUsize::new(0);
static SLOW_RESPONSE_ADDRESS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn record_response(access: *const HttpAccessLog) {
    unsafe {
        assert_eq!((*access).status, 200);
        assert!((*access).duration.seconds >= 0);
        RESPONSE_HOOK_CALLS.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn request_ids_are_unpadded_base64url() {
    const BASE64_URL: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    assert_eq!(
        crate::http::request_id_from_counter(1, BASE64_URL).0,
        "req-AQ"
    );
    assert_eq!(
        crate::http::request_id_from_counter(255, BASE64_URL).0,
        "req-_w"
    );
    assert_eq!(
        crate::http::request_id_from_counter(256, BASE64_URL).0,
        "req-AQA"
    );
}

#[test]
fn response_observation_is_opt_in() {
    assert!(!crate::http::should_observe_response(false, &[]));
    assert!(crate::http::should_observe_response(true, &[]));
    assert!(crate::http::should_observe_response(
        false,
        &[record_response]
    ));
}

#[test]
fn rejected_http_router_configuration_returns_a_structured_error() {
    crate::http::tyrus_http_clear_error();
    let status = unsafe { crate::http::tyrus_http_router_on_response(ptr::null_mut(), None) };
    assert_eq!(status, -1);

    let mut error = crate::abi::HttpErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { crate::http::tyrus_http_take_error(&mut error) };
    assert_eq!(unsafe { error.code.as_str() }, "ERR_HTTP_RESPONSE_HOOK");
    assert_eq!(
        unsafe { error.message.as_str() },
        "router or response hook is invalid"
    );
}

unsafe extern "C" fn health_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        (*response).status = 200;
        (*response).body = HttpBody::from_text(TyrusString::literal(HEALTH_BODY_TEXT));
        ptr::null_mut()
    }
}

unsafe extern "C" fn etag_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        (*response).status = 200;
        (*response).body = HttpBody::from_text(TyrusString::literal(HEALTH_BODY_TEXT));
        let weak = false;
        let last_modified = Instant {
            seconds: 0,
            nanoseconds: 0,
        };
        let control = TyrusString::literal("private, max-age=60");
        let cache = HttpCacheOptions {
            etag: TyrusString::literal("users-v1"),
            weak: &weak,
            last_modified: &last_modified,
            control: &control,
        };
        let options = HttpReplyOptions { cache: &cache };
        assert_eq!(tyrus_http_reply_apply_options(response, &options), 0);
        ptr::null_mut()
    }
}

unsafe extern "C" fn last_modified_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        (*response).status = 200;
        (*response).body = HttpBody::from_text(TyrusString::literal(HEALTH_BODY_TEXT));
        assert_eq!(
            tyrus_http_reply_header(
                response,
                TyrusString::literal("last-modified"),
                TyrusString::literal("Wed, 21 Oct 2015 07:28:00 GMT"),
            ),
            0
        );
        ptr::null_mut()
    }
}

unsafe extern "C" fn compressible_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        (*response).status = 200;
        let body = "tyrus compression ".repeat(160);
        (*response).body = HttpBody::from_text(crate::strings::alloc_string(&body));
        ptr::null_mut()
    }
}

unsafe extern "C" fn cookie_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        (*response).status = 200;
        (*response).body = HttpBody::from_text(TyrusString::literal(HEALTH_BODY_TEXT));
        let name = TyrusString::literal("session");
        let value = TyrusString::literal("abc");
        assert_eq!(
            tyrus_http_reply_set_cookie(response, name.borrow(), value.borrow()),
            0
        );
        let preview = TyrusString::literal("preview");
        assert_eq!(tyrus_http_reply_clear_cookie(response, preview.borrow()), 0);
        ptr::null_mut()
    }
}

unsafe extern "C" fn cookie_options_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        (*response).status = 200;
        (*response).body = HttpBody::from_text(TyrusString::literal(HEALTH_BODY_TEXT));
        let path = TyrusString::literal("/");
        let same_site = TyrusString::literal("lax");
        let max_age = 3600_i64;
        let secure = true;
        let http_only = true;
        let options = CookieOptions {
            domain: std::ptr::null(),
            path: &path,
            max_age: &max_age,
            secure: &secure,
            http_only: &http_only,
            same_site: &same_site,
        };
        assert_eq!(
            tyrus_http_reply_set_cookie_with_options(
                response,
                TyrusString::literal("session"),
                TyrusString::literal("abc"),
                &options,
            ),
            0
        );
        let insecure = false;
        let none = TyrusString::literal("none");
        let invalid_options = CookieOptions {
            secure: &insecure,
            same_site: &none,
            ..options
        };
        assert_eq!(
            tyrus_http_reply_set_cookie_with_options(
                response,
                TyrusString::literal("invalid"),
                TyrusString::literal("abc"),
                &invalid_options,
            ),
            -1
        );
        ptr::null_mut()
    }
}

unsafe extern "C" fn cookie_list_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        (*response).status = 200;
        (*response).body = HttpBody::from_text(TyrusString::literal(HEALTH_BODY_TEXT));
        let path = TyrusString::literal("/");
        let http_only = true;
        let options = CookieOptions {
            domain: std::ptr::null(),
            path: &path,
            max_age: std::ptr::null(),
            secure: std::ptr::null(),
            http_only: &http_only,
            same_site: std::ptr::null(),
        };
        let mut cookies = [
            crate::types::HttpCookie {
                name: TyrusString::literal("session"),
                value: TyrusString::literal("abc"),
                options: &options,
            },
            crate::types::HttpCookie {
                name: TyrusString::literal("theme"),
                value: TyrusString::literal("dark"),
                options: std::ptr::null(),
            },
        ];
        let list = crate::abi::TyrusArray {
            data: cookies.as_mut_ptr().cast(),
            length: cookies.len(),
            capacity: cookies.len(),
        };
        assert_eq!(tyrus_http_reply_set_cookies(response, &list), 0);
        let mut rejected = [
            crate::types::HttpCookie {
                name: TyrusString::literal("kept"),
                value: TyrusString::literal("1"),
                options: std::ptr::null(),
            },
            crate::types::HttpCookie {
                name: TyrusString::literal("bad name"),
                value: TyrusString::literal("1"),
                options: std::ptr::null(),
            },
        ];
        let rejected_list = crate::abi::TyrusArray {
            data: rejected.as_mut_ptr().cast(),
            length: rejected.len(),
            capacity: rejected.len(),
        };
        assert_eq!(tyrus_http_reply_set_cookies(response, &rejected_list), -1);
        let empty = crate::abi::TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        assert_eq!(tyrus_http_reply_set_cookies(response, &empty), 0);
        assert_eq!(tyrus_http_reply_clear_cookies(response), 0);
        ptr::null_mut()
    }
}

unsafe extern "C" fn headers_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        (*response).status = 200;
        (*response).body = HttpBody::from_text(TyrusString::literal(HEALTH_BODY_TEXT));
        assert_eq!(
            tyrus_http_reply_header(
                response,
                TyrusString::literal("x-request-id"),
                TyrusString::literal("first"),
            ),
            0
        );
        assert_eq!(
            tyrus_http_reply_header(
                response,
                TyrusString::literal("x-request-id"),
                TyrusString::literal("second"),
            ),
            0
        );
        assert_eq!(
            tyrus_http_reply_header(
                response,
                TyrusString::literal("x-transient"),
                TyrusString::literal("remove-me"),
            ),
            0
        );
        assert_eq!(
            tyrus_http_reply_remove_header(response, TyrusString::literal("x-transient")),
            0
        );
        assert_eq!(
            tyrus_http_reply_append_header(
                response,
                TyrusString::literal("link"),
                TyrusString::literal("</users?page=2>; rel=next"),
            ),
            0
        );
        assert_eq!(
            tyrus_http_reply_append_header(
                response,
                TyrusString::literal("link"),
                TyrusString::literal("</users?page=1>; rel=prev"),
            ),
            0
        );
        assert_eq!(
            tyrus_http_reply_type(response, TyrusString::literal("application/problem+json"),),
            0
        );
        assert_eq!(
            tyrus_http_reply_header(
                response,
                TyrusString::literal("content-length"),
                TyrusString::literal("1"),
            ),
            -1
        );
        assert_eq!(
            tyrus_http_reply_header(
                response,
                TyrusString::literal("x-invalid"),
                TyrusString::literal("value\r\ninjected: true"),
            ),
            -1
        );
        ptr::null_mut()
    }
}

unsafe extern "C" fn expected_error_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        (*response).status = 404;
        (*response).body = HttpBody::from_text(TyrusString::literal(
            r#"{"statusCode":404,"code":"not_found","message":"User not found"}"#,
        ));
        ptr::null_mut()
    }
}

#[tokio::test]
async fn error_responses_preserve_the_declared_body() {
    for status in [404, 500] {
        let mut source = HttpResponse::empty();
        source.status = status;
        source.body =
            HttpBody::from_text(TyrusString::literal(r#"{ "message": "Request failed" }"#));
        unsafe {
            assert_eq!(
                tyrus_http_reply_type(
                    &mut source,
                    TyrusString::literal("application/json; charset=utf-8"),
                ),
                0
            );
        }

        let source_key = &source as *const HttpResponse;
        let response =
            crate::http::finalize_response(source, source_key, false, false, None, None, None);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value.get("code"), None);
        assert_eq!(value["message"], "Request failed");
    }
}

#[tokio::test]
async fn an_explicit_error_code_is_preserved() {
    let mut source = HttpResponse::empty();
    source.status = 404;
    source.body = HttpBody::from_text(TyrusString::literal(
        r#"{"code":"wrong_code","message":"User not found"}"#,
    ));
    unsafe {
        assert_eq!(
            tyrus_http_reply_type(
                &mut source,
                TyrusString::literal("application/json; charset=utf-8"),
            ),
            0
        );
    }

    let source_key = &source as *const HttpResponse;
    let response =
        crate::http::finalize_response(source, source_key, false, false, None, None, None);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["code"], "wrong_code");
    assert_eq!(value["message"], "User not found");
}

unsafe extern "C" fn binary_body_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        let bytes: [u8; 5] = [0, 255, 10, 0, 66];
        (*response).status = 200;
        assert_eq!(
            tyrus_http_reply_bytes(response, bytes.as_ptr(), bytes.len()),
            0
        );
        ptr::null_mut()
    }
}

unsafe extern "C" fn empty_binary_body_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        (*response).status = 200;
        assert_eq!(tyrus_http_reply_bytes(response, ptr::null(), 0), 0);
        ptr::null_mut()
    }
}

unsafe extern "C" fn streamed_request_handler(
    request: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        let stream = crate::stream::tyrus_readable_stream_retain((*request).body_stream);
        let promise = tyrus_promise_pending();
        tyrus_promise_retain(promise);
        let response = response as usize;
        let promise_address = promise as usize;
        let stream = crate::stream::take_owned_readable_stream(stream);
        crate::executor::executor().spawn(async move {
            let bytes = crate::stream::read_owned_byte_stream(&stream)
                .await
                .unwrap()
                .unwrap();
            let response = response as *mut HttpResponse;
            let promise = promise_address as *mut TyrusPromise;
            {
                (*response).status = 200;
                assert_eq!(
                    tyrus_http_reply_bytes(response, bytes.as_ptr(), bytes.len()),
                    0
                );
                tyrus_promise_fulfill(promise, ptr::null_mut(), None);
                tyrus_promise_free(promise);
            }
        });
        promise
    }
}

unsafe extern "C" fn streamed_relay_handler(
    request: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        (*response).status = 200;
        assert_eq!(tyrus_http_reply_stream(response, (*request).body_stream), 0);
        ptr::null_mut()
    }
}

unsafe extern "C" fn delayed_writable_reply_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        (*response).status = 200;
        let mut writer = ptr::null();
        assert_eq!(tyrus_http_reply_writable_stream(response, &mut writer), 0);
        let promise = tyrus_promise_pending();
        tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        let writer_address = writer as usize;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(60));
            let bytes = [0u8, 255, 10, 0, 66];
            let data = { libc::malloc(bytes.len()) }.cast::<u8>();
            assert!(!data.is_null());
            ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
            let payload = Box::into_raw(Box::new(TyrusArray {
                data,
                length: bytes.len(),
                capacity: bytes.len(),
            }))
            .cast();
            let writer = writer_address as *const crate::stream::TyrusWritableStream;
            let write = {
                crate::stream::tyrus_writable_stream_write(
                    writer,
                    payload,
                    bytes.len(),
                    Some(crate::async_payload::tyrus_drop_async_copy_array),
                )
            };
            crate::wait_until_settled(write);
            crate::tyrus_promise_free(write);
            let close = { crate::stream::tyrus_writable_stream_close(writer) };
            {
                crate::tyrus_promise_free(close);
                crate::stream::tyrus_writable_stream_free(writer);
                tyrus_promise_fulfill(promise_address as *mut TyrusPromise, ptr::null_mut(), None);
                tyrus_promise_free(promise_address as *mut TyrusPromise);
            }
        });
        promise
    }
}

#[tokio::test]
async fn a_writable_reply_stream_transfers_bytes_without_buffering_the_response() {
    let mut response = HttpResponse::empty();
    response.status = 200;
    let mut writer = ptr::null();
    unsafe {
        assert_eq!(
            tyrus_http_reply_writable_stream(&mut response, &mut writer),
            0
        );
    }
    assert!(!writer.is_null());

    let bytes = [0u8, 255, 10, 0, 66];
    let data = unsafe { libc::malloc(bytes.len()) }.cast::<u8>();
    assert!(!data.is_null());
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len()) };
    let payload = Box::into_raw(Box::new(TyrusArray {
        data,
        length: bytes.len(),
        capacity: bytes.len(),
    }))
    .cast();
    let write = unsafe {
        crate::stream::tyrus_writable_stream_write(
            writer,
            payload,
            bytes.len(),
            Some(crate::async_payload::tyrus_drop_async_copy_array),
        )
    };
    unsafe { crate::wait_until_settled(write) };
    assert_eq!(
        unsafe { crate::tyrus_promise_state(write) },
        crate::TYRUS_PROMISE_FULFILLED
    );
    unsafe { crate::tyrus_promise_free(write) };
    let close = unsafe { crate::stream::tyrus_writable_stream_close(writer) };
    unsafe { crate::tyrus_promise_free(close) };

    let response =
        crate::http::finalize_response(response, ptr::null(), false, false, None, None, None);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        bytes
    );
    unsafe { crate::stream::tyrus_writable_stream_free(writer) };
}

#[tokio::test]
async fn writable_reply_stream_sends_the_response_before_its_handler_settles() {
    let router = tyrus_http_router_new();
    let path = TyrusString::literal("/progressive");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                path.borrow(),
                Some(delayed_writable_reply_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };
    let response = tokio::time::timeout(
        std::time::Duration::from_millis(25),
        app.oneshot(
            Request::builder()
                .uri("/progressive")
                .body(Body::empty())
                .unwrap(),
        ),
    )
    .await
    .expect("opening the writable response must not wait for the handler")
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        [0u8, 255, 10, 0, 66]
    );
    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn dropping_a_writable_http_response_rejects_a_blocked_writer() {
    let mut response = HttpResponse::empty();
    response.status = 200;
    let mut writer = ptr::null();
    unsafe {
        assert_eq!(
            tyrus_http_reply_writable_stream(&mut response, &mut writer),
            0
        );
    }

    let full_buffer = vec![7u8; 64 * 1024];
    let data = unsafe { libc::malloc(full_buffer.len()) }.cast::<u8>();
    assert!(!data.is_null());
    unsafe { ptr::copy_nonoverlapping(full_buffer.as_ptr(), data, full_buffer.len()) };
    let payload = Box::into_raw(Box::new(TyrusArray {
        data,
        length: full_buffer.len(),
        capacity: full_buffer.len(),
    }))
    .cast();
    let first = unsafe {
        crate::stream::tyrus_writable_stream_write(
            writer,
            payload,
            full_buffer.len(),
            Some(crate::async_payload::tyrus_drop_async_copy_array),
        )
    };
    unsafe { crate::wait_until_settled(first) };
    unsafe { crate::tyrus_promise_free(first) };

    let one_byte = unsafe { libc::malloc(1) }.cast::<u8>();
    assert!(!one_byte.is_null());
    unsafe { *one_byte = 9 };
    let blocked_payload = Box::into_raw(Box::new(TyrusArray {
        data: one_byte,
        length: 1,
        capacity: 1,
    }))
    .cast();
    let blocked = unsafe {
        crate::stream::tyrus_writable_stream_write(
            writer,
            blocked_payload,
            1,
            Some(crate::async_payload::tyrus_drop_async_copy_array),
        )
    };
    assert_eq!(
        unsafe { crate::tyrus_promise_state(blocked) },
        crate::TYRUS_PROMISE_PENDING
    );

    let response =
        crate::http::finalize_response(response, ptr::null(), false, false, None, None, None);
    drop(response);
    unsafe { crate::wait_until_settled(blocked) };
    assert_eq!(
        unsafe { crate::tyrus_promise_state(blocked) },
        crate::TYRUS_PROMISE_REJECTED
    );
    unsafe {
        crate::tyrus_promise_free(blocked);
        crate::stream::tyrus_writable_stream_free(writer);
    }
}

unsafe extern "C" fn invalid_response_handler(
    _: *const HttpRequest,
    _: *mut HttpResponse,
) -> *mut TyrusPromise {
    ptr::null_mut()
}

unsafe extern "C" fn validation_error_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        let message = TyrusString::literal("Invalid request body");
        tyrus_http_set_validation_error(response, 400, message.borrow());
        ptr::null_mut()
    }
}

unsafe extern "C" fn recorded_issue_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        crate::validation::tyrus_validation_issues_begin();
        crate::validation::tyrus_validation_issue_add(
            TyrusString::literal("email"),
            TyrusString::literal("@email()"),
            TyrusString::literal("value must be a valid e-mail address"),
        );
        tyrus_http_set_validation_error(
            response,
            400,
            TyrusString::literal("Invalid request body").borrow(),
        );
        ptr::null_mut()
    }
}

unsafe extern "C" fn failed_response_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        crate::validation::tyrus_validation_issues_begin();
        crate::validation::tyrus_validation_issue_add(
            TyrusString::literal("internalToken"),
            TyrusString::literal("@minLength(5)"),
            TyrusString::literal("value must be at least 5 characters long"),
        );
        tyrus_http_set_response_error(
            response,
            500,
            TyrusString::literal("the response body does not satisfy the declared schema").borrow(),
        );
        ptr::null_mut()
    }
}

unsafe extern "C" fn not_found_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        (*response).status = 404;
        (*response).body = HttpBody::from_text(TyrusString::literal(NOT_FOUND_BODY_TEXT));
        ptr::null_mut()
    }
}

static FORBIDDEN_BODY_TEXT: &str = "{ \"error\": \"forbidden\" }";
static IGNORED_NO_CONTENT_BODY: &str = "this body must not be sent";

unsafe extern "C" fn allow_pre_handler(_: *const HttpRequest, _: *mut HttpResponse) -> i32 {
    0
}

unsafe extern "C" fn deny_pre_handler(_: *const HttpRequest, response: *mut HttpResponse) -> i32 {
    unsafe {
        (*response).status = 403;
        (*response).body = HttpBody::from_text(TyrusString::literal(FORBIDDEN_BODY_TEXT));
        1
    }
}

unsafe extern "C" fn no_content_pre_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> i32 {
    unsafe {
        (*response).status = 204;
        (*response).body = HttpBody::from_text(TyrusString::literal(IGNORED_NO_CONTENT_BODY));
        1
    }
}

unsafe extern "C" fn echo_request_parts_handler(
    request: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        let request = &*request;
        let params = request.params.as_str();
        let query = request.query.as_str();
        let headers = request.headers.as_str();
        let cookies = request.cookies.as_str();
        let body = format!(
            "{{\"params\":{params},\"query\":{query},\"headers\":{headers},\"cookies\":{cookies}}}"
        );
        (*response).status = 200;
        (*response).body = HttpBody::from_text(crate::strings::alloc_string(&body));
        ptr::null_mut()
    }
}

unsafe extern "C" fn delayed_health_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        let promise = tyrus_promise_pending();
        tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        let response_address = response as usize;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let response = response_address as *mut HttpResponse;
            let promise = promise_address as *mut TyrusPromise;
            {
                (*response).status = 200;
                (*response).body = HttpBody::from_text(TyrusString::literal(HEALTH_BODY_TEXT));
                tyrus_promise_fulfill(promise, ptr::null_mut(), None);
                tyrus_promise_free(promise);
            }
        });
        promise
    }
}

fn native_array(values: &[TyrusString]) -> TyrusStringArray {
    TyrusStringArray {
        data: values.as_ptr(),
        length: values.len(),
        capacity: values.len(),
    }
}

#[tokio::test]
async fn dispatches_registered_routes_and_fallbacks() {
    let router = tyrus_http_router_new();
    let health = TyrusString::literal("/health");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                health.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
        assert_eq!(
            tyrus_http_router_query(
                router,
                health.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
        assert_eq!(
            tyrus_http_router_fallback(router, Some(not_found_handler), std::ptr::null(), 0),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        b"{ \"status\": \"ok\" }"
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("QUERY")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        b"{ \"status\": \"ok\" }"
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn waits_for_async_handler_before_finalizing_response() {
    let router = tyrus_http_router_new();
    let path = TyrusString::literal("/async-health");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                path.borrow(),
                Some(delayed_health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };
    let response = app
        .oneshot(
            Request::builder()
                .uri("/async-health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(body.as_ref(), b"{ \"status\": \"ok\" }");
    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn enforces_per_route_body_limit() {
    let router = tyrus_http_router_new();
    let limited = TyrusString::literal("/limited");
    let default = TyrusString::literal("/default");
    unsafe {
        assert_eq!(
            tyrus_http_router_post(
                router,
                limited.borrow(),
                Some(health_handler),
                10,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
        assert_eq!(
            tyrus_http_router_post(
                router,
                default.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/limited")
                .body(Body::from("x".repeat(100)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        serde_json::json!({
            "code": "payload_too_large",
            "message": "Request body is too large",
            "statusCode": 413
        })
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/default")
                .body(Body::from("x".repeat(100)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn exposes_a_streamed_request_body_to_the_handler() {
    let router = tyrus_http_router_new();
    let path = TyrusString::literal("/upload");
    unsafe {
        assert_eq!(
            tyrus_http_router_post(
                router,
                path.borrow(),
                Some(streamed_request_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                4,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/upload")
                .body(Body::from(vec![0u8, 255, 10, 0, 66]))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        &[0u8, 255, 10, 0, 66]
    );
    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn streamed_request_body_fails_when_it_exceeds_the_route_limit() {
    let stream = crate::http::stream_request_body(Body::from(vec![1u8, 2, 3]), 2);
    let stream = unsafe { crate::stream::take_owned_readable_stream(stream) };
    assert_eq!(
        crate::stream::read_owned_byte_stream(&stream)
            .await
            .unwrap_err()
            .code,
        "ERR_STREAM_SOURCE_FAILED"
    );
}

#[tokio::test]
async fn streamed_request_body_applies_backpressure_to_a_fast_client() {
    const CHUNK_BYTES: usize = 32 * 1024;
    const CHUNK_COUNT: usize = 4;

    let polled = Arc::new(AtomicUsize::new(0));
    let source_polled = Arc::clone(&polled);
    let source = futures_util::stream::unfold(0usize, move |index| {
        let polled = Arc::clone(&source_polled);
        async move {
            if index == CHUNK_COUNT {
                return None;
            }
            polled.fetch_add(1, Ordering::SeqCst);
            Some((
                Ok::<_, std::io::Error>(axum::body::Bytes::from(vec![index as u8; CHUNK_BYTES])),
                index + 1,
            ))
        }
    });
    let stream =
        crate::http::stream_request_body(Body::from_stream(source), CHUNK_BYTES * CHUNK_COUNT);
    let stream = unsafe { crate::stream::take_owned_readable_stream(stream) };

    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while polled.load(Ordering::SeqCst) < 3 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the request pump reaches its bounded buffer");
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert_eq!(
        polled.load(Ordering::SeqCst),
        3,
        "the request pump polled beyond the one chunk waiting for capacity"
    );

    for expected in 0..CHUNK_COUNT {
        let chunk = crate::stream::read_owned_byte_stream(&stream)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(chunk, vec![expected as u8; CHUNK_BYTES]);
    }
    assert_eq!(
        crate::stream::read_owned_byte_stream(&stream)
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn streamed_request_body_waits_for_a_slow_client_and_reports_eof() {
    let source = futures_util::stream::unfold(0usize, |index| async move {
        if index == 2 {
            return None;
        }
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        Some((
            Ok::<_, std::io::Error>(axum::body::Bytes::from(vec![index as u8 + 1])),
            index + 1,
        ))
    });
    let stream = crate::http::stream_request_body(Body::from_stream(source), 16);
    let stream = unsafe { crate::stream::take_owned_readable_stream(stream) };

    let mut first = Box::pin(crate::stream::read_owned_byte_stream(&stream));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(5), &mut first)
            .await
            .is_err(),
        "the request stream completed before the client produced a chunk"
    );
    assert_eq!(first.await.unwrap(), Some(vec![1]));
    assert_eq!(
        crate::stream::read_owned_byte_stream(&stream)
            .await
            .unwrap(),
        Some(vec![2])
    );
    assert_eq!(
        crate::stream::read_owned_byte_stream(&stream)
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn streamed_request_body_preserves_a_client_source_failure() {
    let source = futures_util::stream::unfold(0usize, |state| async move {
        match state {
            0 => Some((
                Ok::<_, std::io::Error>(axum::body::Bytes::from_static(b"first")),
                1,
            )),
            1 => {
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                Some((Err(std::io::Error::other("client disconnected")), 2))
            }
            _ => None,
        }
    });
    let stream = crate::http::stream_request_body(Body::from_stream(source), 16);
    let stream = unsafe { crate::stream::take_owned_readable_stream(stream) };

    assert_eq!(
        crate::stream::read_owned_byte_stream(&stream)
            .await
            .unwrap(),
        Some(b"first".to_vec())
    );
    assert_eq!(
        crate::stream::read_owned_byte_stream(&stream)
            .await
            .unwrap_err()
            .code,
        "ERR_STREAM_SOURCE_FAILED"
    );
}

#[tokio::test]
async fn a_handler_that_ignores_a_streamed_request_cancels_the_idle_client_body() {
    struct DropMarker(Arc<AtomicBool>);

    impl Drop for DropMarker {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let dropped = Arc::new(AtomicBool::new(false));
    let source = futures_util::stream::unfold(DropMarker(Arc::clone(&dropped)), |marker| async {
        std::future::pending::<()>().await;
        Some((Ok::<_, std::io::Error>(axum::body::Bytes::new()), marker))
    });
    let router = tyrus_http_router_new();
    let path = TyrusString::literal("/ignore-upload");
    unsafe {
        assert_eq!(
            tyrus_http_router_post(
                router,
                path.borrow(),
                Some(health_handler),
                16,
                -1,
                -1,
                -1,
                -1,
                -1,
                4,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        app.oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/ignore-upload")
                .body(Body::from_stream(source))
                .unwrap(),
        ),
    )
    .await
    .expect("the route responds without waiting for the unused request body")
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !dropped.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the idle client body is dropped after its request reader leaves");
    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn relays_a_streamed_request_body_without_buffering() {
    let router = tyrus_http_router_new();
    let path = TyrusString::literal("/relay");
    unsafe {
        assert_eq!(
            tyrus_http_router_post(
                router,
                path.borrow(),
                Some(streamed_relay_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                4,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/relay")
                .body(Body::from(vec![0u8, 255, 10, 0, 66]))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        &[0u8, 255, 10, 0, 66]
    );
    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn rate_limit_rejects_before_reading_the_request_body() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/limited");
    unsafe {
        assert_eq!(
            tyrus_http_router_post(
                router,
                route.borrow(),
                Some(health_handler),
                10,
                1,
                60_000,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/limited")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/limited")
                .body(Body::from("x".repeat(100)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers().get("retry-after").unwrap(), "60");
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        serde_json::json!({
            "code": "rate_limit_exceeded",
            "message": "Too many requests",
            "statusCode": 429
        })
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn pre_handler_chain_short_circuits() {
    let router = tyrus_http_router_new();
    let denied = TyrusString::literal("/denied");
    let allowed = TyrusString::literal("/allowed");
    let deny_chain: [HttpPreHandler; 2] = [allow_pre_handler, deny_pre_handler];
    let allow_chain: [HttpPreHandler; 1] = [allow_pre_handler];
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                denied.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                deny_chain.as_ptr(),
                deny_chain.len()
            ),
            0
        );
        assert_eq!(
            tyrus_http_router_get(
                router,
                allowed.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                allow_chain.as_ptr(),
                allow_chain.len()
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/denied")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        b"{ \"error\": \"forbidden\" }"
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/allowed")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        b"{ \"status\": \"ok\" }"
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn fallback_pre_handler_short_circuits() {
    let router = tyrus_http_router_new();
    let chain: [HttpPreHandler; 1] = [deny_pre_handler];
    unsafe {
        assert_eq!(
            tyrus_http_router_fallback(router, Some(health_handler), chain.as_ptr(), chain.len()),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };
    let response = app
        .oneshot(
            Request::builder()
                .uri("/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        b"{ \"error\": \"forbidden\" }"
    );
    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn no_content_response_discards_a_native_callback_body() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/no-content");
    let chain: [HttpPreHandler; 1] = [no_content_pre_handler];
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                chain.as_ptr(),
                chain.len()
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };
    let response = app
        .oneshot(
            Request::builder()
                .uri("/no-content")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(response.headers().get("content-type").is_none());
    assert!(
        to_bytes(response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );
    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn exposes_route_request_parts_as_json() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/users/:id");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(echo_request_parts_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/users/42?active=true&page=2")
                .header("authorization", "Bearer token")
                .header("cookie", "session=abc; theme=dark")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let body = std::str::from_utf8(body.as_ref()).unwrap();
    assert!(body.contains(r#""params":{"id":"42"}"#));
    assert!(body.contains(r#""query":{"active":"true","page":"2"}"#));
    assert!(body.contains(r#""authorization":"Bearer token""#));
    assert!(body.contains(r#""cookies":{"session":"abc","theme":"dark"}"#));

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn head_response_preserves_headers_without_a_body() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/health");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::HEAD)
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );
    assert_eq!(
        response.headers().get("content-length").unwrap(),
        HEALTH_BODY_TEXT.len().to_string().as_str()
    );
    assert!(
        to_bytes(response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn if_none_match_returns_304_without_a_body_and_preserves_the_etag() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/users");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(etag_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/users")
                .header("if-none-match", "W/\"users-v1\", \"older\"")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(response.headers().get("etag").unwrap(), "\"users-v1\"");
    assert_eq!(
        response.headers().get("last-modified").unwrap(),
        "Thu, 01 Jan 1970 00:00:00 GMT"
    );
    assert_eq!(
        response.headers().get("cache-control").unwrap(),
        "private, max-age=60"
    );
    assert!(
        to_bytes(response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn if_modified_since_returns_304_and_if_none_match_has_precedence() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/report");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(last_modified_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/report")
                .header("if-modified-since", "Wed, 21 Oct 2015 07:28:00 GMT")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    assert!(
        to_bytes(response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/report")
                .header("if-none-match", "\"does-not-match\"")
                .header("if-modified-since", "Wed, 21 Oct 2099 07:28:00 GMT")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn gzip_is_negotiated_only_when_accepted_and_beneficial() {
    use std::io::Read;

    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/compressed");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(compressible_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/compressed")
                .header("accept-encoding", "gzip;q=0.8")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers().get("content-encoding").unwrap(), "gzip");
    assert!(
        response
            .headers()
            .get("vary")
            .unwrap()
            .to_str()
            .unwrap()
            .split(',')
            .any(|value| value.trim().eq_ignore_ascii_case("accept-encoding"))
    );
    let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
    let mut text = String::new();
    flate2::read::GzDecoder::new(bytes.as_ref())
        .read_to_string(&mut text)
        .unwrap();
    assert_eq!(text, "tyrus compression ".repeat(160));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/compressed")
                .header("accept-encoding", "gzip;q=0, identity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.headers().get("content-encoding").is_none());
    assert!(response.headers().get("vary").is_none());

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn brotli_is_preferred_when_the_client_accepts_it() {
    use std::io::Read;

    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/brotli");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(compressible_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/brotli")
                .header("accept-encoding", "gzip, br")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers().get("content-encoding").unwrap(), "br");
    let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
    let mut text = String::new();
    brotli::Decompressor::new(bytes.as_ref(), 4096)
        .read_to_string(&mut text)
        .unwrap();
    assert_eq!(text, "tyrus compression ".repeat(160));

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn head_rate_limit_error_preserves_headers_without_a_body() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/limited");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(health_handler),
                -1,
                1,
                60_000,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/limited")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::HEAD)
                .uri("/limited")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers().get("retry-after").unwrap(), "60");
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json; charset=utf-8"
    );
    assert_ne!(response.headers().get("content-length").unwrap(), "0");
    assert!(
        to_bytes(response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn sends_reply_cookie_metadata_as_set_cookie_headers() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/cookies");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(cookie_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/cookies")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookies = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(cookies.iter().any(|cookie| cookie == "session=abc"));
    assert!(
        cookies
            .iter()
            .any(|cookie| cookie == "preview=; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT")
    );

    unsafe { tyrus_http_router_free(router) };
}

async fn set_cookie_headers(app: axum::Router, cookie: Option<&str>) -> Vec<String> {
    let mut request = Request::builder().uri("/cookie-list");
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    let response = app
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn sets_a_cookie_list_at_once_and_clears_every_cookie_the_request_sent() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/cookie-list");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(cookie_list_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let expired = |name: &str| format!("{name}=; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT");
    let cookies = set_cookie_headers(app.clone(), Some("a=1; b=2; a=3; bad name=4")).await;
    assert_eq!(
        cookies,
        vec![
            "session=abc; Path=/; HttpOnly".to_string(),
            "theme=dark".to_string(),
            expired("a"),
            expired("b"),
        ]
    );
    let cookies = set_cookie_headers(app, None).await;
    assert_eq!(
        cookies,
        vec![
            "session=abc; Path=/; HttpOnly".to_string(),
            "theme=dark".to_string()
        ]
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn sends_reply_cookie_options_and_rejects_insecure_samesite_none() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/cookie-options");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(cookie_options_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };
    let response = app
        .oneshot(
            Request::builder()
                .uri("/cookie-options")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let cookies = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(cookies.len(), 1);
    assert_eq!(
        cookies[0],
        "session=abc; Path=/; Max-Age=3600; SameSite=Lax; Secure; HttpOnly"
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn sends_validated_reply_headers_without_exposing_response_abi_metadata() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/headers");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(headers_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/headers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("x-request-id").unwrap(), "second");
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/problem+json"
    );
    assert_eq!(response.headers().get_all("link").iter().count(), 2);
    assert_ne!(response.headers().get("content-length").unwrap(), "1");
    assert!(response.headers().get("x-invalid").is_none());
    assert!(response.headers().get("x-transient").is_none());

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::HEAD)
                .uri("/headers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("x-request-id").unwrap(), "second");
    assert!(
        to_bytes(response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn sends_reply_error_as_standard_json() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/expected-error");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(expected_error_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/expected-error")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(
        std::str::from_utf8(body.as_ref()).unwrap(),
        r#"{"statusCode":404,"code":"not_found","message":"User not found"}"#
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn hides_unexpected_handler_errors_behind_internal_server_error() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/unexpected");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(invalid_response_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/unexpected")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        serde_json::json!({
            "code": "internal_server_error",
            "message": "Internal server error",
            "statusCode": 500
        })
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn sends_http_validation_errors_as_standard_json() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/validation-error");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(validation_error_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/validation-error")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(
        std::str::from_utf8(body.as_ref()).unwrap(),
        r#"{"code":"validation","issues":[],"message":"Invalid request body","statusCode":400}"#
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn handles_cors_preflight_requests() {
    let router = tyrus_http_router_new();
    let health = TyrusString::literal("/health");
    let origin = TyrusString::literal("http://localhost:5173");
    let get = TyrusString::literal("GET");
    let post = TyrusString::literal("POST");
    let options = TyrusString::literal("OPTIONS");
    let content_type = TyrusString::literal("Content-Type");
    let authorization = TyrusString::literal("Authorization");
    let origins = [origin.borrow()];
    let methods = [get.borrow(), post.borrow(), options.borrow()];
    let allowed_headers = [content_type.borrow(), authorization.borrow()];
    let cors = CorsOptions {
        origin: native_array(&origins),
        methods: native_array(&methods),
        allowed_headers: native_array(&allowed_headers),
        exposed_headers: native_array(&[]),
        credentials: true,
        max_age: 3600,
    };
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                health.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
        assert_eq!(tyrus_http_router_cors(router, &cors), 0);
    }
    let app = unsafe {
        (*router)
            .router
            .clone()
            .layer((*router).cors.clone().unwrap())
    };
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/health")
                .header("origin", "http://localhost:5173")
                .header("access-control-request-method", "GET")
                .header("access-control-request-headers", "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .unwrap(),
        "http://localhost:5173"
    );
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-credentials")
            .unwrap(),
        "true"
    );
    unsafe { tyrus_http_router_free(router) };
}

#[test]
fn rejects_credentialed_wildcard_cors() {
    let router = tyrus_http_router_new();
    let wildcard = TyrusString::literal("*");
    let wildcard_values = [wildcard.borrow()];
    let cors = CorsOptions {
        origin: native_array(&wildcard_values),
        methods: native_array(&wildcard_values),
        allowed_headers: native_array(&wildcard_values),
        exposed_headers: native_array(&[]),
        credentials: true,
        max_age: 0,
    };
    unsafe {
        assert_eq!(tyrus_http_router_cors(router, &cors), -1);
        tyrus_http_router_free(router);
    }
}

#[test]
fn rejects_routes_without_an_absolute_path() {
    let router = tyrus_http_router_new();
    let invalid = TyrusString::literal("health");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                invalid.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            -1
        );
        tyrus_http_router_free(router);
    }
}

unsafe extern "C" fn slow_labelled_handler(
    _: *const HttpRequest,
    response: *mut HttpResponse,
) -> *mut TyrusPromise {
    unsafe {
        SLOW_RESPONSE_ADDRESS.store(response as usize, Ordering::SeqCst);
        assert_eq!(
            tyrus_http_reply_header(
                response,
                TyrusString::literal("x-cancelled-request-probe").borrow(),
                TyrusString::literal("yes").borrow(),
            ),
            0
        );
        let promise = tyrus_promise_pending();
        tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        let response_address = response as usize;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(60));
            let response = response_address as *mut HttpResponse;
            let promise = promise_address as *mut TyrusPromise;
            {
                (*response).status = 200;
                (*response).body =
                    HttpBody::from_text(crate::strings::alloc_string(HEALTH_BODY_TEXT));
                tyrus_promise_fulfill(promise, ptr::null_mut(), None);
                tyrus_promise_free(promise);
            }
        });
        promise
    }
}

#[tokio::test]
async fn a_cancelled_request_outlives_its_handler_and_leaves_no_metadata() {
    SLOW_RESPONSE_ADDRESS.store(0, Ordering::SeqCst);
    let router = tyrus_http_router_new();
    let path = TyrusString::literal("/slow");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                path.borrow(),
                Some(slow_labelled_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };
    let call = app.oneshot(
        Request::builder()
            .uri("/slow")
            .header("cookie", "cancelled-probe=1")
            .body(Body::empty())
            .unwrap(),
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), call)
            .await
            .is_err(),
        "the handler was expected to still be running"
    );
    let response_address = SLOW_RESPONSE_ADDRESS.load(Ordering::SeqCst);
    assert_ne!(response_address, 0);
    assert!(crate::http::reply_headers_contains(
        response_address as *const HttpResponse,
        "x-cancelled-request-probe"
    ));
    assert_eq!(
        crate::http::request_cookie_names_holding("cancelled-probe"),
        1
    );

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(
        !crate::http::reply_headers_contains(
            response_address as *const HttpResponse,
            "x-cancelled-request-probe"
        ),
        "an abandoned request left reply metadata behind"
    );
    assert_eq!(
        crate::http::request_cookie_names_holding("cancelled-probe"),
        0,
        "an abandoned request left its cookie names behind"
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn a_rejected_request_reports_the_field_and_rule_that_failed() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/users");
    unsafe {
        assert_eq!(
            tyrus_http_router_post(
                router,
                route.borrow(),
                Some(recorded_issue_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(body.as_ref()).unwrap();
    assert_eq!(
        body["issues"],
        serde_json::json!([{
            "field": "email",
            "validator": "@email()",
            "message": "value must be a valid e-mail address",
        }])
    );
    assert_eq!(body["statusCode"], 400);

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn a_failed_response_tells_the_client_nothing_about_the_server() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/users");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(failed_response_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let raw = String::from_utf8(body.to_vec()).unwrap();
    let body: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(body["statusCode"], 500);
    assert_eq!(body["code"], "internal_server_error");
    assert!(
        body.get("issues").is_none(),
        "the server's own field names reached the client: {raw}"
    );
    assert!(
        !raw.contains("internalToken") && !raw.contains("minLength"),
        "the server's schema leaked into the response: {raw}"
    );

    assert!(
        crate::validation::take_validation_issues().is_empty(),
        "a failed response left its issues behind for the next request"
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn a_forged_forwarding_header_does_not_escape_the_rate_limit() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/who");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(health_handler),
                -1,
                2,
                30_000,
                -1,
                -1,
                -1,
                0,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let mut statuses = Vec::new();
    for index in 0..4 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/who")
                    .header("x-forwarded-for", format!("203.0.113.{index}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        statuses.push(response.status());
    }
    assert_eq!(
        statuses,
        vec![
            StatusCode::OK,
            StatusCode::OK,
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::TOO_MANY_REQUESTS,
        ],
        "a forged header created a bucket per request"
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn repeated_identity_limit_exceedances_become_a_temporary_forbidden_response() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/limited");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(health_handler),
                -1,
                1,
                60_000,
                -1,
                2,
                5_000,
                0,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let mut responses = Vec::new();
    for _ in 0..4 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/limited")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(body.as_ref()).unwrap();
        responses.push((status, body));
    }
    assert_eq!(responses[0].0, StatusCode::OK);
    assert_eq!(responses[1].0, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(responses[1].1["code"], "rate_limit_exceeded");
    assert_eq!(responses[2].0, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(responses[2].1["code"], "rate_limit_exceeded");
    assert_eq!(responses[3].0, StatusCode::FORBIDDEN);
    assert_eq!(responses[3].1["code"], "rate_limit_banned");

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn a_trusted_proxy_gives_each_forwarded_client_its_own_budget() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/who");
    let proxies = [TyrusString::literal("0.0.0.0/32")];
    let array = native_array(&proxies);
    unsafe {
        assert_eq!(
            tyrus_http_router_trusted_proxies(router, &array, ptr::null()),
            0
        );
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(health_handler),
                -1,
                1,
                30_000,
                -1,
                -1,
                -1,
                0,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    for client in ["203.0.113.1", "203.0.113.2", "203.0.113.3"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/who")
                    .header("x-forwarded-for", client)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "client {client}");
    }

    let response = app
        .oneshot(
            Request::builder()
                .uri("/who")
                .header("x-forwarded-for", "203.0.113.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn a_trusted_proxy_can_use_a_custom_identity_header() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/who");
    let proxies = [TyrusString::literal("0.0.0.0/32")];
    let array = native_array(&proxies);
    let identity_header = TyrusString::literal("x-client-ip");
    unsafe {
        assert_eq!(
            tyrus_http_router_trusted_proxies(router, &array, &identity_header),
            0
        );
        assert_eq!(
            tyrus_http_router_get(
                router,
                route.borrow(),
                Some(health_handler),
                -1,
                1,
                30_000,
                -1,
                -1,
                -1,
                0,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    for client in ["203.0.113.1", "203.0.113.2"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/who")
                    .header("x-client-ip", client)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "client {client}");
    }

    unsafe { tyrus_http_router_free(router) };
}

#[test]
fn a_malformed_trusted_proxy_rejects_the_whole_list() {
    let router = tyrus_http_router_new();
    let proxies = [
        TyrusString::literal("10.0.0.0/8"),
        TyrusString::literal("10.0.0.0/33"),
    ];
    let array = native_array(&proxies);
    unsafe {
        assert_eq!(
            tyrus_http_router_trusted_proxies(router, &array, ptr::null()),
            -1
        );
        assert!(
            (*router).config.read().unwrap().trusted_proxies.is_empty(),
            "a rejected list must not be partially applied"
        );
        tyrus_http_router_free(router);
    }
}

fn server_options(request_timeout_millis: i64, max_concurrent_requests: i64) -> HttpServerOptions {
    HttpServerOptions {
        host: TyrusString::null(),
        request_timeout_millis,
        max_concurrent_requests,
        shutdown_timeout_millis: -1,
        header_timeout_millis: -1,
        max_header_bytes: -1,
        max_connections: -1,
        keep_alive: -1,
        logger: -1,
        tls_cert: TyrusString::null(),
        tls_key: TyrusString::null(),
    }
}

#[test]
fn server_logger_is_disabled_by_default_and_opted_in_explicitly() {
    let router = tyrus_http_router_new();
    let default = server_options(-1, -1);
    unsafe {
        assert_eq!(crate::http::apply_server_options(&*router, &default), 0);
        assert!(!(*router).config.read().unwrap().logger);
        let mut enabled = server_options(-1, -1);
        enabled.logger = 1;
        assert_eq!(crate::http::apply_server_options(&*router, &enabled), 0);
        assert!((*router).config.read().unwrap().logger);
        tyrus_http_router_free(router);
    }
}

#[tokio::test]
async fn a_request_past_the_clock_is_answered_and_the_server_keeps_serving() {
    let router = tyrus_http_router_new();
    let slow = TyrusString::literal("/slow");
    let fast = TyrusString::literal("/fast");
    let options = server_options(2_000, -1);
    unsafe {
        assert_eq!(crate::http::apply_server_options(&*router, &options), 0);
        assert_eq!(
            tyrus_http_router_get(
                router,
                slow.borrow(),
                Some(delayed_health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                ptr::null(),
                0,
            ),
            0
        );
        assert_eq!(
            tyrus_http_router_get(
                router,
                fast.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/slow").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let tight = server_options(1, -1);
    unsafe { assert_eq!(crate::http::apply_server_options(&*router, &tight), 0) };
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/slow").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let response = app
        .oneshot(Request::builder().uri("/fast").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn a_server_at_its_concurrency_limit_answers_instead_of_queueing_forever() {
    let router = tyrus_http_router_new();
    let slow = TyrusString::literal("/slow");
    let options = server_options(5, 1);
    unsafe {
        assert_eq!(crate::http::apply_server_options(&*router, &options), 0);
        assert_eq!(
            tyrus_http_router_get(
                router,
                slow.borrow(),
                Some(delayed_health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let first = tokio::spawn({
        let app = app.clone();
        async move {
            app.oneshot(Request::builder().uri("/slow").body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status()
        }
    });
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let second = app
        .oneshot(Request::builder().uri("/slow").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::SERVICE_UNAVAILABLE);
    let _ = first.await;

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn a_route_registered_with_unknown_body_flags_is_refused() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/users");
    unsafe {
        assert_eq!(
            tyrus_http_router_post(
                router,
                route.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                ptr::null(),
                0,
            ),
            -1
        );
        assert_eq!(
            tyrus_http_router_post(
                router,
                route.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                8,
                ptr::null(),
                0,
            ),
            -1
        );
        tyrus_http_router_free(router);
    }
}

#[tokio::test]
async fn a_json_route_refuses_content_in_another_media_type() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/users");
    unsafe {
        assert_eq!(
            tyrus_http_router_post(
                router,
                route.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                1,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let accepted = ["application/json", "application/json; charset=utf-8"];
    for content_type in accepted {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/users")
                    .header("content-type", content_type)
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{content_type}");
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/users")
                .header("content-type", "application/vnd.api+json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    for content_type in ["text/plain", "application/x-www-form-urlencoded"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/users")
                    .header("content-type", content_type)
                    .body(Body::from("name=Ada"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "{content_type}"
        );
        assert_eq!(
            response.headers().get("accept").unwrap(),
            "application/json"
        );
    }

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn a_route_without_a_body_schema_ignores_the_media_type() {
    let router = tyrus_http_router_new();
    let route = TyrusString::literal("/ping");
    unsafe {
        assert_eq!(
            tyrus_http_router_post(
                router,
                route.borrow(),
                Some(health_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                ptr::null(),
                0,
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/ping")
                .header("content-type", "text/plain")
                .body(Body::from("anything"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    unsafe { tyrus_http_router_free(router) };
}

/// A binary body reaches the socket byte for byte, defaults to
/// `application/octet-stream`, and is released exactly once.
///
/// The last part is why this test lives here rather than only in the
/// end-to-end suite: the copy the response takes is freed by
/// `finalize_response`, and a wrong answer about who owns that buffer is a
/// double free or a leak that only a request driven through the real pipeline
/// under a sanitizer can show.
#[tokio::test]
async fn serves_a_binary_body_unchanged() {
    let router = tyrus_http_router_new();
    let icon = TyrusString::literal("/icon");
    let empty = TyrusString::literal("/empty");
    unsafe {
        assert_eq!(
            tyrus_http_router_get(
                router,
                icon.borrow(),
                Some(binary_body_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
        assert_eq!(
            tyrus_http_router_get(
                router,
                empty.borrow(),
                Some(empty_binary_body_handler),
                -1,
                -1,
                -1,
                -1,
                -1,
                -1,
                0,
                std::ptr::null(),
                0
            ),
            0
        );
    }
    let app = unsafe { (*router).router.clone() };

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/icon").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("application/octet-stream")
    );
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        &[0u8, 255, 10, 0, 66]
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/empty")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        to_bytes(response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );

    unsafe { tyrus_http_router_free(router) };
}

#[tokio::test]
async fn serves_a_readable_stream_body_incrementally() {
    let (stream, producer) = crate::stream::byte_readable_stream(64);
    producer.write(vec![0, 255, 10]).await.unwrap();
    producer.write(vec![0, 66]).await.unwrap();
    producer.close();

    let mut source = HttpResponse::empty();
    source.status = 200;
    source.body = HttpBody::from_stream(stream);
    let response =
        crate::http::finalize_response(source, ptr::null(), false, false, None, None, None);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("application/octet-stream")
    );
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        &[0u8, 255, 10, 0, 66]
    );
}

#[tokio::test]
async fn a_streamed_response_waits_for_a_slow_server_producer() {
    use futures_util::StreamExt;

    let (stream, producer) = crate::stream::byte_readable_stream(64);
    let mut source = HttpResponse::empty();
    source.status = 200;
    source.body = HttpBody::from_stream(stream);
    let response =
        crate::http::finalize_response(source, ptr::null(), false, false, None, None, None);
    let mut body = response.into_body().into_data_stream();

    let pending = tokio::time::timeout(std::time::Duration::from_millis(5), body.next()).await;
    assert!(
        pending.is_err(),
        "the response completed before its producer wrote"
    );

    producer.write(b"delayed".to_vec()).await.unwrap();
    producer.close();
    assert_eq!(body.next().await.unwrap().unwrap().as_ref(), b"delayed");
    assert!(body.next().await.is_none());
}

#[tokio::test]
async fn a_streamed_response_preserves_a_source_failure_until_the_http_body() {
    use futures_util::StreamExt;

    let (stream, producer) = crate::stream::byte_readable_stream(64);
    let mut source = HttpResponse::empty();
    source.status = 200;
    source.body = HttpBody::from_stream(stream);
    let response =
        crate::http::finalize_response(source, ptr::null(), false, false, None, None, None);
    let mut body = response.into_body().into_data_stream();

    producer.write(b"before-failure".to_vec()).await.unwrap();
    assert_eq!(
        body.next().await.unwrap().unwrap().as_ref(),
        b"before-failure"
    );
    producer.fail();
    let error = body
        .next()
        .await
        .expect("the HTTP body reports the stream failure")
        .expect_err("the failed stream must not become a clean EOF");
    assert_eq!(error.to_string(), "stream source failed");
}

#[tokio::test]
async fn a_client_that_stops_reading_a_streamed_response_stops_its_producer() {
    use futures_util::StreamExt;

    let (stream, producer) = crate::stream::byte_readable_stream(4);
    let mut source = HttpResponse::empty();
    source.status = 200;
    source.body = HttpBody::from_stream(stream);
    let response =
        crate::http::finalize_response(source, ptr::null(), false, false, None, None, None);
    let mut body = response.into_body().into_data_stream();

    producer.write(vec![1, 2, 3, 4]).await.unwrap();
    let mut waiting = tokio::spawn(async move {
        producer.write(vec![5, 6, 7, 8]).await?;
        producer.close();
        Ok::<(), crate::stream::StreamError>(())
    });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut waiting)
            .await
            .is_err(),
        "the producer ran ahead of a client that had read nothing"
    );

    let first = body.next().await.unwrap().unwrap();
    assert_eq!(first.as_ref(), &[1, 2, 3, 4]);
    tokio::time::timeout(std::time::Duration::from_secs(5), waiting)
        .await
        .expect("the producer continues once the client reads")
        .unwrap()
        .unwrap();
    let second = body.next().await.unwrap().unwrap();
    assert_eq!(second.as_ref(), &[5, 6, 7, 8]);
}

#[tokio::test]
async fn dropping_a_readable_http_response_cancels_its_retained_source() {
    let (stream, producer) = crate::stream::byte_readable_stream(64);
    let mut source = HttpResponse::empty();
    source.status = 200;
    source.body =
        HttpBody::from_stream(unsafe { crate::stream::tyrus_readable_stream_retain(stream) });
    let response =
        crate::http::finalize_response(source, ptr::null(), false, false, None, None, None);

    drop(response);
    assert_eq!(
        producer.write(vec![7]).await.unwrap_err().code,
        "ERR_STREAM_CLOSED"
    );
    unsafe { crate::stream::tyrus_readable_stream_free(stream) };
}

#[tokio::test]
async fn gzip_compresses_a_readable_stream_without_collecting_its_body() {
    use std::io::Read;

    let (stream, producer) = crate::stream::byte_readable_stream(4096);
    producer.write(vec![b'x'; 2048]).await.unwrap();
    producer.write(vec![b'y'; 2048]).await.unwrap();
    producer.close();

    let mut source = HttpResponse::empty();
    source.status = 200;
    unsafe {
        assert_eq!(
            tyrus_http_reply_type(&mut source, TyrusString::literal("text/plain")),
            0
        );
    }
    source.body = HttpBody::from_stream(stream);
    let source_key: *const HttpResponse = &source;
    let response =
        crate::http::finalize_response(source, source_key, false, false, None, None, Some("gzip"));
    assert_eq!(response.headers().get("content-encoding").unwrap(), "gzip");
    assert!(response.headers().get("content-length").is_none());
    let compressed = to_bytes(response.into_body(), 8192).await.unwrap();
    let mut text = Vec::new();
    flate2::read::GzDecoder::new(compressed.as_ref())
        .read_to_end(&mut text)
        .unwrap();
    assert_eq!(text, [vec![b'x'; 2048], vec![b'y'; 2048]].concat());
}

#[tokio::test]
async fn dropping_a_compressed_readable_response_cancels_its_idle_source() {
    let (stream, producer) = crate::stream::byte_readable_stream(64);
    let mut source = HttpResponse::empty();
    source.status = 200;
    unsafe {
        assert_eq!(
            tyrus_http_reply_type(&mut source, TyrusString::literal("text/plain")),
            0
        );
    }
    source.body =
        HttpBody::from_stream(unsafe { crate::stream::tyrus_readable_stream_retain(stream) });
    let source_key: *const HttpResponse = &source;
    let response =
        crate::http::finalize_response(source, source_key, false, false, None, None, Some("gzip"));
    assert_eq!(response.headers().get("content-encoding").unwrap(), "gzip");

    drop(response);
    let code = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Err(error) = producer.write(vec![7]).await {
                return error.code;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the route's producer learns that the compressed response is gone");
    assert_eq!(code, "ERR_STREAM_CLOSED");
    unsafe { crate::stream::tyrus_readable_stream_free(stream) };
}

#[tokio::test]
async fn gzip_carries_a_stream_chunk_larger_than_the_transform_capacity() {
    use std::io::Read;

    let mut state = 0x2545_f491_u32;
    let original: Vec<u8> = (0..600 * 1024)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as u8
        })
        .collect();
    let (stream, producer) = crate::stream::byte_readable_stream(1024 * 1024);
    producer.write(original.clone()).await.unwrap();
    producer.close();

    let mut source = HttpResponse::empty();
    source.status = 200;
    unsafe {
        assert_eq!(
            tyrus_http_reply_type(&mut source, TyrusString::literal("text/plain")),
            0
        );
    }
    source.body = HttpBody::from_stream(stream);
    let source_key: *const HttpResponse = &source;
    let response =
        crate::http::finalize_response(source, source_key, false, false, None, None, Some("gzip"));
    let compressed = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let mut text = Vec::new();
    flate2::read::GzDecoder::new(compressed.as_ref())
        .read_to_end(&mut text)
        .unwrap();
    assert_eq!(text, original);
}

#[tokio::test]
async fn brotli_compresses_a_readable_stream_with_one_continuous_encoder() {
    use std::io::Read;

    let (stream, producer) = crate::stream::byte_readable_stream(4096);
    producer.write(vec![b'x'; 2048]).await.unwrap();
    producer.write(vec![b'y'; 2048]).await.unwrap();
    producer.close();

    let mut source = HttpResponse::empty();
    source.status = 200;
    unsafe {
        assert_eq!(
            tyrus_http_reply_type(&mut source, TyrusString::literal("text/plain")),
            0
        );
    }
    source.body = HttpBody::from_stream(stream);
    let source_key: *const HttpResponse = &source;
    let response = crate::http::finalize_response(
        source,
        source_key,
        false,
        false,
        None,
        None,
        Some("br, gzip"),
    );
    assert_eq!(response.headers().get("content-encoding").unwrap(), "br");
    assert!(response.headers().get("content-length").is_none());
    let compressed = to_bytes(response.into_body(), 8192).await.unwrap();
    let mut text = Vec::new();
    brotli::Decompressor::new(compressed.as_ref(), 4096)
        .read_to_end(&mut text)
        .unwrap();
    assert_eq!(text, [vec![b'x'; 2048], vec![b'y'; 2048]].concat());
}

#[tokio::test]
async fn gzip_compresses_a_writable_reply_stream_with_one_continuous_encoder() {
    use std::io::Read;

    let mut source = HttpResponse::empty();
    source.status = 200;
    unsafe {
        assert_eq!(
            tyrus_http_reply_type(&mut source, TyrusString::literal("text/plain")),
            0
        );
    }
    let source_key: *const HttpResponse = &source;
    let mut writer = ptr::null();
    unsafe {
        assert_eq!(
            tyrus_http_reply_writable_stream(&mut source, &mut writer),
            0
        );
    }
    for bytes in [vec![b'x'; 2048], vec![b'y'; 2048]] {
        let data = unsafe { libc::malloc(bytes.len()) }.cast::<u8>();
        assert!(!data.is_null());
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len()) };
        let payload = Box::into_raw(Box::new(TyrusArray {
            data,
            length: bytes.len(),
            capacity: bytes.len(),
        }))
        .cast();
        let write = unsafe {
            crate::stream::tyrus_writable_stream_write(
                writer,
                payload,
                bytes.len(),
                Some(crate::async_payload::tyrus_drop_async_copy_array),
            )
        };
        unsafe { crate::wait_until_settled(write) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(write) },
            crate::TYRUS_PROMISE_FULFILLED
        );
        unsafe { crate::tyrus_promise_free(write) };
    }
    let close = unsafe { crate::stream::tyrus_writable_stream_close(writer) };
    unsafe { crate::tyrus_promise_free(close) };

    let response =
        crate::http::finalize_response(source, source_key, false, false, None, None, Some("gzip"));
    assert_eq!(response.headers().get("content-encoding").unwrap(), "gzip");
    assert!(response.headers().get("content-length").is_none());
    let compressed = to_bytes(response.into_body(), 8192).await.unwrap();
    let mut text = Vec::new();
    flate2::read::GzDecoder::new(compressed.as_ref())
        .read_to_end(&mut text)
        .unwrap();
    assert_eq!(text, [vec![b'x'; 2048], vec![b'y'; 2048]].concat());
    unsafe { crate::stream::tyrus_writable_stream_free(writer) };
}

#[test]
fn observed_responses_replace_application_request_ids() {
    let request_id = crate::http::next_request_id();
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("x-request-id", "application-value")
        .body(Body::empty())
        .unwrap();
    let response = crate::http::observe_response(
        response,
        &request_id,
        &Method::GET,
        "/health",
        std::time::Instant::now(),
        false,
        Arc::from([]),
    );
    let emitted = response.headers().get("x-request-id").unwrap();
    assert_eq!(emitted, request_id.0.as_str());
    assert!(emitted.as_bytes().starts_with(b"req-"));
}

#[tokio::test]
async fn observed_responses_call_hooks_after_the_body_finishes() {
    RESPONSE_HOOK_CALLS.store(0, Ordering::SeqCst);
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("ok"))
        .unwrap();
    let response = crate::http::observe_response(
        response,
        &crate::http::next_request_id(),
        &Method::GET,
        "/health",
        std::time::Instant::now(),
        false,
        Arc::from(vec![record_response as HttpResponseHook]),
    );
    assert_eq!(RESPONSE_HOOK_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX).await.unwrap(),
        "ok"
    );
    assert_eq!(RESPONSE_HOOK_CALLS.load(Ordering::SeqCst), 1);
}
