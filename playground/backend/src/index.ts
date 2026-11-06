import Fastify from "fastify";
import cors from "@fastify/cors";
import mongodb from "@fastify/mongodb";
import rateLimit from "@fastify/rate-limit";
import helmet from "@fastify/helmet";
import { playgroundRoutes, verifySandbox } from "./routes.js";

try {
  process.loadEnvFile();
} catch (err) {
  if ((err as NodeJS.ErrnoException).code !== "ENOENT") throw err;
}

const app = Fastify({
  logger: true,
  trustProxy: "127.0.0.1",
  // Requests carry at most 8 KB of code plus a little JSON framing. Fastify's
  // 1 MB default meant a megabyte was parsed and buffered before the route
  // could reject it.
  bodyLimit: 64 * 1024,
});

await app.register(helmet, {
  contentSecurityPolicy: {
    directives: {
      defaultSrc: ["'self'"],
      scriptSrc: ["'self'"],
      styleSrc: ["'self'", "'unsafe-inline'"],
      imgSrc: ["'self'", "data:"],
      fontSrc: ["'self'"],
      connectSrc: ["'self'"],
      workerSrc: ["'self'", "blob:"],
      frameSrc: ["'none'"],
      objectSrc: ["'none'"],
      baseUri: ["'self'"],
      formAction: ["'none'"],
    },
  },
  referrerPolicy: { policy: "no-referrer" },
});

// The Vite dev server runs on a different origin than the API, so it needs to
// be allowed, but only while developing. Shipping it in the production list
// let any page served from a developer's own machine call the live API.
const allowedOrigins = ["https://playground.tyruslang.org"];
if (process.env.NODE_ENV !== "production") {
  allowedOrigins.push("http://localhost:3000");
}

await app.register(cors, { origin: allowedOrigins });

await app.register(rateLimit, {
  global: false,
});

await app.register(mongodb, {
  url: process.env.MONGO_URI || "mongodb://localhost:27017",
  database: process.env.DB_NAME || "tyrus_playground",
});

app.addHook("onReady", async () => {
  const db = app.mongo.db!;
  const collection = db.collection("playgrounds");

  await collection.createIndex({ shortId: 1 }, { unique: true });
  await collection.createIndex({ expiresAt: 1 }, { expireAfterSeconds: 0 });

  app.log.info("MongoDB indexes created");
});

// Refuse to serve production traffic without a working sandbox.
//
// The backend degrades to ulimit-only isolation when nsjail is missing, which
// is right for development and silent everywhere else: compilation keeps
// working, so nothing looks wrong while untrusted code is being compiled with
// no isolation. A deployment that cannot jail should fail to start, loudly,
// rather than serve in that state.
//
// The check runs a real jail and asks it to prove containment, see
// `verifySandbox`. A missing binary, an undelegated cgroup and a jail whose
// root turns out to be writable are all caught here.
const sandbox = await verifySandbox();

if (!sandbox.usable) {
  const message =
    `sandbox is not usable: ${sandbox.reason}. ` +
    "Compilation would fall back to ulimit-only isolation.";

  if (process.env.NODE_ENV === "production") {
    app.log.fatal(
      { nsjailBin: process.env.NSJAIL_BIN ?? "/usr/bin/nsjail" },
      `${message} Refusing to start. See playground/README.md, "Sandbox (production)".`
    );
    process.exit(1);
  }

  app.log.warn(`${message} Acceptable for local development only.`);
} else {
  app.log.info("sandbox verified: jail starts, drops privileges and contains writes");
}

await app.register(playgroundRoutes, { prefix: "/api" });

const port = parseInt(process.env.PORT || "3001", 10);
const host = process.env.HOST || "127.0.0.1";

try {
  await app.listen({ port, host });
  app.log.info(`Playground backend listening on ${host}:${port}`);
} catch (err) {
  app.log.error(err);
  process.exit(1);
}
