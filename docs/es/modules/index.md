---
title: Módulos nativos
description: APIs de runtime expuestas mediante rutas de import estables tyrus:*.
---

# Módulos nativos

Los módulos nativos exponen funcionalidades de runtime mediante rutas de import estables `tyrus:*`. Sus API son concisas y sus implementaciones se enlazan en binarios nativos. Cada módulo forma parte de la distribución estándar, sin gestor de paquetes, sin npm install, sin runtime externo.

## Módulos disponibles

| Módulo | Finalidad |
|--------|-----------|
| [`tyrus:algorithm`](./algorithm/) | Estructuras de datos y algoritmos nativos para ordenación, búsqueda, grafos, árboles y colecciones. |
| [`tyrus:archive`](./archive/) | Crea, inspecciona y extrae de forma segura archivos ZIP, TAR y TAR.GZ. |
| [`tyrus:cli`](./cli/) | Helpers interactivos de prompt de terminal. |
| [`tyrus:compress`](./compress/) | Comprime y descomprime bytes con Gzip, Deflate, Deflate crudo y Brotli. |
| [`tyrus:crypto`](./crypto/) | Primitivas criptográficas: digest, HMAC, AES-256-GCM, Argon2, JWT y aleatorio seguro. |
| [`tyrus:db`](./db/) | Snapshots versionados de schema SQLite, comprobaciones de solo lectura y contrato de errores de la base. |
| [`tyrus:encoding`](./encoding/) | Conversión explícita de UTF-8, UTF-16, hexadecimal, Base64, Base32 y radix. |
| [`tyrus:fs`](./fs/) | Lectura y escritura síncrona de archivos para programas nativos. |
| [`tyrus:http`](./http/) | Servidor HTTP, rutas, respuesta, status y helpers orientados a API. |
| [`tyrus:math`](./math/) | Constantes y funciones matemáticas. |
| [`tyrus:memory`](./memory/) | Ownership explícito, borrado seguro y consultas de layout en tiempo de compilación. |
| [`tyrus:net`](./net/) | Parsing de direcciones, resolución de nombres con `Result`, comunicación TCP y UDP asíncrona y sondas condicionadas por capacidad. |
| [`tyrus:os`](./os/) | Identificación multiplataforma del sistema operativo: plataforma, arquitectura, hostname, locale. |
| [`tyrus:process`](./process/) | Subprocesos sin shell con streams estándar de bytes limitados. |
| [`tyrus:simd`](./simd/) | Suma vectorizada de arrays `f32[]` y `f64[]` del mismo tamaño, con errores recuperables en `Result`. |
| [`tyrus:stream`](./stream/) | Transporte progresivo limitado con capacidades separadas de lectura y escritura. |
| [`tyrus:task`](./task/) | Gestión cooperativa de tareas async: cancelación y suspensión temporizada. |
| [`tyrus:temporal`](./temporal/) | Instantes nativos, valores de calendario, duraciones y zonas horarias IANA. |
| [`tyrus:test`](./test/) | Suites de prueba, aserciones, hooks y modificadores ejecutados por `tyrus test`. |
| [`tyrus:utils`](./utils/) | Helpers nativos de texto como estilo ANSI y banners FIGlet. |

## Estilo de import

<!-- @include: ../../.fragments/modules/import-style.md -->

Los imports `tyrus:*` son resueltos por el compilador como módulos integrados.

## Usando módulos juntos

Los módulos nativos componen de forma natural. Un proyecto CLI típico podría leer configuración del sistema de archivos, solicitar entrada y aplicar estilos:

<!-- @include: ../../.fragments/modules/using-modules-together.md -->

## Frontera de runtime

Los módulos nativos deben documentar imports públicos, tipos de retorno, comportamiento de error y diagnósticos antes de tratarse como estables. Las APIs que imprimen UI chrome de terminal usan stderr para mantener stdout reservado a la salida normal del programa.
