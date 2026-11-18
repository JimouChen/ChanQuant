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
  durationMs: number
}

export interface CacheCleanup {
  fileCount: number
  bytes: number
}

export interface BacktestOptions {
  start: string
  end: string
  initialCapital: number
  maxPositions: number
  minAmount: number
  maxPivotAge: number
  source: 'sina' | 'eastmoney'
}

export interface EquityPoint {
  date: string
  equity: number
  drawdown: number
}

export interface Trade {
  code: string
  name: string
  entryDate: string
  entryPrice: number
  exitDate: string
  exitPrice: number
  shares: number
  zg: number
  pnl: number
  pnlPct: number
  holdDays: number
  exitReason: string
}

export interface BacktestStats {
  totalReturnPct: number
  annualizedPct: number
  maxDrawdownPct: number
  winRatePct: number
  profitFactor: number | null
  tradeCount: number
  avgHoldDays: number
  finalEquity: number
}

export interface BacktestResult {
  start: string
  end: string
  source: 'sina' | 'eastmoney'
  universeSize: number
  loadedSize: number
  signalCount: number
  stats: BacktestStats
  equityCurve: EquityPoint[]
  trades: Trade[]
  generatedAt: string
  durationMs: number
}
