import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface Project {
    id: string
    name: string
    local_path: string
    build_command: string
    build_output_dir: string
    remote_host: string
    remote_port: number
    username: string
    auth_method: string
    password_or_key: string
    remote_deploy_path: string
    symlink_name: string
    deploy_mode: string
    max_releases: number
}

export const useProjectStore = defineStore('project', () => {
    const projects = ref<Project[]>([])
    const loading = ref(false)
    const connectionStatus = ref<Record<string, 'checking' | 'online' | 'offline'>>({})

    const fetchProjects = async () => {
        loading.value = true
        try {
            projects.value = await invoke<Project[]>('get_projects')
        } catch (e) {
            console.error(e)
        } finally {
            loading.value = false
        }
    }

    const saveProject = async (p: Project) => {
        try {
            const saved = await invoke<Project>('save_project', { project: p })
            const idx = projects.value.findIndex(existing => existing.id === saved.id)
            if (idx !== -1) {
                projects.value[idx] = saved
            } else {
                projects.value.push(saved)
            }
            return saved
        } catch (e) {
            console.error(e)
            throw e
        }
    }

    const deleteProject = async (id: string) => {
        try {
            await invoke('delete_project', { id })
            projects.value = projects.value.filter(p => p.id !== id)
            delete connectionStatus.value[id]
        } catch (e) {
            console.error(e)
            throw e
        }
    }

    const checkConnection = async (project: Project) => {
        connectionStatus.value[project.id] = 'checking'
        try {
            await invoke<string>('check_connection', { project })
            connectionStatus.value[project.id] = 'online'
        } catch (e) {
            connectionStatus.value[project.id] = 'offline'
        }
    }

    return { projects, loading, connectionStatus, fetchProjects, saveProject, deleteProject, checkConnection }
})
