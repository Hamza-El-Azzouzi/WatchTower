'use client';

import { Wifi, WifiOff, Loader2 } from 'lucide-react';

interface ConnectionStatusProps {
  state: 'connecting' | 'connected' | 'disconnected';
  className?: string;
  showLabel?: boolean;
}

/**
 * Global connection status indicator for WebSocket connection
 * Shows connected/connecting/disconnected state with visual feedback
 */
export default function ConnectionStatus({ 
  state, 
  className = '',
  showLabel = true 
}: ConnectionStatusProps) {
  const getStatusConfig = () => {
    switch (state) {
      case 'connected':
        return {
          icon: Wifi,
          color: 'text-lime-300',
          bgColor: 'bg-lime-400/8',
          borderColor: 'border-lime-400/15',
          label: 'Live',
          pulse: true,
        };
      case 'connecting':
        return {
          icon: Loader2,
          color: 'text-amber-300',
          bgColor: 'bg-amber-400/8',
          borderColor: 'border-amber-400/15',
          label: 'Connecting',
          pulse: false,
          spin: true,
        };
      case 'disconnected':
      default:
        return {
          icon: WifiOff,
          color: 'text-rose-300',
          bgColor: 'bg-rose-400/8',
          borderColor: 'border-rose-400/15',
          label: 'Disconnected',
          pulse: false,
        };
    }
  };

  const config = getStatusConfig();
  const Icon = config.icon;

  return (
    <div 
      className={`flex items-center gap-2 rounded-full border px-3 py-1.5 ${config.bgColor} ${config.borderColor} ${className}`}
      title={`WebSocket: ${state}`}
    >
      <div className="relative">
        <Icon 
          className={`w-4 h-4 ${config.color} ${config.spin ? 'animate-spin' : ''}`} 
        />
        {config.pulse && (
          <span className="absolute -top-0.5 -right-0.5 w-2 h-2 bg-lime-400 rounded-full animate-pulse" />
        )}
      </div>
      {showLabel && (
        <span className={`text-xs font-medium ${config.color}`}>
          {config.label}
        </span>
      )}
    </div>
  );
}

/**
 * Compact version for header/sidebar use
 */
export function ConnectionStatusDot({ 
  state,
  className = ''
}: { 
  state: 'connecting' | 'connected' | 'disconnected';
  className?: string;
}) {
  const getColor = () => {
    switch (state) {
      case 'connected':
        return 'bg-green-400';
      case 'connecting':
        return 'bg-yellow-400';
      case 'disconnected':
      default:
        return 'bg-red-400';
    }
  };

  return (
    <span 
      className={`inline-block w-2 h-2 rounded-full ${getColor()} ${state === 'connected' || state === 'connecting' ? 'animate-pulse' : ''} ${className}`}
      title={`WebSocket: ${state}`}
    />
  );
}
