<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import * as echarts from 'echarts/core'
import { BarChart, CandlestickChart, LineChart, ScatterChart } from 'echarts/charts'
import { DataZoomComponent, GridComponent, MarkAreaComponent, MarkLineComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import type { EChartsOption } from 'echarts'
import type { Signal } from '../types'

echarts.use([BarChart, CandlestickChart, LineChart, ScatterChart, DataZoomComponent, GridComponent, MarkAreaComponent, MarkLineComponent, TooltipComponent, CanvasRenderer])
const UP = '#d9554d'
const DOWN = '#1c9873'
const props = defineProps<{ signal: Signal }>()
const containerRef = ref<HTMLDivElement | null>(null)
let chart: echarts.EChartsType | null = null
let observer: ResizeObserver | null = null

function render() {
  const data = props.signal.chart
  if (!chart || !data) return
  const option: EChartsOption = {
    animation: false,
    backgroundColor: 'transparent',
    axisPointer: { link: [{ xAxisIndex: 'all' }] },
    grid: [{ left: 58, right: 30, top: 28, height: '61%' }, { left: 58, right: 30, top: '73%', height: '14%' }],
    xAxis: [
      { type: 'category', data: data.dates, boundaryGap: true, axisLine: { lineStyle: { color: '#e4e8e2' } }, axisTick: { show: false }, axisLabel: { show: false } },
      { type: 'category', gridIndex: 1, data: data.dates, boundaryGap: true, axisLine: { lineStyle: { color: '#e4e8e2' } }, axisTick: { show: false }, axisLabel: { color: '#939c96', fontSize: 10, interval: 18 } },
    ],
    yAxis: [
      { scale: true, position: 'left', axisLabel: { color: '#8a938d', fontSize: 11, formatter: (value: number) => value.toFixed(2) }, splitLine: { lineStyle: { color: '#edf0eb', type: 'dashed' } } },
      { gridIndex: 1, scale: true, axisLabel: { color: '#a0a8a2', fontSize: 10 }, splitLine: { show: false } },
    ],
    dataZoom: [
      { type: 'inside', xAxisIndex: [0, 1], start: 12, end: 100, zoomOnMouseWheel: true },
      { type: 'slider', xAxisIndex: [0, 1], bottom: 4, height: 16, borderColor: 'transparent', backgroundColor: '#f4f6f2', fillerColor: 'rgba(30,116,91,.13)', handleStyle: { color: '#1e745b' }, textStyle: { color: '#8b948e', fontSize: 10 } },
    ],
    tooltip: {
      trigger: 'axis', axisPointer: { type: 'cross' }, backgroundColor: 'rgba(23,35,30,.94)', borderWidth: 0,
      textStyle: { color: '#f4f6f2', fontSize: 12 },
      formatter: (params: unknown) => {
        const items = (Array.isArray(params) ? params : [params]) as Array<{ seriesType?: string; data: unknown; name: string }>
        const candle = items.find((item) => item.seriesType === 'candlestick')
        if (!candle || !Array.isArray(candle.data)) return ''
        const values = candle.data as number[]
        const [open, close, low, high] = values.slice(1)
        return `<div style="font-weight:600;margin-bottom:7px">${candle.name}</div><div style="display:grid;grid-template-columns:1fr 1fr;gap:5px 22px"><span>开盘&nbsp; ${open.toFixed(2)}</span><span>收盘&nbsp; ${close.toFixed(2)}</span><span>最低&nbsp; ${low.toFixed(2)}</span><span>最高&nbsp; ${high.toFixed(2)}</span></div>`
      },
    },
    series: [
      {
        name: 'K线', type: 'candlestick', data: data.candles,
        itemStyle: { color: UP, color0: DOWN, borderColor: UP, borderColor0: DOWN },
        markArea: { silent: true, itemStyle: { color: 'rgba(220,174,72,.14)', borderColor: 'rgba(196,149,45,.35)', borderWidth: 1 }, data: [[{ name: '中枢', xAxis: data.pivot.startDate, yAxis: data.pivot.zd }, { xAxis: data.pivot.endDate, yAxis: data.pivot.zg }]] },
        markLine: {
          silent: true, symbol: 'none', label: { fontSize: 10, position: 'insideEndTop' },
          data: [
            { yAxis: data.pivot.zg, lineStyle: { color: '#c39a3d', type: 'dashed' }, label: { formatter: `ZG ${data.pivot.zg.toFixed(2)}`, color: '#a07924' } },
            { yAxis: data.pivot.zd, lineStyle: { color: '#c39a3d', type: 'dashed' }, label: { formatter: `ZD ${data.pivot.zd.toFixed(2)}`, color: '#a07924' } },
            { yAxis: data.buyPoint.price, lineStyle: { color: UP, type: 'dotted' }, label: { formatter: `止损 ${data.buyPoint.price.toFixed(2)}`, color: UP } },
          ],
        },
      },
      { name: '笔', type: 'line', data: data.strokePoints, showSymbol: false, connectNulls: true, z: 4, lineStyle: { color: '#537aa6', width: 1.8 }, emphasis: { disabled: true } },
      { name: '三买', type: 'scatter', data: [[data.buyPoint.date, data.buyPoint.price]], symbol: 'triangle', symbolSize: 15, z: 8, itemStyle: { color: UP, borderColor: '#fff', borderWidth: 2 }, label: { show: true, formatter: `三买 ${data.buyPoint.price.toFixed(2)}`, position: 'bottom', color: UP, fontWeight: 600 } },
      { name: '成交额', type: 'bar', xAxisIndex: 1, yAxisIndex: 1, data: data.volumes.map((value, index) => ({ value, itemStyle: { color: data.candles[index][1] >= data.candles[index][0] ? UP : DOWN, opacity: 0.65 } })) },
    ],
  }
  chart.setOption(option, true)
}

onMounted(async () => {
  await nextTick()
  if (containerRef.value) {
    chart = echarts.init(containerRef.value, undefined, { renderer: 'canvas' })
    observer = new ResizeObserver(() => chart?.resize())
    observer.observe(containerRef.value)
    render()
  }
})
watch(() => props.signal, async () => { await nextTick(); render() }, { deep: true })
onBeforeUnmount(() => { observer?.disconnect(); chart?.dispose(); chart = null })
</script>

<template>
  <div v-if="signal.chart" ref="containerRef" class="structure-chart" role="img" :aria-label="`${signal.name}日线三买结构图`" />
  <div v-else class="chart-empty">当前标的无可用绘图数据</div>
</template>
