import type { DefaultTheme } from 'vitepress'

export type DocsLocale = 'en' | 'pt' | 'es'

export interface DocsVersion {
  tag: string
  label: Record<DocsLocale, string>
  link: Record<DocsLocale, string>
  current: boolean
}

export const latestDocsVersion: DocsVersion = {
  tag: 'v0.1.0',
  label: {
    en: 'v0.1.0 (current)',
    pt: 'v0.1.0 (atual)',
    es: 'v0.1.0 (actual)'
  },
  link: {
    en: '/',
    pt: '/pt/',
    es: '/es/'
  },
  current: true
}

export const previousDocsVersions: DocsVersion[] = []

export const docsVersions: DocsVersion[] = [
  latestDocsVersion,
  ...previousDocsVersions
]

export function createVersionItems(locale: DocsLocale): DefaultTheme.NavItemWithLink[] {
  return docsVersions.map((version) => ({
    text: version.label[locale],
    link: version.link[locale],
    activeMatch: version.current ? '/__current-version-is-selected-from-the-menu-only__' : version.link[locale]
  }))
}
