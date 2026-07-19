import { useEffect, useRef, useState } from 'react';

import type { ProvisionPhase, ProvisionProgress } from '@/api';
import {
  Progress,
  ProgressLabel,
  ProgressValue,
} from '@/components/ui/progress';
import { bytes, bytesPerSecond, RateMeter } from '@/lib/format';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

/** Phases whose `current`/`total` are bytes — the ones that carry a speed. */
const BYTE_PHASES: ProvisionPhase[] = ['java', 'server', 'client', 'content'];
/** Phases whose `current`/`total` are completed/total unit counts. */
const COUNT_PHASES: ProvisionPhase[] = ['libraries', 'assets', 'backup'];

/** Map a live provisioning phase to a human label; falls back to the raw id. */
export function phaseLabel(phase: ProvisionPhase): string {
  switch (phase) {
    case 'resolving':
      return m['phase.resolving_profile']();
    case 'backup':
      return m['phase.backing_up']();
    case 'java':
      return m['phase.installing_java']();
    case 'server':
      return m['phase.downloading_server']();
    case 'client':
    case 'libraries':
    case 'assets':
      return m['phase.downloading']({ name: phase });
    case 'content':
      return m['phase.mirroring']();
    default:
      return phase;
  }
}

/**
 * Gauge fill for a progress event. A multi-unit phase (`items > 0`) fills
 * monotonically across the whole batch — completed units plus the current
 * unit's fraction — so cached or instant units still advance the bar instead of
 * snapping back on each per-file reset (mirrors the CLI's `overall_ratio`).
 */
export function overallRatio(p: ProvisionProgress): number {
  const unit = p.total > 0 ? Math.min(1, p.current / p.total) : 0;
  if (p.items && p.items > 0) {
    return Math.min(1, ((p.item ?? 1) - 1 + unit) / p.items);
  }
  return unit;
}

/**
 * The numeric detail line under the bar: `item/items · detail · current /
 * total · rate/s` for a byte phase, `current / total` for a count phase
 * (mirrors the CLI's `bytes_detail`).
 */
function detailLine(p: ProvisionProgress, rate: number): string {
  const parts: string[] = [];
  if (p.items && p.items > 0) parts.push(`${p.item ?? 0}/${p.items}`);
  if (BYTE_PHASES.includes(p.phase)) {
    if (p.detail) parts.push(p.detail);
    const total = p.total > 0 ? bytes(p.total) : '?';
    parts.push(`${bytes(p.current)} / ${total}`);
    if (rate > 0) parts.push(bytesPerSecond(rate));
  } else if (COUNT_PHASES.includes(p.phase) && p.total > 0) {
    parts.push(`${p.current} / ${p.total}`);
  } else if (p.detail) {
    parts.push(p.detail);
  }
  return parts.join(' · ');
}

/**
 * Track byte throughput across successive progress events, resetting the meter
 * on a phase change (each phase is its own stream). Observing in an effect
 * keeps render pure; the meter only refreshes its figure once per window, so
 * intermediate events return the last reading.
 */
function useRate(progress: ProvisionProgress | null): number {
  const meter = useRef(new RateMeter());
  const phase = useRef<ProvisionPhase | null>(null);
  const [rate, setRate] = useState(0);
  useEffect(() => {
    if (!progress) {
      meter.current.reset();
      phase.current = null;
      setRate(0);
      return;
    }
    if (progress.phase !== phase.current) {
      meter.current.reset();
      phase.current = progress.phase;
    }
    setRate(meter.current.observe(progress.current));
  }, [progress]);
  return rate;
}

/**
 * A provisioning progress readout: phase label, percentage, a numeric detail
 * line with live download speed, and the bar itself. `indeterminate` shows a
 * sweeping bar with no figures — for work whose total is unknown (an instance
 * create writes a record; its files download at launch).
 */
export function ProvisionProgressView({
  progress,
  indeterminate = false,
  fallbackLabel,
  className,
}: {
  progress: ProvisionProgress | null;
  indeterminate?: boolean;
  fallbackLabel?: string;
  className?: string;
}) {
  const rate = useRate(indeterminate ? null : progress);
  const label = progress
    ? phaseLabel(progress.phase)
    : (fallbackLabel ?? m['phase.resolving_profile']());

  if (indeterminate) {
    return (
      <div className={cn('flex flex-col gap-3', className)}>
        <span className="text-xs">{label}</span>
        <div className="relative h-1 w-full overflow-hidden bg-muted">
          <div className="progress-sweep absolute inset-y-0 left-0 bg-primary" />
        </div>
      </div>
    );
  }

  const detail = progress ? detailLine(progress, rate) : '';
  return (
    <div className={cn('flex flex-col gap-2', className)}>
      <Progress value={progress ? Math.round(overallRatio(progress) * 100) : 0}>
        <ProgressLabel>{label}</ProgressLabel>
        <ProgressValue />
      </Progress>
      {detail && (
        <p className="text-xs text-muted-foreground tabular-nums">{detail}</p>
      )}
    </div>
  );
}
