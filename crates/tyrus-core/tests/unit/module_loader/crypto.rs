use super::*;

#[test]
fn exposes_only_the_final_crypto_surface() {
    for export in [
        "CryptoError",
        "timingSafeEqual",
        "digest",
        "hmac",
        "cipher",
        "rand",
        "argon2",
        "jwt",
    ] {
        assert!(
            native_export_info("tyrus:crypto", export).is_some(),
            "missing crypto export `{export}`"
        );
    }
    assert!(native_export_info("tyrus:crypto", "sha256").is_none());
    let (_, cipher_fields, _) = native_export_info("tyrus:crypto", "cipher").unwrap();
    let cipher_fields = cipher_fields.unwrap();
    assert!(cipher_fields.iter().any(|field| field.name == "aes"));
    let aes = crate::crypto::cipher_aes_namespace_fields();
    for method in [
        "encrypt",
        "encryptWithAad",
        "decryptText",
        "decryptBytes",
        "decryptTextWithAad",
        "decryptBytesWithAad",
    ] {
        assert!(
            aes.iter().any(|field| field.name == method),
            "missing AES method `{method}`"
        );
    }
    let jwt = crate::crypto::jwt_namespace_fields();
    assert!(jwt.iter().any(|field| field.name == "decodeUnverified"));
    assert!(jwt.iter().any(|field| field.name == "fetchJwks"));
    assert!(jwt.iter().any(|field| field.name == "verifyWithJwks"));
    assert!(!jwt.iter().any(|field| field.name == "decode"));
    for method in ["sign", "verify", "verifyWithJwks"] {
        let field = jwt.iter().find(|field| field.name == method).unwrap();
        let Type::GenericFunction { params, .. } = &field.ty else {
            panic!("jwt.{method} must be a generic function");
        };
        assert_eq!(params.len(), 3);
        assert!(matches!(params[2], Type::Optional(_)));
    }
    let fetch_jwks = jwt.iter().find(|field| field.name == "fetchJwks").unwrap();
    let Type::Function { params, ret } = &fetch_jwks.ty else {
        panic!("jwt.fetchJwks must be a function");
    };
    assert_eq!(params.len(), 2);
    assert!(matches!(params[1], Type::Optional(_)));
    assert_eq!(
        **ret,
        Type::Apply {
            name: "Promise".to_string(),
            args: vec![Type::Result {
                success: Box::new(Type::Str),
                error: Box::new(Type::Named("CryptoError".to_string())),
            }],
        }
    );
    assert!(!crate::crypto::crypto_method_throws(
        "JwtNamespace",
        "fetchJwks"
    ));
    let argon2 = crate::crypto::argon2_namespace_fields();
    let hash = argon2.iter().find(|field| field.name == "hash").unwrap();
    let Type::Function { params, .. } = &hash.ty else {
        panic!("argon2.hash must be a function");
    };
    assert_eq!(
        &params[0],
        &Type::Union(vec![Type::Str, Type::Array(Box::new(Type::U8))])
    );
    assert_eq!(params.len(), 2);
    assert!(matches!(params[1], Type::Optional(_)));
    for removed in ["hashWithPepper", "verifyWithPepper"] {
        assert!(!argon2.iter().any(|field| field.name == removed));
    }
    let needs_rehash = argon2
        .iter()
        .find(|field| field.name == "needsRehash")
        .unwrap();
    let Type::Function { params, ret } = &needs_rehash.ty else {
        panic!("argon2.needsRehash must be a function");
    };
    assert_eq!(params, &[Type::Str]);
    assert_eq!(
        ret.as_ref(),
        &Type::Result {
            success: Box::new(Type::Bool),
            error: Box::new(Type::Named("CryptoError".to_string())),
        }
    );
    let sign = jwt.iter().find(|field| field.name == "sign").unwrap();
    let Type::GenericFunction {
        type_params,
        params,
        ret,
    } = &sign.ty
    else {
        panic!("jwt.sign must be generic over its claims object");
    };
    assert_eq!(type_params.len(), 1);
    assert_eq!(params[0], Type::Generic("T".to_string()));
    assert_eq!(
        ret.as_ref(),
        &Type::Result {
            success: Box::new(Type::Str),
            error: Box::new(Type::Named("CryptoError".to_string())),
        }
    );
    let verify = jwt.iter().find(|field| field.name == "verify").unwrap();
    assert!(matches!(
        &verify.ty,
        Type::GenericFunction { ret, .. }
            if ret.as_ref() == &Type::Result {
                success: Box::new(Type::Generic("T".to_string())),
                error: Box::new(Type::Named("CryptoError".to_string())),
            }
    ));
}

#[test]
fn resolves_crypto_aliases_and_requires_catchable_failures_to_be_handled() {
    let root =
        std::env::temp_dir().join(format!("tyrus-crypto-module-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { CryptoError, jwt as tokens } from "tyrus:crypto";
            interface Claims { sub: string; }
            const key: string = "0123456789abcdef0123456789abcdef";
            const claims: Claims = { sub: "user-42" };
            const token: string = tokens.sign(claims, key);
        "#,
    )
    .unwrap();
    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(error.contains("E0308"), "unexpected diagnostics: {error}");
    fs::write(
        root.join("main.tyr"),
        r#"
            import { sha256 } from "tyrus:crypto";
            console.log(sha256);
        "#,
    )
    .unwrap();
    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(
        error.contains("sha256") && error.contains("export"),
        "unexpected diagnostics: {error}"
    );
    fs::write(
        root.join("main.tyr"),
        r#"
            import { CryptoError, jwt as tokens } from "tyrus:crypto";
            interface Claims { sub: string; }
            const key: string = "0123456789abcdef0123456789abcdef";
            const claims: Claims = { sub: "user-42" };
            const token: Result<string, CryptoError> = tokens.sign(claims, key);
            match (token) {
                Ok(value) => { console.log(value); }
                Err(error) => { console.error(error.code); }
            }
        "#,
    )
    .unwrap();
    load_program(&root.join("main.tyr"), None).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn jwt_requires_typed_json_safe_claim_objects() {
    let root = std::env::temp_dir().join(format!("tyrus-jwt-claims-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { jwt } from "tyrus:crypto";
            const key: string = "0123456789abcdef0123456789abcdef";
            try {
                const token: string = jwt.sign("{}", key);
                console.log(token);
            } catch (error: Error) {
                console.error(error.message);
            }
        "#,
    )
    .unwrap();
    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(error.contains("E3279"), "unexpected diagnostics: {error}");
    fs::write(
        root.join("main.tyr"),
        r#"
            import { jwt } from "tyrus:crypto";
            interface Claims { tags: string[]; }
            const key: string = "0123456789abcdef0123456789abcdef";
            const claims: Claims = { tags: ["admin"] };
            try {
                const token: string = jwt.sign(claims, key);
                console.log(token);
            } catch (error: Error) {
                console.error(error.message);
            }
        "#,
    )
    .unwrap();
    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(error.contains("E3280"), "unexpected diagnostics: {error}");
    fs::remove_dir_all(root).unwrap();
}
