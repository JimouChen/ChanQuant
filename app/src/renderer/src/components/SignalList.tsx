import { ArrowUpRight, Search } from 'lucide-react'
import type { Signal } from '../types'

interface Props {
  signals: Signal[]
  selectedCode: string | null
  keyword: string
  onKeywordChange: (value: string) => void
  onSelect: (code: string) => void
}

export function SignalList({ signals, selectedCode, keyword, onKeywordChange, onSelect }: Props) {
  return (
    <section className="signal-panel panel">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">WATCHLIST</span>
          <h2>今日信号</h2>
        </div>
        <span className="count-badge">{signals.length}</span>
      </div>
      <label className="search-box">
        <Search size={16} />
        <input
          value={keyword}
          onChange={(event) => onKeywordChange(event.target.value)}
          placeholder="搜索代码 / 名称"
          aria-label="搜索信号"
        />
      </label>
      <div className="signal-list">
        {signals.map((signal, index) => (
          <button
            type="button"
            key={signal.code}
            className={`signal-card ${selectedCode === signal.code ? 'selected' : ''}`}
            onClick={() => onSelect(signal.code)}
          >
            <div className="signal-card-top">
              <div className="stock-identity">
                <span className="signal-index">{String(index + 1).padStart(2, '0')}</span>
                <div>
                  <strong>{signal.name}</strong>
                  <span className="stock-code">{signal.code}</span>
                </div>
              </div>
              <ArrowUpRight size={17} className="card-arrow" />
            </div>
            <div className="price-row">
              <span className="price">{signal.close.toFixed(2)}</span>
              <span className={`change ${signal.changePct >= 0 ? 'positive' : 'negative'}`}>
                {signal.changePct >= 0 ? '+' : ''}{signal.changePct.toFixed(2)}%
              </span>
            </div>
            <div className="signal-levels">
              <span>中枢上沿 <b>{signal.zg.toFixed(2)}</b></span>
              <span>止损 <b>{signal.pullbackLow.toFixed(2)}</b></span>
            </div>
          </button>
        ))}
        {signals.length === 0 && (
          <div className="list-empty">
            <Search size={24} />
            <p>{keyword ? '没有匹配的标的' : '当前条件下无确认信号'}</p>
            <span>{keyword ? '试试其他代码或名称' : '耐心等待结构确认'}</span>
          </div>
        )}
      </div>
      <div className="list-footer">按成交额排序 · 点击切换结构图</div>
    </section>
  )
}
