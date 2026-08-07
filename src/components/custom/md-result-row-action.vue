<script setup lang="ts">
withDefaults(
  defineProps<{
    title: string;
    ariaLabel?: string;
    destructive?: boolean;
    disabled?: boolean;
    variant?: 'outline' | 'ghost';
  }>(),
  {
    ariaLabel: undefined,
    destructive: false,
    disabled: false,
    variant: 'outline',
  }
);

defineEmits<{
  click: [event: MouseEvent];
}>();
</script>

<template>
  <button
    class="result-row-action"
    type="button"
    :class="[variant, { destructive }]"
    :title="title"
    :aria-label="ariaLabel ?? title"
    :disabled="disabled"
    @click="$emit('click', $event)"
  >
    <slot />
  </button>
</template>

<style scoped>
@reference "@assets/main.css";

.result-row-action {
  display: grid;
  width: 30px;
  height: 30px;
  place-items: center;
  justify-self: center;
  border-width: 1px;
  border-radius: 8px;
  padding: 0;
  @apply border-border/60 bg-transparent text-muted-foreground/75 transition-colors;
  cursor: pointer;
}

.result-row-action.ghost {
  border-color: transparent;
  background: transparent;
}

.result-row-action:not(:disabled):hover {
  @apply border-primary/25 bg-accent/60 text-primary;
}

.result-row-action.ghost:not(:disabled):hover {
  border-color: transparent;
  @apply bg-muted/75 text-primary;
}

.result-row-action.destructive:not(:disabled):hover {
  @apply border-destructive/25 bg-destructive/8 text-destructive;
}

.result-row-action.ghost.destructive:not(:disabled):hover {
  border-color: transparent;
}

.result-row-action:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
