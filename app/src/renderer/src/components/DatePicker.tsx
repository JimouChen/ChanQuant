import { useEffect, useRef, useState } from 'react'
import { CalendarDays, ChevronLeft, ChevronRight } from 'lucide-react'

const WEEKDAYS = ['一', '二', '三', '四', '五', '六', '日']

interface DatePickerProps {
  value: string
  max: string
  onChange: (value: string) => void
}

function toKey(year: number, month: number, day: number): string {
  return `${year}-${String(month + 1).padStart(2, '0')}-${String(day).padStart(2, '0')}`
}

function monthDays(year: number, month: number): number {
  return new Date(year, month + 1, 0).getDate()
}

/** 圆角风格日历选择器：触发按钮 + 圆角弹层日历，替代原生 date 输入框。 */
export function DatePicker({ value, max, onChange }: DatePickerProps) {
  const [open, setOpen] = useState(false)
  const [view, setView] = useState(() => {
    const [y, m] = value.split('-').map(Number)
    return { year: y, month: m - 1 }
  })
  const rootRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const [y, m] = value.split('-').map(Number)
    setView({ year: y, month: m - 1 })
    const onPointerDown = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false)
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false)
    }
    document.addEventListener('mousedown', onPointerDown)
    document.addEventListener('keydown', onKeyDown)
    return () => {
      document.removeEventListener('mousedown', onPointerDown)
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [open, value])

  const pick = (day: number) => {
    onChange(toKey(view.year, view.month, day))
    setOpen(false)
  }

  const shiftMonth = (delta: number) => {
    setView((current) => {
      const date = new Date(current.year, current.month + delta, 1)
      return { year: date.getFullYear(), month: date.getMonth() }
    })
  }

  const leadingBlanks = (new Date(view.year, view.month, 1).getDay() + 6) % 7
  const days = monthDays(view.year, view.month)
  const todayKey = max

  return (
    <div className="date-picker" ref={rootRef}>
      <button
        type="button"
        className="control-field date-trigger"
        onClick={() => setOpen((current) => !current)}
        aria-label="扫描日期"
        aria-expanded={open}
      >
        <CalendarDays size={16} />
        <span className="date-trigger-text">{value}</span>
        <span className="date-trigger-hint">日期</span>
      </button>

      {open && (
        <div className="date-popover" role="dialog" aria-label="选择扫描日期">
          <div className="calendar-header">
            <button type="button" className="calendar-nav" onClick={() => shiftMonth(-1)} aria-label="上个月">
              <ChevronLeft size={15} />
            </button>
            <strong>{view.year} 年 {view.month + 1} 月</strong>
            <button
              type="button"
              className="calendar-nav"
              onClick={() => shiftMonth(1)}
              disabled={toKey(view.year, view.month + 1, 1) > max}
              aria-label="下个月"
            >
              <ChevronRight size={15} />
            </button>
          </div>

          <div className="calendar-weekdays">
            {WEEKDAYS.map((label) => <span key={label}>{label}</span>)}
          </div>

          <div className="calendar-grid">
            {Array.from({ length: leadingBlanks }, (_, i) => <span key={`blank-${i}`} />)}
            {Array.from({ length: days }, (_, i) => i + 1).map((day) => {
              const key = toKey(view.year, view.month, day)
              const disabled = key > max
              const selected = key === value
              const isToday = key === todayKey
              return (
                <button
                  key={key}
                  type="button"
                  className={`calendar-day${selected ? ' selected' : ''}${isToday ? ' today' : ''}`}
                  onClick={() => pick(day)}
                  disabled={disabled}
                >
                  {day}
                </button>
              )
            })}
          </div>
        </div>
      )}
    </div>
  )
}
