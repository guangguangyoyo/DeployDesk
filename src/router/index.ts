import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
    history: createWebHistory(),
    routes: [
        {
            path: '/',
            name: 'projects',
            component: () => import('../views/Projects.vue')
        },
        {
            path: '/logs',
            name: 'logs',
            component: () => import('../views/Logs.vue')
        }
    ]
})

export default router
