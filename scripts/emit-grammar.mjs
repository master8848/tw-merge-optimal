// emit-grammar.mjs — derive the family->conflict (W) table for the
// tw-merge-optimal matcher-only bundle directly from the REAL Tailwind
// compiler (v4.3.3+), instead of the hand-maintained vendor CSS catalog +
// hand-written Rust tables in crates/twm-core/src/families.rs.
//
// Usage (needs bun; imports the Tailwind TS source directly):
//   bun scripts/emit-grammar.mjs --out /tmp/derived.mjs
//   bun scripts/emit-grammar.mjs --classes /tmp/classes.json --out /tmp/derived.mjs
//
// Options:
//   --out PATH       output bundle path (default: /tmp/derived.mjs)
//   --meta PATH      write machine-readable stats/unresolved list (default: /tmp/derived-meta.json)
//   --classes PATH   use a JSON array of class tokens instead of parsing the Rust files
//   --tailwind DIR   path to the tailwindcss package dir (default: ../tailwindcss/packages/tailwindcss
//                    relative to the repo root; env TAILWIND_PKG also honored)
//
// The derived W replaces the generated one (the matcher bundle's 'const W='
// line, swapped in place). Note: the derived family numbering follows this
// script's own derivation order (first-seen signature), so the swapped W is
// self-consistent with the derived family ids in --meta — the matcher
// records (P), arbitrary-property map (PR) and postfix-special ids (LD/FS/
// CT/CN) keep the Rust generator's numbering. The point of the script is the
// derivation itself: families + conflicts straight from the real compiler.
//
// The class list default = union of the corpus tokens (corpus_data.rs), the
// tailwind-merge benchmark collection (bench/tw-merge-benchmark-data.json) and
// the 200-round synthetic list (bench_gen.rs), mirroring bench_gen.rs.

import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url))
const REPO = join(SCRIPT_DIR, '..')

function arg(name) {
  const i = process.argv.indexOf(name)
  return i !== -1 ? process.argv[i + 1] : undefined
}

const OUT = arg('--out') ?? '/tmp/derived.mjs'
const META = arg('--meta') ?? '/tmp/derived-meta.json'
const CLASSES_FILE = arg('--classes')
const TAILWIND_PKG =
  process.env.TAILWIND_PKG ?? arg('--tailwind') ?? join(REPO, '..', 'tailwindcss', 'packages', 'tailwindcss')

// ---------------------------------------------------------------------------
// 1. Class list (mirrors bench_gen.rs)
// ---------------------------------------------------------------------------

function quotedStrings(raw) {
  return raw
    .split('"')
    .slice(1)
    .filter((_, i) => i % 2 === 0)
}

function readClassTokens() {
  if (CLASSES_FILE) {
    return JSON.parse(readFileSync(CLASSES_FILE, 'utf8'))
  }
  const corpusRaw = readFileSync(join(REPO, 'crates/twm-core/tests/corpus_data.rs'), 'utf8')
  const benchRaw = readFileSync(join(REPO, 'bench/tw-merge-benchmark-data.json'), 'utf8')

  const tokens = []
  const pairRe = /^ {8}\("((?:[^"\\]|\\.)*)", "((?:[^"\\]|\\.)*)"\),?$/gm
  const unescape = (s) => s.replace(/\\(["\\])/g, '$1')
  let m
  while ((m = pairRe.exec(corpusRaw))) {
    for (const s of [unescape(m[1]), unescape(m[2])]) {
      for (const t of s.split(/\s+/)) if (t) tokens.push(t)
    }
  }

  for (const s of quotedStrings(benchRaw)) {
    for (const t of s.split(/\s+/)) if (t) tokens.push(t)
  }

  for (let i = 0; i < 200; i++) {
    for (const p of ['p', 'px', 'py', 'm', 'mx', 'my', 'w', 'h']) {
      tokens.push(`${p}-${i % 20}`)
    }
    tokens.push(`text-${i % 10}`, `bg-${i % 10}`)
    if (i % 10 === 0) {
      tokens.push(`hover:p-${i % 20}`, `focus:m-${i % 20}`)
    }
  }
  return tokens
}

// Strip variants (colon segments outside brackets), important (`!`), and
// postfix (`/…`) from a token, mirroring the JS p() + Rust parse_class_name.
function baseOf(token) {
  let s = token
  if (s.startsWith('!')) s = s.slice(1)
  if (s.endsWith('!')) s = s.slice(0, -1)
  let bs = 0
  let ps = 0
  let pp = -1
  let lastColon = -1
  for (let i = 0; i < s.length; i++) {
    const ch = s[i]
    if (bs === 0 && ps === 0) {
      if (ch === ':') lastColon = i
      if (ch === '/' && pp < 0) pp = i
    }
    if (ch === '[') bs++
    else if (ch === ']') bs--
    else if (ch === '(') ps++
    else if (ch === ')') ps--
  }
  const start = lastColon + 1
  let base = pp > start ? s.slice(start, pp) : s.slice(start)
  if (base.startsWith('!')) base = base.slice(1)
  return base
}

function hasPostfix(token) {
  let s = token
  if (s.startsWith('!')) s = s.slice(1)
  if (s.endsWith('!')) s = s.slice(0, -1)
  let bs = 0
  let ps = 0
  let lastColon = -1
  for (let i = 0; i < s.length; i++) {
    const ch = s[i]
    if (bs === 0 && ps === 0 && ch === ':') lastColon = i
    if (ch === '[') bs++
    else if (ch === ']') bs--
    else if (ch === '(') ps++
    else if (ch === ')') ps--
  }
  const start = lastColon + 1
  let bs2 = 0
  let ps2 = 0
  for (let i = start; i < s.length; i++) {
    const ch = s[i]
    if (bs2 === 0 && ps2 === 0 && ch === '/') return true
    if (ch === '[') bs2++
    else if (ch === ']') bs2--
    else if (ch === '(') ps2++
    else if (ch === ')') ps2--
  }
  return false
}

// `hover:!m-2/3` -> `m-2/3`
function stripVariantsAndImportant(token) {
  let s = token
  if (s.startsWith('!')) s = s.slice(1)
  if (s.endsWith('!')) s = s.slice(0, -1)
  let bs = 0
  let ps = 0
  let lastColon = -1
  for (let i = 0; i < s.length; i++) {
    const ch = s[i]
    if (bs === 0 && ps === 0 && ch === ':') lastColon = i
    if (ch === '[') bs++
    else if (ch === ']') bs--
    else if (ch === '(') ps++
    else if (ch === ')') ps--
  }
  return s.slice(lastColon + 1)
}

const tokens = readClassTokens()
const bases = new Set(tokens.map(baseOf))
// Full token with the variant/important parts stripped, so the compiler
// receives plain classes: `hover:@container-size/sidebar` -> `@container-size/sidebar`.
const postfixTokens = [...new Set(tokens.filter(hasPostfix).map(stripVariantsAndImportant))]
const compileList = [...new Set([...bases, ...postfixTokens])].sort()

// ---------------------------------------------------------------------------
// 2. Compile every class with the REAL compiler
// ---------------------------------------------------------------------------

const tailwindIndex = pathToFileURL(join(TAILWIND_PKG, 'src/index.ts')).href
const { __unstable__loadDesignSystem } = await import(tailwindIndex)

const ds = await __unstable__loadDesignSystem(`@import "tailwindcss";`, {
  base: TAILWIND_PKG,
  loadStylesheet: async (id, base) => {
    const p = id === 'tailwindcss' ? join(TAILWIND_PKG, 'index.css') : join(base, id)
    return { path: p, base, content: readFileSync(p, 'utf8') }
  },
})

const compiled = ds.candidatesToCss(compileList)

// Extract the property-name set from a candidate's CSS. Skips @property /
// @media / other at-rule blocks; collects declarations from the class rule
// and any nested style rules (`&::before`, `:where(…)`, …).
function propsOfCss(css) {
  const props = new Set()
  let i = 0
  let topLevel = true
  while (i < css.length) {
    // Find the next '{'; decide if the preceding header is an at-rule.
    const open = css.indexOf('{', i)
    if (open === -1) break
    const header = css.slice(i, open).trim()
    const isAtRule = header.startsWith('@')
    // Find matching '}'
    let depth = 1
    let close = open + 1
    while (depth > 0 && close < css.length) {
      const ch = css[close]
      if (ch === '{') depth++
      else if (ch === '}') depth--
      close++
    }
    const body = css.slice(open + 1, close - 1)
    if (!isAtRule) {
      // Parse declarations + nested rules inside a style-rule body.
      let j = 0
      let declStart = 0
      let dbs = 0
      let dps = 0
      while (j < body.length) {
        const ch = body[j]
        if (ch === ';' && dbs === 0 && dps === 0) {
          const decl = body.slice(declStart, j).trim()
          if (decl && !decl.includes('{')) {
            const colon = decl.indexOf(':')
            if (colon > 0) props.add(decl.slice(0, colon).trim())
          }
          declStart = j + 1
        } else if (ch === '{') {
          // Nested style rule: recurse into it.
          const subOpen = j
          let subDepth = 1
          let k = subOpen + 1
          while (subDepth > 0 && k < body.length) {
            const c = body[k]
            if (c === '{') subDepth++
            else if (c === '}') subDepth--
            k++
          }
          for (const p of propsOfCss(body.slice(subOpen, k))) props.add(p)
          declStart = k
          j = k
          continue
        }
        if (ch === '[' || ch === '(') {
          const closeCh = ch === '[' ? ']' : ')'
          const end = body.indexOf(closeCh, j)
          if (end !== -1) {
            j = end
            continue
          }
        }
        j++
      }
      const tail = body.slice(declStart).trim()
      if (tail && !tail.includes('{')) {
        const colon = tail.indexOf(':')
        if (colon > 0) props.add(tail.slice(0, colon).trim())
      }
    }
    i = close
  }
  return props
}

const signatures = new Map() // base -> sorted prop array (undefined if unresolved)
const unresolved = []
for (let i = 0; i < compileList.length; i++) {
  const cls = compileList[i]
  const css = compiled[i]
  if (!css) {
    unresolved.push(cls)
    continue
  }
  const props = propsOfCss(css)
  if (props.size === 0) {
    unresolved.push(cls)
    continue
  }
  signatures.set(cls, [...props].sort())
}

// ---------------------------------------------------------------------------
// 3. Derive families (signature equivalence) and conflicts (subset rule)
// ---------------------------------------------------------------------------

const sigKey = (sig) => sig.join('|')
const familyBySig = new Map() // sigKey -> family id
const familySigs = [] // family id -> Set(prop)
const familyId = (sig) => {
  const k = sigKey(sig)
  if (familyBySig.has(k)) return familyBySig.get(k)
  const id = familySigs.length
  familyBySig.set(k, id)
  familySigs.push(new Set(sig))
  return id
}

const fullByBase = new Map()
for (const t of postfixTokens) {
  const b = baseOf(t)
  if (!fullByBase.has(b)) fullByBase.set(b, [])
  fullByBase.get(b).push(t)
}

for (const base of bases) {
  let sig = signatures.get(base)
  const fulls = fullByBase.get(base) ?? []
  const fullSigs = fulls.map((t) => signatures.get(t)).filter(Boolean)
  if (!sig && fullSigs.length > 0) {
    // aspect-8.5/11: base alone doesn't compile, postfix form does.
    sig = fullSigs[0]
  }
  if (!sig) continue
  // Register the base's family (and the postfix forms' families) so the
  // derived W covers them. (The old G entries for postfix keys and
  // arbitrary fallbacks are gone — matcher mode has no G.)
  familyId(sig)
  for (const fsig of fullSigs) familyId(fsig)
}

const N = familySigs.length
const W = Array.from({ length: N }, (_, f) => {
  const out = []
  for (let g = 0; g < N; g++) {
    if (g === f) {
      out.push(g)
      continue
    }
    let sub = true
    for (const p of familySigs[g]) {
      if (!familySigs[f].has(p)) {
        sub = false
        break
      }
    }
    if (sub) out.push(g)
  }
  return out
})

// ---------------------------------------------------------------------------
// 4. Emit the derived bundle (same shape/runtime as the matcher-only bundle)
// ---------------------------------------------------------------------------

const bundle = readFileSync(join(REPO, 'bench/generated/tw-merge-optimal.mjs'), 'utf8')
const lines = bundle.split('\n')
const wIdx = lines.findIndex((l) => l.startsWith('const W='))
if (wIdx === -1) {
  throw new Error('could not locate the W line in the matcher-only bundle')
}

const version = JSON.parse(readFileSync(join(TAILWIND_PKG, 'package.json'), 'utf8')).version
const wJson = W.map((a) => `[${a.join(',')}]`).join(',')

lines[0] = `// Derived by scripts/emit-grammar.mjs from tailwindcss v${version} (${TAILWIND_PKG}). Do not edit.`
lines[wIdx] = `const W=[${wJson}];`

writeFileSync(OUT, lines.join('\n'))

const meta = {
  tailwindVersion: version,
  classTokens: tokens.length,
  uniqueTokens: new Set(tokens).size,
  compileList: compileList.length,
  unresolved: unresolved,
  families: N,
  wTotal: W.reduce((a, b) => a + b.length, 0),
  familySigs: familySigs.map((s) => [...s]),
  out: OUT,
}
writeFileSync(META, JSON.stringify(meta, null, 2))

console.log(JSON.stringify(meta, null, 2))
