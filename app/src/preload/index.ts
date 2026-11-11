import { contextBridge, ipcRenderer } from 'electron'

export interface ScanOptions {
  date: string
  minAmount: number
  maxPivotAge: number
  source: 'sina' | 'eastmoney'
}

contextBridge.exposeInMainWorld('chanQuant', {
  scan: (options: ScanOptions) => ipcRenderer.invoke('scanner:run', options),
  cancel: () => ipcRenderer.invoke('scanner:cancel'),
})
