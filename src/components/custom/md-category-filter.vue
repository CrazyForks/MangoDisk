<script setup lang="ts">
defineProps<{
  ariaLabel: string;
  disabled?: boolean;
  modelValue: string;
  options: Array<{ value: string; label: string; count?: number }>;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();
</script>

<template>
  <nav class="scrollbar-hidden flex min-w-0 items-center gap-1 overflow-x-auto p-0.5" :aria-label="ariaLabel">
    <button
      v-for="option in options"
      :key="option.value"
      type="button"
      class="inline-flex h-7.5 flex-none cursor-pointer items-center gap-1.5 rounded-md border border-transparent px-2.5 text-content-body text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/35 disabled:cursor-default disabled:hover:bg-transparent disabled:hover:text-muted-foreground"
      :class="{
        'border-primary/20 bg-primary/10 font-semibold text-primary': modelValue === option.value,
      }"
      :disabled="disabled"
      @click="emit('update:modelValue', option.value)"
    >
      <span>{{ option.label }}</span>
      <small
        v-if="option.count !== undefined"
        class="min-w-4 px-0.5 py-0.5 text-center text-content-meta text-muted-foreground"
        :class="{ 'text-primary': modelValue === option.value }"
      >
        {{ option.count }}
      </small>
    </button>
  </nav>
</template>
