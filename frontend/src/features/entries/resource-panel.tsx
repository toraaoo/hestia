import type { ReactNode } from 'react';
import { useEffect, useState } from 'react';
import { Area, AreaChart, YAxis } from 'recharts';

import {
  Card,
  CardAction,
  CardContent,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { type ChartConfig, ChartContainer } from '@/components/ui/chart';
import { StatusDot } from '@/components/ui/status-dot';
import { getEntryResources } from '@/features/entries/mock';
import { bytes, memGb } from '@/lib/format';

/** Rolling window length and refresh cadence of the simulated feed. */
const SAMPLES = 40;
const TICK_MS = 1500;

interface Sample {
  cpu: number;
  mem: number;
}

/** A bounded random walk that mimics a live metric drifting toward a baseline. */
function step(
  prev: number,
  target: number,
  jitter: number,
  max: number,
): number {
  const drift = (target - prev) * 0.12;
  const noise = (Math.random() - 0.5) * jitter;
  return Math.max(0, Math.min(max, prev + drift + noise));
}

function seed(running: boolean, cpu: number, mem: number): Sample[] {
  return Array.from({ length: SAMPLES }, () =>
    running
      ? {
          cpu: step(cpu, cpu, 12, 100),
          mem: step(mem, mem, mem * 0.06, mem * 2),
        }
      : { cpu: 0, mem: 0 },
  );
}

/**
 * Keeps a rolling series of CPU/memory samples that ticks on an interval,
 * standing in for a real daemon metrics stream — a running entry drifts around
 * its baseline, a stopped one flatlines.
 */
function useLiveResources(
  running: boolean,
  baseCpu: number,
  baseMem: number,
  memLimitMb: number,
): Sample[] {
  const [series, setSeries] = useState<Sample[]>(() =>
    seed(running, baseCpu, baseMem),
  );

  useEffect(() => {
    setSeries(seed(running, baseCpu, baseMem));
    if (!running) return;
    const id = setInterval(() => {
      setSeries((prev) => {
        const last = prev[prev.length - 1];
        return [
          ...prev.slice(1),
          {
            cpu: step(last.cpu, baseCpu, 14, 100),
            mem: step(last.mem, baseMem, memLimitMb * 0.05, memLimitMb),
          },
        ];
      });
    }, TICK_MS);
    return () => clearInterval(id);
  }, [running, baseCpu, baseMem, memLimitMb]);

  return series;
}

const chartConfig = {
  cpu: { label: 'CPU', color: 'var(--color-ember)' },
  mem: { label: 'Memory', color: 'var(--chart-2)' },
} satisfies ChartConfig;

function Sparkline({
  data,
  dataKey,
  color,
  max,
}: {
  data: Sample[];
  dataKey: 'cpu' | 'mem';
  color: string;
  max: number;
}) {
  return (
    <ChartContainer config={chartConfig} className="aspect-auto h-full w-full">
      <AreaChart data={data} margin={{ top: 4, right: 0, bottom: 0, left: 0 }}>
        <defs>
          <linearGradient id={`fill-${dataKey}`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={color} stopOpacity={0.35} />
            <stop offset="100%" stopColor={color} stopOpacity={0.04} />
          </linearGradient>
        </defs>
        <YAxis domain={[0, max]} hide />
        <Area
          dataKey={dataKey}
          type="monotone"
          stroke={color}
          strokeWidth={1.5}
          fill={`url(#fill-${dataKey})`}
          isAnimationActive={false}
          dot={false}
        />
      </AreaChart>
    </ChartContainer>
  );
}

function MetricCard({
  label,
  value,
  children,
}: {
  label: string;
  value: ReactNode;
  children: ReactNode;
}) {
  return (
    <Card className="gap-3">
      <CardHeader>
        <CardTitle className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
          {label}
        </CardTitle>
        <CardAction className="self-center font-heading text-xl font-semibold tabular-nums">
          {value}
        </CardAction>
      </CardHeader>
      <CardContent className="min-h-24 flex-1">{children}</CardContent>
    </Card>
  );
}

/** A believable, deterministic split of an entry's on-disk footprint. */
const diskParts = [
  { label: 'Worlds', frac: 0.55, color: 'var(--color-ember)' },
  { label: 'Content', frac: 0.3, color: 'var(--chart-2)' },
  { label: 'Other', frac: 0.15, color: 'var(--chart-4)' },
];

/**
 * A full-width disk-usage strip: one continuous segmented bar over an
 * uppercase legend, after Steam's storage-manager breakdown.
 */
function DiskStrip({ diskBytes }: { diskBytes: number }) {
  return (
    <div className="flex flex-col gap-2 bg-card px-4 py-3 ring-1 ring-foreground/10">
      <div className="flex items-baseline justify-between text-xs">
        <span className="tracking-wide text-muted-foreground uppercase">
          Disk
        </span>
        <span className="tabular-nums">{bytes(diskBytes)}</span>
      </div>
      <div className="flex h-1.5 w-full overflow-hidden">
        {diskParts.map((p) => (
          <div
            key={p.label}
            style={{ width: `${p.frac * 100}%`, background: p.color }}
          />
        ))}
      </div>
      <div className="flex flex-wrap items-center gap-x-5 gap-y-1 text-[11px] text-muted-foreground">
        {diskParts.map((p) => (
          <span key={p.label} className="flex items-center gap-1.5">
            <span className="size-2" style={{ background: p.color }} />
            {p.label}
            <span className="tabular-nums text-foreground">
              {bytes(diskBytes * p.frac)}
            </span>
          </span>
        ))}
      </div>
    </div>
  );
}

/**
 * The overview's system-resources area: separate live CPU/memory charts and a
 * disk breakdown, laid out to fill the remaining page height. Resolves its own
 * metrics from the entry id so callers never drill the values.
 */
export function ResourceCards({ id }: { id: string }) {
  const res = getEntryResources(id);
  const limitGb = memGb(res?.memory ?? '');
  const series = useLiveResources(
    res?.running ?? false,
    res?.cpu_pct ?? 0,
    res?.mem_used_mb ?? 0,
    limitGb * 1024,
  );
  const now = series[series.length - 1];

  if (!res) return null;
  const { running, disk_bytes: diskBytes } = res;

  return (
    <div className="flex flex-1 flex-col gap-3">
      <div className="flex items-center gap-2">
        <span className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          System resources
        </span>
        <span className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
          <StatusDot tone={running ? 'on' : 'off'} />
          {running ? 'Live' : 'Idle'}
        </span>
      </div>

      <div className="grid flex-1 gap-3 sm:grid-cols-2">
        <MetricCard
          label="CPU"
          value={running ? `${Math.round(now.cpu)}%` : '—'}
        >
          <Sparkline
            data={series}
            dataKey="cpu"
            color="var(--color-ember)"
            max={100}
          />
        </MetricCard>

        <MetricCard
          label="Memory"
          value={
            running ? (
              <>
                {(now.mem / 1024).toFixed(1)}
                <span className="ml-1 text-xs font-normal text-muted-foreground">
                  / {limitGb} GB
                </span>
              </>
            ) : (
              '—'
            )
          }
        >
          <Sparkline
            data={series}
            dataKey="mem"
            color="var(--chart-2)"
            max={limitGb * 1024}
          />
        </MetricCard>
      </div>

      <DiskStrip diskBytes={diskBytes} />
    </div>
  );
}
