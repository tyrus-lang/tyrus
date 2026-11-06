---
title: exists
description: Check whether a path exists through the native tyrus:fs module.
---

# exists

`exists` checks whether a path exists on the filesystem and returns a `boolean`. `exists` returns `false` for any error (missing file, permission denied, etc.) instead of returning a `Result`.

<SignatureBlock label="Signature">exists(path: string): boolean</SignatureBlock>



<!-- @include: ../../.fragments/modules/fs/exists/basic-usage.md -->
## Notes

`exists` is a convenience check for callers that only need to know whether a path is accessible. Other recoverable operations return `Result` and should be handled with `match` or `?`:

<!-- @include: ../../.fragments/modules/fs/exists/with-explicit-types.md -->
