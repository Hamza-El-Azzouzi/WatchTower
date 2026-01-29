"use client";

import { useConnectionStatus } from "@/hooks/useWebSocket";
import { Badge } from "@/components/ui/badge";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

interface ConnectionStatusProps {
  endpoint: "metrics" | "logs" | "alerts";
  label?: string;
}

export function ConnectionStatus({ endpoint, label }: ConnectionStatusProps) {
  const { statusText, statusColor, connectionState } = useConnectionStatus(endpoint);

  const variant = {
    connected: "default" as const,
    connecting: "secondary" as const,
    disconnected: "destructive" as const,
  }[connectionState];

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Badge variant={variant} className="cursor-help">
            <span
              className={`inline-block w-2 h-2 rounded-full mr-2 ${
                statusColor === "green"
                  ? "bg-green-500 animate-pulse"
                  : statusColor === "yellow"
                  ? "bg-yellow-500 animate-pulse"
                  : "bg-red-500"
              }`}
            />
            {label || endpoint} • {statusText}
          </Badge>
        </TooltipTrigger>
        <TooltipContent>
          <p>
            Real-time {endpoint} updates via WebSocket
            <br />
            Status: {statusText}
          </p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
