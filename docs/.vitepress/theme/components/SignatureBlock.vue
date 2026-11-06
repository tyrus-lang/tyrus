<template>
  <div class="signature-block">
    <span class="signature-block__label">{{ label }}</span>
    <code class="signature-block__code"><slot /></code>
  </div>
</template>

<script setup lang="ts">
defineProps<{ label: string }>()
</script>

<style scoped>
.signature-block {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin: 12px 0;
  padding: 8px 14px;
  border: 1px solid color-mix(in srgb, var(--vp-c-brand-1) 20%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--vp-c-brand-1) 4%, var(--vp-c-bg-soft) 96%);
  font-size: 13px;
  line-height: 1.5;
}

.signature-block__label {
  flex-shrink: 0;
  display: inline-block;
  padding: 1px 7px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--vp-c-brand-1) 12%, transparent);
  color: var(--vp-c-brand-1);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  line-height: 1.6;
}

.signature-block__code {
  color: var(--vp-c-text-1);
  font-family: 'JetBrains Mono', monospace;
  font-size: 13px;
  background: none;
  padding: 0;
}
</style>
