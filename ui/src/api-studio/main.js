import { mount } from 'svelte'
import '../app.css'
import ApiStudio from '$lib/api-studio/ApiStudio.svelte'

const nativeFetch = window.fetch.bind(window)
window.fetch = (input, init = {}) => {
  const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
  const headers = new Headers(init.headers || (typeof Request !== 'undefined' && input instanceof Request ? input.headers : undefined))
  const token = localStorage.getItem('bos_session')
  let bearer = ''
  try {
    bearer = JSON.parse(token || 'null')?.token || ''
  } catch {
    bearer = ''
  }
  const isAuth = url.includes('/api/auth/login') || url.includes('/api/auth/bootstrap') || url.includes('/api/auth/status') || url.includes('/api/health')
  if (bearer && url.includes('/api/') && !isAuth && !headers.has('Authorization')) {
    headers.set('Authorization', `Bearer ${bearer}`)
  }
  if (url.includes('/api/')) {
    try {
      const env = JSON.parse(localStorage.getItem('bos_environment') || 'null')
      if (env?.tenantId && !headers.has('x-bos-tenant-id')) {
        headers.set('x-bos-tenant-id', String(env.tenantId))
      }
      if (env?.companyName && !headers.has('x-bos-company')) {
        headers.set('x-bos-company', String(env.companyName))
      }
    } catch {
      // ignore
    }
  }
  return nativeFetch(input, { ...init, headers })
}

const app = mount(ApiStudio, {
  target: document.getElementById('app'),
  props: { isStandalone: true }
})

export default app
