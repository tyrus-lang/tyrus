import * as monaco from 'monaco-editor/editor/editor.api';
import EditorWorker from 'monaco-editor/editor/editor.worker?worker';
import { buildForTheme, type BuiltTheme } from './tyrus-textmate.js';

globalThis.MonacoEnvironment = {
    getWorker() {
        return new EditorWorker();
    },
};

type ThemeName = 'dark' | 'light';

let monacoEditor: monaco.editor.IStandaloneCodeEditor | null = null;
const tokensProviders: Record<ThemeName, monaco.languages.TokensProvider | null> = { dark: null, light: null };

export async function init(containerId: string): Promise<monaco.editor.IStandaloneCodeEditor> {
    monaco.languages.register({ id: 'tyrus' });
    monaco.languages.setLanguageConfiguration('tyrus', {
        brackets: [
            ['{', '}'],
            ['[', ']'],
            ['(', ')'],
        ],
        autoClosingPairs: [
            { open: '{', close: '}' },
            { open: '[', close: ']' },
            { open: '(', close: ')' },
            { open: '"', close: '"' },
            { open: "'", close: "'" },
            { open: '`', close: '`' },
        ],
        surroundingPairs: [
            { open: '{', close: '}' },
            { open: '[', close: ']' },
            { open: '(', close: ')' },
            { open: '"', close: '"' },
            { open: "'", close: "'" },
            { open: '`', close: '`' },
        ],
    });

    const [dark, light]: [BuiltTheme, BuiltTheme] = await Promise.all([buildForTheme('dark'), buildForTheme('light')]);

    tokensProviders.dark = dark.tokensProvider;
    tokensProviders.light = light.tokensProvider;

    monaco.editor.defineTheme('tyrus-dark', dark.themeData);
    monaco.editor.defineTheme('tyrus-light', light.themeData);
    monaco.languages.setTokensProvider('tyrus', tokensProviders.dark);

    const container = document.getElementById(containerId);
    if (!container) {
        throw new Error('Editor container #' + containerId + ' not found');
    }

    monacoEditor = monaco.editor.create(container, {
        value: '',
        language: 'tyrus',
        theme: 'tyrus-dark',
        automaticLayout: true,
        minimap: { enabled: false },
        fontSize: 14,
        fontFamily: "'JetBrains Mono', 'Consolas', 'Monaco', 'Courier New', monospace",
        tabSize: 4,
        scrollBeyondLastLine: false,
        renderWhitespace: 'selection',
        bracketPairColorization: { enabled: true },
        cursorBlinking: 'smooth',
        smoothScrolling: true,
        padding: { top: 10 },
    });

    return monacoEditor;
}

export function getEditor(): monaco.editor.IStandaloneCodeEditor | null {
    return monacoEditor;
}

export function setTheme(theme: string): void {
    if (!monacoEditor) return;
    const name: ThemeName = theme === 'light' ? 'light' : 'dark';
    const provider = tokensProviders[name];
    if (provider) {
        monaco.languages.setTokensProvider('tyrus', provider);
    }
    monacoEditor.updateOptions({
        theme: name === 'light' ? 'tyrus-light' : 'tyrus-dark',
    });
}
