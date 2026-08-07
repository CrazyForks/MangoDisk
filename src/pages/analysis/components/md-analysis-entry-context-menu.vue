<script setup lang="ts">
import { useI18n } from 'vue-i18n';

import MdIcon from '@/components/icons/md-icon.vue';
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from '@/components/ui/context-menu';
import type { DirectoryEntryInfo } from '@/lib/models/analysis';
import { ICON_NAMES } from '@/lib/models/ui';

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  entry: DirectoryEntryInfo;
}>();

const emit = defineEmits<{
  open: [path: string];
  delete: [entry: DirectoryEntryInfo];
  menuStateChange: [open: boolean];
}>();
</script>

<template>
  <!--
    Keep file actions in one component so menu copy, destructive styling, and
    future capabilities cannot drift across the three analysis result views.
    The trigger stays in the default slot because each view owns its layout.
  -->
  <ContextMenu @update:open="emit('menuStateChange', $event)">
    <ContextMenuTrigger as-child>
      <slot />
    </ContextMenuTrigger>
    <ContextMenuContent>
      <ContextMenuItem @select="emit('open', props.entry.path)">
        <MdIcon :name="ICON_NAMES.folder" :size="16" />
        {{ t('analysis.showInFileManager') }}
      </ContextMenuItem>
      <ContextMenuItem class="text-destructive focus:text-destructive" @select="emit('delete', props.entry)">
        <MdIcon :name="ICON_NAMES.trash" :size="16" />
        {{ t('analysis.deletePermanently') }}
      </ContextMenuItem>
    </ContextMenuContent>
  </ContextMenu>
</template>
