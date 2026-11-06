---
title: exists
description: Verifica se um caminho existe pelo módulo nativo tyrus:fs.
---

# exists

`exists` verifica se um caminho existe no sistema de arquivos e retorna um `boolean`. `exists` retorna `false` para qualquer erro (arquivo inexistente, permissão negada etc.) em vez de retornar um `Result`.

<SignatureBlock label="Assinatura">exists(path: string): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/fs/exists/basic-usage.md -->
## Observações

`exists` é uma verificação conveniente para quem precisa apenas saber se um caminho está acessível. As outras operações recuperáveis retornam `Result` e devem ser tratadas com `match` ou `?`:

<!-- @include: ../../../.fragments/modules/fs/exists/with-explicit-types.md -->
