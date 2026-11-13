<template>
  <div class="glow-card" :class="[`glow-card--${size}`]">
    <div class="glow-card__shine"></div>
    <div class="glow-card__content">
      <div v-if="$slots.icon" class="glow-card__icon">
        <slot name="icon" />
      </div>
      <div v-if="$slots.badge" class="glow-card__badge">
        <slot name="badge" />
      </div>
      <h3 v-if="$slots.title" class="glow-card__title">
        <slot name="title" />
      </h3>
      <p v-if="$slots.description" class="glow-card__description">
        <slot name="description" />
      </p>
      <div v-if="$slots.default" class="glow-card__body">
        <slot />
      </div>
    </div>
  </div>
</template>

<script setup>
defineProps({
  size: { type: String, default: 'md', validator: (v) => ['sm', 'md', 'lg'].includes(v) }
})
</script>

<style scoped>
.glow-card {
  position: relative;
  border-radius: 16px;
  background: color-mix(in srgb, var(--vp-c-bg-soft) 60%, transparent);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(49, 120, 198, 0.12);
  overflow: hidden;
  transition: transform 350ms cubic-bezier(0.22, 1, 0.36, 1), border-color 350ms ease, box-shadow 350ms ease;
}

.glow-card:hover {
  transform: translateY(-4px);
  border-color: rgba(49, 120, 198, 0.3);
  box-shadow:
    0 0 0 1px rgba(49, 120, 198, 0.1),
    0 8px 40px rgba(49, 120, 198, 0.12),
    0 0 80px rgba(96, 165, 250, 0.06);
}

.glow-card__shine {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(49, 120, 198, 0.4), rgba(96, 165, 250, 0.3), transparent);
  opacity: 0;
  transition: opacity 350ms ease;
}

.glow-card:hover .glow-card__shine {
  opacity: 1;
}

.glow-card__content {
  padding: 28px;
}

.glow-card--sm .glow-card__content { padding: 20px; }
.glow-card--lg .glow-card__content { padding: 36px; }

.glow-card__icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 52px;
  height: 52px;
  margin-bottom: 18px;
  border-radius: 14px;
  background: linear-gradient(135deg, rgba(49, 120, 198, 0.12), rgba(96, 165, 250, 0.06));
  border: 1px solid rgba(49, 120, 198, 0.15);
  color: var(--vp-c-brand-1);
  font-size: 24px;
}

.glow-card__icon :deep(svg) {
  width: 24px;
  height: 24px;
}

.glow-card__badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 14px;
  padding: 3px 10px;
  border-radius: 999px;
  background: linear-gradient(135deg, rgba(49, 120, 198, 0.1), rgba(96, 165, 250, 0.06));
  border: 1px solid rgba(49, 120, 198, 0.15);
  color: var(--vp-c-brand-1);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.glow-card__title {
  margin: 0 0 10px;
  color: var(--vp-c-text-1);
  font-size: 20px;
  font-weight: 700;
  line-height: 1.3;
}

.glow-card--sm .glow-card__title { font-size: 17px; }
.glow-card--lg .glow-card__title { font-size: 24px; }

.glow-card__description {
  margin: 0;
  color: var(--vp-c-text-2);
  font-size: 15px;
  line-height: 1.65;
}

.glow-card__body {
  margin-top: 16px;
}
</style>
