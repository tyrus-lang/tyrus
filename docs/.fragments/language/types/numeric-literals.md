```tyrus
// Hexadecimal: 0x or 0X prefix; hexadecimal digits may use A-F or a-f.
const hex: i32 = 0xFF;         // 255
const hexLower: i32 = 0xff;    // 255
const hex2: i32 = 0xABCD;      // 43981
const hexSep: i32 = 0xFF_FF;   // 65535

// Binary: 0b or 0B prefix.
const binary: i32 = 0b1010;    // 10
const binary2: i32 = 0B1111;   // 15

// Octal: 0o or 0O prefix.
const octal: i32 = 0o77;       // 63
const octal2: i32 = 0O755;     // 493

// Decimal integer separators group digits in sets of three.
const big: i32 = 1_000_000;    // 1000000
// Invalid decimal separators: 1_000_, 10_00, 1_00_0_000.
// Positive integer literals above i128.MAX require an explicit u128 context and may not exceed u128.MAX.
// The signed negative boundary is -170_141_183_460_469_231_731_687_303_715_884_105_728.

// Scientific notation is supported for decimal floating-point literals.
const sci: f64 = 1.5e10;       // 15000000000
const sci2: f64 = 2.5E-3;      // 0.0025
const sci3: f64 = 1e5;         // 100000
// Plain decimal float literals with very large integer parts are rejected instead of being widened to f128 implicitly.
```
