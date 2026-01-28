'use client';

import { useEffect, useState, useCallback } from 'react';

interface UsePollingOptions {
  interval?: number;
  enabled?: boolean;
}

export function usePolling<T>(
  fetchFn: () => Promise<T>,
  options: UsePollingOptions = {}
) {
  const { interval = 10000, enabled = true } = options;

  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);

  const fetch = useCallback(async () => {
    try {
      setIsRefreshing(true);
      const result = await fetchFn();
      setData(result);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred');
    } finally {
      setLoading(false);
      setIsRefreshing(false);
    }
  }, [fetchFn]);

  useEffect(() => {
    if (!enabled) return;

    // Initial fetch
    fetch();

    // Set up polling interval
    const intervalId = setInterval(fetch, interval);

    return () => clearInterval(intervalId);
  }, [fetch, interval, enabled]);

  return { data, loading, error, isRefreshing, refetch: fetch };
}
