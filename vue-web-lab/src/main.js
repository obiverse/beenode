import { createApp } from 'vue'
import './style.css'
import App from './App.vue'
import { installShell } from './core/shellContext.js'

const app = createApp(App)
installShell(app)
app.mount('#app')
