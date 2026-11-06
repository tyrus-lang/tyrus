---
title: findMany
description: Consulta registros de modelos con filtros validados y parametrizados.
---

# Model.findMany

Importa una interfaz `@model` exportada y llama a `Model.findMany(filter)` para buscar sus registros. El método devuelve `Promise<Result<Model[], DbError>>` y aplica los mapeos físicos de tabla y columna del modelo.

Espera el método antes de usar el `Result`. Los campos de un filtro plano se combinan con `AND`. Un valor escalar significa igualdad, como en `User.findMany({ active: true })`. Para comparar, usa un objeto anidado con uno o más operadores entre `eq`, `ne`, `gt`, `gte`, `lt` y `lte`, por ejemplo `User.findMany({ age: { gte: 18 } })`.

Los operadores de comparación significan: `eq` es igual a, `ne` es distinto de, `gt` es mayor que, `gte` es mayor o igual que, `lt` es menor que y `lte` es menor o igual que. Cuando un campo tiene más de un operador, todas sus condiciones deben coincidir.

Usa `or` con un array no vacío de objetos de filtro para encontrar cualquier alternativa. Los campos de cada objeto se combinan con `AND`, los objetos se combinan con `OR`, y los campos junto a `or` siguen combinándose con todo el grupo mediante `AND`.

Usa `in` con un array no vacío para buscar cualquiera de los valores indicados. En un patrón `like`, `%` coincide con cero o más caracteres y `_` coincide con un carácter: `"Ada"` coincide exactamente con `Ada`, `"Ada%"` coincide con valores que empiezan por `Ada`, `"%son"` coincide con valores que terminan en `son` y `"%Ada%"` coincide con valores que contienen `Ada` en cualquier posición. La coincidencia sigue el comportamiento `LIKE` del backend configurado. Todos los valores siguen siendo parámetros y se validan respecto a la columna del modelo seleccionado.

Este ejemplo supone que el modelo declara los campos `active`, `age` y `role`.

<!-- @include: ../../../.fragments/modules/db/find-many/or-filter.md -->

## Carga de relaciones

Declara relaciones de modelo con `@belongsTo("foreignKeyField")`, `@hasOne("foreignKeyField")`, `@hasMany("foreignKeyField")` o `@manyToMany("pivotTable", "sourceKey", "targetKey")` y pasa sus nombres en `{ include: ["team"] }`. Selecciona un subconjunto de campos escalares con `{ include: [{ team: ["id", "name"] }] }`; el tipo del resultado refleja los campos seleccionados. Los includes sin proyección devuelven el Model relacionado completo. La carga es explícita, usa una instrucción SQL y admite un solo nivel. Incluir una relación `hasMany` o `manyToMany` requiere `relationLimit` de 1 a 500, que limita cada colección incluida. Las colecciones relacionadas se ordenan por el `@id` del Model relacionado para mantener resultados estables.

<!-- @include: ../../../.fragments/modules/db/find-many/relationships.md -->

## Paginación numerada

Pasa `{ paginate: true, pageSize, page? }` como segundo argumento para devolver una página numerada. La página predeterminada es 1. `Page<T>` contiene `items`, `total`, `pageSize`, `page` y `totalPages`; `total` cuenta todos los registros que coinciden con el filtro antes de paginar. MongoDB admite paginación numerada al ordenar por `_id` y aplicar el desplazamiento solicitado. Importa `Page` desde `tyrus:db` cuando incluyas este resultado en la firma de una función.

<!-- @include: ../../../.fragments/modules/db/find-many/pagination.md -->

## Paginación por cursor

Define `cursor: true` para usar paginación por cursor. Omite `after` en la primera llamada; en la siguiente, pasa el `nextCursor` de la página anterior sin modificar. El cursor es el valor serializado del `@id` de la última fila devuelta, no la cantidad de elementos ni el número de página. Para un `@id` entero, `after: "10"` continúa después del id 10, no en el décimo resultado. Los resultados se ordenan por el `@id` del modelo, y `hasMore` indica si existe otra página. `total` sigue contando todos los registros coincidentes, independientemente de la posición del cursor. MongoDB usa el `_id` generado para ordenar y comparar el cursor; su token es el valor serializado de ObjectId de 24 caracteres devuelto en la página.

El compilador comprueba cada campo con el modelo y valida la consulta generada y los tipos de parámetros con el snapshot local del schema del backend relacional configurado. MongoDB usa la misma API de filtros de modelo, pero no utiliza un snapshot de schema relacional. El recuento y los elementos de la página se leen por separado, por lo que las escrituras concurrentes pueden cambiar el total observado entre solicitudes de página. Los valores se vinculan mediante placeholders; los identificadores de tabla y columna provienen únicamente de la declaración del modelo.

Las mutaciones del modelo están documentadas en la página [Modelos](./models.md). Usa [`findOne`](./find-one.md) para devolver un registro o un valor de ausencia tipado.
