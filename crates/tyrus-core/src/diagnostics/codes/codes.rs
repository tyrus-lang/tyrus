#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCategory {
    Parse,
    Name,
    Type,
    Ownership,
    ControlFlow,
    Module,
    Pattern,
    Enum,
    Validation,
    RuntimeSafety,
}

impl DiagnosticCategory {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Parse => "Parse",
            Self::Name => "Name resolution",
            Self::Type => "Type checking",
            Self::Ownership => "Ownership",
            Self::ControlFlow => "Control flow",
            Self::Module => "Modules",
            Self::Pattern => "Patterns",
            Self::Enum => "Enums",
            Self::Validation => "Validation",
            Self::RuntimeSafety => "Runtime safety",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticCode {
    pub code: &'static str,
    pub title: &'static str,
    pub category: DiagnosticCategory,
    pub documentation_anchor: &'static str,
}

macro_rules! diagnostic {
    ($code:literal, $title:literal, $category:ident) => {
        DiagnosticCode {
            code: $code,
            title: $title,
            category: DiagnosticCategory::$category,
            documentation_anchor: "error-codes",
        }
    };
}

pub static CODES: &[DiagnosticCode] = &[
    diagnostic!("E0001", "Unexpected token", Parse),
    diagnostic!("E0002", "Invalid syntax", Parse),
    diagnostic!("E0003", "Reserved keyword", Parse),
    diagnostic!("E0004", "Non-exhaustive match", Pattern),
    diagnostic!("E0034", "Multiple applicable items", Name),
    diagnostic!("E0057", "Wrong callback parameter count", Type),
    diagnostic!("E0061", "Wrong argument count", Type),
    diagnostic!("E0063", "Missing object field", Type),
    diagnostic!("E0069", "Invalid return", ControlFlow),
    diagnostic!("E0107", "Wrong generic argument count", Type),
    diagnostic!("E0268", "Invalid continue", ControlFlow),
    diagnostic!("E0269", "Invalid break", ControlFlow),
    diagnostic!(
        "E0277",
        "Trait or capability requirement not satisfied",
        Type
    ),
    diagnostic!("E0282", "Type annotations needed", Type),
    diagnostic!("E0308", "Mismatched types", Type),
    diagnostic!("E0309", "Invalid regular expression flag", Validation),
    diagnostic!(
        "E0310",
        "Object literal needs an interface annotation",
        Type
    ),
    diagnostic!("E0315", "Invalid assignment target", Type),
    diagnostic!("E0316", "Invalid destructuring assignment", Type),
    diagnostic!("E0368", "Invalid compound assignment", Type),
    diagnostic!("E0369", "Unsupported binary operation", Type),
    diagnostic!(
        "E0381",
        "Use of possibly uninitialized variable",
        ControlFlow
    ),
    diagnostic!("E0382", "Use of moved value", Ownership),
    diagnostic!("E0384", "Assignment to immutable binding", Ownership),
    diagnostic!("E0412", "Unknown type", Name),
    diagnostic!("E0425", "Unknown name", Name),
    diagnostic!("E0428", "Duplicate definition", Name),
    diagnostic!("E0432", "Unresolved import", Module),
    diagnostic!("E0433", "Unresolved module or type", Module),
    diagnostic!("E0434", "Test framework outside a test file", Module),
    diagnostic!(
        "E0435",
        "Invalid value in a constant expression",
        Validation
    ),
    diagnostic!(
        "E0436",
        "Functional array operation has no return value",
        Type
    ),
    diagnostic!("E0500", "Assignment to immutable captured value", Ownership),
    diagnostic!("E0502", "Conflicting borrow", Ownership),
    diagnostic!("E0505", "Move while borrowed", Ownership),
    diagnostic!("E0506", "Assignment while borrowed", Ownership),
    diagnostic!("E0529", "Invalid pattern", Pattern),
    diagnostic!("E0571", "Invalid break value", ControlFlow),
    diagnostic!("E0572", "Invalid continue target", ControlFlow),
    diagnostic!("E0580", "Invalid function declaration", Type),
    diagnostic!("E0583", "Module file not found", Module),
    diagnostic!("E0599", "Unknown method", Name),
    diagnostic!("E0601", "Missing entry point", Module),
    diagnostic!("E0605", "Invalid type cast", Type),
    diagnostic!("E0606", "Index out of bounds", RuntimeSafety),
    diagnostic!("E0608", "Cannot index this value", Type),
    diagnostic!("E0609", "Unknown field", Name),
    diagnostic!("E0618", "Called non-function value", Type),
    diagnostic!("E0658", "Unsupported language feature", Type),
    diagnostic!("E0701", "Invalid enum representation", Enum),
    diagnostic!("E0702", "Mixed enum member types", Enum),
    diagnostic!("E0703", "Mixed automatic and explicit enum values", Enum),
    diagnostic!("E0705", "Enum member value out of range", Enum),
    diagnostic!("E0706", "Value does not fit its type", Type),
    diagnostic!("E0707", "Invalid numeric limit", Type),
    diagnostic!("E0708", "Invalid character literal", Type),
    diagnostic!("E0716", "Forced optional unwrap", Type),
    diagnostic!("E0717", "Invalid template format specifier", Type),
    diagnostic!("E0747", "Invalid compile-time array argument", Type),
    diagnostic!("E0790", "Invalid array match pattern", Pattern),
    diagnostic!("E0791", "Multiple array rest patterns", Pattern),
    diagnostic!("E0792", "Array rest pattern is not last", Pattern),
    diagnostic!("E0793", "Invalid array rest binding", Pattern),
    diagnostic!("E0794", "Value not representable as unknown", Type),
    diagnostic!("E0795", "Invalid typeof category", Type),
    diagnostic!(
        "E1355",
        "Invalid compile-time evaluation target",
        Validation
    ),
    diagnostic!("E1356", "Compile-time evaluation failed", Validation),
    diagnostic!("E2300", "Duplicate HTTP route", Module),
    diagnostic!("E2353", "Unknown object property", Type),
    diagnostic!("E2493", "Tuple index out of range", RuntimeSafety),
    diagnostic!("E2540", "Assignment to readonly value", Ownership),
    diagnostic!("E2541", "Recursive interface definition", Type),
    diagnostic!("E2542", "Invalid interface implementation", Type),
    diagnostic!("E2543", "Invalid type alias", Type),
    diagnostic!("E2741", "Missing required object property", Type),
    diagnostic!("E2742", "Unsafe CORS credential configuration", Validation),
    diagnostic!("E2743", "Unusable CORS option", Validation),
    diagnostic!("E3000", "`await` outside an async function", ControlFlow),
    diagnostic!("E3001", "`await` on a non-promise", Type),
    diagnostic!("E3002", "Unsupported `await` position", ControlFlow),
    diagnostic!(
        "E3003",
        "Async function missing a promise return type",
        Type
    ),
    diagnostic!("E3004", "Async function missing a return", ControlFlow),
    diagnostic!("E3005", "Unused promise", ControlFlow),
    diagnostic!("E3006", "Promise already awaited", Ownership),
    diagnostic!("E3007", "Unsupported owned async payload", Type),
    diagnostic!("E3008", "External thenable is not awaitable", Type),
    diagnostic!("E3009", "Shadowed native async error type", Name),
    diagnostic!(
        "E3010",
        "Lock guard held across an await in a loop",
        Ownership
    ),
    diagnostic!("E3011", "Unhandled thrown error", ControlFlow),
    diagnostic!("E3012", "`throws` on the program entry point", ControlFlow),
    diagnostic!("E3030", "`spawn` requires an async function", Type),
    diagnostic!("E3031", "Spawned task awaited immediately", ControlFlow),
    diagnostic!("E3032", "Busy wait in an async function", ControlFlow),
    diagnostic!("E3033", "Join handle already consumed", Ownership),
    diagnostic!("E3034", "Lock guard held across an await", Ownership),
    diagnostic!("E3035", "Invalid channel capacity", Validation),
    diagnostic!("E3036", "Unsupported channel payload", Type),
    diagnostic!("E3037", "Unsupported mutex payload", Type),
    diagnostic!("E3038", "Unsupported rwlock payload", Type),
    diagnostic!("E3039", "Unknown channel endpoint type", Name),
    diagnostic!("E3040", "Shadowed native channel type", Name),
    diagnostic!("E3041", "Unknown join handle type", Name),
    diagnostic!("E3042", "Shadowed native join handle type", Name),
    diagnostic!("E3043", "Unknown mutex type", Name),
    diagnostic!("E3044", "Shadowed native mutex type", Name),
    diagnostic!("E3045", "Unknown rwlock type", Name),
    diagnostic!("E3046", "Shadowed native rwlock type", Name),
    diagnostic!("E3047", "Shadowed native semaphore type", Name),
    diagnostic!("E3048", "Unknown promise type", Name),
    diagnostic!("E3049", "Shadowed native promise type", Name),
    diagnostic!("E3070", "Wrong `Ok`/`Err` arity", Type),
    diagnostic!("E3071", "Result error type mismatch", Type),
    diagnostic!("E3072", "Optional payload in a `Result`", Type),
    diagnostic!("E3073", "Result pattern on a non-result", Pattern),
    diagnostic!("E3074", "Result return needs `Ok` or `Err`", ControlFlow),
    diagnostic!("E3075", "Void result return needs `Ok()`", ControlFlow),
    diagnostic!("E3076", "`try` on a non-result", Type),
    diagnostic!("E3077", "`try` outside a function", ControlFlow),
    diagnostic!("E3078", "`try` in a function that returns no result", Type),
    diagnostic!("E3079", "Optional compared against absence", Type),
    diagnostic!("E3080", "`null` literal is not a value", Type),
    diagnostic!("E3081", "Move out of a field", Ownership),
    diagnostic!("E3082", "Borrow held across an await", Ownership),
    diagnostic!("E3083", "Environment file not found", Module),
    diagnostic!("E3085", "Async capture the frame cannot hold", ControlFlow),
    diagnostic!(
        "E3086",
        "Async parameter the frame cannot hold",
        ControlFlow
    ),
    diagnostic!(
        "E3087",
        "Async parameter needing owned frame storage",
        ControlFlow
    ),
    diagnostic!(
        "E3088",
        "Async local needing owned frame storage",
        ControlFlow
    ),
    diagnostic!("E3100", "Route without a declared response", Validation),
    diagnostic!("E3101", "Reply without a status", ControlFlow),
    diagnostic!("E3102", "Reply without a body", ControlFlow),
    diagnostic!("E3103", "Reply status set twice", ControlFlow),
    diagnostic!("E3104", "Reply sent twice", ControlFlow),
    diagnostic!(
        "E3105",
        "Reply body does not match the declared response",
        Type
    ),
    diagnostic!("E3106", "Undeclared response status", Validation),
    diagnostic!("E3107", "Raw numeric status code", Validation),
    diagnostic!("E3108", "Handler must take a reply", Type),
    diagnostic!("E3109", "Reply flow too complex to verify", ControlFlow),
    diagnostic!("E3110", "Handler type not imported", Name),
    diagnostic!("E3111", "Invalid port", Validation),
    diagnostic!("E3112", "Invalid server option", Validation),
    diagnostic!("E3113", "`listen` options are not a literal", Validation),
    diagnostic!("E3114", "Invalid body limit", Validation),
    diagnostic!("E3115", "Invalid time window", Validation),
    diagnostic!("E3116", "Invalid trusted proxy", Validation),
    diagnostic!("E3117", "Invalid header name", Validation),
    diagnostic!("E3118", "Protected response header", Validation),
    diagnostic!("E3119", "Reply used after stream", ControlFlow),
    diagnostic!("E3140", "Environment file not configured", Module),
    diagnostic!("E3141", "Undeclared environment variable", Name),
    diagnostic!("E3142", "Duplicate environment variable", Name),
    diagnostic!("E3143", "Invalid environment value", Validation),
    diagnostic!("E3144", "Environment accessed by index", Validation),
    diagnostic!("E3145", "Invalid `process` use", Validation),
    diagnostic!("E3146", "Invalid environment file syntax", Parse),
    diagnostic!("E3147", "Environment variable without a type", Type),
    diagnostic!("E3148", "Environment variable without a value", Validation),
    diagnostic!("E3149", "Unknown environment variable type", Type),
    diagnostic!("E3150", "Undefined environment reference", Name),
    diagnostic!("E3151", "Cyclic environment reference", Name),
    diagnostic!("E3152", "Invalid environment export decorator", Validation),
    diagnostic!("E3160", "Unknown validator decoration", Validation),
    diagnostic!(
        "E3161",
        "Validator decoration on an invalid target",
        Validation
    ),
    diagnostic!("E3162", "Invalid validator decoration argument", Validation),
    diagnostic!("E3163", "Contradictory validator decorations", Validation),
    diagnostic!("E3164", "Redundant validator decoration", Validation),
    diagnostic!("E3165", "Validator decoration cannot hold", Validation),
    diagnostic!("E3166", "Impossible validator condition", Validation),
    diagnostic!("E3167", "Redundant validator condition", Validation),
    diagnostic!("E3168", "Invalid custom validator function", Validation),
    diagnostic!("E3169", "Invalid custom validator application", Validation),
    diagnostic!("E3200", "Wrong number of type arguments", Type),
    diagnostic!("E3201", "Missing type arguments", Type),
    diagnostic!("E3202", "Unknown generic type", Name),
    diagnostic!("E3203", "Interface extends a non-interface", Type),
    diagnostic!("E3204", "Conflicting inherited field", Type),
    diagnostic!("E3205", "Interface inheritance cycle", Type),
    diagnostic!("E3206", "Type recursion limit reached", Type),
    diagnostic!("E3207", "`keyof` on a non-object", Type),
    diagnostic!("E3208", "Wrong utility type arity", Type),
    diagnostic!("E3209", "Utility key is not a literal", Type),
    diagnostic!("E3210", "Utility type needs a function", Type),
    diagnostic!("E3211", "Utility type needs an object", Type),
    diagnostic!("E3212", "Unknown key in a utility type", Type),
    diagnostic!("E3213", "Intersection operand is not an object type", Type),
    diagnostic!("E3214", "Conflicting field in a type intersection", Type),
    diagnostic!("E3215", "Type intersection outside a type alias", Parse),
    diagnostic!("E3216", "Type intersection cycle", Type),
    diagnostic!("E3217", "Type intersection mixed with a union", Parse),
    diagnostic!("E3218", "Missing semicolon", Parse),
    diagnostic!("E3240", "Invalid map key type", Type),
    diagnostic!("E3241", "Invalid set element type", Type),
    diagnostic!("E3242", "Invalid union-find element type", Type),
    diagnostic!("E3243", "Unsupported type for a memory view", Type),
    diagnostic!("E3244", "Unstable layout for a memory view", Type),
    diagnostic!("E3245", "Mutable copy binding in a memory view", Ownership),
    diagnostic!(
        "E3246",
        "Mutable string source for a memory view",
        Ownership
    ),
    diagnostic!("E3247", "Overlapping memory view bindings", Ownership),
    diagnostic!("E3270", "Wrong `styleText` arity", Type),
    diagnostic!("E3271", "Incomplete `styleText` call", Type),
    diagnostic!("E3272", "`styleText` style is not a literal", Validation),
    diagnostic!("E3273", "Unknown `styleText` style", Validation),
    diagnostic!("E3274", "Wrong `figlet` arity", Type),
    diagnostic!("E3275", "Incomplete `figlet` call", Type),
    diagnostic!("E3276", "`figlet` font is not a literal", Validation),
    diagnostic!("E3277", "Unknown `figlet` font", Validation),
    diagnostic!("E3278", "Insecure MD5 digest", Validation),
    diagnostic!("E3279", "JWT claims must be an object", Type),
    diagnostic!("E3280", "Unsupported JWT claim field", Type),
    diagnostic!("E3290", "Pipe stage is not callable", Type),
    diagnostic!("E3291", "Topic reference outside a pipe", ControlFlow),
    diagnostic!("E3292", "Enum variant payload arity", Enum),
    diagnostic!("E3293", "Shapeless body in the wrong place", Type),
    diagnostic!("E3294", "Server policy written in the wrong place", Type),
    diagnostic!("E3295", "Request method written twice", Type),
    diagnostic!("E3296", "Async return type mismatch", Type),
    diagnostic!(
        "E3297",
        "`is` on an `Error` that did not come from a `catch`",
        Type
    ),
    diagnostic!(
        "E3298",
        "Throwing function passed where its type does not declare the effect",
        Type
    ),
    diagnostic!(
        "E3299",
        "Synchronous error skips a `finally` that awaits",
        ControlFlow
    ),
    diagnostic!("E3300", "Invalid database call arguments", Type),
    diagnostic!("E3301", "Database schema snapshot unavailable", Module),
    diagnostic!("E3302", "Invalid SQL statement", Validation),
    diagnostic!("E3303", "Invalid database result type", Type),
    diagnostic!("E3304", "Invalid ORM model mapping", Validation),
    diagnostic!("E3305", "Database parameter limit exceeded", Validation),
    diagnostic!(
        "E3306",
        "SQL query is not supported by the configured backend",
        Module
    ),
    diagnostic!(
        "E3307",
        "Relation include is not supported by the configured backend",
        Module
    ),
    diagnostic!("E3308", "Protected export outside its folder", Module),
    diagnostic!("E3309", "`protect` combined with a default export", Parse),
    diagnostic!("E3310", "`protect` applied to a re-export", Parse),
    diagnostic!("E3311", "Unsupported archive extension", Validation),
    diagnostic!("E3312", "Invalid compression level", Validation),
];

pub fn codes() -> &'static [DiagnosticCode] {
    CODES
}

pub fn find(code: &str) -> Option<&'static DiagnosticCode> {
    CODES
        .binary_search_by_key(&code, |diagnostic| diagnostic.code)
        .ok()
        .map(|index| &CODES[index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_unique_well_formed_and_sorted() {
        for pair in CODES.windows(2) {
            assert!(
                pair[0].code < pair[1].code,
                "codes must be unique and sorted"
            );
        }
        for diagnostic in CODES {
            let bytes = diagnostic.code.as_bytes();
            assert_eq!(
                bytes.len(),
                5,
                "{} must have five characters",
                diagnostic.code
            );
            assert_eq!(bytes[0], b'E', "{} must start with E", diagnostic.code);
            assert!(bytes[1..].iter().all(u8::is_ascii_digit));
            assert!(!diagnostic.title.is_empty());
        }
    }

    #[test]
    fn lookup_finds_registered_codes_only() {
        assert_eq!(
            find("E0308").map(|diagnostic| diagnostic.title),
            Some("Mismatched types")
        );
        assert!(find("E9999").is_none());
        assert!(find("no-unused-vars").is_none());
    }
}
