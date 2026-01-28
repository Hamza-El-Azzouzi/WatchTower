'use client';

import { useEffect, useState } from 'react';
import { RefreshCw } from 'lucide-react';

export default function PageHeader() {
  const [currentTime, setCurrentTime] = useState('');
  const [isRefreshing, setIsRefreshing] = useState(false);

  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      setCurrentTime(
        now.toLocaleTimeString('en-US', {
          hour: '2-digit',
          minute: '2-digit',
          second: '2-digit',
        })
      );
    };

    updateTime();
    const interval = setInterval(updateTime, 1000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="border-b border-border sticky top-0 z-10 glass-morphism">
      <div className="ml-64 max-w-full px-8 py-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold text-foreground">DevOps Monitoring</h1>
            <p className="text-sm text-muted-foreground mt-1">Real-time infrastructure monitoring dashboard</p>
          </div>
          <div className="text-right flex items-center gap-6">
            <div className="text-center">
              <div className="text-sm text-muted-foreground flex items-center gap-2 justify-end">
                <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse-soft" />
                Live data
              </div>
              <p className="text-xl font-mono text-accent mt-2">{currentTime}</p>
            </div>
            <div className="px-3 py-2 rounded-lg bg-accent/10 border border-accent/30">
              <RefreshCw className="w-4 h-4 animate-spin text-accent" />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
