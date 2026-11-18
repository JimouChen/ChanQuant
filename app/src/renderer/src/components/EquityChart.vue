<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import * as echarts from 'echarts/core'
import { LineChart } from 'echarts/charts'
import { DataZoomComponent, GridComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import type { EChartsOption } from 'echarts'
import type { EquityPoint } from '../types'

echarts.use([LineChart, DataZoomComponent, GridComponent, TooltipComponent, CanvasRenderer])
const props = defineProps<{ curve: EquityPoint[] }>()
const containerRef = ref<HTMLDivElement | null>(null)
let chart: echarts.EChartsType | null = null
let observer: ResizeObserver | null = null

function render() {
  if (!chart || props.curve.length === 0) return
  const dates = props.curve.map((p) => p.date)
  const option: EChartsOption = {
    animation: false,
    backgroundColor: 'transparent',
    axisPointer: { link: [{ xAxisIndex: 'all' }] },
    grid: [{ left: 62, right: 30, top: 26, height: '56%' }, { left: 62, right: 30, top: '72%', height: '15%' }],
    xAxis: [
      { type: 'category', data: dates, boundaryGap: false, axisLine: { lineStyle: { color: '#e4e8e2' } }, axisTick: { show: false }, axisLabel: { show: false } },
      { type: 'category', gridIndex: 1, data: dates, boundaryGap: false, axisLine: { lineStyle: { color: '#e4e8e2' } }, axisTick: { show: false }, axisLabel: { color: '#939c96', fontSize: 10, interval: Math.max(1, Math.floor(dates.length / 8)) } },
    ],
    yAxis: [
      { scale: true, axisLabel: { color: '#8a938d', fontSize: 11, formatter: (v: number) => `${(v / 10000).toFixed(0)}万` }, splitLine: { lineStyle: { color: '#edf0eb', type: 'dashed' } } },
      { gridIndex: 1, max: 0, axisLabel: { color: '#a0a8a2', fontSize: 10, formatter: (v: number) => `${Math.abs(v).toFixed(0)}%` }, splitLine: { show: false } },
    ],
    dataZoom: [
      { type: 'inside', xAxisIndex: [0, 1], zoomOnMouseWheel: true },
      { type: 'slider', xAxisIndex: [0, 1], bottom: 4, height: 16, borderColor: 'transparent', backgroundColor: '#f4f6f2', fillerColor: 'rgba(30,116,91,.13)', handleStyle: { color: '#1e745b' }, textStyle: { color: '#8b948e', fontSize: 10 } },
    ],
    tooltip: {
      trigger: 'axis', axisPointer: { type: 'cross' }, backgroundColor: 'rgba(23,35,30,.94)', borderWidth: 0,
      textStyle: { color: '#f4f6f2', fontSize: 12 },
      formatter: (params: unknown) => {
        const items = (Array.isArray(params) ? params : [params]) as Array<{ seriesName?: string; value: unknown; name: string }>
        const equity = items.find((i) => i.seriesName === '净值')
        const dd = items.find((i) => i.seriesName === '回撤')
        const lines = [`<div style="font-weight:600;margin-bottom:6px">${items[0]?.name ?? ''}</div>`]
        if (equity) lines.push(`<div>净值&nbsp; ${(Number(equity.value) / 10000).toFixed(2)} 万</div>`)
        if (dd) lines.push(`<div>回撤&nbsp; ${Math.abs(Number(dd.value)).toFixed(2)}%</div>`)
        return lines.join('')
      },
    },
    series: [
      {
        name: '净值', type: 'line', data: props.curve.map((p) => p.equity), showSymbol: false,
        lineStyle: { color: '#1e745b', width: 1.8 },
        areaStyle: { color: 'rgba(30,116,91,.08)' },
      },
      {
        name: '回撤', type: 'line', xAxisIndex: 1, yAxisIndex: 1, data: props.curve.map((p) => -p.drawdown), showSymbol: false,
        lineStyle: { color: '#d9554d', width: 1.2 },
        areaStyle: { color: 'rgba(217,85,77,.12)' },
      },
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
watch(() => props.curve, async () => { await nextTick(); render() }, { deep: true })
onBeforeUnmount(() => { observer?.disconnect(); chart?.dispose(); chart = null })
</script>

<template>
  <div ref="containerRef" class="equity-chart" role="img" aria-label="回测净值与回撤曲线" />
</template>
