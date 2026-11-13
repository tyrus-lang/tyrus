---
title: timeout
description: Bound how long a Promise is allowed to take, rejecting and cancelling it once a deadline elapses.
---

# timeout

`timeout` races `task` against a deadline of `ms` milliseconds.

<SignatureBlock label="Signature">timeout&lt;T&gt;(ms: u64, task: Promise&lt;T&gt;): Promise&lt;T&gt;</SignatureBlock>

If `task` settles before the deadline, `timeout` adopts its outcome, resolving or rejecting the same way `task` did. If the deadline elapses first, `timeout` rejects and cooperatively cancels `task` through the same mechanism `cancel` uses, the same way `race` cancels the handles it did not pick.

<!-- @include: ../../.fragments/modules/task/timeout.md -->
