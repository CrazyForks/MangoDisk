<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { computed } from 'vue';
import MdDialogContent from '@/components/custom/md-dialog-content.vue';
import { Button } from '@/components/ui/button';
import { Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import type { PresentedScanRuleResult } from '@/lib/models/cleanup';
import { FormatUtils } from '@/lib/utils/format';

const { locale, t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  busy: boolean;
  leftoverApplicationCount: number;
  leftoverBytes: number;
  leftoverItemCount: number;
  modelValue: boolean;
  rules: PresentedScanRuleResult[];
  selectedBytes: number;
  selectedItemCount: number;
}>();
const emit = defineEmits<{
  execute: [];
  'update:modelValue': [open: boolean];
}>();

const runningProcesses = computed(() => [
  ...new Set(props.rules.flatMap(rule => rule.runningProcesses).filter(Boolean)),
]);
const runningProcessLabel = computed(() => FormatUtils.list(runningProcesses.value, locale.value));
const requiresAppClose = computed(() => props.rules.some(rule => rule.requiresAppClose));
</script>

<template>
  <Dialog :open="modelValue" @update:open="emit('update:modelValue', $event)">
    <MdDialogContent
      class="@container/cleanup-plan flex max-h-[84vh] min-h-0 flex-col overflow-hidden p-0 sm:max-w-[720px]"
    >
      <DialogHeader class="flex-none px-6 pt-6 pr-14">
        <DialogTitle>{{ t('cleanup.planDialogTitle') }}</DialogTitle>
        <DialogDescription>{{ t('cleanup.planDialogDescription') }}</DialogDescription>
      </DialogHeader>

      <div class="modal-summary flex-none">
        <span class="summary-space">
          <small>{{ t('cleanup.estimated') }}</small>
          <strong>{{ FormatUtils.bytes(selectedBytes) }}</strong>
        </span>
        <span class="summary-count">
          {{ t('cleanup.selectedItemCount', { count: FormatUtils.integer(selectedItemCount) }, selectedItemCount) }}
        </span>
      </div>
      <p v-if="requiresAppClose" class="process-warning flex-none">
        {{
          runningProcesses.length
            ? t('cleanup.closeAppsBeforeCleanup', {
                processes: runningProcessLabel,
              })
            : t('cleanup.closeAppsBeforeCleanupGeneric')
        }}
      </p>
      <div class="modal-rules scrollbar-stable min-h-0 flex-1">
        <div v-for="rule in rules" :key="rule.ruleId">
          <span class="risk" :class="rule.risk">
            {{ rule.risk === 'safe' ? t('common.safe') : t('common.recoverable') }}
          </span>
          <span>
            <strong>{{ rule.name }}</strong>
            <small>{{ rule.impact }}</small>
          </span>
          <strong>{{ FormatUtils.bytes(rule.bytes) }}</strong>
        </div>
        <div v-if="leftoverItemCount">
          <span class="risk recoverable">{{ t('common.recoverable') }}</span>
          <span>
            <strong>{{ t('applicationLeftovers.resultTitle') }}</strong>
            <small>
              {{
                t(
                  'applicationLeftovers.planSummary',
                  {
                    applications: FormatUtils.integer(leftoverApplicationCount),
                    locations: FormatUtils.integer(leftoverItemCount),
                  },
                  leftoverApplicationCount
                )
              }}
            </small>
            <small class="leftover-impact">{{ t('applicationLeftovers.planImpact') }}</small>
          </span>
          <strong>{{ FormatUtils.bytes(leftoverBytes) }}</strong>
        </div>
      </div>

      <DialogFooter class="flex-none border-t border-border/70 px-6 py-3.5">
        <Button variant="outline" type="button" :disabled="busy" @click="emit('update:modelValue', false)">
          {{ t('common.cancel') }}
        </Button>
        <Button type="button" :disabled="busy" @click="emit('execute')">
          {{ t('cleanup.execute') }}
        </Button>
      </DialogFooter>
    </MdDialogContent>
  </Dialog>
</template>

<style scoped>
@reference "@assets/main.css";

.modal-summary {
  @apply border border-border/70 bg-muted/30;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin: 0 24px;
  border-radius: 9px;
  padding: 11px 14px;
}

.summary-space {
  display: flex;
  align-items: baseline;
  gap: 10px;
}

.summary-space small,
.summary-count {
  @apply text-muted-foreground;
  font-size: var(--font-content-secondary);
}

.summary-space strong {
  @apply text-primary;
  font-size: 22px;
}

.summary-count {
  white-space: nowrap;
}

.modal-rules {
  @apply border border-border/70;
  margin: 12px 24px;
  border-radius: 9px;
}

.process-warning {
  margin: 12px 24px 0;
  border-radius: 9px;
  padding: 10px 12px;
  @apply bg-warning/15 text-warning-foreground;
  font-size: var(--font-content-secondary);
}

.modal-rules > div {
  @apply border-t border-border/70;
  display: grid;
  grid-template-columns: 78px minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
}

.modal-rules > div:first-child {
  border-top: 0;
}

.modal-rules div > span:nth-child(2) {
  display: flex;
  min-width: 0;
  flex-direction: column;
}

.modal-rules div > span:nth-child(2) > strong,
.modal-rules > div > strong:last-child {
  font-size: 13px;
  line-height: 1.35;
}

.modal-rules small {
  @apply text-muted-foreground;
  margin-top: 2px;
  font-size: 10.5px;
  line-height: 1.45;
}

.modal-rules .leftover-impact {
  @apply text-warning-foreground;
}

.risk {
  align-items: center;
  justify-self: center;
  gap: 4px;
  border-radius: 999px;
  padding: 4px 8px;
  font-size: var(--font-content-secondary);
  font-weight: 500;
}

.risk.safe {
  @apply bg-success/12 text-success-foreground;
}

.risk.recoverable {
  @apply bg-warning/15 text-warning-foreground;
}

@container cleanup-plan (max-width: 560px) {
  .modal-rules > div {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .risk {
    grid-column: 1 / -1;
    justify-self: start;
  }
}
</style>
