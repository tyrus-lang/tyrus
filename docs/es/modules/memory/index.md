---
title: Memory
description: Operaciones explícitas de ciclo de vida para valores sensibles.
---

# Memory

`tyrus:memory` proporciona operaciones explícitas de ciclo de vida para valores que contienen datos sensibles. La API inicial es `zeroize`, que consume un `string` dinámico o `u8[]` de propiedad exclusiva, sobrescribe su asignación con ceros mediante escrituras no eliminables y la libera.

## API

- [drop](./drop): libera un string, array, colección o handle nativo antes del final de su ámbito.
- [swap](./swap): intercambia dos bindings mutables distintos del mismo tipo `Copy` o `string`.
- [replace](./replace): almacena un nuevo valor `Copy` o mueve un `string` desde el binding de origen y luego devuelve el valor anterior.
- [sizeOf](./size-of): el tamaño en bytes, en tiempo de compilación, de un tipo primitivo.
- [alignOf](./align-of): la alineación en bytes, en tiempo de compilación, de un tipo primitivo.
- [zeroize](./zeroize): borra y libera un `string` o `u8[]` de propiedad exclusiva.
- [MemoryError](./errors): el error devuelto cuando un valor es estático, prestado, compartido o no válido para borrado seguro.
