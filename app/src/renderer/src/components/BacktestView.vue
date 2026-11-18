<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import {
  CircleDot, Database, History, ListOrdered, LoaderCircle,
  Percent, Play, Scale, ShieldCheck, Square, TrendingDown, TrendingUp,
} from 'lucide-vue-next'
import DatePicker from './DatePicker.vue'
import EquityChart from './EquityChart.vue'
import type { BacktestResult } from '../types'

function localDate(): string {
  const now = new Date()
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
}
function yearAgo(): string {
  const now = new Date()
  now.setFullYear(now.getFullYear() - 1)
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
}
function formatDuration(ms: number | undefined): string {
  if (ms === undefined) return '—'
  if (ms < 1000) return `${Math.round(ms)} 毫秒`
  if (ms < 60_000) return `${(ms / 1000).toFixed(2)} 秒`
  return `${Math.floor(ms / 60_000)}分${Math.floor((ms % 60_000) / 1000)}秒`
}

const start = ref(yearAgo())
const end = ref(localDate())
const capitalWan = ref(100)
const maxPositions = ref(5)
const minAmount = ref(0.5)
const source = ref<'sina' | 'eastmoney'>('sina')
const result = ref<BacktestResult | null>(null)
const loading = ref(false)
const error = ref('')
const elapsedMs = ref(0)
const requestId = ref(0)
let timer: ReturnType<typeof setInterval> | null = null

async function runBacktest() {
  const id = ++requestId.value
  loading.value = true
  error.value = ''
  elapsedMs.value = 0
  const startedAt = performance.now()
  timer = setInterval(() => { elapsedMs.value = performance.now() - startedAt }, 100)
  try {
    const payload = await invoke<BacktestResult>('run_backtest', {
      options: {
        start: start.value,
        end: end.value,
        initialCapital: capitalWan.value * 10000,
        maxPositions: maxPositions.value,
        minAmount: minAmount.value,
        maxPivotAge: 90,
        source: source.value,
      },
    })
    if (id !== requestId.value) return
    result.value = payload
  } catch (reason) {
    if (id !== requestId.value) return
    error.value = typeof reason === 'string' ? reason : reason instanceof Error ? reason.message : '回测失败，请稍后重试'
  } finally {
    if (timer) clearInterval(timer)
    timer = null
    if (id === requestId.value) {
      elapsedMs.value = performance.now() - startedAt
      loading.value = false
    }
  }
}

async function cancelBacktest() {
  requestId.value += 1
  if (timer) clearInterval(timer)
  timer = null
  try { await invoke<boolean>('cancel_scan') } catch { /* 保留上一次结果 */ }
  loading.value = false
}

const stats = computed(() => result.value?.stats ?? null)
const duration = computed(() => loading.value ? elapsedMs.value : result.value?.durationMs)
const profitFactorText = computed(() => {
  if (!stats.value) return '—'
  if (stats.value.tradeCount === 0) return '—'
  return stats.value.profitFactor === null ? '∞' : stats.value.profitFactor.toFixed(2)
})
onBeforeUnmount(() => { if (timer) clearInterval(timer) })
</script>

<template>
  <header class="topbar">
    <div class="page-title"><span class="eyebrow">BACKTEST · NO LOOKAHEAD</span><h1>历史回测 <span>结构止损 · 逐日滚动</span></h1></div>
    <form class="scan-controls" @submit.prevent="runBacktest">
      <DatePicker :value="start" :max="end" @change="start = $event" />
      <DatePicker :value="end" :max="localDate()" @change="end = $event" />
      <label class="control-field"><span>资金</span><input v-model.number="capitalWan" type="number" min="1" max="10000" style="width:52px" aria-label="初始资金（万元）" /><span>万</span></label>
      <label class="control-field"><span>持仓</span><select v-model.number="maxPositions" aria-label="最大持仓数"><option :value="3">3 只</option><option :value="5">5 只</option><option :value="10">10 只</option></select></label>
      <label class="control-field"><Database :size="15" /><select v-model="source" aria-label="行情数据源"><option value="sina">新浪行情</option><option value="eastmoney">东方财富</option></select></label>
      <button v-if="loading" class="scan-button cancel" type="button" @click="cancelBacktest"><Square :size="14" fill="currentColor" />取消</button>
      <button v-else class="scan-button" type="submit"><Play :size="15" fill="currentColor" />开始回测</button>
    </form>
  </header>

  <div class="content">
    <div v-if="error" class="error-banner" role="alert"><CircleDot :size="16" /><span>{{ error }}</span><button @click="runBacktest">重试</button></div>
    <section class="stats-grid six" aria-label="回测绩效">
      <article class="stat-card featured"><div class="stat-icon"><TrendingUp :size="19" /></div><div><span>总收益</span><strong>{{ stats ? `${stats.totalReturnPct > 0 ? '+' : ''}${stats.totalReturnPct.toFixed(2)}` : '—' }}<small>%</small></strong></div><span class="stat-note">期末净值 {{ stats ? (stats.finalEquity / 10000).toFixed(1) : '—' }} 万</span></article>
      <article class="stat-card"><div class="stat-icon"><TrendingUp :size="18" /></div><div><span>年化收益</span><strong>{{ stats ? `${stats.annualizedPct > 0 ? '+' : ''}${stats.annualizedPct.toFixed(2)}` : '—' }}<small>%</small></strong></div><span class="stat-note">252 交易日折算</span></article>
      <article class="stat-card"><div class="stat-icon"><TrendingDown :size="18" /></div><div><span>最大回撤</span><strong>{{ stats ? stats.maxDrawdownPct.toFixed(2) : '—' }}<small>%</small></strong></div><span class="stat-note">净值峰值回落</span></article>
      <article class="stat-card"><div class="stat-icon"><Percent :size="18" /></div><div><span>胜率</span><strong>{{ stats ? stats.winRatePct.toFixed(1) : '—' }}<small>%</small></strong></div><span class="stat-note">盈利笔数占比</span></article>
      <article class="stat-card"><div class="stat-icon"><Scale :size="18" /></div><div><span>盈亏比</span><strong>{{ profitFactorText }}</strong></div><span class="stat-note">总盈利 / 总亏损</span></article>
      <article class="stat-card duration-card"><div class="stat-icon"><ListOrdered :size="18" /></div><div><span>交易笔数</span><strong>{{ stats?.tradeCount ?? '—' }}<small>笔</small></strong></div><span class="stat-note">信号 {{ result?.signalCount ?? '—' }} · 耗时 {{ formatDuration(duration) }}</span></article>
    </section>

    <div class="bt-workspace" :class="{ 'is-loading': loading }">
      <section class="panel bt-chart-panel">
        <template v-if="result && result.equityCurve.length">
          <div class="chart-heading">
            <div><div class="chart-stock-name"><h2>净值与回撤</h2><span>{{ result.start }} → {{ result.end }}</span></div><div class="chart-tags"><span>日线</span><span>结构止损</span><span>T+1 · 涨跌停约束</span></div></div>
            <div class="chart-legend"><span><i class="legend-equity" />净值</span><span><i class="legend-drawdown" />回撤</span></div>
          </div>
          <EquityChart :curve="result.equityCurve" />
        </template>
        <div v-else class="empty-state">
          <LoaderCircle v-if="loading" :size="38" class="spin" /><History v-else :size="40" />
          <h2>{{ loading ? '正在逐日滚动回测' : '等待首次回测' }}</h2>
          <p>{{ loading ? '信号次日开盘成交，收盘跌破 ZG 次日止损' : '佣金万 2.5 · 印花税万 5 · 滑点千 1 · 每笔按 100 股取整' }}</p>
        </div>
        <div v-if="loading && result" class="loading-overlay"><LoaderCircle :size="30" class="spin" /><strong>正在刷新回测</strong><span>回测期间保留当前结果</span></div>
      </section>

      <section class="panel bt-trades-panel">
        <div class="panel-heading bt-trades-heading"><h2>交易明细</h2><span v-if="result">平均持有 {{ result.stats.avgHoldDays.toFixed(1) }} 个交易日</span></div>
        <div v-if="result && result.trades.length" class="bt-table-wrap">
          <table class="bt-table">
            <thead><tr><th>代码</th><th>名称</th><th>买入日</th><th>买入价</th><th>卖出日</th><th>卖出价</th><th>股数</th><th>盈亏</th><th>盈亏%</th><th>持有</th><th>出场</th></tr></thead>
            <tbody>
              <tr v-for="trade in result.trades" :key="`${trade.code}-${trade.entryDate}`">
                <td class="mono">{{ trade.code }}</td><td>{{ trade.name }}</td>
                <td class="mono">{{ trade.entryDate }}</td><td class="mono">{{ trade.entryPrice.toFixed(2) }}</td>
                <td class="mono">{{ trade.exitDate }}</td><td class="mono">{{ trade.exitPrice.toFixed(2) }}</td>
                <td class="mono">{{ trade.shares.toLocaleString() }}</td>
                <td class="mono" :class="trade.pnl >= 0 ? 'profit' : 'loss'">{{ trade.pnl >= 0 ? '+' : ''}}{{ trade.pnl.toLocaleString() }}</td>
                <td class="mono" :class="trade.pnl >= 0 ? 'profit' : 'loss'">{{ trade.pnlPct >= 0 ? '+' : ''}}{{ trade.pnlPct.toFixed(2) }}</td>
                <td class="mono">{{ trade.holdDays }}</td><td>{{ trade.exitReason }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-else class="empty-state compact"><p>{{ loading ? '回测完成后展示逐笔成交' : '暂无交易记录' }}</p></div>
      </section>
    </div>
  </div>

  <footer class="statusbar">
    <div><ShieldCheck :size="14" /><span>收盘确认 · 次日成交 · 无未来函数</span></div>
    <span>{{ result ? `区间 ${result.start} → ${result.end} · ${result.source === 'sina' ? '新浪行情' : '东方财富'} · 样本 ${result.loadedSize.toLocaleString()} 只 · 本次回测 ${formatDuration(result.durationMs)}` : 'Rust 本地回测内核' }}</span>
    <span class="risk-note">跌破中枢上沿 ZG 即结构失效</span>
  </footer>
</template>
