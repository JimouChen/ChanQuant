import { useCallback, useRef, useState, type FormEvent } from 'react'
import {
  Activity, CheckCircle2, CircleDot, Database, Layers3,
  LineChart, LoaderCircle, Play, ScanLine, ShieldCheck, Square,
} from 'lucide-react'
import { DatePicker } from './components/DatePicker'
import { SignalList } from './components/SignalList'
import { StructureChart } from './components/StructureChart'
import type { ScanResult } from './types'

function localDate(): string {
  const now = new Date()
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
}

export default function App() {
  const [date, setDate] = useState(localDate)
  const [minAmount, setMinAmount] = useState(0.5)
  const [source, setSource] = useState<'sina' | 'eastmoney'>('sina')
  const [result, setResult] = useState<ScanResult | null>(null)
  const [selectedCode, setSelectedCode] = useState<string | null>(null)
  const [keyword, setKeyword] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const requestId = useRef(0)

  const runScan = useCallback(async () => {
    const id = ++requestId.current
    setLoading(true)
    setError('')
    try {
      const payload = await window.chanQuant.scan({
        date, minAmount, maxPivotAge: 90, source,
      })
      if (id !== requestId.current) return
      setResult(payload)
      setSelectedCode((current) => payload.signals.some((item) => item.code === current)
        ? current : payload.signals[0]?.code ?? null)
    } catch (reason) {
      if (id !== requestId.current) return
      setError(reason instanceof Error ? reason.message : '扫描失败，请稍后重试')
    } finally {
      if (id === requestId.current) setLoading(false)
    }
  }, [date, minAmount, source])

  const cancelScan = async () => {
    requestId.current += 1
    await window.chanQuant.cancel()
    setLoading(false)
  }

  const submit = (event: FormEvent) => {
    event.preventDefault()
    void runScan()
  }

  const selected = result?.signals.find((item) => item.code === selectedCode) ?? null
  const normalizedKeyword = keyword.trim().toLowerCase()
  const filteredSignals = (result?.signals ?? []).filter((item) =>
    item.code.includes(normalizedKeyword) || item.name.toLowerCase().includes(normalizedKeyword))

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark"><LineChart size={23} strokeWidth={1.8} /></div>
          <div><strong>ChanQuant</strong><span>缠量 · 结构交易</span></div>
        </div>
        <nav className="navigation" aria-label="主导航">
          <span className="nav-caption">分析工作台</span>
          <button className="nav-item active" type="button">
            <ScanLine size={18} /><span>三买选股</span><i />
          </button>
        </nav>
        <div className="method-card">
          <div className="method-title"><Layers3 size={16} /><span>判定规则</span></div>
          <ol>
            <li><span>01</span>向上离开中枢</li>
            <li><span>02</span>回抽不进中枢</li>
            <li><span>03</span>底分型当日确认</li>
          </ol>
          <p>只输出已确认结构<br />不使用未来函数</p>
        </div>
        <div className="kernel-status">
          <span className={`status-dot ${error ? 'error' : ''}`} />
          <div>
            <strong>{error ? '内核连接异常' : loading ? '正在计算结构' : '本地计算内核'}</strong>
            <span>{result ? 'Python · 数据已就绪' : 'Python · 前复权日线'}</span>
          </div>
        </div>
      </aside>

      <main className="main-area">
        <header className="topbar">
          <div className="page-title">
            <span className="eyebrow">STRUCTURE OVER NOISE</span>
            <h1>三买信号 <span>让走势自己说话</span></h1>
          </div>
          <form className="scan-controls" onSubmit={submit}>
            <DatePicker value={date} max={localDate()} onChange={setDate} />
            <label className="control-field amount-field">
              <span>成交额</span>
              <select value={minAmount} onChange={(event) => setMinAmount(Number(event.target.value))} aria-label="最低成交额">
                <option value={0.5}>≥ 0.5 亿</option>
                <option value={1}>≥ 1 亿</option>
                <option value={2}>≥ 2 亿</option>
              </select>
            </label>
            <label className="control-field source-field">
              <Database size={15} />
              <select value={source} onChange={(event) => setSource(event.target.value as 'sina' | 'eastmoney')} aria-label="行情数据源">
                <option value="sina">新浪行情</option>
                <option value="eastmoney">东方财富</option>
              </select>
            </label>
            {loading ? (
              <button className="scan-button cancel" type="button" onClick={() => void cancelScan()}>
                <Square size={14} fill="currentColor" />取消
              </button>
            ) : (
              <button className="scan-button" type="submit"><Play size={15} fill="currentColor" />开始扫描</button>
            )}
          </form>
        </header>

        <div className="content">
          {error && <div className="error-banner" role="alert"><CircleDot size={16} /><span>{error}</span><button onClick={() => void runScan()}>重试</button></div>}

          <section className="stats-grid" aria-label="扫描概览">
            <article className="stat-card featured">
              <div className="stat-icon"><Activity size={19} /></div>
              <div><span>确认信号</span><strong>{result?.signals.length ?? '—'}<small>只</small></strong></div>
              <span className="stat-note">当日三买</span>
            </article>
            <article className="stat-card">
              <div className="stat-icon"><Database size={18} /></div>
              <div><span>主板样本</span><strong>{result?.loadedSize.toLocaleString() ?? '—'}<small>只</small></strong></div>
              <span className="stat-note">剔除ST / 停牌</span>
            </article>
            <article className="stat-card">
              <div className="stat-icon"><Layers3 size={18} /></div>
              <div><span>结构初筛</span><strong>{result?.funnel.pullback ?? '—'}<small>只</small></strong></div>
              <span className="stat-note">回抽不进中枢</span>
            </article>
            <article className="stat-card">
              <div className="stat-icon"><CheckCircle2 size={18} /></div>
              <div><span>分型确认</span><strong>{result?.funnel.confirm ?? '—'}<small>只</small></strong></div>
              <span className="stat-note">通过时间约束</span>
            </article>
          </section>

          <div className={`workspace ${loading ? 'is-loading' : ''}`}>
            <SignalList
              signals={filteredSignals}
              selectedCode={selectedCode}
              keyword={keyword}
              onKeywordChange={setKeyword}
              onSelect={setSelectedCode}
            />

            <section className="chart-panel panel">
              {selected ? (
                <>
                  <div className="chart-heading">
                    <div>
                      <div className="chart-stock-name"><h2>{selected.name}</h2><span>{selected.code}</span></div>
                      <div className="chart-tags"><span>日线</span><span>前复权</span><span className="confirmed"><CheckCircle2 size={12} />三买已确认</span></div>
                    </div>
                    <div className="chart-legend"><span><i className="legend-stroke" />笔</span><span><i className="legend-pivot" />中枢</span><span><i className="legend-buy" />三买</span></div>
                  </div>
                  <StructureChart signal={selected} />
                  <div className="level-grid">
                    <div><span>中枢区间</span><strong>{selected.zd.toFixed(2)} — {selected.zg.toFixed(2)}</strong></div>
                    <div><span>离开高点</span><strong>{selected.leaveHigh.toFixed(2)}</strong></div>
                    <div><span>回抽低点</span><strong>{selected.pullbackLow.toFixed(2)}<small>距 ZG +{selected.distanceZgPct.toFixed(2)}%</small></strong></div>
                    <div className="stop-level"><span>结构止损位</span><strong>{selected.pullbackLow.toFixed(2)}</strong></div>
                  </div>
                </>
              ) : (
                <div className="empty-state">
                  {loading ? <LoaderCircle size={38} className="spin" /> : <ScanLine size={40} />}
                  <h2>{loading ? '正在解析市场结构' : result ? '当前条件下无确认信号' : '等待首次扫描'}</h2>
                  <p>{loading ? '依次处理包含关系、分型、笔与中枢，请稍候' : '耐心等待结构确认，不主观抄底摸顶'}</p>
                </div>
              )}
              {loading && selected && <div className="loading-overlay"><LoaderCircle size={30} className="spin" /><strong>正在刷新结构</strong><span>扫描期间保留当前结果</span></div>}
            </section>
          </div>
        </div>

        <footer className="statusbar">
          <div><ShieldCheck size={14} /><span>收盘确认 · 无未来函数</span></div>
          <span>{result ? `数据截止 ${result.tradeDate} · ${result.source === 'sina' ? '新浪行情' : '东方财富'}` : '本地数据管线'}</span>
          <span className="risk-note">跌破回抽低点即结构失效</span>
        </footer>
      </main>
    </div>
  )
}
