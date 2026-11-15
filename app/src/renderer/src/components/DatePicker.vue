<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { CalendarDays, ChevronLeft, ChevronRight } from 'lucide-vue-next'

const props = defineProps<{ value: string; max: string }>()
const emit = defineEmits<{ change: [value: string] }>()
const WEEKDAYS = ['一', '二', '三', '四', '五', '六', '日']
const open = ref(false)
const rootRef = ref<HTMLDivElement | null>(null)
const [initialYear, initialMonth] = props.value.split('-').map(Number)
const view = reactive({ year: initialYear, month: initialMonth - 1 })

function toKey(year: number, month: number, day: number): string {
  return `${year}-${String(month + 1).padStart(2, '0')}-${String(day).padStart(2, '0')}`
}
function shiftMonth(delta: number) {
  const date = new Date(view.year, view.month + delta, 1)
  view.year = date.getFullYear()
  view.month = date.getMonth()
}
function pick(day: number) {
  emit('change', toKey(view.year, view.month, day))
  open.value = false
}
const leadingBlanks = computed(() => (new Date(view.year, view.month, 1).getDay() + 6) % 7)
const days = computed(() => new Date(view.year, view.month + 1, 0).getDate())

watch(open, (isOpen, _previous, onCleanup) => {
  if (!isOpen) return
  const [year, month] = props.value.split('-').map(Number)
  view.year = year
  view.month = month - 1
  const onPointerDown = (event: MouseEvent) => {
    if (!rootRef.value?.contains(event.target as Node)) open.value = false
  }
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') open.value = false
  }
  document.addEventListener('mousedown', onPointerDown)
  document.addEventListener('keydown', onKeyDown)
  onCleanup(() => {
    document.removeEventListener('mousedown', onPointerDown)
    document.removeEventListener('keydown', onKeyDown)
  })
})
</script>

<template>
  <div ref="rootRef" class="date-picker">
    <button type="button" class="control-field date-trigger" aria-label="扫描日期" :aria-expanded="open" @click="open = !open">
      <CalendarDays :size="16" />
      <span class="date-trigger-text">{{ value }}</span>
      <span class="date-trigger-hint">日期</span>
    </button>
    <div v-if="open" class="date-popover" role="dialog" aria-label="选择扫描日期">
      <div class="calendar-header">
        <button type="button" class="calendar-nav" aria-label="上个月" @click="shiftMonth(-1)"><ChevronLeft :size="15" /></button>
        <strong>{{ view.year }} 年 {{ view.month + 1 }} 月</strong>
        <button type="button" class="calendar-nav" :disabled="toKey(view.year, view.month + 1, 1) > max" aria-label="下个月" @click="shiftMonth(1)"><ChevronRight :size="15" /></button>
      </div>
      <div class="calendar-weekdays"><span v-for="label in WEEKDAYS" :key="label">{{ label }}</span></div>
      <div class="calendar-grid">
        <span v-for="index in leadingBlanks" :key="`blank-${index}`" />
        <button
          v-for="day in days"
          :key="day"
          type="button"
          class="calendar-day"
          :class="{ selected: toKey(view.year, view.month, day) === value, today: toKey(view.year, view.month, day) === max }"
          :disabled="toKey(view.year, view.month, day) > max"
          @click="pick(day)"
        >{{ day }}</button>
      </div>
    </div>
  </div>
</template>
