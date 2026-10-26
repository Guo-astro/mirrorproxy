export const history = {
  push(path: string) { window.history.pushState({}, '', path); window.dispatchEvent(new PopStateEvent('popstate')) },
  replace(path: string) { window.history.replaceState({}, '', path); window.dispatchEvent(new PopStateEvent('popstate')) },
  listen(callback: () => void) { window.addEventListener('popstate', callback); return () => window.removeEventListener('popstate', callback) },
}
export const setLocale = () => undefined
