---
title: revealInFolder
description: Open the platform's file manager at a path's containing directory through the native tyrus:os module.
---

# revealInFolder

`revealInFolder` opens the platform's file manager at `path`'s containing directory (or at `path` itself, if it is already a directory).

<SignatureBlock label="Signature">revealInFolder(path: string): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-reveal-in-folder.md -->
Returns `Err(OsError)` with code `ERR_OS_INVALID_ARGUMENT` when `path` is null at the native boundary, or `ERR_OS_REVEAL_FAILED` when no file manager can open the containing directory. The error message describes the failure.

::: tip Best effort, not a guaranteed selection
Unlike a native Explorer/Finder integration, this does not guarantee `path` itself ends up pre-selected, no cross-platform crate exposes that, and each desktop's "select in file manager" mechanism is a different, undocumented command-line flag. A headless machine or missing file manager returns `ERR_OS_REVEAL_FAILED`.
:::
