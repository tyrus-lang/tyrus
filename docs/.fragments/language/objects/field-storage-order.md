```tyrus
interface Reading {
    ok: boolean;
    id: i64;
    scale: u8;
    value: f64;
    stale: boolean;
    sequence: i32;
}

const reading: Reading = {
    ok: true,
    id: 900000000001,
    scale: 4,
    value: 2.5,
    stale: false,
    sequence: 77,
};
```
