import { getStatusColor } from '@/lib/metrics-utils';
import type { AgentStatus } from '@/types';

interface StatusBadgeProps {
  status: AgentStatus;
  size?: 'sm' | 'md' | 'lg';
}

export default function StatusBadge({ status, size = 'md' }: StatusBadgeProps) {
  const color = getStatusColor(status);
  
  const bgColorMap = {
    green: 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30',
    yellow: 'bg-amber-500/20 text-amber-300 border border-amber-500/30',
    red: 'bg-red-500/20 text-red-300 border border-red-500/30',
    gray: 'bg-slate-600/20 text-slate-300 border border-slate-500/30',
  };

  const sizeMap = {
    sm: 'px-2.5 py-1 text-xs',
    md: 'px-3.5 py-1.5 text-sm',
    lg: 'px-4 py-2 text-base',
  };

  const dotMap = {
    green: 'bg-emerald-400 animate-pulse-soft',
    yellow: 'bg-amber-400 animate-pulse-soft',
    red: 'bg-red-400 animate-pulse-soft',
    gray: 'bg-slate-400',
  };

  return (
    <span className={`inline-flex items-center rounded-full font-medium transition-smooth ${bgColorMap[color]} ${sizeMap[size]}`}>
      <span className={`w-2 h-2 rounded-full mr-2 ${dotMap[color]}`} />
      {status}
    </span>
  );
}
