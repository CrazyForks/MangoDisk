<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';

interface ResultTableScrollOptions {
  top?: number;
  left?: number;
  behavior?: 'auto' | 'smooth';
}

const scrollElement = ref<HTMLElement | null>(null);
const scrollGutter = ref(0);
let resizeObserver: ResizeObserver | null = null;

function syncScrollGutter() {
  const element = scrollElement.value;
  if (!element) return;

  // `scrollbar-gutter: stable both-edges` reserves equal space on both sides.
  // The width difference reports their combined size, so use one half when
  // aligning the fixed header with the scrolling row content.
  scrollGutter.value = Math.max(0, (element.offsetWidth - element.clientWidth) / 2);
}

function scrollTo(options: ResultTableScrollOptions) {
  scrollElement.value?.scrollTo(options);
}

onMounted(() => {
  syncScrollGutter();
  resizeObserver = new ResizeObserver(syncScrollGutter);
  if (scrollElement.value) resizeObserver.observe(scrollElement.value);
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
});

defineExpose({
  scrollTo,
});
</script>

<template>
  <div class="result-table" :style="{ '--result-table-scroll-gutter': `${scrollGutter}px` }">
    <header v-if="$slots.header" class="result-table-header md-result-header">
      <slot name="header" />
    </header>
    <div ref="scrollElement" class="result-table-scroll scrollbar-stable">
      <slot />
    </div>
  </div>
</template>

<style scoped>
@reference "@assets/main.css";

.result-table {
  --result-table-content-inline-padding: 12px;
  --result-table-hierarchy-indent: 32px;

  display: flex;
  min-width: 0;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  overflow: hidden;
}

.result-table-header {
  min-width: 0;
  flex: none;
  border-bottom-width: 1px;
  padding-inline: calc(var(--result-table-scroll-gutter) + var(--result-table-content-inline-padding));
}

.result-table-scroll {
  min-width: 0;
  min-height: 0;
  flex: 1;
  overflow-x: hidden;
  overscroll-behavior: contain;
}
</style>
