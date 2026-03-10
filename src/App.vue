<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router';
import { 
  ServerStackIcon, 
  DocumentTextIcon 
} from '@heroicons/vue/24/outline';

const router = useRouter();
const route = useRoute();

const navigation = [
  { name: '项目管理', path: '/', icon: ServerStackIcon },
  { name: '操作日志', path: '/logs', icon: DocumentTextIcon },
];

const navigateTo = (path: string) => {
  router.push(path);
}
</script>

<template>
  <div class="flex h-screen bg-slate-900 text-slate-200 overflow-hidden font-sans">
    
    <!-- Sidebar -->
    <aside class="w-64 bg-slate-800/50 border-r border-slate-700/50 flex flex-col backdrop-blur-xl transition-all duration-300">
      <div class="h-16 flex items-center px-6 border-b border-slate-700/50">
        <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-indigo-500 to-emerald-400 flex items-center justify-center mr-3 shadow-lg shadow-emerald-500/20">
          <svg class="w-5 h-5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
          </svg>
        </div>
        <span class="text-lg font-bold tracking-wide text-transparent bg-clip-text bg-gradient-to-r from-slate-100 to-slate-400">前端发布工具</span>
      </div>
      
      <nav class="flex-1 py-6 px-3 space-y-2">
        <a 
          v-for="item in navigation" 
          :key="item.name"
          @click="navigateTo(item.path)"
          :class="[
            route.path === item.path 
              ? 'bg-indigo-500/10 text-indigo-400 border-indigo-500/30' 
              : 'text-slate-400 hover:bg-slate-700/30 hover:text-slate-200 border-transparent',
            'group flex items-center px-3 py-2.5 text-sm font-medium rounded-xl border transition-all duration-200 cursor-pointer'
          ]"
        >
          <component 
            :is="item.icon" 
            :class="[
              route.path === item.path ? 'text-indigo-400' : 'text-slate-500 group-hover:text-slate-300',
              'flex-shrink-0 -ml-1 mr-3 h-5 w-5 transition-colors duration-200'
            ]" 
            aria-hidden="true" 
          />
          {{ item.name }}
        </a>
      </nav>

      <div class="p-4 mt-auto border-t border-slate-700/50">
        <div class="flex items-center">
          <div class="w-2 rounded-full h-2 bg-emerald-500 mr-2 shadow-[0_0_8px_rgba(16,185,129,0.8)]"></div>
          <span class="text-xs text-slate-400">系统就绪</span>
        </div>
      </div>
    </aside>

    <!-- Main Content -->
    <main class="flex-1 flex flex-col relative overflow-hidden bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-slate-800/40 via-slate-900 to-slate-900">
      <div class="flex-1 overflow-y-auto overflow-x-hidden relative">
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </div>
    </main>
  </div>
</template>

<style>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>