import { readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

const DOCS_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const DIST_ROOT = join(DOCS_ROOT, '.vitepress', 'dist')
const CONFIG_PATH = join(DOCS_ROOT, '.vitepress', 'config.mts')
const SITE_ORIGIN = process.env.TYRUS_DOCS_ORIGIN || 'https://tyruslang.org'
const vitepressConfig = readFileSync(CONFIG_PATH, 'utf8')
const BASE_PATH = vitepressConfig.match(/\bbase:\s*['"]([^'"]+)['"]/)?.[1]
if (!BASE_PATH || !BASE_PATH.startsWith('/') || !BASE_PATH.endsWith('/')) {
  throw new Error('VitePress base must be an absolute path ending in /.')
}
const IGNORED_DIRECTORIES = new Set(['.fragments', '.vitepress', 'es', 'pt', 'public', 'scripts'])
const INCLUDE_PATTERN = /<!--\s*@include:\s*(\S+)\s*-->/g

function collectMarkdown(directory, files = []) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (entry.isDirectory() && IGNORED_DIRECTORIES.has(entry.name)) continue
    const fullPath = join(directory, entry.name)
    if (entry.isDirectory()) collectMarkdown(fullPath, files)
    else if (entry.isFile() && entry.name.endsWith('.md')) files.push(fullPath)
  }
  return files
}

function resolveIncludes(filePath, active = new Set()) {
  if (active.has(filePath)) throw new Error(`Circular Markdown include: ${relative(DOCS_ROOT, filePath)}`)
  const nextActive = new Set(active).add(filePath)
  const content = readFileSync(filePath, 'utf8')
  return content.replace(INCLUDE_PATTERN, (_match, includePath) => {
    if (isAbsolute(includePath)) throw new Error(`Absolute Markdown include is not allowed: ${includePath}`)
    const target = resolve(dirname(filePath), includePath)
    if (target !== DOCS_ROOT && !target.startsWith(`${DOCS_ROOT}${sep}`)) {
      throw new Error(`Markdown include escapes the documentation directory: ${includePath}`)
    }
    return resolveIncludes(target, nextActive)
  })
}

function pageUrl(sourcePath) {
  const route = sourcePath.replaceAll(sep, '/').replace(/\.md$/, '')
  const publicPath = route === 'index' ? '' : route.endsWith('/index') ? `${route.slice(0, -6)}/` : route
  return new URL(`${BASE_PATH}${publicPath}`, SITE_ORIGIN).toString()
}

function pageTitle(sourcePath, content) {
  const title = content.match(/^#\s+(.+)\s*$/m)?.[1]
  return title || sourcePath.split('/').pop().replace(/\.md$/, '').replaceAll('-', ' ')
}

function collectHtml(directory, files = []) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const fullPath = join(directory, entry.name)
    if (entry.isDirectory()) collectHtml(fullPath, files)
    else if (entry.isFile() && entry.name.endsWith('.html') && entry.name !== '404.html') files.push(fullPath)
  }
  return files
}

function escapeXml(value) {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&apos;')
}

const files = collectMarkdown(DOCS_ROOT).sort((left, right) => left.localeCompare(right, 'en'))
const versionSource = readFileSync(join(DOCS_ROOT, '.vitepress', 'versions.ts'), 'utf8')
const docsVersion = versionSource.match(/latestDocsVersion[\s\S]*?tag:\s*'([^']+)'/)?.[1]
if (!docsVersion) throw new Error('Could not determine the current documentation version.')
const pages = files.map((filePath) => {
  const sourcePath = relative(DOCS_ROOT, filePath).replaceAll(sep, '/')
  const content = resolveIncludes(filePath).trim()
  return { sourcePath, content, title: pageTitle(sourcePath, content), url: pageUrl(sourcePath) }
})

const index = [
  `# Tyrus Documentation ${docsVersion}`,
  '',
  '> A programming language made for developers.',
  '> TypeScript syntax. Rust semantics.',
  '',
  '## Documentation',
  '',
  ...pages.map((page) => `- [${page.title}](${page.url}) (${page.sourcePath})`),
  ''
].join('\n')

const full = [
  `# Tyrus Full Documentation ${docsVersion}`,
  '',
  ...pages.flatMap((page) => [`---`, '', `# File: ${page.sourcePath}`, '', page.content, '']),
].join('\n')

writeFileSync(join(DIST_ROOT, 'llms.txt'), index)
writeFileSync(join(DIST_ROOT, 'llms-full.txt'), full)
const sitemapUrls = collectHtml(DIST_ROOT)
  .map((filePath) => {
    const relativePath = relative(DIST_ROOT, filePath).replaceAll(sep, '/')
    const route = relativePath === 'index.html' ? '' : relativePath.replace(/index\.html$/, '').replace(/\.html$/, '')
    return new URL(`${BASE_PATH}${route}`, SITE_ORIGIN).toString()
  })
  .sort((left, right) => left.localeCompare(right, 'en'))
const sitemap = [
  '<?xml version="1.0" encoding="UTF-8"?>',
  '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
  ...sitemapUrls.map((url) => `  <url><loc>${escapeXml(url)}</loc></url>`),
  '</urlset>',
  ''
].join('\n')
writeFileSync(join(DIST_ROOT, 'sitemap.xml'), sitemap)
process.stdout.write(`Generated llms.txt and llms-full.txt from ${pages.length} English documentation pages.\n`)
process.stdout.write(`Generated sitemap.xml with ${sitemapUrls.length} public pages.\n`)
