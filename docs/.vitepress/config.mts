import { defineConfig } from 'vitepress'
import type { LanguageRegistration, ShikiTransformer } from 'shiki'
import { createNav, nav } from './nav'
import { createSidebar, sidebar } from './sidebars'
import tyrusGrammarJson from '../../editors/vscode/syntaxes/tyrus.tmLanguage.json'

const tyrusGrammar = tyrusGrammarJson as unknown as LanguageRegistration

const logo = { src: '/tyrus-logo.png', alt: 'tyrus' }

const isDecorated = (token: { htmlStyle?: Record<string, string> }) =>
  Object.entries(token.htmlStyle ?? {}).some(
    ([property, value]) => property.endsWith('text-decoration') && value !== 'inherit' && value !== 'none'
  )

const splitDecoratedIndent: ShikiTransformer = {
  name: 'tyrus:split-decorated-indent',
  tokens(lines) {
    return lines.map((line) =>
      line.flatMap((token) => {
        const split = /^(\s+)(\S[\s\S]*)$/.exec(token.content)
        if (!split || !isDecorated(token)) return [token]

        const [, whitespace, rest] = split
        return [
          { content: whitespace, offset: token.offset },
          { ...token, content: rest, offset: token.offset + whitespace.length }
        ]
      })
    )
  }
}

const searchTranslations = {
  en: {
    button: {
      buttonText: 'Search',
      buttonAriaLabel: 'Search documentation'
    },
    modal: {
      noResultsText: 'No results for',
      resetButtonTitle: 'Clear search',
      displayDetails: 'Display detailed list',
      footer: {
        selectText: 'to select',
        navigateText: 'to navigate',
        closeText: 'to close'
      }
    }
  },
  pt: {
    button: {
      buttonText: 'Buscar',
      buttonAriaLabel: 'Buscar na documentação'
    },
    modal: {
      noResultsText: 'Nenhum resultado para',
      resetButtonTitle: 'Limpar busca',
      displayDetails: 'Exibir lista detalhada',
      footer: {
        selectText: 'para selecionar',
        navigateText: 'para navegar',
        closeText: 'para fechar'
      }
    }
  },
  es: {
    button: {
      buttonText: 'Buscar',
      buttonAriaLabel: 'Buscar en la documentación'
    },
    modal: {
      noResultsText: 'Sin resultados para',
      resetButtonTitle: 'Limpiar búsqueda',
      displayDetails: 'Mostrar lista detallada',
      footer: {
        selectText: 'para seleccionar',
        navigateText: 'para navegar',
        closeText: 'para cerrar'
      }
    }
  }
}

export default defineConfig({
  title: 'Tyrus',
  description: 'TypeScript syntax. Rust semantics.',
  base: '/',
  cleanUrls: true,
  ignoreDeadLinks: false,
  appearance: 'dark',
  locales: {
    root: {
      label: 'English',
      lang: 'en-US'
    },
    pt: {
      label: 'Brazilian Portuguese',
      lang: 'pt-BR',
      link: '/pt/',
      themeConfig: {
        nav: createNav('pt'),
        sidebar: createSidebar('pt'),
        outline: {
          level: [2, 3],
          label: 'Nesta página'
        }
      }
    },
    es: {
      label: 'Spanish',
      lang: 'es',
      link: '/es/',
      themeConfig: {
        nav: createNav('es'),
        sidebar: createSidebar('es'),
        outline: {
          level: [2, 3],
          label: 'En esta página'
        }
      }
    }
  },
  head: [
    ['link', { rel: 'stylesheet', href: '/theme/style.css' }],
    ['link', { rel: 'icon', type: 'image/png', href: '/favicon.png' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:site_name', content: 'Tyrus' }],
    ['meta', { property: 'og:url', content: 'https://tyruslang.org/' }],
    ['meta', { property: 'og:title', content: 'Tyrus - TypeScript syntax Rust semantics' }],
    ['meta', { property: 'og:description', content: 'A programming language made for devs' }],
    ['meta', { property: 'og:image', content: 'https://tyruslang.org/tyrus-1200x630-compress.png' }],
    ['meta', { property: 'og:image:type', content: 'image/png' }],
    ['meta', { property: 'og:image:width', content: '1200' }],
    ['meta', { property: 'og:image:height', content: '630' }],
    ['meta', { property: 'og:image:alt', content: 'Tyrus - TypeScript syntax Rust semantics' }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
    ['meta', { name: 'twitter:title', content: 'Tyrus - TypeScript syntax Rust semantics' }],
    ['meta', { name: 'twitter:description', content: 'A programming language made for devs' }],
    ['meta', { name: 'twitter:image', content: 'https://tyruslang.org/tyrus-1200x630-compress.png' }],
    ['meta', { name: 'twitter:image:alt', content: 'Tyrus - TypeScript syntax Rust semantics' }]
  ],
  markdown: {
    theme: {
      light: 'light-plus',
      dark: 'monokai'
    },
    languages: ['typescript', 'javascript', 'bash', 'markdown', tyrusGrammar],
    languageLabel: {
      tyrus: 'tyrus'
    },
    codeTransformers: [splitDecoratedIndent],
    defaultHighlightLang: 'typescript'
  },
  themeConfig: {
    logo,
    nav,
    sidebar,
    search: {
      provider: 'local',
      options: {
        detailedView: true,
        translations: searchTranslations.en,
        locales: {
          root: {
            translations: searchTranslations.en
          },
          pt: {
            translations: searchTranslations.pt
          },
          es: {
            translations: searchTranslations.es
          }
        }
      }
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/tyrus-lang/tyrus' },
      {
        icon: {
          svg: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="m21.198 3.08l-18.6 7.16c-1.29.5-1.27 1.22-.23 1.53l4.77 1.49l11.04-6.93c.52-.31 1-.14.61.2l-8.93 8.05h-.01l-.01.01l-.35 4.98c.51 0 .73-.23 1.01-.5l2.43-2.37l5.05 3.71c.93.51 1.6.25 1.84-.87l3.33-15.63c.33-1.34-.51-1.96-1.37-1.59z"/></svg>'
        },
        link: 'https://t.me/tyruslang'
      }
    ],

    outline: {
      level: [2, 3],
      label: 'On this page'
    }
  }
})
