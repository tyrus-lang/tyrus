---
title: documentsDir
description: Read the user's documents directory through the native tyrus:os module.
---

# documentsDir

`documentsDir` returns the user's documents directory.

<SignatureBlock label="Signature">documentsDir(): string?</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-documents-dir.md -->
::: tip Absence is a valid outcome
A headless system (a server, a container, CI) commonly has no documents directory at all, that is a normal result, not a failure.
:::
