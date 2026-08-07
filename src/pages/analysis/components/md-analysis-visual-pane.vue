<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { ref } from 'vue';

import MdIcon from '@/components/icons/md-icon.vue';
import { ANALYSIS_VIEW_IDS } from '@/lib/models/analysis';
import { ICON_NAMES } from '@/lib/models/ui';
import type { AnalysisResult, DirectoryEntryInfo } from '@/lib/models/analysis';
import { FormatUtils } from '@/lib/utils/format';

import MdAnalysisDetailsTable from './md-analysis-details-table.vue';
import MdAnalysisTreemap from './md-analysis-treemap.vue';

const { t } = useI18n({ useScope: 'global' });

defineProps<{
  result: AnalysisResult;
  entries: DirectoryEntryInfo[];
  folderCount: number;
}>();

const emit = defineEmits<{
  activate: [entry: DirectoryEntryInfo];
  open: [path: string];
  delete: [entry: DirectoryEntryInfo];
}>();

const viewMode = ref<(typeof ANALYSIS_VIEW_IDS)[keyof typeof ANALYSIS_VIEW_IDS]>(ANALYSIS_VIEW_IDS.treemap);
</script>

<template>
  <section class="visual-pane">
    <header class="md-workspace-toolbar">
      <p>
        {{
          t(
            'analysis.folderSpaceSummary',
            { folders: FormatUtils.integer(folderCount), size: FormatUtils.bytes(result.totalBytes) },
            folderCount
          )
        }}
      </p>
      <div class="view-switcher" role="group" :aria-label="t('analysis.result')">
        <button
          type="button"
          :class="{ active: viewMode === ANALYSIS_VIEW_IDS.treemap }"
          :aria-pressed="viewMode === ANALYSIS_VIEW_IDS.treemap"
          @click="viewMode = ANALYSIS_VIEW_IDS.treemap"
        >
          <MdIcon :name="ICON_NAMES.grid" :size="15" />
          {{ t('analysis.treemap') }}
        </button>
        <button
          type="button"
          :class="{ active: viewMode === ANALYSIS_VIEW_IDS.details }"
          :aria-pressed="viewMode === ANALYSIS_VIEW_IDS.details"
          @click="viewMode = ANALYSIS_VIEW_IDS.details"
        >
          <MdIcon :name="ICON_NAMES.list" :size="15" />
          {{ t('analysis.details') }}
        </button>
      </div>
    </header>

    <MdAnalysisTreemap
      v-if="viewMode === ANALYSIS_VIEW_IDS.treemap"
      :entries="entries"
      :total-bytes="result.totalBytes"
      @activate="emit('activate', $event)"
      @open="emit('open', $event)"
      @delete="emit('delete', $event)"
    />
    <MdAnalysisDetailsTable
      v-else
      :entries="entries"
      @activate="emit('activate', $event)"
      @open="emit('open', $event)"
      @delete="emit('delete', $event)"
    />
  </section>
</template>

<style scoped>
@reference "@assets/main.css";

.visual-pane {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  overflow: hidden;
}

.visual-pane > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 12px;
}

.visual-pane header p {
  min-width: 0;
  margin: 0;
  overflow: hidden;
  @apply text-muted-foreground;
  font-size: var(--font-content-secondary);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.view-switcher {
  display: flex;
  height: var(--layout-workspace-control-height);
  flex: none;
  overflow: hidden;
  border-width: 1px;
  border-radius: 8px;
  @apply border-border;
}

.view-switcher button {
  display: flex;
  height: 100%;
  align-items: center;
  gap: 6px;
  border: 0;
  border-left-width: 1px;
  padding: 0 11px;
  @apply border-border bg-card text-card-foreground transition-colors duration-200;
  font: inherit;
  font-size: var(--font-content-secondary);
  cursor: pointer;
}

.view-switcher button:first-child {
  border-left: 0;
}

.view-switcher button:hover:not(.active) {
  @apply border-primary/40 bg-accent/65 text-accent-foreground;
}

.view-switcher button.active {
  @apply bg-accent text-accent-foreground;
}
</style>
