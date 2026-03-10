<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { 
  CheckCircleIcon, 
  XCircleIcon,
  ArrowPathIcon,
  ClockIcon
} from '@heroicons/vue/24/outline';

interface LogEntry {
  id: string;
  timestamp: string;
  project_name: string;
  action: string;
  status: string;
  details: string;
}

const logs = ref<LogEntry[]>([]);
const loading = ref(true);

const fetchLogs = async () => {
  loading.value = true;
  try {
    logs.value = await invoke('get_logs');
  } catch (error) {
    console.error('Failed to fetch logs:', error);
  } finally {
    loading.value = false;
  }
};

onMounted(fetchLogs);
</script>

<template>
  <div class="p-6 h-full text-slate-100 flex flex-col max-w-6xl mx-auto w-full">
    <div class="mb-8 flex justify-between items-end border-b border-slate-700/50 pb-6">
      <div>
        <h1 class="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-white to-slate-400">操作记录</h1>
        <p class="text-sm text-slate-400 mt-2 flex items-center gap-2">
          <ClockIcon class="w-4 h-4" />
          仅显示最近 100 条发布与回退记录
        </p>
      </div>
      <button 
        @click="fetchLogs" 
        class="flex items-center gap-2 px-4 py-2 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-lg text-sm font-medium transition-all active:scale-95"
      >
        <ArrowPathIcon :class="['w-4 h-4', loading ? 'animate-spin' : '']" />
        刷新
      </button>
    </div>
    
    <div class="flex-1 overflow-hidden flex flex-col bg-slate-800/30 rounded-2xl border border-slate-700/50 backdrop-blur-sm shadow-xl">
      <div v-if="loading && logs.length === 0" class="flex-1 flex flex-col items-center justify-center gap-3">
        <ArrowPathIcon class="w-8 h-8 text-indigo-500 animate-spin" />
        <span class="text-slate-500 text-sm">正在加载操作记录...</span>
      </div>

      <div v-else-if="logs.length === 0" class="flex-1 flex flex-col items-center justify-center gap-4 opacity-50 py-20">
        <div class="w-16 h-16 rounded-full bg-slate-700/30 flex items-center justify-center">
          <ClockIcon class="w-8 h-8 text-slate-500" />
        </div>
        <div class="text-center">
          <p class="text-slate-300 font-medium">暂无最近的操作记录</p>
          <p class="text-xs text-slate-500 mt-1">当您进行发布或回退操作后，记录将显示在此处</p>
        </div>
      </div>

      <div v-else class="flex-1 overflow-y-auto custom-scrollbar">
        <table class="w-full text-left border-collapse">
          <thead class="sticky top-0 bg-slate-800/90 backdrop-blur-md z-10 border-b border-slate-700/50">
            <tr>
              <th class="px-6 py-4 text-xs font-semibold text-slate-500 uppercase tracking-wider">项目名称</th>
              <th class="px-6 py-4 text-xs font-semibold text-slate-500 uppercase tracking-wider">动作</th>
              <th class="px-6 py-4 text-xs font-semibold text-slate-500 uppercase tracking-wider">状态</th>
              <th class="px-6 py-4 text-xs font-semibold text-slate-500 uppercase tracking-wider">操作详情</th>
              <th class="px-6 py-4 text-xs font-semibold text-slate-500 uppercase tracking-wider text-right">时间</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-700/30">
            <tr v-for="log in logs" :key="log.id" class="hover:bg-slate-700/20 transition-colors group">
              <td class="px-6 py-4">
                <span class="font-medium text-slate-200">{{ log.project_name }}</span>
              </td>
              <td class="px-6 py-4">
                <span :class="[
                  'px-2 py-0.5 rounded-md text-xs font-medium border',
                  log.action === '发布' ? 'bg-indigo-500/10 text-indigo-400 border-indigo-500/20' : 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                ]">
                  {{ log.action }}
                </span>
              </td>
              <td class="px-6 py-4">
                <div class="flex items-center gap-1.5">
                  <CheckCircleIcon v-if="log.status === '成功'" class="w-4 h-4 text-emerald-500" />
                  <XCircleIcon v-else class="w-4 h-4 text-rose-500" />
                  <span :class="log.status === '成功' ? 'text-emerald-500' : 'text-rose-500'" class="text-sm">
                    {{ log.status }}
                  </span>
                </div>
              </td>
              <td class="px-6 py-4">
                <p class="text-xs text-slate-400 truncate max-w-md group-hover:text-slate-300 transition-colors" :title="log.details">
                  {{ log.details }}
                </p>
              </td>
              <td class="px-6 py-4 text-right">
                <span class="text-xs font-mono text-slate-500">{{ log.timestamp }}</span>
              </td>
            </tr>
          </tbody>
        </table>
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
  border-radius: 3px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #475569;
}
</style>
