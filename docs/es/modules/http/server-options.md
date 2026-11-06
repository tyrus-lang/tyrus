---
title: ServerOptions
description: Configura la asociación, los límites, el cierre, el logging y TLS del servidor HTTP.
---

# ServerOptions

`ServerOptions` configura el listener que se pasa a `app.listen(...)`. Pase las opciones como objeto, directamente en la llamada o mediante un valor tipado como `ServerOptions`.

<SignatureBlock label="Firma">interface ServerOptions</SignatureBlock>

## Opciones

| Opción | Finalidad |
|---------|---------|
| `host` | Dirección IP a la que asociarse. De forma predeterminada, todas las interfaces. |
| `requestTimeout` | Tiempo máximo de una petición, incluido el cuerpo y el handler. Las peticiones que exceden el límite reciben `503`. |
| `maxConcurrentRequests` | Número máximo de peticiones ejecutándose a la vez. Las demás esperan un permiso y siguen limitadas por `requestTimeout`. |
| `maxConnections` | Número máximo de conexiones abiertas. No tiene límite por defecto; al alcanzar el valor, el servidor deja de aceptar conexiones. |
| `shutdownTimeout` | Tiempo máximo para esperar las peticiones activas durante el cierre. |
| `logger` | Con `true`, escribe un evento estructurado de acceso por respuesta completada en stderr; el valor predeterminado es `false`. |
| `headerTimeout` | Tiempo máximo para recibir las cabeceras de la petición. El valor predeterminado es 10 segundos y no se puede desactivar. |
| `maxHeaderSize` | Tamaño máximo combinado de la línea de petición y las cabeceras. El valor predeterminado es 64 KiB y el mínimo es 8 KiB. |
| `keepAlive` | Permite que una conexión lleve varias peticiones. Está activado de forma predeterminada. |
| `tls` | Sirve HTTPS con el certificado y la clave privada configurados. Ambos son obligatorios. |

`rateLimit` se configura en el router con `app.rateLimit(...)`, no en `ServerOptions`. Los límites de cabeceras se aplican antes de ejecutar una ruta, mientras que `requestTimeout` cubre la petición después de recibir las cabeceras.

## Ejemplo

<!-- @include: ../../../.fragments/modules/http/server-options-example.md -->
