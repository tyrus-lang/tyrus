---
title: Seguridad SQL
description: Vinculación de parámetros y superficie SQL admitida por tyrus:db.
---

# Seguridad SQL

`db.query<T>` para `SELECT` y `db.query` para mutaciones reciben literales de cadena SQL y un literal de array separado para los valores. Cada marcador `?` se vincula como valor mediante el controlador relacional configurado; los valores de los parámetros nunca se concatenan al texto SQL.

Los marcadores no representan nombres de tablas, nombres de columnas, operadores ni fragmentos SQL. Estas API rechazan las expresiones SQL dinámicas y la concatenación. Si la aplicación necesita un conjunto pequeño de formas de consulta, elige entre alternativas completas de SQL literal en el flujo del programa.

El compilador valida las sentencias admitidas con el snapshot local del schema. `db.query` admite un único `INSERT`, `UPDATE` o `DELETE` de una tabla; `UPDATE` y `DELETE` requieren `WHERE`. `db.query<T>` admite el subconjunto documentado de `SELECT`, incluidos los [joins internos y externos](./joins.md) con comparaciones de igualdad en `ON`. `CROSS JOIN` todavía no se admite. Califica las columnas con el nombre o alias de la tabla cuando más de una tabla unida contiene ese nombre; las referencias ambiguas se rechazan.

[`GROUP BY`](./aggregates.md#group-by) admite columnas de agrupación explícitas y valida cada proyección normal respecto a ellas. `HAVING` se admite para filtrar grupos después de la agregación y puede referirse a columnas agrupadas o alias de agregación. Consulta el [ejemplo de GROUP BY](./aggregates.md#group-by), que usa un parámetro enlazado en `HAVING`.

En `SELECT *`, la generación de código sustituye `*` por los campos escapados de la interfaz de resultado solicitada, para que el backend relacional configurado devuelva solo las columnas que necesita el decodificador. Con joins, cada campo solicitado debe corresponder a una única columna sin ambigüedad; de lo contrario, selecciona las columnas explícitamente y usa alias. Las proyecciones explícitas pueden usar alias, pero sus nombres de salida deben coincidir de forma única y exacta con los campos de la interfaz de resultado. Las columnas desconocidas, los campos ausentes o adicionales, los alias ambiguos, las expresiones no admitidas y las cantidades o tipos de parámetros incompatibles se rechazan durante la validación.

Estas restricciones y los parámetros vinculados impiden que los valores de consulta alteren la estructura SQL mediante las API de `tyrus:db`. Valida también la autorización y las reglas de negocio en el código de la aplicación; la validación del schema no determina si quien llama puede acceder a una fila.

Usa [`tyrus db explain-sql`](/es/tooling/cli#comandos-del-schema-de-la-base) con SQL enviado por stdin para inspeccionar el SQL normalizado y los tipos de parámetros inferidos. El comando valida con el snapshot local sin abrir la base, oculta los literales de cadena y numéricos y nunca muestra los valores vinculados.
