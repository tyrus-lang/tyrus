---
title: Page
description: Tipo de paginación exportado por tyrus:db.
---

# Page

Importa desde `tyrus:db` los tipos usados en anotaciones. En particular, importa `Page` explícitamente siempre que una firma de función use el tipo de resultado paginado.

<SignatureBlock label="Firma">interface Page&lt;T&gt; {<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly items: T[];<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly total: u64;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly pageSize: u64;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly page?: u64;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly totalPages?: u64;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly nextCursor?: string;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly hasMore?: boolean;<br />}</SignatureBlock>

La paginación numerada devuelve `items`, `total`, `pageSize`, `page` y `totalPages`. La paginación por cursor devuelve `items`, `total`, `pageSize` y `hasMore`, además de `nextCursor` cuando hay otra página. Se omiten los campos del modo de paginación inactivo.

Los elementos y el `total` de la página se leen desde la misma instantánea de transacción en todos los backends relacionales, así que las escrituras concurrentes no hacen que el recuento y los elementos describan estados distintos de la base de datos. MongoDB usa el `_id` generado para mantener un orden estable; el recuento y la lectura de elementos son operaciones separadas, por lo que las escrituras concurrentes pueden afectar su consistencia. Su token de cursor es el ObjectId serializado de 24 caracteres del último elemento devuelto; pasa `nextCursor` sin modificar a `after`.

## Ejemplo de paginación numerada

Pasa el número de página con `paginate: true`. La primera página es la página 1; usa el `totalPages` devuelto para saber si hay más páginas numeradas.

<!-- @include: ../../../.fragments/modules/db/page/numbered-example.md -->

## Ejemplo de paginación por cursor

Omite `after` para obtener la primera página. Pasa el `nextCursor` devuelto sin modificar como `after` para obtener la página siguiente. El cursor es el valor serializado del `@id` de la última fila devuelta, no la cantidad de elementos ni el número de página. Para un `@id` entero, `"25"` significa continuar después de la fila cuyo id es 25; `after: "10"` empieza después del id 10, no en el décimo resultado. No calcules el cursor a partir de `pageSize`.

<!-- @include: ../../../.fragments/modules/db/page/cursor-example.md -->
