---
title: zeroize
description: Borra y libera strings y bytes sensibles.
---

# zeroize

<SignatureBlock label="Firma">zeroize(value: string): Result&lt;void, MemoryError&gt;</SignatureBlock>

<SignatureBlock label="Firma">zeroize(value: u8[]): Result&lt;void, MemoryError&gt;</SignatureBlock>

`zeroize(value)` consume un `string` o `u8[]` local. Sobrescribe la asignación con ceros mediante escrituras volátiles, coloca una barrera de compilador después de las escrituras y libera la asignación. El binding no puede usarse después.

Los strings dinámicos deben tener exactamente una referencia viva. Los literales de string, préstamos y strings compartidos se rechazan con `MemoryError`, porque borrarlos escribiría almacenamiento de solo lectura o mutaría datos todavía visibles mediante otro valor.

<!-- @include: ../../../.fragments/modules/memory/zeroize/zeroize.md -->
