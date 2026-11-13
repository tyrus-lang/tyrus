<template>
  <div class="stat-counter">
    <div class="stat-counter__value">
      <span ref="numberEl" class="stat-counter__number">{{ display }}</span>
      <span v-if="suffix" class="stat-counter__suffix">{{ suffix }}</span>
    </div>
    <div class="stat-counter__label">{{ label }}</div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'

const props = defineProps({
  value: { type: Number, required: true },
  label: { type: String, required: true },
  suffix: { type: String, default: '' },
  duration: { type: Number, default: 1200 }
})

const display = ref(0)
const numberEl = ref(null)
let observer = null

const animate = (target) => {
  const start = performance.now()
  const step = (now) => {
    const elapsed = now - start
    const progress = Math.min(elapsed / props.duration, 1)
    const eased = 1 - Math.pow(1 - progress, 3)
    display.value = Math.round(eased * target)
    if (progress < 1) requestAnimationFrame(step)
  }
  requestAnimationFrame(step)
}

onMounted(() => {
  observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          animate(props.value)
          observer.disconnect()
        }
      })
    },
    { threshold: 0.3 }
  )
  if (numberEl.value) observer.observe(numberEl.value)
})

onUnmounted(() => observer?.disconnect())
</script>

<style scoped>
.stat-counter {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 20px 24px;
  border-radius: 14px;
  background: color-mix(in srgb, var(--vp-c-bg-soft) 50%, transparent);
  border: 1px solid rgba(49, 120, 198, 0.1);
  transition: border-color 300ms ease, box-shadow 300ms ease;
}

.stat-counter:hover {
  border-color: rgba(49, 120, 198, 0.2);
  box-shadow: 0 0 30px rgba(49, 120, 198, 0.06);
}

.stat-counter__value {
  display: flex;
  align-items: baseline;
  gap: 2px;
}

.stat-counter__number {
  font-size: 36px;
  font-weight: 800;
  line-height: 1;
  background: linear-gradient(135deg, var(--vp-c-brand-1), rgba(96, 165, 250, 0.9));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.stat-counter__suffix {
  font-size: 18px;
  font-weight: 600;
  color: var(--vp-c-brand-1);
}

.stat-counter__label {
  color: var(--vp-c-text-2);
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.02em;
  text-align: center;
}
</style>
