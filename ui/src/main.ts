import { mount } from 'svelte'
import './app.css'
import './lib/design/winter/winter.css'
import App from './App.svelte'

// Focus rings are for keyboard navigation only (Apple HIG, Focus and selection).
// WebView2 re-shows :focus-visible on the last clicked control when the window
// regains focus, so we track the input modality ourselves: app.css hides the
// ring while the pointer is in use, and a clicked control gives focus back when
// the window is deactivated.
function isTextEntry(el: Element | null): boolean {
  if (!(el instanceof HTMLElement)) return false
  return el.matches('input, textarea, select') || el.isContentEditable
}
const root = document.documentElement
root.dataset.input = 'pointer'
window.addEventListener('keydown', (e) => {
  if (e.key === 'Tab' || e.key.startsWith('Arrow')) root.dataset.input = 'keyboard'
}, true)
window.addEventListener('pointerdown', () => { root.dataset.input = 'pointer' }, true)
window.addEventListener('blur', () => {
  const el = document.activeElement
  if (root.dataset.input === 'pointer' && el instanceof HTMLElement && el !== document.body && !isTextEntry(el)) {
    el.blur()
  }
})

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
