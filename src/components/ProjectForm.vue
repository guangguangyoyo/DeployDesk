<script setup lang="ts">
import { ref, watch } from 'vue';
import type { Project } from '../stores/projectStore';
import { open } from '@tauri-apps/plugin-dialog';

const props = defineProps<{
  initialData?: Project | null;
  isOpen: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save', project: Project): void;
}>();

const formData = ref<Project>({
  id: '',
  name: '',
  local_path: '',
  build_command: 'npm run build',
  build_output_dir: 'dist',
  remote_host: '',
  remote_port: 22,
  username: 'root',
  auth_method: 'password',
  password_or_key: '',
  remote_deploy_path: '/var/www/html/project',
  symlink_name: 'current',
  deploy_mode: 'build',
  max_releases: 15
});

watch(() => props.isOpen, (newVal) => {
  if (newVal) {
    if (props.initialData) {
      formData.value = { ...props.initialData };
    } else {
      formData.value = {
        id: '',
        name: '',
        local_path: '',
        build_command: 'npm run build',
        build_output_dir: 'dist',
        remote_host: '',
        remote_port: 22,
        username: 'root',
        auth_method: 'password',
        password_or_key: '',
        remote_deploy_path: '/var/www/html/project',
        symlink_name: 'current',
        deploy_mode: 'build',
        max_releases: 15
      };
    }
  }
});

const selectLocalPath = async () => {
  const selected = await open({ directory: true, multiple: false });
  if (selected && !Array.isArray(selected)) {
    formData.value.local_path = selected;
  }
};

const selectKeyFile = async () => {
  const selected = await open({ multiple: false });
  if (selected && !Array.isArray(selected)) {
    formData.value.password_or_key = selected;
  }
};

const save = () => {
  emit('save', formData.value);
};
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/80 backdrop-blur-sm">
    <div class="bg-slate-800 border border-slate-700 rounded-2xl w-full max-w-2xl shadow-2xl flex flex-col max-h-[90vh]">
      
      <!-- 标题栏 -->
      <div class="px-6 py-4 border-b border-slate-700/50 flex justify-between items-center bg-slate-800/50 rounded-t-2xl">
        <h2 class="text-xl font-semibold text-slate-100">{{ initialData ? '编辑项目' : '新建项目' }}</h2>
        <button @click="$emit('close')" class="text-slate-400 hover:text-white transition">
          <svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
      
      <!-- 表单内容 -->
      <div class="p-6 overflow-y-auto custom-scrollbar flex-1 space-y-6">
        
        <!-- 基本信息 -->
        <div>
          <h3 class="text-sm font-semibold text-slate-300 uppercase tracking-wider mb-4 border-b border-slate-700/50 pb-2">基本信息</h3>
          <div class="grid grid-cols-2 gap-4">
            <div class="col-span-2">
              <label class="block text-sm font-medium text-slate-400 mb-1">项目名称</label>
              <input v-model="formData.name" placeholder="例如：管理后台、官网等" class="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition" />
            </div>
            <div class="col-span-2">
              <label class="block text-sm font-medium text-slate-400 mb-1">本地项目根目录</label>
              <div class="flex gap-2">
                <input v-model="formData.local_path" readonly placeholder="请选择目录..." class="flex-1 bg-slate-900 border shadow-inner border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none cursor-not-allowed" />
                <button @click="selectLocalPath" class="px-4 py-2 bg-slate-700 hover:bg-slate-600 rounded-lg text-sm text-white font-medium transition whitespace-nowrap">浏览...</button>
              </div>
            </div>
          </div>
        </div>

        <!-- 部署配置 -->
        <div>
          <h3 class="text-sm font-semibold text-slate-300 uppercase tracking-wider mb-4 border-b border-slate-700/50 pb-2">部署配置</h3>
          <div class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-slate-400 mb-2">部署模式</label>
              <div class="flex gap-4">
                <label class="flex items-center cursor-pointer group">
                  <input type="radio" v-model="formData.deploy_mode" value="build" class="sr-only" />
                  <div class="flex items-center px-4 py-2 border rounded-lg transition" :class="formData.deploy_mode === 'build' ? 'bg-indigo-600/20 border-indigo-500 text-indigo-400' : 'bg-slate-900 border-slate-700 text-slate-500 group-hover:border-slate-600'">
                    <svg class="w-4 h-4 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                    </svg>
                    构建并发布
                  </div>
                </label>
                <label class="flex items-center cursor-pointer group">
                  <input type="radio" v-model="formData.deploy_mode" value="upload" class="sr-only" />
                  <div class="flex items-center px-4 py-2 border rounded-lg transition" :class="formData.deploy_mode === 'upload' ? 'bg-emerald-600/20 border-emerald-500 text-emerald-400' : 'bg-slate-900 border-slate-700 text-slate-500 group-hover:border-slate-600'">
                    <svg class="w-4 h-4 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
                    </svg>
                    直接上传静态文件
                  </div>
                </label>
              </div>
              <p class="text-xs text-slate-500 mt-2">
                {{ formData.deploy_mode === 'build' ? '每次发布前，先在本地执行构建命令，然后上传构建出的产物目录。' : '跳过本地构建步骤，直接压缩并上传“构建产物目录”中的文件。' }}
              </p>
            </div>
          </div>
        </div>

        <!-- 构建配置 -->
        <div v-if="formData.deploy_mode === 'build'">
          <h3 class="text-sm font-semibold text-slate-300 uppercase tracking-wider mb-4 border-b border-slate-700/50 pb-2">构建配置</h3>
          <div class="grid grid-cols-2 gap-4">
            <div class="col-span-2">
              <label class="block text-sm font-medium text-slate-400 mb-1">构建命令</label>
              <input v-model="formData.build_command" placeholder="npm run build" class="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 focus:border-indigo-500 outline-none transition" />
            </div>
          </div>
        </div>

        <!-- 产物配置 -->
        <div>
          <h3 class="text-sm font-semibold text-slate-300 uppercase tracking-wider mb-4 border-b border-slate-700/50 pb-2">产物目录</h3>
          <div class="grid grid-cols-2 gap-4">
            <div class="col-span-2">
              <label class="block text-sm font-medium text-slate-400 mb-1">{{ formData.deploy_mode === 'build' ? '构建产物目录' : '静态文件目录' }}（相对路径）</label>
              <input v-model="formData.build_output_dir" placeholder="dist" class="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 focus:border-indigo-500 outline-none transition" />
              <p class="text-xs text-slate-500 mt-1">
                {{ formData.deploy_mode === 'build' ? '构建命令生成的文件夹，例如：dist、build。' : '存放已打包好的静态文件的文件夹。' }}
              </p>
            </div>
          </div>
        </div>

        <!-- 服务器配置 -->
        <div>
          <h3 class="text-sm font-semibold text-slate-300 uppercase tracking-wider mb-4 border-b border-slate-700/50 pb-2">服务器配置</h3>
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="block text-sm font-medium text-slate-400 mb-1">远程主机（IP / 域名）</label>
              <input v-model="formData.remote_host" class="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 focus:border-indigo-500 outline-none transition" />
            </div>
            <div>
              <label class="block text-sm font-medium text-slate-400 mb-1">端口</label>
              <input type="number" v-model="formData.remote_port" class="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 focus:border-indigo-500 outline-none transition" />
            </div>
            
            <div class="col-span-2">
              <label class="block text-sm font-medium text-slate-400 mb-1">远程部署路径</label>
              <input v-model="formData.remote_deploy_path" placeholder="/var/www/my-project" class="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 focus:border-indigo-500 outline-none transition" />
              <p class="text-xs text-slate-500 mt-1">例如：/var/www/demo，项目将部署到 /var/www/demo/releases/时间戳/ 并软链到 /var/www/demo/{{formData.symlink_name || 'current'}}</p>
            </div>

            <div class="col-span-2">
              <label class="block text-sm font-medium text-slate-400 mb-1">软链接名称</label>
              <input v-model="formData.symlink_name" placeholder="current" class="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 focus:border-indigo-500 outline-none transition" />
              <p class="text-xs text-slate-500 mt-1">Nginx 指向的文件夹名称（例如：current、dist）。留空默认为 current。</p>
            </div>

            <div class="col-span-2">
              <label class="block text-sm font-medium text-slate-400 mb-1">最大保留版本数</label>
              <input type="number" v-model="formData.max_releases" min="3" class="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 focus:border-indigo-500 outline-none transition" />
              <p class="text-xs text-slate-500 mt-1">服务器上保留的历史版本数量。发布新版本后，将自动删除最旧的版本（最小为 3）。</p>
            </div>

            <div>
              <label class="block text-sm font-medium text-slate-400 mb-1">用户名</label>
              <input v-model="formData.username" class="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 focus:border-indigo-500 outline-none transition" />
            </div>
            
            <div>
              <label class="block text-sm font-medium text-slate-400 mb-1">认证方式</label>
              <select v-model="formData.auth_method" class="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 focus:border-indigo-500 outline-none transition">
                <option value="password">密码认证</option>
                <option value="key">私钥认证</option>
              </select>
            </div>

            <div class="col-span-2">
              <label class="block text-sm font-medium text-slate-400 mb-1">{{ formData.auth_method === 'password' ? '密码' : '私钥文件路径' }}</label>
              <div class="flex gap-2">
                <input v-if="formData.auth_method === 'password'" type="password" v-model="formData.password_or_key" class="flex-1 bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 focus:border-indigo-500 outline-none transition" />
                <template v-else>
                  <input v-model="formData.password_or_key" readonly placeholder="请选择私钥文件..." class="flex-1 bg-slate-900 border shadow-inner border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-300 outline-none cursor-not-allowed" />
                  <button @click="selectKeyFile" class="px-4 py-2 bg-slate-700 hover:bg-slate-600 rounded-lg text-sm text-white font-medium transition whitespace-nowrap">浏览...</button>
                </template>
              </div>
            </div>

          </div>
        </div>
      </div>
      
      <!-- 底部操作 -->
      <div class="p-6 border-t border-slate-700/50 bg-slate-800/80 rounded-b-2xl flex justify-end gap-3 shrink-0">
        <button @click="$emit('close')" class="px-5 py-2 rounded-lg text-sm font-medium text-slate-300 hover:bg-slate-700 transition">取消</button>
        <button @click="save" :disabled="!formData.name || !formData.local_path || !formData.remote_host || formData.max_releases < 3" class="px-5 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-sm font-medium text-white shadow-lg shadow-indigo-500/30 transition-all">保存项目</button>
      </div>

    </div>
  </div>
</template>
