import { cn } from '@/lib/utils';

/** The progress dots shared by the step wizards (create, content install). */
export function StepDots({
  steps,
  active,
  className,
}: {
  steps: readonly string[];
  active: number;
  className?: string;
}) {
  return (
    <div className={cn('flex items-center gap-1.5', className)}>
      {steps.map((s, i) => (
        <span
          key={s}
          className={cn(
            'h-1.5 rounded-full transition-all',
            i === active ? 'w-4 bg-ember' : 'w-1.5 bg-border',
          )}
        />
      ))}
    </div>
  );
}
