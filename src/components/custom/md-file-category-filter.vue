<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { FileCategoryId } from '@/lib/models/file-category';

const { t } = useI18n({ useScope: 'global' });

defineProps<{
  modelValue: FileCategoryId;
  options: Array<{ value: FileCategoryId; label: string; count?: number }>;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: FileCategoryId];
}>();
</script>

<template>
  <nav
    class="scrollbar-hidden flex min-w-0 items-center gap-1 overflow-x-auto p-0.5"
    :aria-label="t('common.filterFileCategory')"
  >
    <button
      v-for="option in options"
      :key="option.value"
      type="button"
      class="inline-flex h-7.5 flex-none cursor-pointer items-center gap-1.5 rounded-md border border-transparent px-2.5 text-content-body text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/35"
      :class="{
        'border-primary/20 bg-primary/10 font-semibold text-primary': modelValue === option.value,
      }"
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
