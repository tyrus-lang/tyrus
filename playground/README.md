# Tyrus Playground

Web playground for writing, compiling and executing Tyrus code directly in the browser.

## Structure

```
playground/
├── frontend/               # Vite + TypeScript frontend (real ES modules, bundled)
│   ├── index.html
│   ├── vite.config.ts      # Dev server (with /api proxy to the backend) + build config
│   ├── tsconfig.json       # Main app (DOM lib), excludes worker-runtime.ts, see tsconfig.worker.json
│   ├── tsconfig.worker.json  # worker-runtime.ts alone (WebWorker lib, incompatible with DOM in one program)
│   ├── css/styles.css
│   ├── src/
│   │   ├── main.ts         # Main application logic
│   │   ├── editor.ts       # Monaco Editor wiring (themes, tokens provider, worker setup)
│   │   ├── tyrus-textmate.ts  # Tokenizes Tyrus via the real TextMate grammar
│   │   ├── examples.ts     # Pre-loaded code examples
│   │   ├── share.ts        # Share via compressed short URLs
│   │   ├── worker.ts       # Web Worker communication layer
│   │   ├── worker-runtime.ts  # Worker runtime (WASM integration)
│   │   └── protocol.ts     # Typed message contract between worker.ts and worker-runtime.ts
│   └── public/             # Served as-is at the site root (not bundled/hashed)
│       ├── img/
│       ├── grammars/       # tyrus.tmLanguage.json, synced from editors/vscode/syntaxes/ by scripts/editors
│       └── themes/         # VS Code Dark+/Light+ theme JSON, used to color the TextMate tokens
├── backend/                 # TypeScript + Fastify API
│   ├── src/
│   │   ├── index.ts        # Server entry point
│   │   ├── models.ts       # TypeScript interfaces
│   │   ├── routes.ts       # API routes with validation
│   │   └── routes.test.ts  # Integration tests
│   ├── package.json
│   └── tsconfig.json
├── scripts/
│   ├── dev.sh              # Local development script
│   └── setup.sh            # Local setup script
└── README.md
```

## Quick Start

```bash
# Prepare dependencies and the backend environment once
npm run playground:setup

# Start both frontend and backend for development
npm run playground:dev
```

This starts:
- Frontend at `http://localhost:3000`
- Backend at `http://localhost:3001`

The setup script installs Node.js dependencies and creates `backend/.env` from the example. The backend also requires a local MongoDB server; start MongoDB before using the Playground API or running backend tests.

## Frontend

- Monaco Editor tokenized by the same generated TextMate grammar as the VS Code extension (`public/grammars/tyrus.tmLanguage.json`, via `vscode-textmate` + `vscode-oniguruma`), colored with copies of VS Code's own Dark+/Light+ themes (`public/themes/`), so the Playground matches VS Code's colors exactly instead of a hand-picked palette
- Dark/light theme toggle
- Pre-loaded examples (Hello World, Fibonacci, Match, Loops, Const, Enums)
- Share snippets via compressed short URLs
- `Ctrl+Enter` to run code
- Status bar with execution state

## Sandbox (production)

Compilation runs inside [nsjail](https://github.com/google/nsjail). Without it the backend logs a warning and falls back to plain `ulimit`, which is enough for local development and not enough for a public deployment.

For a public deployment, the host needs to support unprivileged user namespaces and cgroup v2, with nsjail configured to enforce process, memory, filesystem, and network limits. Verify the jail by compiling through it and checking that it runs as an unprivileged user and cannot write outside its temporary project directory.

### Kernel requirements

nsjail builds its jail out of unprivileged user namespaces. Two things commonly deny them on a VPS, and neither is obvious from an nsjail error message:

- **Ubuntu 23.10 and later ship `kernel.apparmor_restrict_unprivileged_userns=1`**, which blocks them through AppArmor even when every other setting looks permissive. Ubuntu 24.04 LTS (the usual VPS image) is affected:

  ```bash
  echo 'kernel.apparmor_restrict_unprivileged_userns=0' | sudo tee /etc/sysctl.d/60-nsjail.conf
  sudo sysctl --system
  ```

- **Container-based VPS** (OpenVZ, LXC) generally cannot create user namespaces at all. A KVM instance can. `systemd-detect-virt` tells you which you have.

### Building nsjail

nsjail is not packaged by Debian or Ubuntu, so it has to be built:

```bash
sudo apt install -y protobuf-compiler libprotobuf-dev libnl-route-3-dev \
    autoconf bison flex gcc g++ libtool make pkg-config
git clone --recursive --depth 1 https://github.com/google/nsjail /tmp/nsjail
make -C /tmp/nsjail -j"$(nproc)"
sudo install -m 0755 /tmp/nsjail/nsjail /usr/bin/nsjail
```

The `--recursive` matters: nsjail builds against the `kafel` submodule and the build fails without it. Release tarballs do not include submodules, so the clone is the simpler route.

Confirm the result:

```bash
nsjail --help | head -1
```

### Cgroup delegation

The jail's memory and pid limits live in a cgroup, and creating one under `/sys/fs/cgroup` requires privilege the backend does not have. A systemd service can delegate its own cgroup subtree to the backend process.

`NSJAIL_CGROUP_MOUNT` points at that subtree. For a service named `tyrus-playground` it is:

```
/sys/fs/cgroup/system.slice/tyrus-playground.service
```

The path depends on the systemd service name and must be configured for the host.

Without delegation nsjail refuses to start:

```
Couldn't initialize cgroup 2 user namespace
```

and the backend then refuses to serve, so a mistake here prevents production startup rather than quietly disabling isolation.

**Checking it on the server.** The subtree must be writable by the service user and offer the three controllers the jail uses:

```bash
systemctl show tyrus-playground -p ControlGroup     # the delegated path
cat /sys/fs/cgroup/system.slice/tyrus-playground.service/cgroup.controllers
```

The second command must list `cpu memory pids`. If `memory` or `pids` is missing, the parent's `cgroup.subtree_control` has not enabled them for children and the limits will not apply, a failure that is silent in the jail but caught by the startup verification below.

### Startup refuses an unusable sandbox

In production (`NODE_ENV=production`) the backend runs a throwaway jail at startup and asks it to prove containment, that it drops to an unprivileged user and that its read-only root refuses a write. If that fails, the process logs the reason and exits rather than serving.

This is deliberate. The fallback to `ulimit`-only isolation is silent by design: compilation keeps working, so nothing looks wrong while untrusted code is compiled with no isolation. A missing binary, an undelegated cgroup and a jail whose root turns out to be writable are all caught here instead.

Outside production the same check runs and only warns, so local development works without nsjail installed.

Because startup fails without a working jail, a successful `GET /api/health` in production is proof that the sandbox is active.

### What the jail is verified to do

Checked by compiling through it, not by reading the flags:

| Property | Result |
| --- | --- |
| Identity inside the jail | `uid 65534` (nobody) |
| Writes outside the project | refused, the host filesystem is mounted read-only |
| `/etc/shadow` | permission denied |
| Network | one loopback interface, no DNS resolution |
| Wall-clock runaway | stopped at `--time_limit` (5s) |
| Fork bomb | contained by `--cgroup_pids_max` |
| Memory | capped at 256 MB, enforced because swap is denied |

A compile peaks near 84 MB resident; the ceilings above leave room without being generous.

## Backend

- TypeScript + Fastify REST API
- MongoDB for snippet storage with gzip compression
- TTL: temporary snippets expire after 30 days, permanent snippets (docs) never expire
- Rate limiting (10 requests/minute per IP)
- Admin token authentication for permanent snippets

## API

### `POST /api/playground/compile`

Compile Tyrus code to WASM. Returns the `.wasm` binary on success or JSON errors on failure.

```json
// Request
{ "code": "function main() { print(\"Hello!\"); }" }

// Response (200), application/wasm binary
// Response (422), compilation errors
{ "error": "Compilation failed", "diagnostics": "..." }
```

### `POST /api/playground`

Create a shareable snippet.

```json
// Request
{ "code": "function main(): void { print(\"Hello!\"); }", "permanent": false }

// Response (201)
{ "url": "https://playground.tyruslang.org/s/a1b2c3d4", "shortId": "a1b2c3d4" }
```

### `GET /api/playground/:shortId`

Retrieve a snippet. Returns `code` and `encoding` (`"gzip"` or `"none"`).

### `DELETE /api/playground/:shortId`

Remove a snippet. Requires `Authorization: Bearer <token>` header.

## Configuration

Backend environment variables (`.env`):

```bash
MONGO_URI=mongodb://localhost:27017
DB_NAME=tyrus_playground
PLAYGROUND_ADMIN_TOKEN=your-secret-token
PORT=3001

# Path to the compiler. Defaults to `tyrus` on PATH, which may be an older
# installed copy than the one being deployed.
TYRUS_BIN=/usr/local/bin/tyrus

# Sandbox. Without NSJAIL_BIN present the backend degrades to ulimit-only
# isolation and says so in the log. NSJAIL_CGROUP_MOUNT is required whenever
# the backend does not run as root, see "Sandbox (production)" above.
NSJAIL_BIN=/usr/bin/nsjail
# Required unless running as root, see "Cgroup delegation" above. The wrong
# value stops the service from starting rather than disabling isolation.
NSJAIL_CGROUP_MOUNT=/sys/fs/cgroup/system.slice/tyrus-playground.service

# Origin used to build share links. Defaults to the request's own host.
PLAYGROUND_ORIGIN=https://playground.tyruslang.org
```

## Testing

Start MongoDB first. The integration tests use `MONGO_URI` when set, or `mongodb://localhost:27017` by default, and create and drop the `tyrus_playground_test` database.

```bash
cd backend
npm test
npm run typecheck
npm run lint

cd ../frontend
npm run check   # tsc --noEmit (tsconfig.json + tsconfig.worker.json)
npm run lint    # oxlint
npm run build   # vite build, produces dist/
```

### Installer download counts

The installer sources live in `../docs/public/install.sh` and `../docs/public/install.ps1`; VitePress publishes them as `/install.sh` and `/install.ps1`. A reverse proxy can forward requests for these paths to the Playground API, which increments `installer_downloads` and returns the file. The counter stores one aggregate per installer, with no IP address, user agent, or per-request record.

The main `tyruslang.org` host must proxy these two paths to the Playground API. If the domain is served by a static host, configure its server-side proxy equivalent; serving the files directly from static storage bypasses the counter.

To test against the deployed service, download an installer with a GET and read the aggregate using the server's `PLAYGROUND_ADMIN_TOKEN`:

```bash
curl -fsSL https://tyruslang.org/install.sh -o /tmp/tyrus-install.sh
curl -fsS -H "Authorization: Bearer $PLAYGROUND_ADMIN_TOKEN" https://playground.tyruslang.org/api/downloads/stats
```

The stats endpoint is admin-token protected. HEAD requests and failed requests do not increment the count. A request is counted when the backend accepts the GET and returns the script; the backend cannot verify whether the client saved the complete response or executed it.

## Architecture

1. User writes code in Monaco Editor.
2. Code is sent to the backend (`POST /api/playground/compile`).
3. Backend invokes `tyrus build --compile-target wasm` in an isolated temp directory.
4. WASM binary is returned to the browser.
5. Web Worker loads and executes the WASM via `WebAssembly.instantiate`.
6. stdout/stderr are captured via exported buffer functions and displayed in the output panel.
7. Shared snippets are compressed with gzip and stored in MongoDB.
8. Short URLs (`/s/{shortId}`) load snippets from the backend.

