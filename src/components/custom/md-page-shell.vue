<script setup lang="ts">
import { OperatingSystemService } from '@/lib/services/operating-system-service';

withDefaults(
  defineProps<{
    title: string;
    subtitle?: string;
    contentMode?: 'document' | 'workspace';
  }>(),
  {
    subtitle: undefined,
    contentMode: 'document',
  }
);

// Tauri drag regions do not inherit through child elements. Page titles and
// their surrounding whitespace are marked explicitly so macOS gets a generous
// drag surface while the actions column remains fully interactive.
const macOsDragRegion = OperatingSystemService.isMacOs() ? '' : undefined;
</script>

<template>
  <section class="md-page-shell" :class="`md-page-shell--${contentMode}`">
    <header
      :data-tauri-drag-region="macOsDragRegion"
      class="md-page-header"
      :class="{ 'md-page-header--draggable': macOsDragRegion !== undefined }"
    >
      <div :data-tauri-drag-region="macOsDragRegion" class="md-page-heading">
        <h1
          :data-tauri-drag-region="macOsDragRegion"
          class="m-0 leading-tight font-normal tracking-tight text-foreground"
        >
          {{ title }}
        </h1>
        <slot name="subtitle"
          ><p
            v-if="subtitle"
            :data-tauri-drag-region="macOsDragRegion"
            class="mt-1.5 mb-0 text-sm leading-relaxed text-muted-foreground"
          >
            {{ subtitle }}
          </p></slot
        >
      </div>
      <div v-if="$slots.actions" class="md-page-actions"><slot name="actions" /></div>
    </header>
    <div
      class="md-page-content"
      :class="[
        `md-page-content--${contentMode}`,
        {
          'md-page-content--with-footer': $slots.footer,
          'scrollbar-stable-end': contentMode === 'document',
        },
      ]"
    >
      <slot />
    </div>
    <footer v-if="$slots.footer" class="md-page-footer"><slot name="footer" /></footer>
  </section>
</template>

<style scoped>
@reference "@assets/main.css";
.md-page-shell {
  display: flex;
  width: 100%;
  height: 100%;
  min-height: 0;
  flex-direction: column;
  overflow: hidden;
  container-type: inline-size;
  padding: var(--layout-page-padding-top) var(--layout-page-padding-inline) 0;
}

.md-page-header {
  display: grid;
  width: 100%;
  min-height: var(--layout-page-header-height);
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: start;
  gap: 18px;
  flex: none;
}

.md-page-header--draggable .md-page-heading {
  user-select: none;
}

.md-page-shell--document .md-page-header {
  padding-inline-end: var(--layout-scrollbar-width);
}

.md-page-heading {
  min-width: 0;
  padding-top: 1px;
}

.md-page-heading h1 {
  font-size: 26px;
}

.md-page-actions {
  display: flex;
  width: auto;
  min-width: 0;
  min-height: 40px;
  flex: none;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
}

/*
 * The header stays outside the scroll container. Avoiding nested sticky
 * regions prevents layout movement in lists and treemaps.
 */
.md-page-content {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  gap: 10px;
  overflow-x: hidden;
  overscroll-behavior: contain;
}

.md-page-content--document {
  /*
   * Keep document content aligned with the page header while extending only
   * the scroll container to the pane edge. This places the scrollbar beside
   * the window edge without changing card widths or workspace-page geometry.
   */
  margin-inline-end: calc(-1 * var(--layout-page-padding-inline));
  padding-inline-end: var(--layout-page-padding-inline);
  padding-bottom: 24px;
}

.md-page-content--workspace {
  /* Reserve bottom spacing unless a page footer provides it. */
  overflow-y: hidden;
  padding-bottom: 14px;
}

.md-page-content--workspace.md-page-content--with-footer {
  padding-bottom: 0;
}

.md-page-footer {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 8px;
  padding-bottom: 14px;
}

/* Respond to the content pane rather than the viewport including the sidebar. */
@container (max-width: 840px) {
  .md-page-header {
    min-height: 0;
    grid-template-columns: minmax(0, 1fr);
    gap: 10px;
    padding-bottom: 14px;
  }

  .md-page-actions {
    width: 100%;
    justify-content: flex-start;
  }
}
</style>
