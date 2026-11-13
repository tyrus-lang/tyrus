import crypto from 'crypto';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const DOCS_ROOT = path.resolve(__dirname, '..');

const LANGS = ['en', 'pt', 'es'];
const TRANSLATIONS = ['pt', 'es'];
const IGNORED = ['.vitepress', '.fragments', 'scripts', 'public'];
const MAX_REPORTED = 12;

const localeRoot = (lang) => (lang === 'en' ? DOCS_ROOT : path.join(DOCS_ROOT, lang));
const label = (lang) => (lang === 'en' ? 'root (en)' : `/${lang}`);
const shortHash = (value) => crypto.createHash('sha1').update(value).digest('hex').slice(0, 8);

function listMarkdown(lang) {
  const root = localeRoot(lang);
  const found = [];

  const walk = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      const relative = path.relative(root, full).replace(/\\/g, '/');
      const top = relative.split('/')[0];

      if (IGNORED.includes(top)) continue;
      if (lang === 'en' && TRANSLATIONS.includes(top)) continue;

      if (entry.isDirectory()) walk(full);
      else if (entry.name.endsWith('.md')) found.push(relative);
    }
  };

  walk(root);
  return found.sort();
}

/**
 * Reduces a markdown file to its language-independent skeleton: the ordered
 * sequence of block-level constructs. Prose text is translated and therefore
 * ignored, but its shape (how many paragraphs, in which order) is not.
 */
function skeleton(text) {
  const lines = text.split('\n');
  const tokens = [];
  const frontmatter = [];
  let i = 0;

  const push = (id, description, line) => tokens.push({ id, description, line });

  if (lines[0] === '---') {
    let end = 1;
    while (end < lines.length && lines[end] !== '---') {
      const key = lines[end].match(/^([A-Za-z0-9_-]+):/);
      if (key) frontmatter.push(key[1]);
      end++;
    }
    frontmatter.sort();
    i = end + 1;
  }

  while (i < lines.length) {
    const line = lines[i].trim();
    const lineNo = i + 1;

    if (line === '') {
      i++;
      continue;
    }

    const fence = line.match(/^(`{3,}|~{3,})(.*)$/);
    if (fence) {
      const [, marker, info] = fence;
      const lang = info.trim();
      const body = [];
      i++;
      while (i < lines.length) {
        const current = lines[i].trim();
        if (current.startsWith(marker) && current.replace(/[`~]/g, '') === '') break;
        body.push(lines[i]);
        i++;
      }
      i++;
      push(`code:${lang}:${shortHash(body.join('\n'))}`, `code block (${lang || 'no language'})`, lineNo);
      continue;
    }

    const include = line.match(/^<!--\s*@include:\s*(\S+)\s*-->$/);
    if (include) {
      const fragment = path.basename(include[1]);
      push(`include:${fragment}`, `fragment include (${fragment})`, lineNo);
      i++;
      continue;
    }

    if (line.startsWith(':::')) {
      const type = line.slice(3).trim().split(/\s+/)[0];
      push(type ? `container:${type}` : 'container:end', type ? `container ::: ${type}` : 'container close', lineNo);
      i++;
      continue;
    }

    const heading = line.match(/^(#{1,6})\s+(.*)$/);
    if (heading) {
      const level = heading[1].length;
      push(`h${level}`, `heading h${level} "${heading[2].trim()}"`, lineNo);
      i++;
      continue;
    }

    const html = line.match(/^<(\/?)([A-Za-z][\w-]*)/);
    if (html) {
      const tag = `${html[1]}${html[2]}`;
      push(`html:${tag}`, `html/vue tag <${tag}>`, lineNo);
      i++;
      continue;
    }

    if (line.startsWith('|')) {
      const start = i;
      const columns = [];
      while (i < lines.length && lines[i].trim().startsWith('|')) {
        columns.push(lines[i].trim().split(/(?<!\\)\|/).length - 2);
        i++;
      }
      const rows = i - start;
      push(`table:${columns.join('-')}`, `table (${rows} rows, ${columns[0]} columns)`, start + 1);
      continue;
    }

    if (/^([-*+]|\d+\.)\s/.test(line)) {
      const start = i;
      let items = 0;
      while (i < lines.length && lines[i].trim() !== '') {
        if (/^([-*+]|\d+\.)\s/.test(lines[i].trim())) items++;
        i++;
      }
      push(`list:${items}`, `list (${items} items)`, start + 1);
      continue;
    }

    const start = i;
    while (i < lines.length && lines[i].trim() !== '' && !/^(#{1,6}\s|:::|```|~~~|\||<)/.test(lines[i].trim())) i++;
    push('p', 'paragraph', start + 1);
  }

  return { frontmatter, tokens };
}

/** Longest common subsequence over token ids, so a single insertion does not cascade. */
function diff(base, other) {
  const n = base.length;
  const m = other.length;
  const table = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1));

  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      table[i][j] = base[i].id === other[j].id
        ? table[i + 1][j + 1] + 1
        : Math.max(table[i + 1][j], table[i][j + 1]);
    }
  }

  const operations = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (base[i].id === other[j].id) {
      i++;
      j++;
    } else if (table[i + 1][j] >= table[i][j + 1]) {
      operations.push({ type: 'missing', token: base[i++] });
    } else {
      operations.push({ type: 'extra', token: other[j++] });
    }
  }
  while (i < n) operations.push({ type: 'missing', token: base[i++] });
  while (j < m) operations.push({ type: 'extra', token: other[j++] });

  return mergeChanges(operations);
}

/** A block present on both sides whose payload drifted shows up as missing+extra; report it as one change. */
function mergeChanges(operations) {
  const kindOf = (token) => {
    const [kind, lang] = token.id.split(':');
    return kind === 'code' ? `code:${lang}` : kind;
  };

  const merged = [];
  for (let i = 0; i < operations.length; i++) {
    const current = operations[i];
    const next = operations[i + 1];
    const pairs = next
      && current.type !== next.type
      && kindOf(current.token) === kindOf(next.token);

    if (pairs) {
      const base = current.type === 'missing' ? current : next;
      const other = current.type === 'missing' ? next : current;
      merged.push({ type: 'changed', token: base.token, counterpart: other.token });
      i++;
      continue;
    }
    merged.push(current);
  }
  return merged;
}

function checkSynchronization() {
  console.log('🔍 Executing translation integrity check...');

  const inventory = Object.fromEntries(LANGS.map((lang) => [lang, new Set(listMarkdown(lang))]));
  const canonical = [...new Set(LANGS.flatMap((lang) => [...inventory[lang]]))].sort();
  let hasError = false;

  for (const file of canonical) {
    const absent = LANGS.filter((lang) => !inventory[lang].has(file));
    if (absent.length === 0) continue;

    hasError = true;
    console.error(`\n🚨 [Missing File] "${file}" does not exist in every locale:`);
    for (const lang of LANGS) {
      console.error(`   - ${label(lang)}: ${inventory[lang].has(file) ? '✅ Present' : '❌ Missing'}`);
    }
  }

  const shared = canonical.filter((file) => LANGS.every((lang) => inventory[lang].has(file)));

  for (const file of shared) {
    const skeletons = Object.fromEntries(
      LANGS.map((lang) => [lang, skeleton(fs.readFileSync(path.join(localeRoot(lang), file), 'utf8'))])
    );

    const keyUnion = [...new Set(LANGS.flatMap((lang) => skeletons[lang].frontmatter))].sort();
    const unbalanced = keyUnion.filter((key) => !LANGS.every((lang) => skeletons[lang].frontmatter.includes(key)));

    if (unbalanced.length > 0) {
      hasError = true;
      console.error(`\n🚨 [Frontmatter Mismatch] "${file}" does not declare the same keys in every locale:`);
      for (const key of unbalanced) {
        const absent = LANGS.filter((lang) => !skeletons[lang].frontmatter.includes(key));
        console.error(`   - \`${key}\` missing in: ${absent.map(label).join(', ')}`);
      }
    }

    for (const lang of TRANSLATIONS) {
      const operations = diff(skeletons.en.tokens, skeletons[lang].tokens);
      if (operations.length === 0) continue;

      hasError = true;
      console.error(`\n🚨 [Structure Mismatch] "${file}" differs between ${label('en')} and ${label(lang)}:`);

      for (const { type, token, counterpart } of operations.slice(0, MAX_REPORTED)) {
        if (type === 'changed') {
          console.error(`   - ⚠️ differs in /${lang}: ${token.description}  →  ${file}:${token.line} (en) vs ${lang}/${file}:${counterpart.line}`);
          continue;
        }
        const where = type === 'missing'
          ? `${file}:${token.line} (en)`
          : `${lang}/${file}:${token.line}`;
        const verb = type === 'missing' ? `❌ missing in /${lang}` : `❌ extra in /${lang}`;
        console.error(`   - ${verb}: ${token.description}  →  ${where}`);
      }

      if (operations.length > MAX_REPORTED) {
        console.error(`   - ... and ${operations.length - MAX_REPORTED} more differences`);
      }
    }
  }

  if (hasError) {
    console.error('\n🛑 Validation failed. Synchronize the structure across languages before proceeding.');
    process.exit(1);
  }

  console.log(`✅ Alignment complete! ${shared.length} pages share the same structure across all 3 languages.`);
  process.exit(0);
}

checkSynchronization();
