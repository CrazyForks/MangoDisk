<script setup lang="ts">
import { computed } from 'vue';

import { Toaster } from '@/components/ui/sonner';
import MdAppShell from '@/layouts/md-app-shell.vue';
import type { AppSettings } from '@/lib/models/settings';
import { useAppStore } from '@/stores/app-store';

const appStore = useAppStore();
const toastTheme = computed<AppSettings['theme']>(() => appStore.settings.theme);
</script>

<template>
  <MdAppShell />
  <Toaster :theme="toastTheme" position="bottom-right" rich-colors close-button />
</template>
