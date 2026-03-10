<script setup lang="ts">
import { ref, watch } from 'vue';
import type { Project } from '../stores/projectStore';
import { invoke } from '@tauri-apps/api/core';
import { ask } from '@tauri-apps/plugin-dialog';

const props = defineProps<{
  project?: Project | null;
  isOpen: boolean;
}>();

interface ReleaseInfo {
  name: string;
  remark: string;
  is_current: boolean;
}

const releases = ref<ReleaseInfo[]>([]);
const loading = ref(false);
const errorMsg = ref('');
const rollBacking = ref(false);
const successMsg = ref('');

watch(() => props.isOpen, async (newVal) => {
  if (newVal && props.project) {
    await fetchReleases();
  } else {
    releases.value = [];
    errorMsg.value = '';
    successMsg.value = '';
  }
});

const fetchReleases = async () => {
  if (!props.project) return;
  loading.value = true;
  errorMsg.value = '';
  try {
    const list = await invoke<ReleaseInfo[]>('get_releases', { project: props.project });
    // Sort descending by name, but we trust backend for is_current
    releases.value = [...list].sort((a, b) => b.name.localeCompare(a.name));
  } catch (err: any) {
    errorMsg.value = err;
  } finally {
    loading.value = false;
  }
};

const doRollback = async (releaseName: string) => {
  if (!props.project) return;
  const confirmed = await ask(`确定要回滚到版本 ${releaseName} 吗？\n该操作会将远程服务器的软链接指向该版本。`, {
    title: '确认回滚',
    kind: 'warning',
    okLabel: '回滚',
    cancelLabel: '取消'
  });
  if (!confirmed) return;
  
  rollBacking.value = true;
  errorMsg.value = '';
  successMsg.value = '';
  try {
    const msg = await invoke<string>('rollback', { project: props.project, release: releaseName });
    successMsg.value = msg;
    await fetchReleases();
  } catch (err: any) {
    errorMsg.value = err;
  } finally {
    rollBacking.value = false;
  }
};
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/80 backdrop-blur-sm">
    <div class="bg-slate-800 border border-slate-700 rounded-2xl w-full max-w-lg shadow-2xl flex flex-col max-h-[80vh]">
      
      <div class="px-6 py-4 border-b border-slate-700/50 flex justify-between items-center bg-slate-800/50 rounded-t-2xl shrink-0">
        <h2 class="text-xl font-semibold text-slate-100">发布历史</h2>
        <button @click="$emit('close')" class="text-slate-400 hover:text-white transition">
           <svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
      
      <div class="p-6 overflow-y-auto custom-scrollbar flex-1 relative min-h-[300px]">
        <div v-if="loading" class="absolute inset-0 flex items-center justify-center">
           <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-500"></div>
        </div>
        
        <div v-else-if="errorMsg" class="p-4 bg-red-900/30 border border-red-500/50 rounded-lg text-red-300 text-sm mb-4">
          {{ errorMsg }}
        </div>

        <div v-if="successMsg" class="p-4 bg-emerald-900/30 border border-emerald-500/50 rounded-lg text-emerald-300 text-sm mb-4">
          {{ successMsg }}
        </div>

        <div v-if="!loading && releases.length === 0" class="text-center text-slate-500 mt-10">
          远程服务器上暂无发布记录。
        </div>
        
        <div v-if="!loading && releases.length > 0" class="space-y-3">
          <div v-for="release in releases" :key="release.name" class="p-4 bg-slate-900/50 rounded-xl border border-white/5 hover:border-white/10 transition-colors">
            <div class="flex items-center justify-between">
              <div class="flex items-center min-w-0">
                <div v-if="release.is_current" class="w-2 h-2 rounded-full bg-emerald-500 mr-3 shrink-0 shadow-[0_0_8px_rgba(16,185,129,0.8)]"></div>
                <div v-else class="w-2 h-2 rounded-full bg-slate-600 mr-3 shrink-0"></div>
                <div class="min-w-0">
                  <span class="font-mono text-slate-200 block text-sm">{{ release.name }}</span>
                  <span v-if="release.is_current" class="text-xs text-emerald-400 font-medium tracking-wide">当前版本</span>
                  <span v-else class="text-xs text-slate-500 tracking-wider">历史版本</span>
                </div>
              </div>
              
              <button 
                v-if="!release.is_current"
                @click="doRollback(release.name)" 
                :disabled="rollBacking"
                class="px-3 py-1.5 text-xs font-medium bg-slate-700 hover:bg-slate-600 text-white rounded-lg transition-colors ml-4 disabled:opacity-50 shrink-0">
                回滚
              </button>
            </div>
            <!-- 备注 -->
            <div v-if="release.remark" class="mt-2 ml-5 pl-3 border-l-2 border-slate-700 text-xs text-slate-400 leading-relaxed">
              {{ release.remark }}
            </div>
          </div>
        </div>
      </div>
      
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #334155;
  border-radius: 4px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #475569;
}
</style>
