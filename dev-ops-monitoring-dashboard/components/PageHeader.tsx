'use client';

import { useEffect, useState } from 'react';
import { usePathname } from 'next/navigation';
import { Clock3 } from 'lucide-react';
import ConnectionStatus from '@/components/ConnectionStatus';
import { useMetricsContext } from '@/contexts/MetricsContext';

const pageDetails: Record<string, { title: string; description: string }> = {
  '/': { title: 'Fleet overview', description: 'Live health and capacity across your infrastructure' },
  '/performance': { title: 'Performance', description: 'Compare resource pressure and system efficiency' },
  '/databases': { title: 'Databases', description: 'Connection health and workload telemetry' },
  '/logs': { title: 'Logs', description: 'Search and inspect events across every agent' },
  '/alerts': { title: 'Alerts', description: 'Prioritize and resolve active incidents' },
};

export default function PageHeader() {
  const pathname = usePathname();
  const { connectionState } = useMetricsContext();
  const [currentTime, setCurrentTime] = useState('');

  useEffect(() => {
    const updateTime = () => setCurrentTime(new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }));
    updateTime();
    const interval = setInterval(updateTime, 30_000);
    return () => clearInterval(interval);
  }, []);

  const details = pageDetails[pathname] ?? pageDetails['/'];

  return (
    <header className="sticky top-16 z-40 border-b border-white/8 bg-[#070b12]/78 backdrop-blur-2xl lg:top-0">
      <div className="mx-auto flex min-h-[82px] max-w-[1600px] items-center justify-between gap-4 px-4 sm:px-6 lg:px-10">
        <div className="min-w-0">
          <p className="eyebrow mb-1">WatchTower / Operations</p>
          <h1 className="truncate text-xl font-semibold tracking-[-0.025em] text-white sm:text-2xl">{details.title}</h1>
          <p className="mt-0.5 hidden text-sm text-muted-foreground sm:block">{details.description}</p>
        </div>
        <div className="flex shrink-0 items-center gap-2 sm:gap-3">
          <div className="hidden items-center gap-2 rounded-xl border border-white/8 bg-white/[.025] px-3 py-2 text-xs text-slate-400 sm:flex">
            <Clock3 className="h-3.5 w-3.5" />
            <time>{currentTime}</time>
          </div>
          <ConnectionStatus state={connectionState} />
        </div>
      </div>
    </header>
  );
}
