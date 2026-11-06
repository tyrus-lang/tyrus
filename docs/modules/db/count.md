---
title: count
description: Count model records that match a filter.
---

# count

Call `Model.count(filter)` to count model records matching a filter. The result is `Promise<Result<u64, DbError>>`; await it before using the `Result`.

Pass an empty object, `Model.count({})`, to count every record in the model table.

## Example

<!-- @include: ../../.fragments/modules/db/count/example.md -->
