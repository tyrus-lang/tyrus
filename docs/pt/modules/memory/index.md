---
title: Memory
description: Operações explícitas de ciclo de vida para valores sensíveis.
---

# Memory

`tyrus:memory` fornece operações explícitas de ciclo de vida para valores que contêm dados sensíveis. A API inicial é `zeroize`, que consome uma `string` dinâmica ou `u8[]` de posse exclusiva, sobrescreve sua alocação com zeros usando escritas não elimináveis e a libera.

## API

- [drop](./drop): libera uma string, array, coleção ou handle nativo antes do fim do seu escopo.
- [swap](./swap): troca dois bindings mutáveis distintos do mesmo tipo `Copy` ou `string`.
- [replace](./replace): armazena um novo valor `Copy` ou move uma `string` do binding de origem e então retorna o valor anterior.
- [sizeOf](./size-of): o tamanho em bytes, em tempo de compilação, de um tipo primitivo.
- [alignOf](./align-of): o alinhamento em bytes, em tempo de compilação, de um tipo primitivo.
- [zeroize](./zeroize): apaga e libera uma `string` ou `u8[]` com posse exclusiva.
- [MemoryError](./errors): o erro retornado quando um valor é estático, emprestado, compartilhado ou inválido para apagamento seguro.
