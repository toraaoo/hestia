import type { LogLine, LogLevel } from "@/lib/types";

const LEVELS: Record<LogLevel, string> = {
  INFO: "text-grass-400",
  WARN: "text-gold-400",
  ERROR: "text-tnt-400",
};

export function LogLines({ lines }: { lines: readonly LogLine[] }) {
  return (
    <>
      {lines.map(([time, level, message], i) => (
        <div key={i} className="flex gap-2 whitespace-pre-wrap">
          <span className="shrink-0 text-text-3">[{time}]</span>
          <span className={`shrink-0 ${LEVELS[level]}`}>[{level}]</span>
          <span className="text-text-2">{message}</span>
        </div>
      ))}
    </>
  );
}
