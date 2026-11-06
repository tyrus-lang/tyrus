---
title: Pipe Operator
description: The pipe operator, topic token, and composing pipeline transformations.
---

# Pipe Operator

The `|>` operator passes a value into a function call. The operator is compile-time syntax and does not create a runtime pipeline object.

<!-- @include: ../.fragments/language/operators/pipeline.md -->

Without `%`, the left value is inserted as the first argument of the right call. With `%`, the left value is inserted exactly at the topic position.

Pipeline chains compose with closures and functional methods, making data transformations read top-to-bottom:

<!-- @include: ../.fragments/language/operators/pipeline-closures.md -->

The topic token `%` is useful when a function needs the piped value in a position other than the first argument:

<!-- @include: ../.fragments/language/operators/pipeline-topic.md -->
