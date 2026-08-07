<script setup lang="ts">
import { Button } from '@/components/ui/button';

withDefaults(
  defineProps<{
    selectedLabel: string;
    selectedValue: string;
    spaceLabel: string;
    spaceValue: string;
    clearLabel?: string;
    actionLabel: string;
    hint?: string;
    disabled?: boolean;
    busy?: boolean;
  }>(),
  {
    clearLabel: undefined,
    hint: undefined,
    disabled: false,
    busy: false,
  }
);

const emit = defineEmits<{
  clear: [];
  action: [];
}>();
</script>

<template>
  <div
    class="@container/selection-bar flex min-h-[var(--layout-action-bar-height)] w-full flex-wrap items-center justify-start gap-x-4 gap-y-2 rounded-lg border border-border bg-card/95 py-2 pr-3 pl-4 text-card-foreground shadow-sm shadow-foreground/5 backdrop-blur-md"
  >
    <div class="flex min-w-0 flex-none items-center gap-3">
      <span class="flex flex-col items-start gap-0.5">
        <small class="text-content-meta text-muted-foreground">{{ selectedLabel }}</small>
        <strong class="text-content-primary">{{ selectedValue }}</strong>
      </span>
      <i class="h-7.5 w-px flex-none bg-border" />
      <span class="flex flex-col items-start gap-0.5">
        <small class="text-content-meta text-muted-foreground">{{ spaceLabel }}</small>
        <strong class="text-content-section-title" :class="disabled ? 'text-muted-foreground' : 'text-primary'">
          {{ spaceValue }}
        </strong>
      </span>
    </div>

    <p
      v-if="hint"
      class="text-content-meta m-0 hidden min-w-0 flex-1 text-right text-muted-foreground @3xl/selection-bar:block"
    >
      {{ hint }}
    </p>
    <div v-if="$slots.options" class="ml-auto flex min-w-0 flex-none items-center justify-end">
      <slot name="options" />
    </div>

    <div class="flex flex-none items-center gap-2" :class="$slots.options ? 'ml-0' : 'ml-auto'">
      <Button v-if="clearLabel" variant="ghost" type="button" :disabled="disabled || busy" @click="emit('clear')">
        {{ clearLabel }}
      </Button>
      <Button
        class="min-w-31 disabled:bg-muted disabled:text-muted-foreground disabled:shadow-none"
        size="lg"
        type="button"
        :disabled="disabled || busy"
        @click="emit('action')"
      >
        <slot name="action-icon" />
        {{ actionLabel }}
      </Button>
    </div>
  </div>
</template>
