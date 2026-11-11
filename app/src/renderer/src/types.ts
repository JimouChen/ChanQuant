export interface ScanOptions {
  date: string
  minAmount: number
  maxPivotAge: number
  source: 'sina' | 'eastmoney'
}

export interface ChartData {
  dates: string[]
  candles: number[][]
  volumes: number[]
  strokePoints: [string, number][]
  pivot: {
    startDate: string
    endDate: string
    zg: number
    zd: number
  }
  buyPoint: {
    date: string
    price: number
    confirmDate: string
  }
}

export interface Signal {
  code: string
  name: string
  close: number
  changePct: number
  amount: number
  zg: number
  zd: number
  leaveHigh: number
  pullbackLow: number
  distanceZgPct: number
  pullbackPct: number
  pivotStart: string
  confirmDate: string
  chart: ChartData | null
}

export interface ScanResult {
  tradeDate: string
  source: 'sina' | 'eastmoney'
  universeSize: number
  loadedSize: number
  funnel: Record<string, number>
  signals: Signal[]
  generatedAt: string
}

declare global {
  interface Window {
    chanQuant: {
      scan(options: ScanOptions): Promise<ScanResult>
      cancel(): Promise<boolean>
    }
  }
}
