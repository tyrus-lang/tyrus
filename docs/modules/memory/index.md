---
title: Memory
description: Explicit lifecycle operations for sensitive values.
---

# Memory

`tyrus:memory` provides explicit lifecycle operations for values that contain sensitive data. The initial API is `zeroize`, which consumes an exclusively owned dynamic `string` or `u8[]`, overwrites its allocation with zeroes using non-elidable writes, and releases it.

## API

- [drop](./drop): release an owned string, array, collection, or native handle before the end of its scope.
- [swap](./swap): exchange two distinct mutable bindings of the same `Copy` type or `string`.
- [replace](./replace): store a new `Copy` value or move a `string` from its source binding, then return the previous value.
- [sizeOf](./size-of): the compile-time size in bytes of a primitive type.
- [alignOf](./align-of): the compile-time alignment in bytes of a primitive type.
- [zeroize](./zeroize): erase and release an owned `string` or `u8[]`.
- [MemoryError](./errors): the error returned when a value is static, borrowed, shared, or invalid for secure erasure.
