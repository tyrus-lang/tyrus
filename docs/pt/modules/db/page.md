---
title: Page
description: Tipo de paginação exportado por tyrus:db.
---

# Page

Importe de `tyrus:db` os tipos usados em anotações. Em especial, importe `Page` explicitamente sempre que uma assinatura de função usar o tipo de resultado paginado.

<SignatureBlock label="Assinatura">interface Page&lt;T&gt; {<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly items: T[];<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly total: u64;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly pageSize: u64;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly page?: u64;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly totalPages?: u64;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly nextCursor?: string;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly hasMore?: boolean;<br />}</SignatureBlock>

A paginação numerada retorna `items`, `total`, `pageSize`, `page` e `totalPages`. A paginação por cursor retorna `items`, `total`, `pageSize` e `hasMore`, além de `nextCursor` quando existe outra página. Os campos do modo de paginação inativo são omitidos.

Os itens e o `total` da página são lidos do mesmo snapshot de transação em todos os backends relacionais, então gravações concorrentes não fazem a contagem e os itens descreverem estados diferentes do banco. O MongoDB usa o `_id` gerado para manter a ordenação estável; a contagem e a leitura dos itens são operações separadas, então gravações concorrentes podem afetar a consistência entre elas. Seu token de cursor é o ObjectId serializado de 24 caracteres do último item retornado; passe `nextCursor` sem alterações para `after`.

## Exemplo de paginação numerada

Passe o número da página com `paginate: true`. A primeira página é a página 1; use o `totalPages` retornado para saber se há outras páginas numeradas.

<!-- @include: ../../../.fragments/modules/db/page/numbered-example.md -->

## Exemplo de paginação por cursor

Omita `after` para buscar a primeira página. Passe o `nextCursor` retornado sem alterações como `after` para buscar a próxima página. O cursor é o valor serializado do `@id` da última linha retornada, não a quantidade de itens nem o número da página. Para um `@id` inteiro, `"25"` significa continuar após a linha cujo id é 25; `after: "10"` começa após o id 10, não no décimo resultado. Não calcule o cursor usando `pageSize`.

<!-- @include: ../../../.fragments/modules/db/page/cursor-example.md -->
