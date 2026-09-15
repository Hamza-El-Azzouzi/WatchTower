'use client';

import { useState, useEffect, useCallback } from 'react';
import { getAgents } from '@/lib/api';
import { Agent } from '@/types';
import { useLogsWebSocket } from '@/hooks/useWebSocket';
import { WsLogMessage } from '@/lib/websocket';
import { getAuthHeaders } from '@/lib/auth-utils';

// Log level enum
type LogLevel = 'DEBUG' | 'INFO' | 'WARN' | 'ERROR' | 'FATAL';

// Log entry type
interface LogEntry {
  id: number;
  agent_id: string;
  timestamp: string;
  level: LogLevel;
  source: string;
  message: string;
}

// Logs response
interface LogsResponse {
  logs: LogEntry[];
  count: number;
  total_count: number;
}

// Level badge colors
const levelColors: Record<LogLevel, string> = {
  DEBUG: 'bg-gray-100 text-gray-800',
  INFO: 'bg-blue-100 text-blue-800',
  WARN: 'bg-yellow-100 text-yellow-800',
  ERROR: 'bg-red-100 text-red-800',
  FATAL: 'bg-purple-100 text-purple-800',
};

export default function LogsPage() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [totalCount, setTotalCount] = useState(0);

  // Filters
  const [selectedAgent, setSelectedAgent] = useState<string>('');
  const [selectedLevel, setSelectedLevel] = useState<string>('');
  const [keyword, setKeyword] = useState('');
  const [limit, setLimit] = useState(100);
  const [autoRefresh, setAutoRefresh] = useState(true);

  // Handle real-time log messages via WebSocket
  const handleLogMessage = useCallback((message: WsLogMessage) => {
    const newLog: LogEntry = {
      id: Date.now(), // Generate a temporary ID
      agent_id: message.agent_id,
      timestamp: message.timestamp,
      level: message.level.toUpperCase() as LogLevel,
      source: 'websocket',
      message: message.message
    };
    
    // Apply filters
    if (selectedAgent && newLog.agent_id !== selectedAgent) return;
    if (selectedLevel && newLog.level !== selectedLevel) return;
    if (keyword && !newLog.message.toLowerCase().includes(keyword.toLowerCase())) return;
    
    // Add to logs and keep max limit
    setLogs(prev => [newLog, ...prev].slice(0, limit));
    setTotalCount(prev => prev + 1);
  }, [selectedAgent, selectedLevel, keyword, limit]);

  const { isConnected: wsConnected } = useLogsWebSocket(handleLogMessage);

  // Initial load from HTTP API (get historical logs)
  const fetchLogs = useCallback(async () => {
    try {
      const params = new URLSearchParams();
      if (selectedAgent) params.append('agent_id', selectedAgent);
      if (selectedLevel) params.append('level', selectedLevel);
      if (keyword) params.append('keyword', keyword);
      params.append('limit', limit.toString());

      const response = await fetch(
        `${process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'}/api/v1/logs?${params}`,
        { headers: getAuthHeaders() },
      );
      
      if (!response.ok) {
        throw new Error(`Failed to fetch logs: ${response.status}`);
      }

      const data: LogsResponse = await response.json();
      setLogs(data.logs);
      setTotalCount(data.total_count);
      setError(null);
    } catch (err) {
      console.error('Error fetching logs:', err);
      setError(err instanceof Error ? err.message : 'Failed to fetch logs');
    } finally {
      setLoading(false);
    }
  }, [keyword, limit, selectedAgent, selectedLevel]);

  // Fetch agents for filter dropdown
  useEffect(() => {
    const loadAgents = async () => {
      try {
        const agentsList = await getAgents();
        setAgents(agentsList);
      } catch (err) {
        console.error('Error fetching agents:', err);
      }
    };
    loadAgents();
  }, []);

  // Fetch logs on mount and when filters change
  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  // Remove auto-refresh polling - WebSocket handles real-time updates
  // Auto-refresh is now just for clearing the view
  const handleClearLogs = () => {
    setLogs([]);
    fetchLogs();
  };

  // Format timestamp
  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp);
    return date.toLocaleString();
  };

  if (loading && logs.length === 0) {
    return (
      <div className="p-6">
        <div className="flex justify-center items-center h-64">
          <div className="text-gray-500">Loading logs...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <h1 className="text-3xl font-bold">System Logs</h1>
        <div className="flex items-center gap-4">
          <div className="text-sm text-gray-600">
            Showing {logs.length} of {totalCount} logs
          </div>
          <button
            onClick={fetchLogs}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
          >
            Refresh
          </button>
        </div>
      </div>

      {/* Filters */}
      <div className="bg-white p-4 rounded-lg shadow space-y-4">
        <div className="flex items-center gap-4 flex-wrap">
          {/* Agent Filter */}
          <div className="flex-1 min-w-[200px]">
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Agent
            </label>
            <select
              value={selectedAgent}
              onChange={(e) => setSelectedAgent(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            >
              <option value="">All Agents</option>
              {agents.map((agent) => (
                <option key={agent.id} value={agent.id}>
                  {agent.name}
                </option>
              ))}
            </select>
          </div>

          {/* Level Filter */}
          <div className="flex-1 min-w-[200px]">
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Level
            </label>
            <select
              value={selectedLevel}
              onChange={(e) => setSelectedLevel(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            >
              <option value="">All Levels</option>
              <option value="DEBUG">DEBUG</option>
              <option value="INFO">INFO</option>
              <option value="WARN">WARN</option>
              <option value="ERROR">ERROR</option>
              <option value="FATAL">FATAL</option>
            </select>
          </div>

          {/* Keyword Filter */}
          <div className="flex-1 min-w-[200px]">
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Keyword Search
            </label>
            <input
              type="text"
              value={keyword}
              onChange={(e) => setKeyword(e.target.value)}
              placeholder="Search in messages..."
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
          </div>

          {/* Limit */}
          <div className="w-32">
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Limit
            </label>
            <select
              value={limit}
              onChange={(e) => setLimit(Number(e.target.value))}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            >
              <option value={50}>50</option>
              <option value={100}>100</option>
              <option value={250}>250</option>
              <option value={500}>500</option>
            </select>
          </div>
        </div>

        {/* Auto-refresh toggle */}
        <div className="flex items-center gap-2">
          <input
            type="checkbox"
            id="auto-refresh"
            checked={autoRefresh}
            onChange={(e) => setAutoRefresh(e.target.checked)}
            className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
          />
          <label htmlFor="auto-refresh" className="text-sm text-gray-700">
            Auto-refresh every 5 seconds
          </label>
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <div className="bg-red-50 border border-red-200 text-red-800 px-4 py-3 rounded-lg">
          {error}
        </div>
      )}

      {/* Logs Table */}
      <div className="bg-white rounded-lg shadow overflow-hidden">
        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Timestamp
                </th>
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Agent
                </th>
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Level
                </th>
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Source
                </th>
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Message
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {logs.length === 0 ? (
                <tr>
                  <td colSpan={5} className="px-4 py-8 text-center text-gray-500">
                    No logs found. Try adjusting your filters or wait for logs to be collected.
                  </td>
                </tr>
              ) : (
                logs.map((log) => (
                  <tr key={log.id} className="hover:bg-gray-50">
                    <td className="px-4 py-3 whitespace-nowrap text-sm text-gray-900">
                      {formatTimestamp(log.timestamp)}
                    </td>
                    <td className="px-4 py-3 whitespace-nowrap text-sm text-gray-900">
                      {log.agent_id}
                    </td>
                    <td className="px-4 py-3 whitespace-nowrap">
                      <span
                        className={`inline-flex px-2 py-1 text-xs font-semibold rounded-full ${
                          levelColors[log.level]
                        }`}
                      >
                        {log.level}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-sm text-gray-600 truncate max-w-xs">
                      {log.source}
                    </td>
                    <td className="px-4 py-3 text-sm text-gray-900">
                      {log.message}
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
