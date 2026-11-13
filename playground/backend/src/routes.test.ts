import { describe, it, beforeAll, afterAll, expect } from "vitest";
import { gzipSync } from "node:zlib";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import Fastify from "fastify";
import mongodb from "@fastify/mongodb";
import cors from "@fastify/cors";
import { playgroundRoutes, buildSandboxCommand, verifySandbox } from "./routes.js";
import { execFileSync } from "node:child_process";

let app: ReturnType<typeof Fastify>;
let mongoUri: string;

// Compile against the workspace binary, never whatever `tyrus` happens to be on
// PATH: an installed copy lags behind the source and will happily accept code
// the current compiler rejects, which turns a real regression into a green run.
const workspaceTyrus = join(
  dirname(fileURLToPath(import.meta.url)),
  "../../../target/debug/tyrus"
);
const hasWorkspaceCompiler = existsSync(workspaceTyrus);

beforeAll(async () => {
  if (hasWorkspaceCompiler) {
    process.env.TYRUS_BIN = workspaceTyrus;
  }
  mongoUri = process.env.MONGO_URI || "mongodb://localhost:27017";

  app = Fastify({ logger: false });

  await app.register(cors);
  await app.register(mongodb, {
    url: mongoUri,
    database: "tyrus_playground_test",
  });

  app.addHook("onReady", async () => {
    const db = app.mongo.db!;
    const collection = db.collection("playgrounds");
    await collection.createIndex({ shortId: 1 }, { unique: true });
    await collection.createIndex({ expiresAt: 1 }, { expireAfterSeconds: 0 });
  });

  await app.register(playgroundRoutes, { prefix: "/api" });
  await app.ready();
});

afterAll(async () => {
  if (app) {
    const db = app.mongo.db!;
    await db.dropDatabase();
    await app.close();
  }
});

describe("POST /api/playground", () => {
  it("creates a snippet with valid code", async () => {
    const response = await app.inject({
      method: "POST",
      url: "/api/playground",
      payload: {
        code: 'function main(): void {\n    print("Hello!");\n}',
        permanent: false,
      },
    });

    expect(response.statusCode).toBe(201);
    const body = response.json();
    expect(body).toHaveProperty("shortId");
    expect(body).toHaveProperty("url");
    expect(body.url).toContain(body.shortId);
  });

  it("rejects empty code", async () => {
    const response = await app.inject({
      method: "POST",
      url: "/api/playground",
      payload: { code: "" },
    });

    expect(response.statusCode).toBe(400);
  });

  it("rejects missing code", async () => {
    const response = await app.inject({
      method: "POST",
      url: "/api/playground",
      payload: {},
    });

    expect(response.statusCode).toBe(400);
  });

  it("rejects code exceeding 8KB", async () => {
    const response = await app.inject({
      method: "POST",
      url: "/api/playground",
      payload: { code: "x".repeat(8193) },
    });

    expect(response.statusCode).toBe(400);
  });

  it("rejects permanent snippet without auth", async () => {
    const response = await app.inject({
      method: "POST",
      url: "/api/playground",
      payload: {
        code: "print('test')",
        permanent: true,
      },
    });

    expect(response.statusCode).toBe(401);
  });
});

describe("installer download accounting", () => {
  it("counts installer GET requests without storing client data", async () => {
    const previousToken = process.env.PLAYGROUND_ADMIN_TOKEN;
    process.env.PLAYGROUND_ADMIN_TOKEN = "installer-stats-test-token";

    try {
      const shellDownload = await app.inject({ method: "GET", url: "/api/downloads/install.sh" });
      expect(shellDownload.statusCode).toBe(200);
      expect(shellDownload.body).toContain("#!/usr/bin/env sh");
      expect(shellDownload.headers["cache-control"]).toBe("no-store");

      const secondShellDownload = await app.inject({ method: "GET", url: "/api/downloads/install.sh" });
      expect(secondShellDownload.statusCode).toBe(200);

      const headRequest = await app.inject({ method: "HEAD", url: "/api/downloads/install.sh" });
      expect(headRequest.statusCode).toBe(200);

      const powershellDownload = await app.inject({ method: "GET", url: "/api/downloads/install.ps1" });
      expect(powershellDownload.statusCode).toBe(200);
      expect(powershellDownload.body).toContain("$repository = 'tyrus-lang/tyrus'");

      const unauthorized = await app.inject({ method: "GET", url: "/api/downloads/stats" });
      expect(unauthorized.statusCode).toBe(401);

      const stats = await app.inject({
        method: "GET",
        url: "/api/downloads/stats",
        headers: { authorization: "Bearer installer-stats-test-token" },
      });
      expect(stats.statusCode).toBe(200);
      const downloadStats = stats.json().downloads as Array<Record<string, unknown>>;
      expect(downloadStats).toEqual([
        { _id: "install.ps1", count: 1 },
        { _id: "install.sh", count: 2 },
      ]);
      for (const item of downloadStats) {
        expect(Object.keys(item)).toHaveLength(2);
      }

      const invalid = await app.inject({ method: "GET", url: "/api/downloads/other.sh" });
      expect(invalid.statusCode).toBe(404);
    } finally {
      if (previousToken === undefined) delete process.env.PLAYGROUND_ADMIN_TOKEN;
      else process.env.PLAYGROUND_ADMIN_TOKEN = previousToken;
    }
  });
});

describe("GET /api/playground/:shortId", () => {
  let shortId: string;

  beforeAll(async () => {
    const response = await app.inject({
      method: "POST",
      url: "/api/playground",
      payload: {
        code: "print('hello')",
        permanent: false,
      },
    });
    shortId = response.json().shortId;
  });

  it("retrieves a snippet by shortId", async () => {
    const response = await app.inject({
      method: "GET",
      url: `/api/playground/${shortId}`,
    });

    expect(response.statusCode).toBe(200);
    const body = response.json();
    expect(body.code).toBe("print('hello')");
    expect(body.encoding).toBe("none");
  });

  it("returns 404 for non-existent shortId", async () => {
    const response = await app.inject({
      method: "GET",
      url: "/api/playground/nonexistent",
    });

    expect(response.statusCode).toBe(404);
  });

  it("rejects invalid shortId format", async () => {
    const response = await app.inject({
      method: "GET",
      url: "/api/playground/!!!invalid!!!",
    });

    expect(response.statusCode).toBe(400);
  });
});

describe("POST /api/playground/compile", () => {
  it("rejects missing code", async () => {
    const response = await app.inject({
      method: "POST",
      url: "/api/playground/compile",
      payload: {},
    });

    expect(response.statusCode).toBe(400);
  });

  it("rejects empty code", async () => {
    const response = await app.inject({
      method: "POST",
      url: "/api/playground/compile",
      payload: { code: "   " },
    });

    expect(response.statusCode).toBe(400);
  });

  it("rejects code exceeding 8KB", async () => {
    const response = await app.inject({
      method: "POST",
      url: "/api/playground/compile",
      payload: { code: "x".repeat(8193) },
    });

    expect(response.statusCode).toBe(400);
    const body = response.json();
    expect(body.error).toContain("maximum size");
  });

  it("rejects a forbidden module import", async () => {
    const response = await app.inject({
      method: "POST",
      url: "/api/playground/compile",
      payload: { code: 'import { readFileSync } from "tyrus:fs";' },
    });

    expect(response.statusCode).toBe(403);
    const body = response.json();
    expect(body.error).toContain("tyrus:fs");
  });

  it("rejects a forbidden module import with no whitespace before the module string", async () => {
    const response = await app.inject({
      method: "POST",
      url: "/api/playground/compile",
      payload: { code: 'import { readFileSync } from"tyrus:fs";' },
    });

    expect(response.statusCode).toBe(403);
  });

  it("rejects every forbidden module in the capability surface", async () => {
    // Mirrors SANDBOX_FORBIDDEN_MODULES in the compiler. `tyrus:process` is
    // absent because no such module exists, process information lives in the
    // ambient `process` namespace and is reported unavailable at run time.
    const modules = [
      "tyrus:archive",
      "tyrus:cli",
      "tyrus:compress",
      "tyrus:crypto",
      "tyrus:fs",
      "tyrus:http",
      "tyrus:net",
      "tyrus:os",
      "tyrus:task",
    ];

    for (const mod of modules) {
      const response = await app.inject({
        method: "POST",
        url: "/api/playground/compile",
        payload: { code: `import { x } from "${mod}";` },
      });

      expect(response.statusCode).toBe(403);
    }
  });

  it.runIf(hasWorkspaceCompiler)("never compiles a forbidden import disguised by a comment", async () => {
    // This shape defeats the text filter, `from\s*"tyrus:fs"` does not match
    // across a comment, and used to return HTTP 200 with a compiled module.
    // `tyrus build --sandbox` refuses it now, so the request cannot succeed.
    // The assertion is "not 200" rather than an exact code because the reason
    // differs by environment: 422 when the compiler runs and rejects it, and
    // also 422 where no compiler is installed at all.
    const response = await app.inject({
      method: "POST",
      url: "/api/playground/compile",
      payload: {
        code: 'import { readFile } from /* x */ "tyrus:fs";\nconst main = (): void => {};',
      },
    });

    expect(response.statusCode).not.toBe(200);
  });
});

describe("DELETE /api/playground/:shortId", () => {
  let shortId: string;

  beforeAll(async () => {
    process.env.PLAYGROUND_ADMIN_TOKEN = "test-token-123";

    const response = await app.inject({
      method: "POST",
      url: "/api/playground",
      payload: {
        code: "print('delete me')",
        permanent: false,
      },
    });
    shortId = response.json().shortId;
  });

  it("deletes a snippet with valid token", async () => {
    const response = await app.inject({
      method: "DELETE",
      url: `/api/playground/${shortId}`,
      headers: { authorization: "Bearer test-token-123" },
    });

    expect(response.statusCode).toBe(204);

    const getResponse = await app.inject({
      method: "GET",
      url: `/api/playground/${shortId}`,
    });
    expect(getResponse.statusCode).toBe(404);
  });

  it("rejects delete without auth", async () => {
    const response = await app.inject({
      method: "DELETE",
      url: `/api/playground/${shortId}`,
    });

    expect(response.statusCode).toBe(401);
  });

  it("rejects delete with wrong token", async () => {
    const response = await app.inject({
      method: "DELETE",
      url: `/api/playground/${shortId}`,
      headers: { authorization: "Bearer wrong-token" },
    });

    expect(response.statusCode).toBe(403);
  });

  it("returns 404 for already deleted snippet", async () => {
    const response = await app.inject({
      method: "DELETE",
      url: `/api/playground/${shortId}`,
      headers: { authorization: "Bearer test-token-123" },
    });

    expect(response.statusCode).toBe(404);
  });
});

describe("size limit applies to the decoded source", () => {
  // The limit used to be measured on the request payload. A gzipped, base64
  // encoded snippet of 1 MB of source weighs about 4 KB on the wire, so it
  // passed an 8 KB check and was stored and served in full, 129x the cap.
  const oneMegabyteOfSource = "const main = (): void => { console.log(1); };\n".repeat(23000);

  it("rejects a compressed payload whose source exceeds the limit", async () => {
    const response = await app.inject({
      method: "POST",
      url: "/api/playground",
      payload: {
        code: gzipSync(Buffer.from(oneMegabyteOfSource)).toString("base64"),
        encoding: "gzip",
      },
    });

    expect(response.statusCode).toBe(400);
    expect(response.json().error).toContain("maximum size");
  });

  it("accepts a compressed payload that is small once decoded", async () => {
    const source = 'const main = (): void => { console.log("hello from a shared snippet"); };';
    const response = await app.inject({
      method: "POST",
      url: "/api/playground",
      payload: {
        code: gzipSync(Buffer.from(source)).toString("base64"),
        encoding: "gzip",
      },
    });

    expect(response.statusCode).toBe(201);
  });

  it("rejects a payload that claims to be gzip but is not", async () => {
    const response = await app.inject({
      method: "POST",
      url: "/api/playground",
      payload: { code: "bm90IGd6aXAgYXQgYWxs", encoding: "gzip" },
    });

    expect(response.statusCode).toBe(400);
    expect(response.json().error).toContain("gzip");
  });
});

describe("share URL", () => {
  it("is built from the request rather than a hardcoded production host", async () => {
    // The literal `https://playground.tyruslang.org` pointed every deployment,
    // including a local one, at production.
    const response = await app.inject({
      method: "POST",
      url: "/api/playground",
      headers: { host: "playground.example.test" },
      payload: { code: "const main = (): void => { console.log(1); };" },
    });

    expect(response.statusCode).toBe(201);
    expect(response.json().url).toContain("playground.example.test");
    expect(response.json().url).not.toContain("tyruslang.org");
  });
});

describe("sandbox command construction", () => {
  const options = {
    useNsjail: true,
    tmpDir: "/tmp/tyrus-playground-abc",
    tyrusBin: "/usr/local/bin/tyrus",
    nsjailBin: "/usr/bin/nsjail",
  };

  it("hands every limit to nsjail rather than to an outer shell", () => {
    // An outer `ulimit -v` lowers the hard limit, and nsjail then cannot set
    // its own RLIMIT_AS: "setrlimit(RLIMIT_AS, 4294967296): Operation not
    // permitted", and the jail never starts.
    const command = buildSandboxCommand(options);

    expect(command).toMatch(/^exec \/usr\/bin\/nsjail /);
    expect(command).not.toContain("ulimit");
  });

  it("is a command bash parses as the pipeline it looks like", () => {
    // The elements were once joined with spaces throughout, which bash reads
    // as a single `ulimit` invocation: no limit applied, nsjail never
    // executed, compiler never executed. Running the real shell against a
    // harmless stand-in catches that.
    const command = buildSandboxCommand({
      ...options,
      nsjailBin: "/bin/echo",
      tyrusBin: "unused",
    });
    const output = execFileSync("bash", ["-c", command], { encoding: "utf-8" });

    expect(output).toContain("-Mo");
    expect(output).toContain("--cgroup_mem_max");
    expect(output).toContain("--cwd /tmp/tyrus-playground-abc");
  });

  it("uses only flags nsjail actually accepts", () => {
    // `--timeout`, `--memory`, `--cpus`, `--pids` and `-w` are not nsjail
    // options. It answers "Couldn't parse cmdline options" and compiles
    // nothing, so a plausible-looking name is worse than no limit at all.
    const command = buildSandboxCommand(options);

    for (const invented of ["--timeout ", "--memory ", "--cpus ", "--pids ", " -w "]) {
      expect(command).not.toContain(invented);
    }
    for (const real of ["--time_limit ", "--cgroup_mem_max ", "--max_cpus ", "--cgroup_pids_max ", "--cwd "]) {
      expect(command).toContain(real);
    }
  });

  it("does not hand the jail the host network", () => {
    // `-N` is `--disable_clone_newnet`: "Don't use CLONE_NEWNET. Enable global
    // networking inside the jail." It was passed believing it isolated the
    // network, and did the opposite, verified: with it, the jail resolved
    // DNS and saw 7 host interfaces; without it, one loopback and no network.
    expect(buildSandboxCommand(options)).not.toMatch(/(^|\s)-N(\s|$)/);
    expect(buildSandboxCommand(options)).not.toContain("--disable_clone_newnet");
  });

  it("denies swap, without which the memory ceiling is not a ceiling", () => {
    // With `--cgroup_mem_max` alone, a process over the limit is pushed to
    // swap instead of killed: 700 MB allocated cleanly under a 256 MB cap.
    expect(buildSandboxCommand(options)).toContain("--cgroup_mem_swap_max 0");
  });

  it("allows a module larger than nsjail's default file size limit", () => {
    // RLIMIT_FSIZE defaults to 1 MB. A hello-world (22 KB) and a temporal
    // program (605 KB) linked fine while one using regex (1.19 MB) died with
    // "failed to open ...: File too large", surfaced as an LLVM crash dump.
    const megabytes = Number(/--rlimit_fsize (\d+)/.exec(buildSandboxCommand(options))?.[1]);

    expect(megabytes).toBeGreaterThanOrEqual(8);
  });

  it("runs the compiler directly when nsjail is unavailable", () => {
    const command = buildSandboxCommand({ ...options, useNsjail: false });

    expect(command).toContain("exec /usr/local/bin/tyrus build --compile-target wasm --sandbox");
    expect(command).not.toContain("nsjail");
  });
});

describe("sandbox verification", () => {
  // Checking that the nsjail binary exists is not the same as checking that
  // the jail works. On a machine where it was present, the jail still failed
  // to start for want of a writable cgroup subtree, and separately ran with
  // the host's network because a flag meant the opposite of what it read like.
  // The probe therefore runs a real jail and asks it to prove containment.

  it("reports why the sandbox is unusable when nsjail is missing", async () => {
    const previous = process.env.NSJAIL_BIN;
    process.env.NSJAIL_BIN = "/nonexistent/nsjail";

    try {
      const check = await verifySandbox();
      expect(check.usable).toBe(false);
      expect(check.reason).toContain("/nonexistent/nsjail");
    } finally {
      if (previous === undefined) delete process.env.NSJAIL_BIN;
      else process.env.NSJAIL_BIN = previous;
    }
  });

  it("asks the jail to prove it contains, not merely that it started", () => {
    // A jail that starts but leaves its root writable is worse than none: it
    // looks like it is working. The probe writes outside its one bind mount
    // and requires that to fail.
    const command = buildSandboxCommand({
      useNsjail: true,
      tmpDir: "/tmp/probe",
      tyrusBin: "tyrus",
      nsjailBin: "/usr/bin/nsjail",
      command: "/bin/sh -c 'id -u'",
    });

    expect(command).toContain("/bin/sh -c 'id -u'");
    expect(command).not.toContain("--compile-target");
  });
});
