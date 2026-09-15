import React from "react"
import { getMetricColor } from '@/lib/metrics-utils';

interface MetricCardProps {
  label: string;
  value: string | number;
  unit?: string;
  percentage?: number;
  color?: 'green' | 'yellow' | 'red' | 'blue';
  icon?: React.ReactNode;
  secondaryValue?: string;
  size?: 'sm' | 'md' | 'lg';
}

export default function MetricCard({
  label,
  value,
  unit,
  percentage,
  color,
  icon,
  secondaryValue,
  size = 'md',
}: MetricCardProps) {
  const borderColorMap = {
    green: 'hover:border-emerald-400/25',
    yellow: 'hover:border-amber-400/25',
    red: 'hover:border-rose-400/25',
    blue: 'hover:border-cyan-400/25',
  };

  const bgColorMap = {
    green: '', yellow: '', red: '', blue: '',
  };

  const textColorMap = {
    green: 'text-lime-300',
    yellow: 'text-amber-300',
    red: 'text-rose-300',
    blue: 'text-cyan-300',
  };

  const sizeMap = {
    sm: {
      card: 'p-4',
      label: 'text-xs',
      value: 'text-2xl',
      icon: 'w-5 h-5',
    },
    md: {
      card: 'p-6',
      label: 'text-sm',
      value: 'text-4xl',
      icon: 'w-6 h-6',
    },
    lg: {
      card: 'p-8',
      label: 'text-base',
      value: 'text-5xl',
      icon: 'w-8 h-8',
    },
  };

  const finalColor = color || 'blue';
  const sizeConfig = sizeMap[size];

  return (
    <div className={`surface-panel rounded-[20px] transition-smooth group ${borderColorMap[finalColor]} ${bgColorMap[finalColor]} ${sizeConfig.card}`}>
      <div className="flex items-start justify-between mb-4">
        <div className="flex-1">
          <p className={`text-muted-foreground ${sizeConfig.label} font-medium`}>{label}</p>
        </div>
        {icon && <div className={`${textColorMap[finalColor]} flex h-10 w-10 items-center justify-center rounded-xl bg-white/[.035] ring-1 ring-white/6 transition-smooth`}>{icon}</div>}
      </div>

      <div className="flex items-baseline gap-2 mb-3">
        <span className={`font-semibold tracking-[-.04em] text-white ${sizeConfig.value}`}>
          {typeof value === 'number' ? value.toFixed(1) : value}
        </span>
        {unit && <span className={`text-muted-foreground ${sizeConfig.label}`}>{unit}</span>}
      </div>

      {secondaryValue && (
        <p className={`text-muted-foreground ${sizeConfig.label} mb-2`}>{secondaryValue}</p>
      )}

      {typeof percentage === 'number' && (
        <div className="metric-track mt-4">
          <div
            className={`h-full rounded-full transition-all ${
              finalColor === 'green'
                ? 'bg-gradient-to-r from-emerald-400 to-emerald-500'
                : finalColor === 'yellow'
                  ? 'bg-gradient-to-r from-amber-400 to-amber-500'
                  : finalColor === 'red'
                    ? 'bg-gradient-to-r from-red-400 to-red-500'
                    : 'bg-gradient-to-r from-blue-400 to-blue-500'
            }`}
            style={{ width: `${Math.min(percentage, 100)}%` }}
          />
        </div>
      )}
    </div>
  );
}
