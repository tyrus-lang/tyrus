<template>
  <div class="terminal-block">
    <div class="terminal-block__bar">
      <span class="terminal-block__dot terminal-block__dot--red"></span>
      <span class="terminal-block__dot terminal-block__dot--yellow"></span>
      <span class="terminal-block__dot terminal-block__dot--green"></span>
      <span v-if="$slots.title" class="terminal-block__title">
        <slot name="title" />
      </span>
    </div>
    <div class="terminal-block__body">
      <slot />
    </div>
  </div>
</template>

<script setup>
defineProps({
  title: { type: String, default: '' }
})
</script>

<style scoped>
.terminal-block {
  border-radius: 12px;
  border: 1px solid rgba(49, 120, 198, 0.15);
  background: var(--vp-c-bg-soft);
  overflow: hidden;
  box-shadow:
    0 0 0 1px rgba(49, 120, 198, 0.04),
    0 4px 24px rgba(0, 0, 0, 0.15);
  transition: box-shadow 350ms ease, border-color 350ms ease;
}

.terminal-block:hover {
  border-color: rgba(49, 120, 198, 0.25);
  box-shadow:
    0 0 0 1px rgba(49, 120, 198, 0.08),
    0 8px 40px rgba(0, 0, 0, 0.2),
    0 0 60px rgba(49, 120, 198, 0.04);
}

.terminal-block__bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px;
  background: color-mix(in srgb, var(--vp-c-bg-soft) 80%, transparent);
  border-bottom: 1px solid rgba(49, 120, 198, 0.08);
}

.terminal-block__dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  transition: box-shadow 200ms ease;
}

.terminal-block__dot--red { background: #ff5f57; }
.terminal-block__dot--yellow { background: #febc2e; }
.terminal-block__dot--green { background: #28c840; }

.terminal-block:hover .terminal-block__dot--red { box-shadow: 0 0 6px rgba(255, 95, 87, 0.4); }
.terminal-block:hover .terminal-block__dot--yellow { box-shadow: 0 0 6px rgba(254, 188, 46, 0.4); }
.terminal-block:hover .terminal-block__dot--green { box-shadow: 0 0 6px rgba(40, 200, 64, 0.4); }

.terminal-block__title {
  margin-left: auto;
  color: var(--vp-c-text-3);
  font-size: 12px;
  font-family: 'JetBrains Mono', monospace;
}

.terminal-block__body {
  padding: 0;
}
</style>
