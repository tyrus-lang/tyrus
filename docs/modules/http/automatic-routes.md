---
title: Automatic Route Registration
description: Discover and register route plugins from `src/routes/`.
---

# Automatic Route Registration

In an API project, the Tyrus entrypoint automatically discovers route modules recursively under `src/routes/`. Each route module can export one or more functions with the signature `(router: HttpRouter) => void`; Tyrus calls each function with the application router when `createRouter()` runs, before the server listens. A default export remains supported. No `index.tyr` or per-route imports are needed.

<!-- @include: ../../.fragments/modules/http/automatic-routes.md -->

## Discovery rules

A `.tyr` file in `src/routes/` without a matching exported route function is treated as a helper module and is not invoked. A default export in that directory must match the route plugin signature or the project fails to load. Files ending in `.test.tyr` are excluded from discovery.

Exports outside `src/routes/` are never registered automatically. Import and invoke those modules explicitly when needed.
