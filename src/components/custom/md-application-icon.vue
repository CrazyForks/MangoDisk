<script setup lang="ts">
import { computed } from 'vue';

import MdIcon from '@/components/icons/md-icon.vue';
import type { ApplicationUninstallPlatform } from '@/lib/models/application';
import { ICON_NAMES } from '@/lib/models/ui';

const props = withDefaults(
  defineProps<{
    src?: string;
    platform?: ApplicationUninstallPlatform;
    size?: number;
    artworkSize?: number;
  }>(),
  {
    src: '',
    platform: 'macosBundle',
    size: 40,
    artworkSize: 0,
  }
);
const emit = defineEmits<{
  error: [];
}>();

const resolvedArtworkSize = computed(() => {
  if (props.artworkSize > 0) return props.artworkSize;
  // Windows icon resources usually fill their canvas while macOS ICNS artwork includes optical
  // padding. Normalizing only the artwork keeps alignment slots identical across platforms.
  return props.platform === 'windowsRegistry' ? Math.round(props.size * 0.85) : props.size;
});
</script>

<template>
  <span
    class="md-application-icon"
    :class="{ resolved: Boolean(src) }"
    :style="{ width: `${size}px`, height: `${size}px` }"
  >
    <img
      v-if="src"
      :src="src"
      alt=""
      :style="{ width: `${resolvedArtworkSize}px`, height: `${resolvedArtworkSize}px` }"
      @error="emit('error')"
    />
    <span v-else-if="platform === 'windowsRegistry'" class="windows-fallback" aria-hidden="true">
      <MdIcon :name="ICON_NAMES.windowsApplication" :size="Math.round(size * 0.7)" :stroke-width="1.7" />
      <span class="windows-fallback-disc">
        <MdIcon :name="ICON_NAMES.disc" :size="Math.round(size * 0.35)" :stroke-width="1.7" />
      </span>
    </span>
    <MdIcon v-else :name="ICON_NAMES.application" :size="Math.round(size * 0.56)" />
  </span>
</template>

<style scoped>
@reference "@assets/main.css";

.md-application-icon {
  @apply bg-primary/10 text-primary;
  display: grid;
  flex: none;
  overflow: hidden;
  place-items: center;
  border-radius: calc(var(--radius) - 2px);
}

.md-application-icon.resolved {
  background: transparent;
}

img {
  object-fit: contain;
}

.windows-fallback {
  position: relative;
  display: grid;
  width: 80%;
  height: 75%;
  place-items: center;
  color: var(--chart-2);
}

.windows-fallback-disc {
  position: absolute;
  bottom: -1px;
  left: -2px;
  display: grid;
  width: 43%;
  aspect-ratio: 1;
  place-items: center;
  border-radius: 999px;
  background: var(--background);
  @apply text-muted-foreground;
}
</style>
