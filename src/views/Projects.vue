<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useProjectStore, type Project } from '../stores/projectStore';
import ProjectForm from '../components/ProjectForm.vue';
import HistoryModal from '../components/HistoryModal.vue';
import { invoke } from '@tauri-apps/api/core';
import { ask } from '@tauri-apps/plugin-dialog';

const store = useProjectStore();
const isFormOpen = ref(false);
const editingProject = ref<Project | null>(null);

const isHistoryOpen = ref(false);
const historyProject = ref<Project | null>(null);

const deployStatus = ref<{ [key: string]: { status: string, log: string } }>({});

onMounted(async () => {
  await store.fetchProjects();
  // Fetch connection status for all projects
  store.projects.forEach(p => store.checkConnection(p));
});

const openNew = () => {
  editingProject.value = null;
  isFormOpen.value = true;
};

const editProject = (p: Project) => {
  editingProject.value = p;
  isFormOpen.value = true;
};

const viewHistory = (p: Project) => {
  historyProject.value = p;
  isHistoryOpen.value = true;
};

const confirmDelete = async (id: string, name: string) => {
  const confirmed = await ask(`确定要删除项目「${name}」吗？此操作不可撤销。`, {
    title: '删除项目',
    kind: 'warning',
    okLabel: '确定',
    cancelLabel: '取消'
  });
  if (confirmed) {
    await store.deleteProject(id);
  }
};

const handleSave = async (project: Project) => {
  const saved = await store.saveProject(project);
  isFormOpen.value = false;
  // Check connection for the newly saved project
  store.checkConnection(saved);
};

// 发布备注弹窗
const isDeployDialogOpen = ref(false);
const deployTarget = ref<Project | null>(null);
const deployRemark = ref('');

const startDeploy = (project: Project) => {
  deployTarget.value = project;
  deployRemark.value = '';
  isDeployDialogOpen.value = true;
};

const confirmDeploy = async () => {
  if (!deployTarget.value) return;
  const project = deployTarget.value;
  isDeployDialogOpen.value = false;
  
  deployStatus.value[project.id] = { status: 'deploying', log: '正在开始部署...' };
  try {
    const result = await invoke<string>('run_deployment', { project, remark: deployRemark.value });
    deployStatus.value[project.id] = { status: 'success', log: result };
  } catch (err: any) {
    deployStatus.value[project.id] = { status: 'error', log: err };
  }
};
</script>

<template>
  <div class="p-6 h-full text-slate-100 flex flex-col relative w-full overflow-hidden">
    <div class="flex justify-between items-center mb-6 border-b border-slate-700/50 pb-4 shrink-0">
      <div>
        <h1 class="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-indigo-400 to-emerald-400">项目管理</h1>
        <p class="text-sm text-slate-400 mt-1">管理和部署您的 Web 前端项目</p>
      </div>
      <button @click="openNew" class="bg-indigo-600 hover:bg-indigo-500 text-white px-4 py-2 rounded-lg font-medium transition-colors duration-200 flex items-center shadow-lg shadow-indigo-500/30">
        <svg class="w-5 h-5 mr-1.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        新建项目
      </button>
    </div>
    
    <div class="flex-1 overflow-y-auto pr-2 custom-scrollbar relative">
      <div v-if="store.loading" class="flex justify-center mt-20">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-500"></div>
      </div>
      
      <!-- 空状态 -->
      <div v-else-if="store.projects.length === 0" class="h-full flex flex-col items-center justify-center text-slate-500 border-2 border-dashed border-slate-700 rounded-2xl bg-slate-800/20 backdrop-blur-sm p-10">
        <svg class="w-16 h-16 mb-4 text-slate-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
        </svg>
        <p class="text-lg font-medium">尚未配置任何项目</p>
        <p class="text-sm mt-1">点击上方「新建项目」按钮添加您的第一个项目</p>
      </div>

      <!-- 项目列表 -->
      <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-2 gap-6 pb-10">
        <div v-for="project in store.projects" :key="project.id" class="bg-slate-800/80 border border-slate-700/80 hover:border-indigo-500/50 rounded-2xl p-5 shadow-lg backdrop-blur-md transition-all duration-300 group">
          <div class="flex justify-between items-start mb-4">
            <div class="overflow-hidden pr-2">
              <h3 class="text-lg font-semibold text-slate-100 flex items-center">
                <span class="truncate">{{ project.name }}</span>
              </h3>
              <p class="text-xs text-slate-400 mt-1 font-mono truncate" :title="project.local_path">{{ project.local_path }}</p>
            </div>
            <div class="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
              <button @click="editProject(project)" title="编辑" class="p-1.5 text-slate-400 hover:text-indigo-400 bg-slate-900/50 rounded-lg hover:bg-slate-900 transition">
                <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                   <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                </svg>
              </button>
              <button @click="confirmDelete(project.id, project.name)" title="删除" class="p-1.5 text-slate-400 hover:text-red-400 bg-slate-900/50 rounded-lg hover:bg-slate-900 transition">
                <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                   <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
              </button>
            </div>
          </div>
          
          <div class="grid grid-cols-2 gap-y-3 gap-x-4 mb-5 p-3 bg-slate-900/40 rounded-xl border border-white/5">
            <div class="col-span-2 text-xs flex justify-between">
              <span class="text-slate-500">服务器</span>
              <span class="text-slate-300 font-mono">{{ project.username }}@{{ project.remote_host }}</span>
            </div>
            <div class="col-span-2 text-xs flex justify-between">
              <span class="text-slate-500">部署路径</span>
              <span class="text-slate-300 font-mono truncate max-w-[180px]" :title="project.remote_deploy_path">{{ project.remote_deploy_path }}</span>
            </div>
          </div>

          <div class="flex items-center justify-between">
            <div class="flex items-center text-sm w-1/2">
              <template v-if="deployStatus[project.id]?.status === 'deploying'">
                <div class="w-2 h-2 rounded-full bg-blue-500 animate-pulse mr-2 shrink-0"></div>
                <span class="text-blue-400 font-medium truncate">正在部署...</span>
              </template>
              <template v-else-if="deployStatus[project.id]?.status === 'success'">
                <div class="w-2 h-2 rounded-full bg-emerald-500 mr-2 shadow-[0_0_8px_rgba(16,185,129,0.8)] shrink-0"></div>
                <span class="text-emerald-400 font-medium truncate" :title="deployStatus[project.id].log">部署成功</span>
              </template>
              <template v-else-if="deployStatus[project.id]?.status === 'error'">
                <div class="w-2 h-2 rounded-full bg-red-500 mr-2 shadow-[0_0_8px_rgba(239,68,68,0.8)] shrink-0"></div>
                <span class="text-red-400 font-medium truncate" :title="deployStatus[project.id].log">部署失败</span>
              </template>
              <template v-else-if="store.connectionStatus[project.id] === 'checking'">
                <div class="w-2 h-2 rounded-full bg-yellow-500 animate-pulse mr-2 shrink-0"></div>
                <span class="text-yellow-400 font-medium truncate">检测连接...</span>
              </template>
              <template v-else-if="store.connectionStatus[project.id] === 'online'">
                <div class="w-2 h-2 rounded-full bg-emerald-500 mr-2 shrink-0"></div>
                <span class="text-emerald-400 font-medium truncate">连通正常</span>
              </template>
              <template v-else-if="store.connectionStatus[project.id] === 'offline'">
                <div class="w-2 h-2 rounded-full bg-red-500 mr-2 shrink-0"></div>
                <span class="text-red-400 font-medium truncate">连接失败</span>
              </template>
              <template v-else>
                <div class="w-2 h-2 rounded-full bg-slate-600 mr-2 shrink-0"></div>
                <span class="text-slate-500 truncate">未检测</span>
              </template>
            </div>
            
            <div class="flex gap-2 shrink-0">
              <button 
                @click="store.checkConnection(project)" 
                class="px-3 py-2 bg-slate-700 hover:bg-slate-600 text-white text-sm font-medium rounded-lg transition-all flex items-center shrink-0 mr-1"
                title="重新检测连接">
                <svg class="w-4 h-4" :class="{'animate-spin text-yellow-400': store.connectionStatus[project.id] === 'checking'}" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
              </button>
              <button 
                @click="viewHistory(project)" 
                class="px-3 py-2 bg-slate-700 hover:bg-slate-600 text-white text-sm font-medium rounded-lg transition-all flex items-center shrink-0">
                <svg class="w-4 h-4 mr-1.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                历史
              </button>
              <button 
                @click="startDeploy(project)" 
                :disabled="deployStatus[project.id]?.status === 'deploying'"
                class="px-4 py-2 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 disabled:opacity-50 disabled:cursor-wait text-white text-sm font-medium rounded-lg shadow-lg shadow-emerald-600/20 transition-all flex items-center shrink-0">
                <svg v-if="deployStatus[project.id]?.status === 'deploying'" class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                <svg v-else class="w-4 h-4 mr-1.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
                </svg>
                发布
              </button>
            </div>
          </div>
          
          <!-- 错误日志输出 -->
          <div v-if="deployStatus[project.id]?.status === 'error'" class="mt-4 p-3 bg-red-900/20 border border-red-500/30 rounded-lg text-xs font-mono text-red-200 overflow-x-auto max-h-32">
            {{ deployStatus[project.id].log }}
          </div>
        </div>
      </div>
    </div>
    
    <ProjectForm 
      :is-open="isFormOpen" 
      :initial-data="editingProject" 
      @close="isFormOpen = false" 
      @save="handleSave" 
    />

    <HistoryModal 
      :is-open="isHistoryOpen"
      :project="historyProject"
      @close="isHistoryOpen = false"
    />

    <!-- 发布备注弹窗 -->
    <div v-if="isDeployDialogOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/80 backdrop-blur-sm">
      <div class="bg-slate-800 border border-slate-700 rounded-2xl w-full max-w-md shadow-2xl">
        <div class="px-6 py-4 border-b border-slate-700/50">
          <h2 class="text-lg font-semibold text-slate-100">发布确认</h2>
          <p class="text-sm text-slate-400 mt-1">即将发布项目「{{ deployTarget?.name }}」</p>
        </div>
        <div class="p-6">
          <label class="block text-sm font-medium text-slate-400 mb-2">发布备注（可选）</label>
          <textarea v-model="deployRemark" rows="3" placeholder="例如：修复了首页样式问题" class="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition resize-none"></textarea>
        </div>
        <div class="px-6 pb-5 flex justify-end gap-3">
          <button @click="isDeployDialogOpen = false" class="px-4 py-2 rounded-lg text-sm font-medium text-slate-300 hover:bg-slate-700 transition">取消</button>
          <button @click="confirmDeploy" class="px-5 py-2 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white text-sm font-medium rounded-lg shadow-lg shadow-emerald-600/20 transition-all">确认发布</button>
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
