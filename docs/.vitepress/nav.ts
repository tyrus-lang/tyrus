import type { DefaultTheme } from 'vitepress'
import { createVersionItems, type DocsLocale } from './versions'

const labels: Record<DocsLocale, Record<string, string>> = {
  en: {
    docs: 'Docs',
    versions: 'Versions'
  },
  pt: {
    docs: 'Docs',
    versions: 'Versões'
  },
  es: {
    docs: 'Docs',
    versions: 'Versiones'
  }
}

export function createNav(locale: DocsLocale): DefaultTheme.NavItem[] {
  const t = labels[locale]
  const withLocale = (path: string) => locale === 'en' ? path : `/${locale}${path}`

  return [
    { text: t.docs, link: withLocale('/docs/getting-started') },
    {
      text: t.versions,
      items: createVersionItems(locale)
    }
  ]
}

export const nav = createNav('en')
