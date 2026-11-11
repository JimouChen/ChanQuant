import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { existsSync } from 'node:fs'
import path from 'node:path'
import { ipcMain } from 'electron'

export interface ScanOptions {
  date: string
  minAmount: number
  maxPivotAge: number
  source: 'sina' | 'eastmoney'
}

let activeProcess: ChildProcessWithoutNullStreams | null = null

function projectRoot(): string {
  return process.env.CHANQUANT_ROOT ?? path.resolve(__dirname, '../../..')
}

function pythonExecutable(root: string): string {
  const candidates = [
    process.env.CHANQUANT_PYTHON,
    path.join(root, '.venv', 'bin', 'python'),
    path.join(root, '.venv', 'Scripts', 'python.exe'),
  ].filter((value): value is string => Boolean(value))
  return candidates.find((candidate) => existsSync(candidate)) ?? 'python3'
}

function runScan(options: ScanOptions): Promise<unknown> {
  const root = projectRoot()
  const args = [
    '-m', 'chanquant.api',
    '--date', options.date.replaceAll('-', ''),
    '--min-amount', String(options.minAmount),
    '--max-pivot-age', String(options.maxPivotAge),
    '--source', options.source,
  ]

  return new Promise((resolve, reject) => {
    const child = spawn(pythonExecutable(root), args, {
      cwd: root,
      env: { ...process.env, PYTHONUNBUFFERED: '1' },
    })
    activeProcess = child
    let stdout = ''
    let stderr = ''
    const timeout = setTimeout(() => {
      child.kill()
      reject(new Error('扫描超时，请检查行情接口或网络代理'))
    }, 15 * 60 * 1000)
    timeout.unref()

    child.stdout.on('data', (chunk) => { stdout += chunk.toString() })
    child.stderr.on('data', (chunk) => { stderr += chunk.toString() })
    child.on('error', (error) => {
      clearTimeout(timeout)
      activeProcess = null
      reject(new Error(`无法启动 Python 内核：${error.message}`))
    })
    child.on('close', (code) => {
      clearTimeout(timeout)
      activeProcess = null
      if (code !== 0) {
        reject(new Error(stderr.trim().split('\n').slice(-4).join('\n') || `扫描进程退出（${code}）`))
        return
      }
      try {
        resolve(JSON.parse(stdout.trim()))
      } catch {
        reject(new Error('扫描结果解析失败，请检查 Python 内核输出'))
      }
    })
  })
}

export function registerScannerIpc(): void {
  ipcMain.handle('scanner:run', (_event, raw: ScanOptions) => {
    if (activeProcess) throw new Error('已有扫描任务正在运行')
    if (!/^\d{4}-\d{2}-\d{2}$/.test(raw.date)) throw new Error('日期格式无效')
    const options: ScanOptions = {
      date: raw.date,
      minAmount: Math.min(Math.max(Number(raw.minAmount) || 0.5, 0), 100),
      maxPivotAge: Math.min(Math.max(Number(raw.maxPivotAge) || 90, 10), 250),
      source: raw.source === 'eastmoney' ? 'eastmoney' : 'sina',
    }
    return runScan(options)
  })

  ipcMain.handle('scanner:cancel', () => {
    activeProcess?.kill()
    activeProcess = null
    return true
  })
}

export function stopScanner(): void {
  activeProcess?.kill()
  activeProcess = null
}
