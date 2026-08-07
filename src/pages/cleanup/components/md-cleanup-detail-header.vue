<script setup lang="ts">
import { useI18n } from 'vue-i18n';

import MdResultCheckbox from '@/components/custom/md-result-checkbox.vue';
import MdIcon from '@/components/icons/md-icon.vue';
import type { IconName } from '@/lib/models/ui';
import { FormatUtils } from '@/lib/utils/format';

withDefaults(
  defineProps<{
    icon: IconName;
    title: string;
    description: string;
    selectedBytes: number;
    totalBytes: number;
    selection: 'all' | 'partial' | 'none';
    disabled?: boolean;
  }>(),
  { disabled: false }
);
const emit = defineEmits<{
  'update:selected': [selected: boolean];
}>();
const { t } = useI18n({ useScope: 'global' });
</script>

<template>
  <header class="detail-header">
    <span class="detail-icon">
      <MdIcon :name="icon" :size="23" />
    </span>
    <span class="detail-heading">
      <strong>{{ title }}</strong>
      <small>{{ description }}</small>
    </span>
    <span class="detail-size">
      <small>{{ t('cleanup.selected') }} / {{ t('cleanup.cleanableFound') }}</small>
      <span>
        <strong>{{ FormatUtils.bytes(selectedBytes) }}</strong>
        <i>/ {{ FormatUtils.bytes(totalBytes) }}</i>
      </span>
    </span>
    <label class="category-selection">
      <MdResultCheckbox
        :checked="selection === 'all'"
        :indeterminate="selection === 'partial'"
        :disabled="disabled"
        @update:checked="emit('update:selected', $event)"
      />
      <span>{{ t('cleanup.selectAll') }}</span>
    </label>
  </header>
</template>

<style scoped>
@reference "@assets/main.css";

.detail-header {
  @apply border-border;
  display: grid;
  flex: none;
  grid-template-columns: 32px minmax(0, 1fr) auto auto;
  align-items: center;
  gap: 10px;
  border-bottom-width: 1px;
  padding: 11px 18px;
}

.detail-icon {
  @apply text-primary;
  display: grid;
  width: 32px;
  height: 38px;
  flex: none;
  place-items: center;
}

.detail-heading {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 3px;
}

.detail-heading strong {
  font-size: var(--font-content-section-title);
  font-weight: 600;
}

.detail-heading small {
  @apply text-muted-foreground;
  overflow: hidden;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.detail-size {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 1px;
}

.detail-size small {
  @apply text-muted-foreground;
  font-size: 10px;
}

.detail-size strong {
  @apply text-primary;
  font-size: 15px;
}

.detail-size > span {
  display: flex;
  align-items: baseline;
  gap: 5px;
  white-space: nowrap;
}

.detail-size i {
  @apply text-muted-foreground;
  font-size: 11px;
  font-style: normal;
}

.category-selection {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 12px;
  cursor: pointer;
}

@container cleanup (max-width: 760px) {
  .detail-header {
    grid-template-columns: 30px minmax(0, 1fr) auto;
    padding-inline: 12px;
  }

  .detail-size,
  .category-selection span {
    display: none;
  }

  .detail-icon {
    width: 30px;
    height: 36px;
  }
}
</style>
