import { Registry, INITIAL, type IRawGrammar, type IRawTheme, type IOnigLib } from 'vscode-textmate';
import { loadWASM, createOnigScanner, createOnigString } from 'vscode-oniguruma';
import onigWasmUrl from 'vscode-oniguruma/release/onig.wasm?url';
import type * as monaco from 'monaco-editor/editor/editor.api';

const GRAMMAR_SCOPE = 'source.tyr';
const GRAMMAR_URL = '/grammars/tyrus.tmLanguage.json';
const THEME_URLS: Record<ThemeName, string> = {
    dark: '/themes/monokai.json',
    light: '/themes/light-plus.json',
};

const FONT_STYLE_MASK = 30720; // bits 11-14
const FONT_STYLE_OFFSET = 11;
const FOREGROUND_MASK = 16744448; // bits 15-23
const FOREGROUND_OFFSET = 15;
const FONT_STYLE_COMBINATIONS = 16; // 4 flag bits (italic, bold, underline, strikethrough)

type ThemeName = 'dark' | 'light';

interface VSCodeThemeJson {
    name?: string;
    colors?: Record<string, string>;
    tokenColors?: unknown[];
}

export interface BuiltTheme {
    themeData: monaco.editor.IStandaloneThemeData;
    tokensProvider: monaco.languages.TokensProvider;
}

let onigLibPromise: Promise<IOnigLib> | null = null;
function loadOnigLib(): Promise<IOnigLib> {
    if (!onigLibPromise) {
        onigLibPromise = fetch(onigWasmUrl)
            .then((response) => loadWASM(response))
            .then(() => ({
                createOnigScanner,
                createOnigString,
            }));
    }
    return onigLibPromise;
}

let grammarJsonPromise: Promise<IRawGrammar> | null = null;
function loadGrammarJson(): Promise<IRawGrammar> {
    if (!grammarJsonPromise) {
        grammarJsonPromise = fetch(GRAMMAR_URL).then((response) => response.json());
    }
    return grammarJsonPromise;
}

function loadThemeJson(themeName: ThemeName): Promise<VSCodeThemeJson> {
    return fetch(THEME_URLS[themeName]).then((response) => response.json());
}

function toRawTheme(vsCodeTheme: VSCodeThemeJson): IRawTheme {
    const colors = vsCodeTheme.colors ?? {};
    const defaultSetting = {
        settings: {
            foreground: colors['editor.foreground'],
            background: colors['editor.background'],
        },
    };
    return {
        name: vsCodeTheme.name,
        settings: [defaultSetting, ...((vsCodeTheme.tokenColors ?? []) as IRawTheme['settings'])],
    };
}

function fontStyleToString(fontStyle: number): string {
    const parts: string[] = [];
    if (fontStyle & 1) parts.push('italic');
    if (fontStyle & 2) parts.push('bold');
    if (fontStyle & 4) parts.push('underline');
    if (fontStyle & 8) parts.push('strikethrough');
    return parts.join(' ');
}

function tokenName(foregroundId: number, fontStyle: number): string {
    return 'mtc' + foregroundId + '-' + fontStyle;
}

function buildThemeRules(colorMap: string[], defaultForeground: string): monaco.editor.ITokenThemeRule[] {
    const rules: monaco.editor.ITokenThemeRule[] = [{ token: '', foreground: defaultForeground }];
    for (let colorId = 1; colorId < colorMap.length; colorId++) {
        const color = colorMap[colorId];
        if (!color) continue;
        const hex = color.replace('#', '');
        for (let fontStyle = 0; fontStyle < FONT_STYLE_COMBINATIONS; fontStyle++) {
            rules.push({
                token: tokenName(colorId, fontStyle),
                foreground: hex,
                fontStyle: fontStyleToString(fontStyle),
            });
        }
    }
    return rules;
}

export async function buildForTheme(themeName: ThemeName): Promise<BuiltTheme> {
    const [onigLib, rawGrammar, rawTheme] = await Promise.all([
        loadOnigLib(),
        loadGrammarJson(),
        loadThemeJson(themeName),
    ]);

    const registry = new Registry({
        onigLib: Promise.resolve(onigLib),
        loadGrammar: (scopeName) => Promise.resolve(scopeName === GRAMMAR_SCOPE ? rawGrammar : null),
        theme: toRawTheme(rawTheme),
    });

    const grammar = await registry.loadGrammar(GRAMMAR_SCOPE);
    if (!grammar) {
        throw new Error('Failed to load the "' + GRAMMAR_SCOPE + '" TextMate grammar');
    }

    const colorMap = registry.getColorMap();
    const defaultForeground = (rawTheme.colors?.['editor.foreground'] ?? colorMap[1] ?? '#000000').replace('#', '');

    const themeData: monaco.editor.IStandaloneThemeData = {
        base: themeName === 'light' ? 'vs' : 'vs-dark',
        inherit: false,
        rules: buildThemeRules(colorMap, defaultForeground),
        colors: rawTheme.colors ?? {},
    };

    const tokensProvider: monaco.languages.TokensProvider = {
        getInitialState: () => INITIAL,
        tokenize: (line, state) => {
            const result = grammar.tokenizeLine2(line, state as Parameters<typeof grammar.tokenizeLine2>[1]);
            const tokens: monaco.languages.IToken[] = [];
            for (let i = 0; i < result.tokens.length; i += 2) {
                const startIndex = result.tokens[i]!;
                const metadata = result.tokens[i + 1]!;
                const foregroundId = (metadata & FOREGROUND_MASK) >>> FOREGROUND_OFFSET;
                const fontStyle = (metadata & FONT_STYLE_MASK) >>> FONT_STYLE_OFFSET;
                tokens.push({
                    startIndex,
                    scopes: foregroundId ? tokenName(foregroundId, fontStyle) : '',
                });
            }
            return { tokens, endState: result.ruleStack };
        },
    };

    return { themeData, tokensProvider };
}
