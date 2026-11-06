import * as TyrusEditor from './editor.js';
import * as TyrusExamples from './examples.js';
import * as TyrusWorker from './worker.js';
import * as TyrusShare from './share.js';
import type { TyrusWorkerHandle } from './worker.js';
import type * as monaco from 'monaco-editor/editor/editor.api';

function $<T extends HTMLElement>(id: string): T {
    const el = document.getElementById(id);
    if (!el) {
        throw new Error('Expected element #' + id + ' to exist in index.html');
    }
    return el as T;
}

let editor: monaco.editor.IStandaloneCodeEditor | null = null;
let worker: TyrusWorkerHandle | null = null;
let isRunning = false;
// Lets Cancel actually stop the in-flight compile request. Without it the
// fetch ran to completion and the server kept working on a result nobody
// would read.
let compileAbort: AbortController | null = null;

function init(): void {
    setStatus('Loading editor...', 'running');

    TyrusEditor.init('editor')
        .then((ed) => {
            editor = ed;
            TyrusExamples.load('examples');

            worker = TyrusWorker.init();

            $('run-btn').addEventListener('click', () => {
                if (isRunning) {
                    cancelRun();
                } else {
                    runCode();
                }
            });
            $('share-btn').addEventListener('click', shareCode);
            $('theme-toggle').addEventListener('click', toggleTheme);
            $('clear-btn').addEventListener('click', clearOutput);

            document.addEventListener('keydown', (e) => {
                if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
                    e.preventDefault();
                    if (!isRunning) runCode();
                    return;
                }
                if (e.key === 'Escape' && isRunning) {
                    e.preventDefault();
                    cancelRun();
                }
            });

            initResizeHandle();

            const params = new URLSearchParams(window.location.search);
            const shortId = window.location.pathname.match(/^\/s\/([a-zA-Z0-9_-]{6,12})\/?$/)?.[1]
                ?? params.get('id');

            if (shortId) {
                loadSharedCode(shortId);
            } else {
                setStatus('Ready', '');
            }
        })
        .catch((err: Error) => {
            setStatus('Error: ' + err.message, 'error');
            console.error('Failed to initialize editor:', err);
        });
}

interface CompileDiagnostic {
    code?: string;
    message: string;
}

type OutputKind = 'stdout' | 'info' | 'error';

function outputLine(text: string, kind: OutputKind = 'stdout'): HTMLDivElement {
    const line = document.createElement('div');
    line.className = kind === 'stdout' ? 'output-line' : `output-line ${kind}`;
    line.textContent = text;
    return line;
}

function setOutput(...lines: HTMLDivElement[]): void {
    $('output').replaceChildren(...lines);
}

function appendOutput(line: HTMLDivElement): void {
    $('output').appendChild(line);
}

async function parseCompileError(response: Response): Promise<Error> {
    try {
        const data: { diagnostics?: CompileDiagnostic[]; error?: string } = await response.json();

        if (Array.isArray(data.diagnostics) && data.diagnostics.length) {
            return new Error(data.diagnostics.map((d) => (d.code ? d.code + ': ' : '') + d.message).join('\n'));
        }

        return new Error(data.error || 'Compilation failed');
    } catch {
        return new Error('Compilation failed');
    }
}

function runCode(): void {
    if (isRunning || !editor || !worker) return;

    const code = editor.getValue();
    const runBtn = $<HTMLButtonElement>('run-btn');
    const activeWorker = worker;

    if (!code.trim()) {
        setOutput(outputLine('No code to run.', 'info'));
        return;
    }

    isRunning = true;
    runBtn.disabled = false;
    runBtn.textContent = '⏹ Cancel';
    setOutput(outputLine('Compiling...', 'info'));
    setStatus('Compiling...', 'running');

    compileAbort = new AbortController();

    fetch('/api/playground/compile', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ code }),
        signal: compileAbort.signal,
    })
        .then((response) => {
            if (!response.ok) {
                return parseCompileError(response).then((err) => {
                    throw err;
                });
            }
            return response;
        })
        .then((response) => {
            if (!isRunning) return null;

            setOutput(outputLine('Running...', 'info'));
            setStatus('Running...', 'running');

            const stream = response.body;
            if (!stream) {
                throw new Error('Compile response had no body');
            }
            return activeWorker.postMessage({ type: 'run', stream }, [stream], TyrusWorker.RUN_TIMEOUT_MS);
        })
        .then((execResult) => {
            if (!execResult) return;

            const lines: HTMLDivElement[] = [];
            if (execResult.stdout) {
                lines.push(outputLine(execResult.stdout));
            }
            if (execResult.stderr) {
                lines.push(outputLine(execResult.stderr, 'error'));
            }
            if (execResult.error) {
                lines.push(outputLine('Error: ' + execResult.error, 'error'));
            }
            if (lines.length === 0) {
                lines.push(outputLine('(no output)', 'info'));
            }

            setOutput(...lines);
            setStatus('Done', '');
        })
        .catch((err: Error) => {
            // An abort is the user's own doing; cancelRun already said so.
            if (!isRunning || err.name === 'AbortError') return;
            setOutput(outputLine(err.message, 'error'));
            setStatus('Error', 'error');
        })
        .finally(() => {
            isRunning = false;
            compileAbort = null;
            runBtn.disabled = false;
            runBtn.textContent = '▶ Run';
        });
}

function cancelRun(): void {
    if (!isRunning) return;
    isRunning = false;
    compileAbort?.abort();
    compileAbort = null;
    if (worker) worker.reset();
    const runBtn = $<HTMLButtonElement>('run-btn');
    runBtn.disabled = false;
    runBtn.textContent = '▶ Run';
    appendOutput(outputLine('Execution cancelled.', 'info'));
    setStatus('Cancelled', 'error');
}

function shareCode(): void {
    if (!editor) return;
    const code = editor.getValue();
    const shareBtn = $<HTMLButtonElement>('share-btn');

    if (!code.trim()) {
        showShareModal(null, 'No code to share.');
        return;
    }

    shareBtn.disabled = true;
    shareBtn.textContent = '⏳ Sharing';

    TyrusShare.share(code)
        .then((url) => {
            showShareModal(url, null);
        })
        .catch((err: Error) => {
            showShareModal(null, err.message);
        })
        .finally(() => {
            shareBtn.disabled = false;
            shareBtn.textContent = 'Share';
        });
}

function loadSharedCode(shortId: string): void {
    setStatus('Loading shared code...', 'running');

    TyrusShare.load(shortId)
        .then((code) => {
            editor?.setValue(code);
            setStatus('Ready', '');
        })
        .catch((err: Error) => {
            setOutput(outputLine('Failed to load shared code: ' + err.message, 'error'));
            setStatus('Error', 'error');
        });
}

function showShareModal(url: string | null, error: string | null): void {
    document.querySelector('.modal-overlay')?.remove();

    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay';

    const modal = document.createElement('div');
    modal.className = 'modal';

    const heading = document.createElement('h2');
    heading.textContent = 'Share Code';
    modal.appendChild(heading);

    if (error) {
        const message = document.createElement('p');
        message.className = 'modal-error';
        message.textContent = error;
        modal.appendChild(message);
    } else {
        const field = document.createElement('input');
        field.type = 'text';
        field.readOnly = true;
        field.id = 'share-url';
        field.value = url ?? '';
        modal.appendChild(field);
    }

    const actions = document.createElement('div');
    actions.className = 'actions';

    const closeBtn = document.createElement('button');
    closeBtn.className = 'close-btn';
    closeBtn.textContent = 'Close';
    closeBtn.addEventListener('click', () => overlay.remove());
    actions.appendChild(closeBtn);

    if (url) {
        const copyBtn = document.createElement('button');
        copyBtn.className = 'copy-btn';
        copyBtn.textContent = 'Copy URL';
        copyBtn.addEventListener('click', () => {
            navigator.clipboard.writeText(url).then(() => {
                copyBtn.textContent = 'Copied!';
                setTimeout(() => {
                    copyBtn.textContent = 'Copy URL';
                }, 2000);
            });
        });
        actions.appendChild(copyBtn);
    }

    modal.appendChild(actions);
    overlay.appendChild(modal);
    document.body.appendChild(overlay);

    overlay.addEventListener('click', (e) => {
        if (e.target === overlay) overlay.remove();
    });

    const onEscape = (e: KeyboardEvent): void => {
        if (e.key !== 'Escape') return;
        e.stopPropagation();
        overlay.remove();
        document.removeEventListener('keydown', onEscape, true);
    };
    // Capture phase, so closing the dialog wins over the editor's own Escape
    // handling while the dialog is the thing in front of the user.
    document.addEventListener('keydown', onEscape, true);

    closeBtn.focus();
}

function clearOutput(): void {
    $('output').replaceChildren();
    setStatus('Ready', '');
}

function toggleTheme(): void {
    const body = document.body;
    const current = body.getAttribute('data-theme');
    const next = current === 'light' ? 'dark' : 'light';
    body.setAttribute('data-theme', next);
    $('theme-toggle').textContent = next === 'dark' ? '☀️' : '🌙';
    TyrusEditor.setTheme(next);
}

function setStatus(text: string, state: string): void {
    const el = $('status');
    el.textContent = text;
    el.className = 'status' + (state ? ' ' + state : '');
}

const isColumnLayout = (): boolean => window.matchMedia('(max-width: 768px)').matches;

function initResizeHandle(): void {
    const handle = $('resize-handle');
    const main = document.querySelector<HTMLElement>('main');
    const editorContainer = $('editor-container');
    const outputContainer = $('output-container');
    let dragging = false;

    if (!main) return;


    handle.addEventListener('pointerdown', (e) => {
        e.preventDefault();
        dragging = true;
        handle.classList.add('active');
        document.body.style.cursor = isColumnLayout() ? 'row-resize' : 'col-resize';
        document.body.style.userSelect = 'none';
    });

    document.addEventListener('pointermove', (e) => {
        if (!dragging) return;
        const rect = main.getBoundingClientRect();

        if (isColumnLayout()) {
            const total = rect.height;
            const handleH = handle.offsetHeight;
            const minEditor = Math.round(total * 0.2);
            const minOutput = Math.round(total * 0.15);
            const maxEditor = total - minOutput - handleH;

            const editorH = Math.max(minEditor, Math.min(maxEditor, e.clientY - rect.top));
            const outputH = total - editorH - handleH;

            editorContainer.style.flex = 'none';
            editorContainer.style.height = editorH + 'px';
            editorContainer.style.width = '';
            outputContainer.style.flex = 'none';
            outputContainer.style.height = outputH + 'px';
            outputContainer.style.width = '';
        } else {
            const x = e.clientX - rect.left;
            const total = rect.width;
            const handleW = handle.offsetWidth;
            const minEditor = 200;
            const minOutput = 250;
            const maxEditor = total - minOutput - handleW;

            const editorW = Math.max(minEditor, Math.min(maxEditor, x));
            const outputW = total - editorW - handleW;

            editorContainer.style.flex = 'none';
            editorContainer.style.width = editorW + 'px';
            editorContainer.style.height = '';
            outputContainer.style.flex = 'none';
            outputContainer.style.width = outputW + 'px';
            outputContainer.style.height = '';
        }

        if (editor) editor.layout();
    });

    const endDrag = (): void => {
        if (!dragging) return;
        dragging = false;
        handle.classList.remove('active');
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
    };

    document.addEventListener('pointerup', endDrag);
    document.addEventListener('pointercancel', endDrag);
}

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
