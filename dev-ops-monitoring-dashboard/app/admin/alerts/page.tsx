import Link from 'next/link';
import { KeyRound } from 'lucide-react';

export default function AdminAlertsPage() {
  return (
    <div className="p-8">
      <div className="glass-morphism rounded-xl p-8 max-w-2xl">
        <KeyRound className="w-8 h-8 text-primary mb-4" />
        <h1 className="text-2xl font-semibold mb-3">Enterprise-owned alerting</h1>
        <p className="text-muted-foreground mb-6">
          Enterprise API key owners manage their own rules, alerts, notification channels,
          and maintenance windows. Platform administration manages API keys, not enterprise policies.
        </p>
        <Link href="/admin/api-keys" className="inline-flex rounded-lg bg-primary px-4 py-2 text-primary-foreground">
          Manage API keys
        </Link>
      </div>
    </div>
  );
}
