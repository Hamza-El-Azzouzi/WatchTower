export function CardSkeleton() {
  return (
    <div className="rounded-lg border border-slate-700 bg-slate-900/50 p-6 animate-pulse">
      <div className="h-6 bg-slate-800 rounded w-1/3 mb-4" />
      <div className="h-8 bg-slate-800 rounded w-1/2 mb-2" />
      <div className="h-4 bg-slate-800 rounded w-2/3" />
    </div>
  );
}

export function MetricSkeleton() {
  return (
    <div className="rounded-lg border-2 border-slate-700 bg-slate-900/20 p-6 animate-pulse">
      <div className="flex items-start justify-between mb-4">
        <div className="flex-1">
          <div className="h-4 bg-slate-800 rounded w-1/3 mb-2" />
        </div>
        <div className="w-6 h-6 bg-slate-800 rounded" />
      </div>
      <div className="h-10 bg-slate-800 rounded w-1/2 mb-4" />
      <div className="h-2 bg-slate-800 rounded-full w-full" />
    </div>
  );
}

export function ChartSkeleton() {
  return (
    <div className="rounded-lg border border-slate-700 bg-slate-900/50 p-6 animate-pulse">
      <div className="h-6 bg-slate-800 rounded w-1/3 mb-6" />
      <div className="h-64 bg-slate-800 rounded" />
    </div>
  );
}

export function ServerCardSkeleton() {
  return (
    <div className="rounded-lg border-2 border-slate-700 bg-slate-900/50 p-6 animate-pulse">
      <div className="flex items-start justify-between mb-4">
        <div>
          <div className="h-6 bg-slate-800 rounded w-2/3 mb-2" />
          <div className="h-3 bg-slate-800 rounded w-1/3" />
        </div>
        <div className="h-6 bg-slate-800 rounded w-1/4" />
      </div>
      <div className="grid grid-cols-3 gap-3">
        {[1, 2, 3].map(i => (
          <div key={i}>
            <div className="h-3 bg-slate-800 rounded w-1/2 mb-2" />
            <div className="h-6 bg-slate-800 rounded w-2/3" />
            <div className="h-1.5 bg-slate-800 rounded-full mt-2" />
          </div>
        ))}
      </div>
    </div>
  );
}
