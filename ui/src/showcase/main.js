import { mount } from 'svelte'
import '../app.css'
import ComponentShowcase from './ComponentShowcase.svelte'

const app = mount(ComponentShowcase, {
  target: document.getElementById('app'),
})

export default app
