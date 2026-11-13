---
title: listen
description: Inicia un servidor HTTP y gestiona errores de inicio.
---

# listen

`app.listen(port, options?)` asocia el router y devuelve `Result<HttpServer, HttpError>`. El servidor empieza a aceptar solicitudes de inmediato, y `main` puede continuar ejecutando el código de inicio. Use `onfail` para gestionar un error al enlazar el puerto o al iniciar TLS; en una función `void`, el bloque de error retorna de esa función.

<SignatureBlock label="Firma">app.listen(port: i32, options?: ServerOptions): Result&lt;HttpServer, HttpError&gt;</SignatureBlock>

## Mostrar el puerto enlazado

`listen` devuelve el handle del servidor enlazado cuando tiene éxito, así que `server.port()` informa del puerto real, incluso cuando el puerto solicitado era `0`. El runtime mantiene el listener activo después de que `main` retorna y espera una señal de apagado del sistema operativo; llame a `server.close()` cuando la aplicación deba detenerlo antes.

<!-- @include: ../../../.fragments/modules/http/listen-example.md -->

## Gestionar un error de inicio

La variable de `onfail` contiene `HttpError`, con un `code` estable y un `message` legible. El error se gestiona sin lanzar una excepción; el bloque retorna de esta función `void` después de informarlo.

<!-- @include: ../../../.fragments/modules/http/listen-catch.md -->

## Opciones

Pase un objeto `ServerOptions` como segundo argumento de `listen`. Consulte [ServerOptions](./server-options.md) para ver las opciones disponibles.
