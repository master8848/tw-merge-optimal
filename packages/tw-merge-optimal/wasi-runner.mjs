// Internal runner: executes the twm-gen WASI module under Node's built-in
// WASI runtime (node:wasi). Invoked by cli.mjs as
// `node wasi-runner.mjs <twm-gen.wasm> [args...]` so stdout/stderr/exit-code
// semantics match a native binary.
//
// Filesystem access is sandboxed to the pre-opened directories: the process
// cwd, plus the directories of every absolute path argument and every
// `--css`/`--out` value.
import { mkdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, isAbsolute, resolve } from 'node:path';
import { WASI } from 'node:wasi';

const [wasmPath, ...args] = process.argv.slice(2);

const dirs = new Set([process.cwd()]);
for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === '--out') {
        if (args[i + 1]) {
            const outDir = dirname(resolve(args[i + 1]));
            mkdirSync(outDir, { recursive: true });
            dirs.add(outDir);
        }
        i += 1;
    } else if (a === '--css') {
        if (args[i + 1]) dirs.add(dirname(resolve(args[i + 1])));
        i += 1;
    } else if (!a.startsWith('-') && isAbsolute(a)) {
        dirs.add(dirname(a));
    }
}
const preopens = { '.': process.cwd() };
for (const d of dirs) {
    if (d !== process.cwd()) {
        try {
            const st = statSync(d);
            if (st.isDirectory()) preopens[d] = d;
        } catch {
            // path doesn't exist — skip; the engine will report it
        }
    }
}

const major = Number(process.versions.node.split('.')[0]);
const wasi = new WASI(
    major >= 20
        ? { version: 'preview1', args: ['twm-gen', ...args], env: process.env, preopens, returnOnExit: true }
        : { args: ['twm-gen', ...args], env: process.env, preopens, returnOnExit: true }
);

const module = await WebAssembly.compile(readFileSync(wasmPath));
const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
const code = wasi.start(instance);
if (code) process.exitCode = code;
