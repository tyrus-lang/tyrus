---
title: API Flow
description: Follow a User request through the main, model, schemas, routes, controllers, services, and repository of a SQLite API.
---

# API Flow

This guide follows User requests through a SQLite API, from `tyrus.json` and server startup to the SQL or Model operation and back to the HTTP response. The code uses the same file layout and formatting as the `api-sqlite` project.

## Project layout

The `src/routes/` directory is discovered automatically when `createRouter()` runs. Tyrus calls each exported route function with the application router, so `main.tyr` does not need one import per route module.

## Complete example

The example exposes list, numeric pagination, search, read, create, update, replace, and delete endpoints. The Model supplies the generated `id` and timestamps; the repository uses `db.query` for SQL and Model methods for typed record operations.

<!-- @include: ../../.fragments/modules/http/api-flow/complete-example.md -->

## Request flow

- `main.tyr` creates the router, configures Swagger metadata, and starts the server with `app.listen`.
- `models/users.tyr` maps the `User` interface to the `users` table; its generated `id`, `createdAt`, and `updatedAt` do not need declarations in the interface.
- `schemas/user.tyr` defines the route inputs, response bodies, and error bodies; application errors declare `code` and `message`.
- `routes/user.tyr` declares HTTP methods, validation schemas, response schemas, and the controller for each endpoint.
- `controllers/user.tyr` handles HTTP status codes and translates `Result` success or failure into response bodies.
- `services/user.tyr` names the application operations and returns typed asynchronous results.
- `repositories/user.tyr` owns database access and returns `Result` values; it uses bound parameters for SQL and Model operations for records.

The file names and paths in the example are illustrative; keep the route modules under `src/routes/` for automatic registration. See [automatic route registration](./automatic-routes.md), [`tyrus:db`](../db/), and [OpenAPI / Swagger](./openapi-swagger.md) for the related contracts.
