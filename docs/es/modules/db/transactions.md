---
title: Transacciones
description: Handles explícitos de transacción SQL para agrupar consultas y operaciones de Model.
---

# Transacciones de base de datos

Usa `db.beginTransaction()` para iniciar una transacción, pasa su handle como `{ transaction }` a `db.query` o a una operación de Model sin paginación y finaliza con `db.commitTransaction(transaction)` o `db.rollbackTransaction(transaction)`. Cada llamada es asíncrona y devuelve `Promise<Result<T, DbError>>`; gestiona cada resultado con `onfail`, `?` o pattern matching.

`db.beginTransaction()` devuelve `Promise<Result<DbTransaction, DbError>>`; commit y rollback devuelven `Promise<Result<boolean, DbError>>`, donde `true` confirma que la transacción finalizó. Importa `DbTransaction` desde `tyrus:db` solo cuando lo escribas en una anotación de tipo explícita; los handles inferidos no necesitan un import separado.

## Alcance

El handle de la transacción mantiene las consultas SQL y las operaciones de Model sin paginación en la misma conexión y transacción para SQLite, PostgreSQL o MySQL. Pasa `{ transaction }` como objeto de opciones a `findOne` y `count`, inclúyelo en el objeto de opciones existente de `findMany` y pásalo como último argumento de opciones a las mutaciones de Model. `findMany` con paginación administra su propia transacción de lectura consistente y rechaza un handle explícito. Las transacciones de MongoDB no están disponibles en esta API.

Las transacciones son explícitas: una secuencia exitosa debe ejecutar commit, mientras que un error detectado por la aplicación debe ejecutar rollback antes de devolver el error. Si una consulta SQL u operación de Model con el handle devuelve un error, Tyrus revierte la transacción y cierra el handle automáticamente; las operaciones posteriores con ese handle devuelven `ERR_DB_INVALID_TRANSACTION`. Si se abandona una transacción sin ninguna de estas operaciones, el runtime la revierte cuando se cierra el servicio de base de datos; no uses esto como flujo normal.

El handle es opaco y debe devolverse sin cambios. No es un ID de fila ni un token de seguridad, y las aplicaciones no deben exponerlo a clientes.

## Ejemplo con Model

Pasa `{ transaction }` como último argumento de opciones para una mutación. Las lecturas pueden usar el mismo handle en el objeto de opciones existente.

<!-- @include: ../../../.fragments/modules/db/transactions/example.md -->
