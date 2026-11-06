<script setup lang="ts">
import { useData, useRouter } from 'vitepress'
import DefaultTheme from 'vitepress/theme'
import { nextTick, onMounted, provide } from 'vue'

const { isDark } = useData()
const router = useRouter()

let localeScrollRatio: number | null = null

const stripLocalePrefix = (path: string) => {
  const stripped = path.replace(/^\/(pt|es)(?=\/|$)/, '')
  return stripped === '' ? '/' : stripped
}

const isLocaleSwitch = (href: string) => {
  if (window.location.hash) return false
  const target = new URL(href, window.location.origin).pathname
  const current = window.location.pathname
  return current !== target && stripLocalePrefix(current) === stripLocalePrefix(target)
}

const captureLocaleScroll = (href: string) => {
  if (!isLocaleSwitch(href)) {
    localeScrollRatio = null
    return
  }
  const maxScroll = document.documentElement.scrollHeight - window.innerHeight
  localeScrollRatio = maxScroll > 0 ? window.scrollY / maxScroll : 0
}

const enableTransitions = () =>
  'startViewTransition' in document &&
  window.matchMedia('(prefers-reduced-motion: no-preference)').matches

provide('toggle-appearance', async ({ clientX: x, clientY: y }: MouseEvent) => {
  if (!enableTransitions()) {
    isDark.value = !isDark.value
    return
  }

  const clipPath = [
    `circle(0px at ${x}px ${y}px)`,
    `circle(${Math.hypot(
      Math.max(x, innerWidth - x),
      Math.max(y, innerHeight - y)
    )}px at ${x}px ${y}px)`
  ]

  await document.startViewTransition(async () => {
    isDark.value = !isDark.value
    await nextTick()
  }).ready

  document.documentElement.animate(
    { clipPath: isDark.value ? clipPath.reverse() : clipPath },
    {
      duration: 300,
      easing: 'ease-in',
      fill: 'forwards',
      pseudoElement: `::view-transition-${isDark.value ? 'old' : 'new'}(root)`
    }
  )
})

onMounted(() => {
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          const el = entry.target
          el.classList.remove('tyrus-visible')
          void (el as HTMLElement).offsetHeight
          el.classList.add('tyrus-visible')
        } else {
          entry.target.classList.remove('tyrus-visible')
        }
      }
    },
    { threshold: 0, rootMargin: '0px 0px -120px 0px' }
  )

  const observeSections = () => {
    document.querySelectorAll('.tyrus-home-section').forEach((el) => {
      observer.observe(el)
    })
  }

  observeSections()

  router.onBeforeRouteChange = captureLocaleScroll

  router.onAfterRouteChange = async () => {
    const restoreRatio = localeScrollRatio
    localeScrollRatio = null

    await nextTick()

    if (enableTransitions()) {
      const content = document.querySelector('.VPContent')
      if (content) {
        content.classList.remove('tyrus-page-enter')
        void (content as HTMLElement).offsetHeight
        content.classList.add('tyrus-page-enter')
      }
    }

    if (restoreRatio !== null) {
      const maxScroll = document.documentElement.scrollHeight - window.innerHeight
      window.scrollTo(0, Math.round(restoreRatio * maxScroll))
    }

    observer.disconnect()
    observeSections()
  }
})
</script>

<template>
  <DefaultTheme.Layout />
  <TyrusFooter />
</template>

<style>
::view-transition-old(root),
::view-transition-new(root) {
  animation: none;
  mix-blend-mode: normal;
}

::view-transition-old(root),
.dark::view-transition-new(root) {
  z-index: 1;
}

::view-transition-new(root),
.dark::view-transition-old(root) {
  z-index: 9999;
}

.VPSwitchAppearance {
  width: 22px !important;
}

.VPSwitchAppearance .check {
  transform: none !important;
}

@keyframes tyrus-page-fade-in {
  from {
    opacity: 0;
    filter: blur(6px);
    transform: translateX(-20px);
  }
  to {
    opacity: 1;
    filter: blur(0);
    transform: translateX(0);
  }
}

.tyrus-page-enter {
  animation: tyrus-page-fade-in 0.4s cubic-bezier(0.22, 1, 0.36, 1) both;
}
</style>
