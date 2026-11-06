---
title: Aliases de ruta
description: Aliases estáticos de carpetas para importaciones internas.
---

# Aliases de ruta

Declara alias estáticos para carpetas internas de código fuente en el objeto `imports` de `tyrus.json`.

<!-- @include: ../../.fragments/docs/path-aliases.md -->

Cada clave de alias comienza con `#` y asigna una carpeta relativa a la raíz del proyecto. Usa `#` para una carpeta `src/` convencional e importa sus hijos con `#/carpeta`.

Usa `#config` para importar `src/config/index.tyr`, o `#config/mongodb` para importar un módulo hijo. Un alias nunca apunta directamente a un archivo fuente, expande otro alias, sale de la raíz del proyecto ni se resuelve en runtime.

Cuando se aplica más de un alias, gana el prefijo más largo. Un specifier `#` sin resolver es un error de compilación y un diagnóstico del editor.
