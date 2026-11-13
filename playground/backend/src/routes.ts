import type { FastifyInstance, FastifyBaseLogger } from "fastify";
import { randomBytes, createHash, timingSafeEqual } from "node:crypto";
import { execFile } from "node:child_process";
import { writeFile, readFile, mkdtemp, rm, readdir, mkdir, access } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createGunzip } from "node:zlib";
import { getCollection, type Playground, type CreatePlaygroundRequest } from "./models.js";

const CACHE_MAX_ENTRIES = 50;
// A module's size depends on what the program uses: a `console.log` compiles to
// ~22 KB, one pulling in regex or the timezone database to well over a
// megabyte. Counting entries alone therefore says little about memory held, so
// the cache is bounded by both.
const CACHE_MAX_BYTES = 64 * 1024 * 1024;
const compileCache = new Map<string, Buffer>();
let compileCacheBytes = 0;

function cacheGet(codeHash: string): Buffer | undefined {
  const entry = compileCache.get(codeHash);
  if (entry) {
    compileCache.delete(codeHash);
    compileCache.set(codeHash, entry);
  }
  return entry;
}

function cacheEvictOldest(): void {
  const oldest = compileCache.keys().next();
  if (oldest.done) return;
  compileCacheBytes -= compileCache.get(oldest.value)?.byteLength ?? 0;
  compileCache.delete(oldest.value);
}

function cacheSet(codeHash: string, wasm: Buffer): void {
  // A single module larger than the whole budget would evict everything and
  // still not fit, so it is simply not cached.
  if (wasm.byteLength > CACHE_MAX_BYTES) return;

  while (
    compileCache.size >= CACHE_MAX_ENTRIES ||
    compileCacheBytes + wasm.byteLength > CACHE_MAX_BYTES
  ) {
    cacheEvictOldest();
  }

  compileCache.set(codeHash, wasm);
  compileCacheBytes += wasm.byteLength;
}

function hashcode(code: string): string {
  return createHash("sha256").update(code).digest("hex");
}

function generateShortId(): string {
  return randomBytes(6)
    .toString("base64url")
    .slice(0, 8);
}

function validateShortId(shortId: string): boolean {
  return /^[a-zA-Z0-9_-]{6,12}$/.test(shortId);
}

function isValidAdminToken(token: string): boolean {
  const expectedToken = process.env.PLAYGROUND_ADMIN_TOKEN;
  if (!expectedToken) return false;
  const expectedHash = createHash("sha256").update(expectedToken).digest();
  const actualHash = createHash("sha256").update(token).digest();
  return timingSafeEqual(expectedHash, actualHash);
}

function getTyrusBin(): string {
  return process.env.TYRUS_BIN || "tyrus";
}

function getNsjailBin(): string {
  return process.env.NSJAIL_BIN || "/usr/bin/nsjail";
}

const COMPILE_TIMEOUT_MS = 10_000;
const MAX_CODE_SIZE = 8192;
// Mirrors SANDBOX_FORBIDDEN_MODULES in crates/tyrus-core/src/module_loader.rs,
// which is the authoritative list. `tyrus:process` used to appear here and is
// gone because no such module exists, process information lives in the ambient
// `process` namespace, which cannot be filtered by import at all and is instead
// reported as unavailable at run time.
const FORBIDDEN_MODULES = [
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
// Address-space ceiling for a compile, used by both paths. `ulimit -v` counts
// reserved virtual memory, not memory in use: a compile touches about 84 MB
// resident but needs at least 512 MB of address space to start at all, so this
// is a backstop against runaway growth rather than a tight bound. nsjail's
// --memory is what actually constrains memory in production.
const DEGRADED_MEMORY_LIMIT_BYTES = 768 * 1024 * 1024;
// nsjail clears inherited variables; forward only compiler paths, never the backend's secrets.
const TYRUS_SANDBOX_ENVIRONMENT = [
  "TYRUS_WASM_RUNTIME_LIB",
  "TYRUS_LLC",
  "TYRUS_WASM_LD",
  "LD_LIBRARY_PATH",
] as const;

let nsjailAvailable: boolean | null = null;

/// Outcome of actually exercising the sandbox, not of looking for its binary.
export interface SandboxCheck {
  usable: boolean;
  /// Present when the sandbox is unusable: what failed, in one line.
  reason?: string;
}

/// Run a throwaway jail and confirm it both starts *and* contains.
///
/// Checking that the nsjail binary exists is not the same as checking that the
/// jail works, and the difference is not academic, every one of these was hit
/// on a machine where the binary was present:
///
///   * flags nsjail does not accept ("Couldn't parse cmdline options"),
///   * no writable cgroup subtree ("Couldn't initialize cgroup 2 user
///     namespace") when the backend does not run as root,
///   * a kernel with unprivileged user namespaces disabled.
///
/// So the probe runs a shell inside a real jail and asks it to prove
/// containment: that it is nobody, and that the read-only root refuses a write.
/// A jail that starts but does not contain is worse than none, because it looks
/// like it is working.
export async function verifySandbox(): Promise<SandboxCheck> {
  try {
    await access(getNsjailBin());
  } catch {
    return { usable: false, reason: `nsjail not found at ${getNsjailBin()}` };
  }

  const probeDir = await mkdtemp(join(tmpdir(), "tyrus-sandbox-probe-"));
  try {
    // `/etc` is outside the one writable bind, so this write must fail. If it
    // succeeds, the jail is not read-only and must not be trusted.
    const probe =
      'id -u; touch /etc/tyrus-sandbox-probe 2>/dev/null && echo ESCAPED || echo CONTAINED';
    const command = buildSandboxCommand({
      useNsjail: true,
      tmpDir: probeDir,
      tyrusBin: getTyrusBin(),
      nsjailBin: getNsjailBin(),
      cgroupMount: process.env.NSJAIL_CGROUP_MOUNT,
      command: `/bin/sh -c ${JSON.stringify(probe)}`,
    });

    const output = await new Promise<{ ok: boolean; text: string }>((resolve) => {
      execFile("bash", ["-c", command], { timeout: 15_000 }, (error, stdout, stderr) => {
        resolve({
          ok: !error,
          text: `${stdout ?? ""}${stderr ?? ""}`,
        });
      });
    });

    if (!output.ok) {
      const detail = output.text
        .split("\n")
        .find((line) => line.includes("[E]") || line.includes("[F]"))
        ?.trim();
      return { usable: false, reason: detail ?? "the jail failed to start" };
    }
    if (!output.text.includes("CONTAINED")) {
      return { usable: false, reason: "the jail started but its root was writable" };
    }
    if (!/^\s*65534\s*$/m.test(output.text)) {
      return { usable: false, reason: "the jail did not drop to an unprivileged user" };
    }

    return { usable: true };
  } finally {
    await rm(probeDir, { recursive: true, force: true }).catch(() => {});
  }
}

async function detectNsjail(log: FastifyBaseLogger): Promise<boolean> {
  if (nsjailAvailable !== null) return nsjailAvailable;

  const check = await verifySandbox();
  nsjailAvailable = check.usable;

  if (!check.usable) {
    log.warn(
      { nsjailBin: getNsjailBin(), reason: check.reason },
      "sandbox unusable, falling back to degraded ulimit-only isolation for " +
        "compilation. This is acceptable for local development only."
    );
  }
  return nsjailAvailable;
}

/// The shell command that compiles one project, for either sandbox path.
///
/// Exported so a test can assert on the string itself. That is not ceremony:
/// this command was once assembled by joining every element with a space,
/// producing `ulimit -t 5 ulimit -v 786432 exec nsjail ...`, one `ulimit`
/// invocation as far as bash is concerned. No limit applied, nsjail never ran,
/// the compiler never ran, and every request failed with a missing artifact.
/// Nothing in the type system or the linter can catch that; a test on the
/// assembled string can.
export function buildSandboxCommand(options: {
  useNsjail: boolean;
  tmpDir: string;
  tyrusBin: string;
  nsjailBin: string;
  environment?: Partial<Record<(typeof TYRUS_SANDBOX_ENVIRONMENT)[number], string>>;
  /// cgroup v2 subtree the jail may create groups under. Required wherever the
  /// backend does not run as root: creating a cgroup under `/sys/fs/cgroup`
  /// needs privilege, while systemd delegates a writable subtree per service
  /// (`Delegate=yes`) or per user session.
  cgroupMount?: string;
  /// Command to run instead of a compile. Only [`verifySandbox`] uses this, to
  /// exercise the jail without needing a project to build.
  command?: string;
}): string {
  const build =
    options.command ?? `${options.tyrusBin} build --compile-target wasm --sandbox`;
  // `ulimit -v` bounds address space, which Rust and LLVM reserve far more of
  // than they use, 256 MB was not enough for the compiler to spawn its parser
  // thread. A compile touches about 84 MB resident.
  const addressSpaceKb = DEGRADED_MEMORY_LIMIT_BYTES / 1024;

  if (!options.useNsjail) {
    return [
      `ulimit -v ${addressSpaceKb}`,
      "ulimit -t 10",
      `exec ${build}`,
    ].join(" && ");
  }

  // Words of one command line, joined with spaces.
  //
  // Every flag here was verified against nsjail 3.4 by compiling through it.
  // The previous set was written without one ever running, and none of it held
  // up: `--timeout`, `--memory`, `--cpus`, `--pids` and `-w` are not nsjail
  // options at all (it answers "Couldn't parse cmdline options" and runs
  // nothing), and `-N` is `--disable_clone_newnet`, it *grants* the jail the
  // host's network rather than taking it away.
  const nsjail = [
    options.nsjailBin,
    // Run the command once, as a subprocess.
    "-Mo",
    // Network isolation is nsjail's default; naming a flag for it is what
    // turned it off. A compile needs no network, and untrusted code being
    // compiled must not reach one.
    //
    // The host filesystem is mounted read-only and the project is the only
    // writable path. Mounting the pieces individually (`-R /usr -R /lib ...`)
    // was tried and rejected: the toolchain the compiler shells out to, llc
    // and wasm-ld, then failed to resolve, and enumerating every transitive
    // dependency of an external toolchain is a maintenance trap.
    "--chroot /",
    // ProtectKernelTunables=yes creates /proc overmounts in the systemd unit,
    // which prevents nsjail from mounting a fresh procfs in its mount namespace.
    "--disable_proc",
    "--user 65534",
    "--group 65534",
    // The cgroup limits below address the v1 layout unless this is passed, and
    // on a host with pure cgroup v2 (current Debian/Ubuntu) the jail then fails
    // to start: "mkdir('/sys/fs/cgroup/memory/NSJAIL/...') failed".
    "--detect_cgroupv2",
    ...(options.cgroupMount ? [`--cgroupv2_mount ${options.cgroupMount}`] : []),
    // Wall clock, then CPU seconds: a compile that spins is stopped even if it
    // sleeps its way under the wall-clock budget.
    "--time_limit 5",
    "--rlimit_cpu 5",
    // Bytes. A compile peaks near 84 MB resident, measured across hello-world,
    // regex and the timezone database.
    `--cgroup_mem_max ${256 * 1024 * 1024}`,
    // Without this the limit above is not a limit: a process that exceeds it
    // is pushed to swap rather than killed, and 700 MB allocated cleanly under
    // a 256 MB cap. Denying swap is what makes the ceiling real.
    "--cgroup_mem_swap_max 0",
    "--cgroup_pids_max 64",
    "--max_cpus 1",
    // nsjail's default RLIMIT_FSIZE is 1 MB, which the linker hits while
    // writing the module: a hello-world (22 KB) and a temporal program
    // (605 KB) linked fine, while one using regex (1.19 MB) died with
    // "failed to open ...: File too large", reported as an LLVM crash, not as
    // a limit. 16 MB clears any module the 8 KB source cap can produce.
    "--rlimit_fsize 16",
    ...Object.entries(options.environment ?? {})
      .filter((entry): entry is [string, string] => Boolean(entry[1]))
      .map(([name, value]) => `--env ${shellQuote(`${name}=${value}`)}`),
    // Bind the project at the same path inside the jail, and work there. A
    // different mount point needs a directory to exist under a read-only root,
    // which fails.
    `-B ${options.tmpDir}`,
    `--cwd ${options.tmpDir}`,
    "--",
    build,
  ].join(" ");

  // No outer `ulimit` on this path. nsjail sets RLIMIT_AS and RLIMIT_CPU
  // itself, and a shell that has already lowered the hard limit makes that
  // fail outright: "setrlimit(RLIMIT_AS, 4294967296): Operation not
  // permitted", and the jail never starts. Every limit here is nsjail's.
  return `exec ${nsjail}`;
}

function shellQuote(value: string): string {
  return `'${value.replaceAll("'", "'\\''")}'`;
}

async function compileTyrusCode(code: string, log: FastifyBaseLogger): Promise<Buffer> {
  const codeHash = hashcode(code);
  const cached = cacheGet(codeHash);

  if (cached) {
    return cached;
  }

  const tmpDir = await mkdtemp(join(tmpdir(), "tyrus-playground-"));

  try {
    await mkdir(join(tmpDir, "src"), { recursive: true });
    await writeFile(
      join(tmpDir, "tyrus.json"),
      JSON.stringify({
        project: { name: "playground", version: "0.1.0" },
        scripts: {},
        dependencies: {},
      }),
      "utf-8"
    );
    await writeFile(join(tmpDir, "src", "main.tyr"), code, "utf-8");

    const useNsjail = await detectNsjail(log);

    const result = await new Promise<{ ok: boolean; stdout: string; stderr: string }>(
      (resolve) => {
        const command = buildSandboxCommand({
          useNsjail,
          tmpDir,
          tyrusBin: getTyrusBin(),
          nsjailBin: getNsjailBin(),
          cgroupMount: process.env.NSJAIL_CGROUP_MOUNT,
          environment: Object.fromEntries(
            TYRUS_SANDBOX_ENVIRONMENT.flatMap((name) =>
              process.env[name] ? [[name, process.env[name]!]] : []
            )
          ),
        });

        execFile(
          "bash",
          ["-c", command],
          // nsjail binds the project and chdirs into it via --cwd; the
          // fallback path runs the compiler directly and needs the cwd.
          {
            ...(useNsjail ? {} : { cwd: tmpDir }),
            timeout: COMPILE_TIMEOUT_MS,
            maxBuffer: 1024 * 1024,
          },
          (error, stdout, stderr) => {
            resolve({
              ok: !error,
              stdout: stdout?.toString() || "",
              stderr: stderr?.toString() || (error ? error.message : ""),
            });
          }
        );
      }
    );

    if (!result.ok) {
      throw new CompilationError(result.stderr || result.stdout);
    }

    // A compiler that exits 0 without producing `target/` leaves `readdir` to
    // throw ENOENT, which used to surface as an opaque HTTP 500. Treat it as
    // what it is: the build did not produce an artifact.
    const targetDir = join(tmpDir, "target");
    const files = await readdir(targetDir).catch(() => [] as string[]);
    const wasmFile = files.find((f) => f.endsWith(".wasm"));

    if (!wasmFile) {
      throw new CompilationError(
        "The compiler exited successfully but produced no WASM module."
      );
    }

    const wasm = await readFile(join(targetDir, wasmFile));
    cacheSet(codeHash, wasm);

    return wasm;
  } finally {
    await rm(tmpDir, { recursive: true, force: true }).catch(() => {});
  }
}

class CompilationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CompilationError";
  }
}

function codeSizeExceeded(code: string): boolean {
  return Buffer.byteLength(code, "utf8") > MAX_CODE_SIZE;
}

/// Bytes of actual source behind a stored payload.
///
/// A shared snippet may arrive gzipped and base64-encoded, and measuring *that*
/// measures the wrong thing: 1 MB of source compresses to about 4 KB, so it
/// sailed past an 8 KB limit and was stored and served in full, 129 times the
/// advertised cap. Decompression is bounded so a crafted payload cannot spend
/// unbounded memory proving it is too large.
async function decodedCodeSize(
  code: string,
  encoding: "gzip" | "none"
): Promise<number | null> {
  if (encoding === "none") {
    return Buffer.byteLength(code, "utf8");
  }

  let compressed: Buffer;
  try {
    compressed = Buffer.from(code, "base64");
  } catch {
    return null;
  }

  return new Promise((resolve) => {
    const inflate = createGunzip();
    let total = 0;
    let settled = false;

    const finish = (value: number | null) => {
      if (settled) return;
      settled = true;
      inflate.destroy();
      resolve(value);
    };

    inflate.on("data", (chunk: Buffer) => {
      total += chunk.byteLength;
      // Stop the moment it cannot pass, rather than inflating the rest.
      if (total > MAX_CODE_SIZE) finish(total);
    });
    inflate.on("end", () => finish(total));
    inflate.on("error", () => finish(null));
    inflate.end(compressed);
  });
}

interface Diagnostic {
  code: string;
  message: string;
  line: number;
  column: number;
  help?: string;
}

/// Strip the compile directory out of a message before it leaves the server.
///
/// Diagnostics quote the path they were produced from, which here is a
/// `mkdtemp` directory under the server's temp root. That says nothing useful
/// to the author and discloses server-side layout, so it is reduced to the file
/// name the author actually wrote.
function redactPaths(message: string): string {
  return message.replace(/\S*tyrus-playground-[^\s:]*\/(?:src\/)?/g, "");
}

function parseDiagnostics(stderr: string): Diagnostic[] {
  const blocks = stderr.split(/(?=^error\[E\d+\]:)/m).filter((block) => block.trim());
  const diagnostics: Diagnostic[] = [];

  for (const block of blocks) {
    const header = block.match(/^error\[(E\d+)\]:\s*(.+)$/m);
    if (!header) continue;

    const location = block.match(/^at .*?:(\d+):(\d+)\s*$/m);
    const help = block.match(/^help:\s*(.+)$/m);

    diagnostics.push({
      code: header[1],
      message: redactPaths(header[2].trim()),
      line: location ? parseInt(location[1], 10) : 0,
      column: location ? parseInt(location[2], 10) : 0,
      ...(help ? { help: redactPaths(help[1].trim()) } : {}),
    });
  }

  if (diagnostics.length > 0) {
    return diagnostics;
  }

  // Not every rejection carries a diagnostic code. A sandboxed build refusing a
  // capability module reports plain `error: module \`tyrus:fs\` is not
  // available...`, and reducing that to "Compilation failed" left the author
  // with no idea what was wrong, the one thing the message was written to say.
  for (const line of stderr.split("\n")) {
    const plain = line.match(/^error(?:\[\w+\])?:\s*(.+)$/);
    if (plain) {
      diagnostics.push({
        code: "E0000",
        message: redactPaths(plain[1].trim()),
        line: 0,
        column: 0,
      });
    }
  }

  return diagnostics;
}

export async function playgroundRoutes(app: FastifyInstance) {
  app.get("/health", async (_request, reply) => {
    return reply.send({ status: "ok", cache: compileCache.size });
  });

  const installerDownloads = app.mongo.db!.collection<{
    _id: "install.sh" | "install.ps1";
    count: number;
  }>("installer_downloads");

  app.get<{ Params: { installer: string } }>(
    "/downloads/:installer",
    async (request, reply) => {
      const installer = request.params.installer;
      if (installer !== "install.sh" && installer !== "install.ps1") {
        return reply.status(404).send({ error: "Installer not found" });
      }

      let contents: string;
      try {
        const installerDirectories = process.env.TYRUS_INSTALLER_DIR
          ? [process.env.TYRUS_INSTALLER_DIR]
          : [join(process.cwd(), "installers"), join(process.cwd(), "../../docs/public")];
        let loaded = false;
        contents = "";
        for (const directory of installerDirectories) {
          try {
            contents = await readFile(join(directory, installer), "utf8");
            loaded = true;
            break;
          } catch (error) {
            if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
          }
        }
        if (!loaded) throw new Error("installer asset was not found");
      } catch {
        request.log.error({ installer }, "Installer asset is unavailable");
        return reply.status(503).send({ error: "Installer temporarily unavailable" });
      }

      if (request.method === "GET") {
        await installerDownloads.updateOne(
          { _id: installer },
          { $inc: { count: 1 } },
          { upsert: true },
        );
      }

      return reply
        .header("Cache-Control", "no-store")
        .header("Content-Type", "text/plain; charset=utf-8")
        .header("Content-Disposition", `attachment; filename="${installer}"`)
        .send(contents);
    },
  );

  app.get<{ Headers: { authorization?: string } }>(
    "/downloads/stats",
    async (request, reply) => {
      const authHeader = request.headers.authorization;
      if (!authHeader?.startsWith("Bearer ")) {
        return reply.status(401).send({ error: "Authorization required" });
      }
      if (!isValidAdminToken(authHeader.slice(7))) {
        return reply.status(403).send({ error: "Invalid admin token" });
      }

      const counts = await installerDownloads
        .find({}, { projection: { _id: 1, count: 1 } })
        .sort({ _id: 1 })
        .toArray();
      return reply.send({ downloads: counts });
    },
  );

  app.post<{ Body: CreatePlaygroundRequest }>(
    "/playground",
    { config: { rateLimit: { max: 5, timeWindow: "1 minute" } } },
    async (request, reply) => {
    const { code, permanent = false, encoding = "none" } = request.body;

    if (!code || typeof code !== "string") {
      return reply.status(400).send({ error: "code is required and must be a string" });
    }

      const trimmed = code.trim();
      
    if (trimmed.length === 0) {
      return reply.status(400).send({ error: "code must not be empty" });
    }

    const storedEncoding: "gzip" | "none" = encoding === "gzip" ? "gzip" : "none";
    const sourceBytes = await decodedCodeSize(trimmed, storedEncoding);

    if (sourceBytes === null) {
      return reply.status(400).send({ error: "code is not a valid gzip payload" });
    }

    if (sourceBytes > MAX_CODE_SIZE) {
      return reply
        .status(400)
        .send({ error: `code exceeds maximum size (${MAX_CODE_SIZE} bytes)` });
    }

    if (permanent) {
      const authHeader = request.headers.authorization;

      if (!authHeader?.startsWith("Bearer ")) {
        return reply.status(401).send({ error: "Authorization required for permanent snippets" });
      }

      const token = authHeader.slice(7);

      if (!isValidAdminToken(token)) {
        return reply.status(403).send({ error: "Invalid admin token" });
      }
    }

    const db = app.mongo.db!;
    const collection = getCollection(db);

    const createdAt = new Date();
    const expiresAt = permanent ? null : new Date(createdAt.getTime() + 30 * 24 * 60 * 60 * 1000);

    // `shortId` is random and the index is unique, so a collision is possible
    // however unlikely. Retrying turns it into a non-event; letting the
    // duplicate-key error escape turned it into an HTTP 500 for a user who did
    // nothing wrong.
    let shortId = "";
    let inserted = false;

    for (let attempt = 0; attempt < 5 && !inserted; attempt += 1) {
      shortId = generateShortId();
      const playground: Playground = {
        shortId,
        code: trimmed,
        encoding: storedEncoding,
        permanent: !!permanent,
        createdAt,
        expiresAt,
      };

      try {
        await collection.insertOne(playground);
        inserted = true;
      } catch (err) {
        const isDuplicate = (err as { code?: number }).code === 11000;
        if (!isDuplicate) throw err;
        app.log.warn({ shortId }, "shortId collision, retrying");
      }
    }

    if (!inserted) {
      return reply.status(503).send({ error: "Could not allocate a snippet id, try again" });
    }

    app.log.info({ shortId, permanent }, "Playground created");

    // Built from the request rather than hardcoded: the previous literal
    // pointed every deployment (including a local one) at production.
    const origin = process.env.PLAYGROUND_ORIGIN
      ?? `${request.protocol}://${request.hostname}`;

    return reply.status(201).send({
      url: `${origin}/s/${shortId}`,
      shortId,
    });
  });

  app.get<{ Params: { shortId: string } }>(
    "/playground/:shortId",
    { config: { rateLimit: { max: 100, timeWindow: "1 minute" } } },
    async (request, reply) => {
    const { shortId } = request.params;

    if (!validateShortId(shortId)) {
      return reply.status(400).send({ error: "Invalid shortId format" });
    }

    const db = app.mongo.db!;
    const collection = getCollection(db);

    const playground = await collection.findOne({ shortId });

    if (!playground) {
      return reply.status(404).send({ error: "Snippet not found" });
    }

    return reply
      .header("X-Encoding", playground.encoding)
      .header("Cache-Control", "public, max-age=86400")
      .send({ code: playground.code, encoding: playground.encoding });
    }
  );

  app.delete<{ Params: { shortId: string } }>(
    "/playground/:shortId",
    { config: { rateLimit: { max: 5, timeWindow: "1 minute" } } },
    async (request, reply) => {
      const { shortId } = request.params;

      if (!validateShortId(shortId)) {
        return reply.status(400).send({ error: "Invalid shortId format" });
      }

      const authHeader = request.headers.authorization;

      if (!authHeader?.startsWith("Bearer ")) {
        return reply.status(401).send({ error: "Authorization required" });
      }

      const token = authHeader.slice(7);

      if (!isValidAdminToken(token)) {
        return reply.status(403).send({ error: "Invalid admin token" });
      }

      const db = app.mongo.db!;
      const collection = getCollection(db);
      const result = await collection.deleteOne({ shortId });

      if (result.deletedCount === 0) {
        return reply.status(404).send({ error: "Snippet not found" });
      }

      app.log.info({ shortId }, "Playground deleted");

      return reply.status(204).send();
    }
  );

  app.post<{ Body: { code: string } }>(
    "/playground/compile",
    { config: { rateLimit: { max: 10, timeWindow: "1 minute" } } },
    async (request, reply) => {
      const ip = request.ip;
      const { code } = request.body;

      if (!code || typeof code !== "string") {
        return reply.status(400).send({ error: "code is required and must be a string" });
      }

      const trimmed = code.trim();
      if (trimmed.length === 0) {
        return reply.status(400).send({ error: "code must not be empty" });
      }

      if (codeSizeExceeded(trimmed)) {
        return reply.status(400).send({ error: `code exceeds maximum size (${MAX_CODE_SIZE} bytes)` });
      }

      // Defence in depth only. The authority is `tyrus build --sandbox`, which
      // refuses these modules against *resolved* imports; this pattern matches
      // text, and text can be disguised (`from /* x */ "tyrus:fs"` slips
      // through). It stays because rejecting the obvious case here avoids
      // spawning a compiler at all, but it is no longer what makes the rule
      // hold.
      for (const mod of FORBIDDEN_MODULES) {
        const importRegex = new RegExp(`from\\s*["']${mod.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}["']`);

        if (importRegex.test(trimmed)) {
          return reply.status(403).send({
            error: `Import of "${mod}" is not allowed in the playground`,
          });
        }
      }

      try {
        const startTime = Date.now();
        const wasmBytes = await compileTyrusCode(trimmed, app.log);
        const elapsed = Date.now() - startTime;

        app.log.info({ ip, wasmSize: wasmBytes.length, elapsed }, "Compilation succeeded");

        return reply
          .header("Content-Type", "application/wasm")
          .header("Cache-Control", "no-store")
          .send(wasmBytes);
      } catch (err) {
        if (err instanceof CompilationError) {
          app.log.warn({ ip, stderr: err.message }, "Compilation failed");
          const diagnostics = parseDiagnostics(err.message);
          
          return reply.status(422).send({
            error: "Compilation failed",
            diagnostics: diagnostics.length > 0
              ? diagnostics
              : [{ code: "E0000", message: "Compilation failed", line: 0, column: 0 }],
          });
        }

        app.log.error(err, "Internal compilation error");
        return reply.status(500).send({ error: "Internal compilation error" });
      }
    }
  );
}
