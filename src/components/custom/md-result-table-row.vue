<template>
  <div class="result-table-row">
    <slot />
  </div>
</template>

<style scoped>
@reference "@assets/main.css";

.result-table-row {
  position: relative;
  padding-inline: var(--result-table-content-inline-padding, 14px);
  background: transparent;
}

.result-table-row::before {
  position: absolute;
  top: var(--result-item-background-inset, 3px);
  right: 0;
  bottom: var(--result-item-background-inset, 3px);
  left: 0;
  border-radius: 8px;
  content: '';
  pointer-events: none;
  @apply transition-colors duration-200;
}

.result-table-row:hover,
.result-table-row:is([data-selected='true'], [data-expanded='true']) {
  background: transparent;
}

.result-table-row:hover::before,
.result-table-row:focus-within::before {
  @apply bg-muted/60;
}

.result-table-row:is([data-selected='true'], [data-expanded='true'])::before {
  @apply bg-primary/5;
}

.result-table-row:is([data-selected='true'], [data-expanded='true']):is(:hover, :focus-within)::before {
  @apply bg-muted/60;
}

.result-table-row > :deep(*) {
  position: relative;
}
</style>
