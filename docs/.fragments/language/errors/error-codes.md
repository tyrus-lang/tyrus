<!-- GENERATED from crates/tyrus-core/src/diagnostics/codes.rs by `xtask` - do not edit. -->

| Code | Title | Category |
| --- | --- | --- |
| `E0001` | Unexpected token | Parse |
| `E0002` | Invalid syntax | Parse |
| `E0003` | Reserved keyword | Parse |
| `E0004` | Non-exhaustive match | Patterns |
| `E0034` | Multiple applicable items | Name resolution |
| `E0057` | Wrong callback parameter count | Type checking |
| `E0061` | Wrong argument count | Type checking |
| `E0063` | Missing object field | Type checking |
| `E0069` | Invalid return | Control flow |
| `E0107` | Wrong generic argument count | Type checking |
| `E0268` | Invalid continue | Control flow |
| `E0269` | Invalid break | Control flow |
| `E0277` | Trait or capability requirement not satisfied | Type checking |
| `E0282` | Type annotations needed | Type checking |
| `E0308` | Mismatched types | Type checking |
| `E0309` | Invalid regular expression flag | Validation |
| `E0310` | Object literal needs an interface annotation | Type checking |
| `E0315` | Invalid assignment target | Type checking |
| `E0316` | Invalid destructuring assignment | Type checking |
| `E0368` | Invalid compound assignment | Type checking |
| `E0369` | Unsupported binary operation | Type checking |
| `E0381` | Use of possibly uninitialized variable | Control flow |
| `E0382` | Use of moved value | Ownership |
| `E0384` | Assignment to immutable binding | Ownership |
| `E0412` | Unknown type | Name resolution |
| `E0425` | Unknown name | Name resolution |
| `E0428` | Duplicate definition | Name resolution |
| `E0432` | Unresolved import | Modules |
| `E0433` | Unresolved module or type | Modules |
| `E0434` | Test framework outside a test file | Modules |
| `E0435` | Invalid value in a constant expression | Validation |
| `E0436` | Functional array operation has no return value | Type checking |
| `E0500` | Assignment to immutable captured value | Ownership |
| `E0502` | Conflicting borrow | Ownership |
| `E0505` | Move while borrowed | Ownership |
| `E0506` | Assignment while borrowed | Ownership |
| `E0529` | Invalid pattern | Patterns |
| `E0571` | Invalid break value | Control flow |
| `E0572` | Invalid continue target | Control flow |
| `E0580` | Invalid function declaration | Type checking |
| `E0583` | Module file not found | Modules |
| `E0599` | Unknown method | Name resolution |
| `E0601` | Missing entry point | Modules |
| `E0605` | Invalid type cast | Type checking |
| `E0606` | Index out of bounds | Runtime safety |
| `E0608` | Cannot index this value | Type checking |
| `E0609` | Unknown field | Name resolution |
| `E0618` | Called non-function value | Type checking |
| `E0658` | Unsupported language feature | Type checking |
| `E0701` | Invalid enum representation | Enums |
| `E0702` | Mixed enum member types | Enums |
| `E0703` | Mixed automatic and explicit enum values | Enums |
| `E0705` | Enum member value out of range | Enums |
| `E0706` | Value does not fit its type | Type checking |
| `E0707` | Invalid numeric limit | Type checking |
| `E0708` | Invalid character literal | Type checking |
| `E0716` | Forced optional unwrap | Type checking |
| `E0717` | Invalid template format specifier | Type checking |
| `E0747` | Invalid compile-time array argument | Type checking |
| `E0790` | Invalid array match pattern | Patterns |
| `E0791` | Multiple array rest patterns | Patterns |
| `E0792` | Array rest pattern is not last | Patterns |
| `E0793` | Invalid array rest binding | Patterns |
| `E0794` | Value not representable as unknown | Type checking |
| `E0795` | Invalid typeof category | Type checking |
| `E1355` | Invalid compile-time evaluation target | Validation |
| `E1356` | Compile-time evaluation failed | Validation |
| `E2300` | Duplicate HTTP route | Modules |
| `E2353` | Unknown object property | Type checking |
| `E2493` | Tuple index out of range | Runtime safety |
| `E2540` | Assignment to readonly value | Ownership |
| `E2541` | Recursive interface definition | Type checking |
| `E2542` | Invalid interface implementation | Type checking |
| `E2543` | Invalid type alias | Type checking |
| `E2741` | Missing required object property | Type checking |
| `E2742` | Unsafe CORS credential configuration | Validation |
| `E2743` | Unusable CORS option | Validation |
| `E3000` | `await` outside an async function | Control flow |
| `E3001` | `await` on a non-promise | Type checking |
| `E3002` | Unsupported `await` position | Control flow |
| `E3003` | Async function missing a promise return type | Type checking |
| `E3004` | Async function missing a return | Control flow |
| `E3005` | Unused promise | Control flow |
| `E3006` | Promise already awaited | Ownership |
| `E3007` | Unsupported owned async payload | Type checking |
| `E3008` | External thenable is not awaitable | Type checking |
| `E3009` | Shadowed native async error type | Name resolution |
| `E3010` | Lock guard held across an await in a loop | Ownership |
| `E3011` | Unhandled thrown error | Control flow |
| `E3012` | `throws` on the program entry point | Control flow |
| `E3030` | `spawn` requires an async function | Type checking |
| `E3031` | Spawned task awaited immediately | Control flow |
| `E3032` | Busy wait in an async function | Control flow |
| `E3033` | Join handle already consumed | Ownership |
| `E3034` | Lock guard held across an await | Ownership |
| `E3035` | Invalid channel capacity | Validation |
| `E3036` | Unsupported channel payload | Type checking |
| `E3037` | Unsupported mutex payload | Type checking |
| `E3038` | Unsupported rwlock payload | Type checking |
| `E3039` | Unknown channel endpoint type | Name resolution |
| `E3040` | Shadowed native channel type | Name resolution |
| `E3041` | Unknown join handle type | Name resolution |
| `E3042` | Shadowed native join handle type | Name resolution |
| `E3043` | Unknown mutex type | Name resolution |
| `E3044` | Shadowed native mutex type | Name resolution |
| `E3045` | Unknown rwlock type | Name resolution |
| `E3046` | Shadowed native rwlock type | Name resolution |
| `E3047` | Shadowed native semaphore type | Name resolution |
| `E3048` | Unknown promise type | Name resolution |
| `E3049` | Shadowed native promise type | Name resolution |
| `E3070` | Wrong `Ok`/`Err` arity | Type checking |
| `E3071` | Result error type mismatch | Type checking |
| `E3072` | Optional payload in a `Result` | Type checking |
| `E3073` | Result pattern on a non-result | Patterns |
| `E3074` | Result return needs `Ok` or `Err` | Control flow |
| `E3075` | Void result return needs `Ok()` | Control flow |
| `E3076` | `try` on a non-result | Type checking |
| `E3077` | `try` outside a function | Control flow |
| `E3078` | `try` in a function that returns no result | Type checking |
| `E3079` | Optional compared against absence | Type checking |
| `E3080` | `null` literal is not a value | Type checking |
| `E3081` | Move out of a field | Ownership |
| `E3082` | Borrow held across an await | Ownership |
| `E3083` | Environment file not found | Modules |
| `E3085` | Async capture the frame cannot hold | Control flow |
| `E3086` | Async parameter the frame cannot hold | Control flow |
| `E3087` | Async parameter needing owned frame storage | Control flow |
| `E3088` | Async local needing owned frame storage | Control flow |
| `E3100` | Route without a declared response | Validation |
| `E3101` | Reply without a status | Control flow |
| `E3102` | Reply without a body | Control flow |
| `E3103` | Reply status set twice | Control flow |
| `E3104` | Reply sent twice | Control flow |
| `E3105` | Reply body does not match the declared response | Type checking |
| `E3106` | Undeclared response status | Validation |
| `E3107` | Raw numeric status code | Validation |
| `E3108` | Handler must take a reply | Type checking |
| `E3109` | Reply flow too complex to verify | Control flow |
| `E3110` | Handler type not imported | Name resolution |
| `E3111` | Invalid port | Validation |
| `E3112` | Invalid server option | Validation |
| `E3113` | `listen` options are not a literal | Validation |
| `E3114` | Invalid body limit | Validation |
| `E3115` | Invalid time window | Validation |
| `E3116` | Invalid trusted proxy | Validation |
| `E3117` | Invalid header name | Validation |
| `E3118` | Protected response header | Validation |
| `E3119` | Reply used after stream | Control flow |
| `E3140` | Environment file not configured | Modules |
| `E3141` | Undeclared environment variable | Name resolution |
| `E3142` | Duplicate environment variable | Name resolution |
| `E3143` | Invalid environment value | Validation |
| `E3144` | Environment accessed by index | Validation |
| `E3145` | Invalid `process` use | Validation |
| `E3146` | Invalid environment file syntax | Parse |
| `E3147` | Environment variable without a type | Type checking |
| `E3148` | Environment variable without a value | Validation |
| `E3149` | Unknown environment variable type | Type checking |
| `E3150` | Undefined environment reference | Name resolution |
| `E3151` | Cyclic environment reference | Name resolution |
| `E3152` | Invalid environment export decorator | Validation |
| `E3160` | Unknown validator decoration | Validation |
| `E3161` | Validator decoration on an invalid target | Validation |
| `E3162` | Invalid validator decoration argument | Validation |
| `E3163` | Contradictory validator decorations | Validation |
| `E3164` | Redundant validator decoration | Validation |
| `E3165` | Validator decoration cannot hold | Validation |
| `E3166` | Impossible validator condition | Validation |
| `E3167` | Redundant validator condition | Validation |
| `E3168` | Invalid custom validator function | Validation |
| `E3169` | Invalid custom validator application | Validation |
| `E3200` | Wrong number of type arguments | Type checking |
| `E3201` | Missing type arguments | Type checking |
| `E3202` | Unknown generic type | Name resolution |
| `E3203` | Interface extends a non-interface | Type checking |
| `E3204` | Conflicting inherited field | Type checking |
| `E3205` | Interface inheritance cycle | Type checking |
| `E3206` | Type recursion limit reached | Type checking |
| `E3207` | `keyof` on a non-object | Type checking |
| `E3208` | Wrong utility type arity | Type checking |
| `E3209` | Utility key is not a literal | Type checking |
| `E3210` | Utility type needs a function | Type checking |
| `E3211` | Utility type needs an object | Type checking |
| `E3212` | Unknown key in a utility type | Type checking |
| `E3213` | Intersection operand is not an object type | Type checking |
| `E3214` | Conflicting field in a type intersection | Type checking |
| `E3215` | Type intersection outside a type alias | Parse |
| `E3216` | Type intersection cycle | Type checking |
| `E3217` | Type intersection mixed with a union | Parse |
| `E3218` | Missing semicolon | Parse |
| `E3240` | Invalid map key type | Type checking |
| `E3241` | Invalid set element type | Type checking |
| `E3242` | Invalid union-find element type | Type checking |
| `E3243` | Unsupported type for a memory view | Type checking |
| `E3244` | Unstable layout for a memory view | Type checking |
| `E3245` | Mutable copy binding in a memory view | Ownership |
| `E3246` | Mutable string source for a memory view | Ownership |
| `E3247` | Overlapping memory view bindings | Ownership |
| `E3270` | Wrong `styleText` arity | Type checking |
| `E3271` | Incomplete `styleText` call | Type checking |
| `E3272` | `styleText` style is not a literal | Validation |
| `E3273` | Unknown `styleText` style | Validation |
| `E3274` | Wrong `figlet` arity | Type checking |
| `E3275` | Incomplete `figlet` call | Type checking |
| `E3276` | `figlet` font is not a literal | Validation |
| `E3277` | Unknown `figlet` font | Validation |
| `E3278` | Insecure MD5 digest | Validation |
| `E3279` | JWT claims must be an object | Type checking |
| `E3280` | Unsupported JWT claim field | Type checking |
| `E3290` | Pipe stage is not callable | Type checking |
| `E3291` | Topic reference outside a pipe | Control flow |
| `E3292` | Enum variant payload arity | Enums |
| `E3293` | Shapeless body in the wrong place | Type checking |
| `E3294` | Server policy written in the wrong place | Type checking |
| `E3295` | Request method written twice | Type checking |
| `E3296` | Async return type mismatch | Type checking |
| `E3297` | `is` on an `Error` that did not come from a `catch` | Type checking |
| `E3298` | Throwing function passed where its type does not declare the effect | Type checking |
| `E3299` | Synchronous error skips a `finally` that awaits | Control flow |
| `E3300` | Invalid database call arguments | Type checking |
| `E3301` | Database schema snapshot unavailable | Modules |
| `E3302` | Invalid SQL statement | Validation |
| `E3303` | Invalid database result type | Type checking |
| `E3304` | Invalid ORM model mapping | Validation |
| `E3305` | Database parameter limit exceeded | Validation |
| `E3306` | SQL query is not supported by the configured backend | Modules |
| `E3307` | Relation include is not supported by the configured backend | Modules |
| `E3308` | Protected export outside its folder | Modules |
| `E3309` | `protect` combined with a default export | Parse |
| `E3310` | `protect` applied to a re-export | Parse |
| `E3311` | Unsupported archive extension | Validation |
| `E3312` | Invalid compression level | Validation |
