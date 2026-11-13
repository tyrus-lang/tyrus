---
title: Documentación en Español
description: Documentación del lenguaje Tyrus en español.
layout: home
hero:
  text: '<span class="hero-line">Sintaxis TypeScript</span><span class="hero-line">Semántica Rust</span><span class="hero-subtitle">Un lenguaje de programación hecho para devs</span>'
  image:
    src: /tyrus-logo.png
    alt: Logotipo de Tyrus
  actions:
    - theme: brand
      text: Getting Started
      link: /es/docs/getting-started
    - theme: alt
      text: Playground
      link: https://playground.tyruslang.org
    - theme: alt
      text: Explore Modules
      link: /es/modules/
features:
  - title: Binarios nativos
    details: Compilación nativa de archivos .tyr con LLVM y runtime integrado en el ejecutable.
  - title: Seguridad por default
    details: Ownership, borrowing, Non-Lexical Lifetimes, límites numéricos, validadores y exhaustividad antes de producción.
  - title: Tooling enterprise para APIs
    details: Scaffolding, formatter, linter, LSP, REPL, web playground, entornos sellados y 20 módulos nativos juntos.
---

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Lenguaje backend compilado</p>
<h2>Escribe código familiar en .tyr</h2>
<p>Tyrus usa sintaxis concisa en archivos <code>.tyr</code> y agrega ejecución nativa, semántica de memoria determinística con NLL, diagnósticos precisos y tooling enfocado en backend.</p>

</div>
<div>

<!-- @include: ../.fragments/home/http-controller.md -->

</div>
</section>

<section class="tyrus-home-section">
<p class="tyrus-home-eyebrow">Superpoderes del lenguaje</p>
<div class="tyrus-home-glow-grid">
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12l2 2l4-4"/></g></svg></template>
<template #title>Validator decorations</template>
<template #description>Reglas integradas como @email, @minLength y @min, más validaciones personalizadas con validator function. Validación declarativa sin boilerplate en runtime.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 2L3 14h9l-1 8l10-12h-9l1-8z"/></svg></template>
<template #title>Comptime en compilación</template>
<template #description>Ejecuta lógica pura durante la compilación con comptime(...). Precalcula tablas de búsqueda, constantes y configuraciones sin costo en runtime.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></g></svg></template>
<template #title>Env sellado y espejo</template>
<template #description>Cifra .env en target/&lt;profile&gt;/.env.sealed con huellas digitales binarias. El espejo .env.example se sincroniza automáticamente con cada cambio.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0a2.34 2.34 0 0 0 3.319 1.915a2.34 2.34 0 0 1 2.33 4.033a2.34 2.34 0 0 0 0 3.831a2.34 2.34 0 0 1-2.33 4.033a2.34 2.34 0 0 0-3.319 1.915a2.34 2.34 0 0 1-4.659 0a2.34 2.34 0 0 0-3.32-1.915a2.34 2.34 0 0 1-2.33-4.033a2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915"/><circle cx="12" cy="12" r="3"/></g></svg></template>
<template #title>Tipos</template>
<template #description>Enteros y floats u8..u128, i8..i128, f32..f128, decimal, boolean, string, char y tipos Result&lt;T, E&gt; nativos.</template>
</GlowCard>
</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Seguridad determinística de memoria</p>
<h2>Ownership, borrowing y NLL.</h2>
<p>Seguridad de memoria sin garbage collector. Los tipos que no son Copy se mueven por asignación, mientras que los préstamos se validan en compilación. Con Non-Lexical Lifetimes (NLL), los préstamos expiran después de su último uso real, liberando variables para mutación inmediata sin esperar el final del bloque.</p>
</div>
<div>

<!-- @include: ../.fragments/home/ownership-nll.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Limpieza por ámbito</p>
<h2>Ejecuta la limpieza siempre o solo ante errores.</h2>
<p><code>defer</code> se ejecuta al salir del ámbito. <code>errdefer</code> se ejecuta solo cuando un retorno opcional o <code>Result</code> sale con error, haciendo explícita la reversión.</p>
<p><a href="/es/language/control-flow#defer-and-errdefer">Más información sobre la limpieza diferida</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/defer-errdefer.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Pattern matching exhaustivo</p>
<h2>Cualquier forma. Cada rama estrechada.</h2>
<p>Unions marcadas, variantes de enums algebraicos y <code>match</code> con guard clauses dan flujo de control exhaustivo. El compilador garantiza que cada variante se maneja y estrecha los tipos en cada brazo.</p>
</div>
<div>

<!-- @include: ../.fragments/home/pattern-matching.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Sin null. Sin undefined. Solo <code>T?</code></p>
<h2>La ausencia es un estado, no un valor.</h2>
<p>Un <code>return;</code> vacío produce ausencia. <code>?.</code> y <code>??</code> la propagan de forma segura. El compilador te obliga a manejar el caso ausente antes de que el valor llegue a producción, sin sorpresas en runtime.</p>
</div>
<div>

<!-- @include: ../.fragments/home/optionals.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Éxito y fallo explícitos</p>
<h2>Representa errores recuperables con <code>Result</code>.</h2>
<p><code>Result&lt;T, E&gt;</code> contiene un valor <code>Ok</code> o un valor <code>Err</code>. Combina ambas variantes o propaga el error con <code>?</code>.</p>
<p><a href="/es/language/types#result-values">Más información sobre Result</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/result.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Gestión de errores en la llamada</p>
<h2>Mantén claro el camino exitoso con <code>onfail</code>.</h2>
<p>Adjunta un handler de error al inicializador de un <code>Result</code>. El handler recibe el error, mientras que la variable contiene el valor exitoso si la ejecución continúa.</p>
<p><a href="/es/language/types#inline-failure-handling">Más información sobre onfail</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/onfail.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Operador pipe</p>
<h2>Los datos fluyen de arriba a abajo.</h2>
<p>El operador <code>|&gt;</code> enhebra valores a través de cadenas de funciones. El token <code>%</code> permite colocar el valor pipeado exactamente donde debe estar, sin variables intermedias, sin anidamiento.</p>
</div>
<div>

<!-- @include: ../.fragments/home/pipe-operator.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Código asíncrono nativo</p>
<h2>Escribe funciones async y espera sus resultados.</h2>
<p>Las funciones <code>async</code> devuelven <code>Promise&lt;T&gt;</code>. Usa <code>await</code> para obtener el valor cuando termine la operación asíncrona.</p>
<p><a href="/es/language/async-await">Más información sobre async y await</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/async-await.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Kernels numéricos de alto rendimiento</p>
<h2>Métodos algebraicos de punto flotante.</h2>
<p>Los primitivos <code>f32</code> e <code>f64</code> proporcionan <code>algebraicAdd</code>, <code>algebraicSub</code>, <code>algebraicMul</code>, <code>algebraicDiv</code> y <code>algebraicRem</code>. Otorgan permiso explícito al compilador para reasociar cálculos de punto flotante para vectorización SIMD y aceleración matemática en hardware.</p>
</div>
<div>

<!-- @include: ../.fragments/home/algebraic-float.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Protección de secretos en producción</p>
<h2>Env sellado y .env.example sincronizado.</h2>
<p>La configuración <code>security.envSealed</code> cifra <code>.env</code> en <code>target/&lt;profile&gt;/.env.sealed</code> con claves asimétricas e inserta una huella digital en el binario para que los contenedores nunca expongan secretos en texto plano. En desarrollo, el compilador refleja claves y comentarios en <code>.env.example</code> con valores limpios automáticamente.</p>
</div>
<div>

<!-- @include: ../.fragments/home/sealed-env.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">ORM y ODM tipados</p>
<h2>Usa una API de Model con SQL y MongoDB.</h2>
<p>Define modelos como interfaces tipadas y consulta de forma asíncrona. La misma API de Model funciona con bases de datos relacionales y MongoDB, y devuelve los errores recuperables en <code>Result</code>.</p>
<p><a href="/es/modules/db/">Explora el módulo de bases de datos</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/orm-odm.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Evaluación en tiempo de compilación</p>
<h2>Ejecuta código durante la compilación. Costo cero en runtime.</h2>
<p>Las funciones declaradas con <code>comptime</code> se ejecutan durante la compilación e incorporan su resultado como escalares o literales estáticos directamente en el binario. Calcula tablas de búsqueda, constantes matemáticas y configuraciones con seguridad de tipos completa.</p>
</div>
<div>

<!-- @include: ../.fragments/home/comptime.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Exploración interactiva</p>
<h2>Pruébalo en la terminal. Ejecútalo en el navegador.</h2>
<p>Explora Tyrus al instante sin configurar proyectos. <code>tyrus repl</code> proporciona una consola interactiva con evaluación JIT a través de LLVM en proceso, edición multilínea, persistencia de sesiones e inspección con <code>:type</code>. ¿Prefieres el navegador? Abre el <strong>Web Playground</strong> en <a href="https://playground.tyruslang.org" target="_blank" rel="noreferrer">playground.tyruslang.org</a> para escribir, compilar a WebAssembly y ejecutar código directamente en el navegador con enlaces compartibles.</p>
</div>
<div>

<!-- @include: ../.fragments/home/repl-session.md -->

</div>
</section>

<section class="tyrus-home-section">
<p class="tyrus-home-eyebrow">Una toolchain de lenguaje, no solo un parser</p>
<div class="tyrus-home-glow-grid">
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m4 7l8-4l8 4v10l-8 4l-8-4zm8 14V11m8-4l-8 4l-8-4"/></svg></template>
<template #title>REPL interactivo y CLI</template>
<template #description>tyrus repl para evaluación JIT a través de LLVM en proceso, tyrus dev para live reload y tyrus fmt para formateo sin configuración.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m21.73 18l-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3M12 9v4m0 4h.01"/></svg></template>
<template #title>Diagnósticos y tyrus explain</template>
<template #description>Códigos de error estables con tyrus explain, rangos precisos, mensajes de ayuda, ejemplos de fallo y Quick Fixes en el editor.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497zM15 5l4 4"/></svg></template>
<template #title>Editores con soporte de LSP</template>
<template #description>Completion, hover, signature help, inlay hints, semantic tokens y code actions para VS Code, Neovim, Zed, IntelliJ y Kate.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 2v6a2 2 0 0 0 .245.96l5.51 10.08A2 2 0 0 1 18 22H6a2 2 0 0 1-1.755-2.96l5.51-10.08A2 2 0 0 0 10 8V2M6.453 15h11.094M8.5 2h7"/></svg></template>
<template #title>Web Playground y tests</template>
<template #description>Ejecuta en el navegador mediante WebAssembly en playground.tyruslang.org y corre suites locales con tyrus test para archivos *.test.tyr.</template>
</GlowCard>
</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Formato del código</p>
<h2>Tyrus Formatter</h2>
<p><code>tyrus fmt</code> formatea archivos `.tyr` de forma consistente y aplica las opciones de <code>tyrus-fmt.json</code> del proyecto.</p>
<p><a href="/es/tooling/formatter">Lee la guía del Formatter</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/formatter-config.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Análisis de código</p>
<h2>Tyrus Lint</h2>
<p><code>tyrus lint</code> informa problemas de estilo, corrección, seguridad, rendimiento y arquitectura. Configura la severidad de cada regla en <code>tyrus-lint.json</code> del proyecto.</p>
<p><a href="/es/tooling/linter">Lee la guía de Lint</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/lint-config.md -->

</div>
</section>

<section class="tyrus-home-section">
<p class="tyrus-home-eyebrow">Baterías incluidas</p>
<h2>Módulos nativos integrados.</h2>
<p>Todo lo necesario para construir servicios backend resilientes, herramientas de CLI y software de sistemas ya forma parte del runtime de Tyrus.</p>
<div class="tyrus-home-grid">
<a class="tyrus-home-card" href="/modules/algorithm/">
<strong>tyrus:algorithm</strong>
<span>Más de 50 algoritmos y estructuras de datos integrados: Dijkstra, A*, Trie y árbol rojo-negro.</span>
</a>
<a class="tyrus-home-card" href="/modules/archive/">
<strong>tyrus:archive</strong>
<span>Creación, inspección y extracción de archivos Tar y Zip.</span>
</a>
<a class="tyrus-home-card" href="/modules/cli/">
<strong>tyrus:cli</strong>
<span>Prompts interactivos en la terminal: select, multiselect, password, confirm e input.</span>
</a>
<a class="tyrus-home-card" href="/modules/compress/">
<strong>tyrus:compress</strong>
<span>Compresión y descompresión con Zstandard, Brotli, Gzip y Deflate.</span>
</a>
<a class="tyrus-home-card" href="/modules/crypto/">
<strong>tyrus:crypto</strong>
<span>Argon2, SHA-256, SHA-512, AES-GCM, HMAC, JWT, UUID v7 y CSPRNG.</span>
</a>
<a class="tyrus-home-card" href="/modules/db/">
<strong>tyrus:db</strong>
<span>Cliente de base de datos y ORM integrados para PostgreSQL, MySQL, SQLite y MongoDB.</span>
</a>
<a class="tyrus-home-card" href="/modules/encoding/">
<strong>tyrus:encoding</strong>
<span>Codificadores nativos para Hex, Base64, URL, Varint, CSV y JSON.</span>
</a>
<a class="tyrus-home-card" href="/modules/fs/">
<strong>tyrus:fs</strong>
<span>Sistema de archivos síncrono y asíncrono, escritura atómica, streaming, glob y vigilancia de archivos.</span>
</a>
<a class="tyrus-home-card" href="/modules/http/">
<strong>tyrus:http</strong>
<span>Servidor HTTP/1.1 y HTTP/2, rutas automáticas, Swagger y WebSocket.</span>
</a>
<a class="tyrus-home-card" href="/modules/math/">
<strong>tyrus:math</strong>
<span>Constantes, trigonometría, teoría de números, geometría y combinatoria.</span>
</a>
<a class="tyrus-home-card" href="/modules/memory/">
<strong>tyrus:memory</strong>
<span>Operaciones deterministas de memoria: sizeOf, alignOf, zeroize, swap y replace.</span>
</a>
<a class="tyrus-home-card" href="/modules/net/">
<strong>tyrus:net</strong>
<span>Sockets TCP y UDP, resolución DNS, búsqueda inversa, ping y traceroute.</span>
</a>
<a class="tyrus-home-card" href="/modules/os/">
<strong>tyrus:os</strong>
<span>Metadatos del sistema, núcleos de CPU, estadísticas de memoria, temperaturas y notificaciones nativas.</span>
</a>
<a class="tyrus-home-card" href="/modules/process/">
<strong>tyrus:process</strong>
<span>Procesos secundarios, ejecución de comandos, señales y supervisión de recursos.</span>
</a>
<a class="tyrus-home-card" href="/modules/simd/">
<strong>tyrus:simd</strong>
<span>Intrínsecos portables de matemática vectorial para operaciones numéricas de alto rendimiento.</span>
</a>
<a class="tyrus-home-card" href="/modules/stream/">
<strong>tyrus:stream</strong>
<span>Primitivas componibles de streaming binario y de texto para I/O de alto rendimiento.</span>
</a>
<a class="tyrus-home-card" href="/modules/task/">
<strong>tyrus:task</strong>
<span>Runtime asíncrono, ejecución en múltiples hilos, canales, mutex, rwlock y cancelación.</span>
</a>
<a class="tyrus-home-card" href="/modules/temporal/">
<strong>tyrus:temporal</strong>
<span>Gestión moderna de fechas, horas, duraciones y zonas horarias con sintaxis ergonómica.</span>
</a>
<a class="tyrus-home-card" href="/modules/test/">
<strong>tyrus:test</strong>
<span>Ejecutor de pruebas integrado con describe, it y expect para archivos *.test.tyr.</span>
</a>
<a class="tyrus-home-card" href="/modules/utils/">
<strong>tyrus:utils</strong>
<span>Estilización de texto en la terminal con styleText, colores ANSI y banners ASCII FIGlet.</span>
</a>
</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Feedback de seguridad mientras escribes</p>
<h2>Los errores forman parte de la experiencia del producto.</h2>
<p>Los diagnósticos del compilador y del linter pasan por el mismo análisis usado por la CLI y el Language Server, así que terminal y editor coinciden sobre qué está mal y cómo corregirlo.</p>
</div>
<div>

::: danger error
<!-- @include: ../.fragments/home/type-error.md -->
:::

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Empieza con un comando</p>

<!-- @include: ../.fragments/home/new-command.md -->

</div>
<div>
<p class="tyrus-home-eyebrow">Ejecuta con un comando</p>

<!-- @include: ../.fragments/home/run-command.md -->

</div>
</section>

<section class="tyrus-home-section">
<p class="tyrus-home-eyebrow">Por qué este nombre</p>
<h2>Una roca y un dios.</h2>
<p>Tyrus es la grafia en latín de Tiro, la ciudad fenicia cuyo nombre significa "roca". Tyr es el dios de la guerra, la justicia y el coraje en la mitología nórdica.</p>
<p>Pero el nombre es la unión de las iniciales de <span class="tyrus-home-strong">Ty</span>peScript y <span class="tyrus-home-strong">Rus</span>t.</p>
</section>
