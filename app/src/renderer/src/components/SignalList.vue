<script setup lang="ts">
import { ArrowUpRight, Search } from 'lucide-vue-next'
import type { Signal } from '../types'

defineProps<{
  signals: Signal[]
  selectedCode: string | null
  keyword: string
}>()
const emit = defineEmits<{
  select: [code: string]
  'update:keyword': [value: string]
}>()
</script>

<template>
  <section class="signal-panel panel">
    <div class="panel-heading">
      <div><span class="eyebrow">WATCHLIST</span><h2>今日信号</h2></div>
      <span class="count-badge">{{ signals.length }}</span>
    </div>
    <label class="search-box">
      <Search :size="16" />
      <input :value="keyword" placeholder="搜索代码 / 名称" aria-label="搜索信号" @input="emit('update:keyword', ($event.target as HTMLInputElement).value)" />
    </label>
    <div class="signal-list">
      <button v-for="(signal, index) in signals" :key="signal.code" type="button" class="signal-card" :class="{ selected: selectedCode === signal.code }" @click="emit('select', signal.code)">
        <div class="signal-card-top">
          <div class="stock-identity">
            <span class="signal-index">{{ String(index + 1).padStart(2, '0') }}</span>
            <div><strong>{{ signal.name }}</strong><span class="stock-code">{{ signal.code }}</span></div>
          </div>
          <ArrowUpRight :size="17" class="card-arrow" />
        </div>
        <div class="price-row">
          <span class="price">{{ signal.close.toFixed(2) }}</span>
          <span class="change" :class="signal.changePct >= 0 ? 'positive' : 'negative'">{{ signal.changePct >= 0 ? '+' : '' }}{{ signal.changePct.toFixed(2) }}%</span>
        </div>
        <div class="signal-levels"><span>中枢上沿 <b>{{ signal.zg.toFixed(2) }}</b></span><span>止损 <b>{{ signal.pullbackLow.toFixed(2) }}</b></span></div>
      </button>
      <div v-if="signals.length === 0" class="list-empty">
        <Search :size="24" /><p>{{ keyword ? '没有匹配的标的' : '当前条件下无确认信号' }}</p><span>{{ keyword ? '试试其他代码或名称' : '耐心等待结构确认' }}</span>
      </div>
    </div>
    <div class="list-footer">按成交额排序 · 点击切换结构图</div>
  </section>
</template>
