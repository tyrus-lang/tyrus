---
title: hungarian
description: "Minimum-cost assignment from tyrus:algorithm."
---

# hungarian

`hungarian(costMatrix)` assigns rows to distinct columns with minimum total cost. The input is an `i32[][]` cost matrix and the result contains the optimal cost plus row-column pairs.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `hungarian(costMatrix: i32[][])` | `[i32, [usize, usize][]]` | O(n³) for an n × n matrix |

For rectangular matrices, every row is assigned when enough columns exist. Negative and positive costs are supported; the returned pair list identifies the chosen column for each assigned row.

## Example

<!-- @include: ../../.fragments/modules/algorithm/hungarian/example.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/hungarian/study-result.md -->
