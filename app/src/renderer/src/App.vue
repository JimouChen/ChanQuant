<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import {
  Activity, CheckCircle2, CircleDot, Clock3, Database, Layers3,
  LineChart, LoaderCircle, Play, ScanLine, ShieldCheck, Square,
} from 'lucide-vue-next'
import DatePicker from './components/DatePicker.vue'
import SignalList from './components/SignalList.vue'
import StructureChart from './components/StructureChart.vue'
import type { ScanResult } from './types'

function localDate(): string {
  const now = new Date()
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
}
function formatDuration(milliseconds: number | undefined): string {
  if (milliseconds === undefined) return '—'
  if (milliseconds < 1000) return `${Math.round(milliseconds)}`
  if (milliseconds < 60_000) return `${(milliseconds / 1000).toFixed(2)}`
  const minutes = Math.floor(milliseconds / 60_000)
  const seconds = Math.floor((milliseconds % 60_000) / 1000)
  return `${minutes}分${seconds}秒`
}

const date = ref(localDate())
const minAmount = ref(0.5)
const source = ref<'sina' | 'eastmoney'>('sina')
const result = ref<ScanResult | null>(null)
const selectedCode = ref<string | null>(null)
const keyword = ref('')
const loading = ref(false)
const error = ref('')
const elapsedMs = ref(0)
const requestId = ref(0)
let timer: ReturnType<typeof setInterval> | null = null

async function runScan() {
  const id = ++requestId.value
  loading.value = true
  error.value = ''
  elapsedMs.value = 0
  const startedAt = performance.now()
  timer = setInterval(() => { elapsedMs.value = performance.now() - startedAt }, 100)
  try {
    const payload = await invoke<ScanResult>('scan_market', {
      options: { date: date.value, minAmount: minAmount.value, maxPivotAge: 90, source: source.value },
    })
    if (id !== requestId.value) return
    result.value = payload
    selectedCode.value = payload.signals.some((item) => item.code === selectedCode.value)
      ? selectedCode.value : payload.signals[0]?.code ?? null
  } catch (reason) {
    if (id !== requestId.value) return
    error.value = typeof reason === 'string' ? reason : reason instanceof Error ? reason.message : '扫描失败，请稍后重试'
  } finally {
    if (timer) clearInterval(timer)
    timer = null
    if (id === requestId.value) {
      elapsedMs.value = performance.now() - startedAt
      loading.value = false
    }
  }
}

async function cancelScan() {
  requestId.value += 1
  if (timer) clearInterval(timer)
  timer = null
  try { await invoke<boolean>('cancel_scan') } catch { /* 保留上一次结果 */ }
  loading.value = false
}

const selected = computed(() => result.value?.signals.find((item) => item.code === selectedCode.value) ?? null)
const filteredSignals = computed(() => {
  const normalized = keyword.value.trim().toLowerCase()
  return (result.value?.signals ?? []).filter((item) => item.code.includes(normalized) || item.name.toLowerCase().includes(normalized))
})
const duration = computed(() => loading.value ? elapsedMs.value : result.value?.durationMs)
onBeforeUnmount(() => { if (timer) clearInterval(timer) })
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <div class="brand">
        <div class="brand-mark"><LineChart :size="23" :stroke-width="1.8" /></div>
        <div><strong>ChanQuant</strong><span>缠量 · 结构交易</span></div>
      </div>
      <nav class="navigation" aria-label="主导航">
        <span class="nav-caption">分析工作台</span>
        <button class="nav-item active" type="button"><ScanLine :size="18" /><span>三买选股</span><i /></button>
      </nav>
      <div class="method-card">
        <div class="method-title"><Layers3 :size="16" /><span>判定规则</span></div>
        <ol><li><span>01</span>向上离开中枢</li><li><span>02</span>回抽不进中枢</li><li><span>03</span>底分型当日确认</li></ol>
        <p>只输出已确认结构<br />不使用未来函数</p>
      </div>
      <div class="kernel-status">
        <span class="status-dot" :class="{ error }" />
        <div><strong>{{ error ? '内核连接异常' : loading ? '正在计算结构' : '本地计算内核' }}</strong><span>{{ result ? 'Rust · 数据已就绪' : 'Rust · 前复权日线' }}</span></div>
      </div>
    </aside>

    <main class="main-area">
      <header class="topbar">
        <div class="page-title"><span class="eyebrow">STRUCTURE OVER NOISE</span><h1>三买信号 <span>让走势自己说话</span></h1></div>
        <form class="scan-controls" @submit.prevent="runScan">
          <DatePicker :value="date" :max="localDate()" @change="date = $event" />
          <label class="control-field amount-field"><span>成交额</span><select v-model.number="minAmount" aria-label="最低成交额"><option :value="0.5">≥ 0.5 亿</option><option :value="1">≥ 1 亿</option><option :value="2">≥ 2 亿</option></select></label>
          <label class="control-field source-field"><Database :size="15" /><select v-model="source" aria-label="行情数据源"><option value="sina">新浪行情</option><option value="eastmoney">东方财富</option></select></label>
          <button v-if="loading" class="scan-button cancel" type="button" @click="cancelScan"><Square :size="14" fill="currentColor" />取消</button>
          <button v-else class="scan-button" type="submit"><Play :size="15" fill="currentColor" />开始扫描</button>
        </form>
      </header>

      <div class="content">
        <div v-if="error" class="error-banner" role="alert"><CircleDot :size="16" /><span>{{ error }}</span><button @click="runScan">重试</button></div>
        <section class="stats-grid" aria-label="扫描概览">
          <article class="stat-card featured"><div class="stat-icon"><Activity :size="19" /></div><div><span>确认信号</span><strong>{{ result?.signals.length ?? '—' }}<small>只</small></strong></div><span class="stat-note">当日三买</span></article>
          <article class="stat-card"><div class="stat-icon"><Database :size="18" /></div><div><span>主板样本</span><strong>{{ result?.loadedSize.toLocaleString() ?? '—' }}<small>只</small></strong></div><span class="stat-note">剔除ST / 停牌</span></article>
          <article class="stat-card"><div class="stat-icon"><Layers3 :size="18" /></div><div><span>结构初筛</span><strong>{{ result?.funnel.pullback ?? '—' }}<small>只</small></strong></div><span class="stat-note">回抽不进中枢</span></article>
          <article class="stat-card"><div class="stat-icon"><CheckCircle2 :size="18" /></div><div><span>分型确认</span><strong>{{ result?.funnel.confirm ?? '—' }}<small>只</small></strong></div><span class="stat-note">通过时间约束</span></article>
          <article class="stat-card duration-card"><div class="stat-icon"><Clock3 :size="18" /></div><div><span>扫描耗时</span><strong>{{ formatDuration(duration) }}<small v-if="duration === undefined || duration < 60000">{{ (duration ?? 0) < 1000 ? '毫秒' : '秒' }}</small></strong></div><span class="stat-note">行情 + 结构计算</span></article>
        </section>

        <div class="workspace" :class="{ 'is-loading': loading }">
          <SignalList :signals="filteredSignals" :selected-code="selectedCode" :keyword="keyword" @update:keyword="keyword = $event" @select="selectedCode = $event" />
          <section class="chart-panel panel">
            <template v-if="selected">
              <div class="chart-heading">
                <div><div class="chart-stock-name"><h2>{{ selected.name }}</h2><span>{{ selected.code }}</span></div><div class="chart-tags"><span>日线</span><span>前复权</span><span class="confirmed"><CheckCircle2 :size="12" />三买已确认</span></div></div>
                <div class="chart-legend"><span><i class="legend-stroke" />笔</span><span><i class="legend-pivot" />中枢</span><span><i class="legend-buy" />三买</span></div>
              </div>
              <StructureChart :signal="selected" />
              <div class="level-grid">
                <div><span>中枢区间</span><strong>{{ selected.zd.toFixed(2) }} — {{ selected.zg.toFixed(2) }}</strong></div>
                <div><span>离开高点</span><strong>{{ selected.leaveHigh.toFixed(2) }}</strong></div>
                <div><span>回抽低点</span><strong>{{ selected.pullbackLow.toFixed(2) }}<small>距 ZG +{{ selected.distanceZgPct.toFixed(2) }}%</small></strong></div>
                <div class="stop-level"><span>结构止损位</span><strong>{{ selected.pullbackLow.toFixed(2) }}</strong></div>
              </div>
            </template>
            <div v-else class="empty-state">
              <LoaderCircle v-if="loading" :size="38" class="spin" /><ScanLine v-else :size="40" />
              <h2>{{ loading ? '正在解析市场结构' : result ? '当前条件下无确认信号' : '等待首次扫描' }}</h2>
              <p>{{ loading ? '依次处理包含关系、分型、笔与中枢，请稍候' : '耐心等待结构确认，不主观抄底摸顶' }}</p>
            </div>
            <div v-if="loading && selected" class="loading-overlay"><LoaderCircle :size="30" class="spin" /><strong>正在刷新结构</strong><span>扫描期间保留当前结果</span></div>
          </section>
        </div>
      </div>

      <footer class="statusbar">
        <div><ShieldCheck :size="14" /><span>收盘确认 · 无未来函数</span></div>
        <span>{{ result ? `数据截止 ${result.tradeDate} · ${result.source === 'sina' ? '新浪行情' : '东方财富'} · 本次扫描 ${formatDuration(result.durationMs)}${result.durationMs < 60000 ? (result.durationMs < 1000 ? ' 毫秒' : ' 秒') : ''}` : 'Rust 本地数据管线' }}</span>
        <span class="risk-note">跌破回抽低点即结构失效</span>
      </footer>
    </main>
  </div>
</template>
