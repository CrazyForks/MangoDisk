<script setup lang="ts">
import { computed } from 'vue';

import MdIcon from '@/components/icons/md-icon.vue';
import { ICON_NAMES } from '@/lib/models/ui';
import { FileTypeUtils, type FileVisualKind } from '@/lib/utils/file-type';

const props = withDefaults(
  defineProps<{
    name: string;
    compact?: boolean;
  }>(),
  {
    compact: false,
  }
);

const descriptor = computed(() => FileTypeUtils.descriptor(props.name));

const iconNames: Readonly<Record<FileVisualKind, (typeof ICON_NAMES)[keyof typeof ICON_NAMES]>> = {
  pdf: ICON_NAMES.fileText,
  document: ICON_NAMES.fileText,
  spreadsheet: ICON_NAMES.fileSpreadsheet,
  presentation: ICON_NAMES.filePresentation,
  text: ICON_NAMES.fileText,
  code: ICON_NAMES.fileCode,
  data: ICON_NAMES.database,
  audio: ICON_NAMES.fileAudio,
  video: ICON_NAMES.fileVideo,
  image: ICON_NAMES.fileImage,
  archive: ICON_NAMES.fileArchive,
  installer: ICON_NAMES.package,
  'disk-image': ICON_NAMES.disc,
  binary: ICON_NAMES.fileSettings,
  other: ICON_NAMES.file,
};

const iconName = computed(() => iconNames[descriptor.value.kind]);
const iconSize = computed(() => (props.compact ? 16 : 18));
</script>

<template>
  <span
    class="file-type-icon relative grid size-[34px] flex-none place-items-center rounded-md bg-accent text-accent-foreground"
    :class="[
      descriptor.kind,
      {
        compact,
        'size-[30px] rounded-sm': compact,
      },
    ]"
    :title="descriptor.extensionLabel"
    aria-hidden="true"
  >
    <MdIcon :name="iconName" :size="iconSize" />
    <small v-if="descriptor.extensionLabel">{{ descriptor.extensionLabel }}</small>
  </span>
</template>

<style scoped>
@reference "@assets/main.css";

/* File formats share one geometry and differ only through semantic color. */
.file-type-icon.pdf {
  @apply bg-destructive/10 text-destructive;
}
.file-type-icon.document {
  @apply bg-primary/10 text-primary;
}
.file-type-icon.spreadsheet {
  @apply bg-success/12 text-success;
}
.file-type-icon.presentation,
.file-type-icon.archive {
  @apply bg-warning/15 text-warning-foreground;
}
.file-type-icon.code,
.file-type-icon.video {
  @apply bg-file-code/10 text-file-code;
}
.file-type-icon.audio {
  @apply bg-file-audio/10 text-file-audio;
}
.file-type-icon.image {
  @apply bg-file-image/10 text-file-image;
}
.file-type-icon.data {
  @apply bg-file-data/10 text-file-data;
}
.file-type-icon.installer,
.file-type-icon.disk-image {
  @apply bg-file-package/10 text-file-package;
}
.file-type-icon.binary {
  @apply bg-file-binary/10 text-file-binary;
}

.file-type-icon small {
  position: absolute;
  right: -0.1875rem;
  bottom: -0.1875rem;
  min-width: 0.875rem;
  border-width: 1px;
  border-radius: var(--radius-sm);
  padding: 0.0625rem 0.125rem;
  @apply border-card bg-card text-foreground;
  font-size: 0.4375rem;
  font-weight: 700;
  line-height: 1;
  letter-spacing: -0.02em;
  text-align: center;
}

.file-type-icon.compact small {
  right: -0.125rem;
  bottom: -0.125rem;
  font-size: 0.375rem;
}
</style>
