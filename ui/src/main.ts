import '@fontsource/lato/300.css'
import '@fontsource/lato/400.css'
import '@fontsource/lato/700.css'
import './app.css'
import { mount } from 'svelte'
import App from './App.svelte'

export default mount(App, { target: document.getElementById('app')! })
