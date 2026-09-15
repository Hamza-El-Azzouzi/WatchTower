import { getStatusColor } from '@/lib/metrics-utils';
import type { AgentStatus } from '@/types';

interface StatusBadgeProps {
  status: AgentStatus;
  size?: 'sm' | 'md' | 'lg';
}

export default function StatusBadge({ status, size = 'md' }: StatusBadgeProps) {
  const color = getStatusColor(status);
  
  const bgColorMap = {
    green: 'bg-lime-400/8 text-lime-300 ring-1 ring-inset ring-lime-400/18',
    yellow: 'bg-amber-400/8 text-amber-300 ring-1 ring-inset ring-amber-400/18',
    red: 'bg-rose-400/8 text-rose-300 ring-1 ring-inset ring-rose-400/18',
    gray: 'bg-slate-400/8 text-slate-300 ring-1 ring-inset ring-slate-400/18',
  };

  const sizeMap = {
    sm: 'px-2.5 py-1 text-xs',
    md: 'px-3.5 py-1.5 text-sm',
    lg: 'px-4 py-2 text-base',
  };

  const dotMap = {
    green: 'bg-lime-400',
    yellow: 'bg-amber-400 animate-pulse-soft',
    red: 'bg-rose-400 animate-pulse-soft',
    gray: 'bg-slate-400',
  };

  return (
    <span className={`inline-flex items-center rounded-full font-medium ${bgColorMap[color]} ${sizeMap[size]}`}>
      <span className={`w-2 h-2 rounded-full mr-2 ${dotMap[color]}`} />
      {status}
    </span>
  );
}
