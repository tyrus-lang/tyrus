import { spawn } from 'node:child_process'
import { mkdir, mkdtemp, readFile, rm, stat } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const docsDir = join(rootDir, 'docs')
const distDir = join(docsDir, '.vitepress', 'dist')
const versionsFile = join(docsDir, '.vitepress', 'versions.ts')
const vitepressBin = join(rootDir, 'node_modules', '.bin', process.platform === 'win32' ? 'vitepress.cmd' : 'vitepress')

function run(command, args, options = {}) {
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(command, args, {
      cwd: rootDir,
      stdio: 'inherit',
      ...options
    })

    child.on('error', rejectRun)
    child.on('close', (code) => {
      if (code === 0) {
        resolveRun()
        return
      }

      rejectRun(new Error(`${command} ${args.join(' ')} exited with status ${code}`))
    })
  })
}

async function tagExists(tag) {
  try {
    await run('git', ['rev-parse', '--verify', `refs/tags/${tag}`], { stdio: 'ignore' })
    return true
  } catch {
    return false
  }
}

async function readPreviousVersions() {
  const source = await readFile(versionsFile, 'utf8')
  const matches = [...source.matchAll(/tag:\s*'([^']+)'[\s\S]*?current:\s*false/g)]
  return matches.map((match) => match[1])
}

async function directoryExists(path) {
  try {
    return (await stat(path)).isDirectory()
  } catch {
    return false
  }
}

async function buildCurrentDocs() {
  await run(vitepressBin, ['build', 'docs'])
}

async function archiveTag(tag, archivePath) {
  const paths = [
    'docs',
    'package.json',
    'package-lock.json',
    'editors/vscode/syntaxes/tyrus.tmLanguage.json'
  ]

  await run('git', ['archive', '--format=tar', `--output=${archivePath}`, tag, ...paths])
}

async function buildArchivedDocs(tag) {
  if (!await tagExists(tag)) {
    throw new Error(`Configured documentation tag does not exist: ${tag}`)
  }

  const tempDir = await mkdtemp(join(tmpdir(), `tyrus-docs-${tag}-`))

  try {
    const archivePath = join(tempDir, `${tag}.tar`)
    const worktreeDir = join(tempDir, 'worktree')
    await archiveTag(tag, archivePath)
    await mkdir(worktreeDir, { recursive: true })
    await run('tar', ['-xf', archivePath, '-C', worktreeDir])

    if (!await directoryExists(join(worktreeDir, 'docs'))) {
      throw new Error(`Configured documentation tag does not contain docs/: ${tag}`)
    }

    await run(vitepressBin, ['build', 'docs', '--base', `/${tag}/`, '--outDir', join(distDir, tag)], { cwd: worktreeDir })
  } finally {
    await rm(tempDir, { force: true, recursive: true })
  }
}

const previousVersions = await readPreviousVersions()

await buildCurrentDocs()

for (const tag of previousVersions) {
  await buildArchivedDocs(tag)
}
