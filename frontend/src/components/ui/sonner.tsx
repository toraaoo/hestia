import {
  CheckCircleIcon,
  InfoIcon,
  SpinnerIcon,
  WarningIcon,
  XCircleIcon,
} from '@phosphor-icons/react';
import { Toaster as Sonner, type ToasterProps } from 'sonner';

const Toaster = ({ ...props }: ToasterProps) => {
  return (
    <Sonner
      theme="dark"
      className="toaster group"
      position="top-right"
      offset={{ top: 56, right: 16 }}
      icons={{
        success: <CheckCircleIcon className="size-4" />,
        info: <InfoIcon className="size-4" />,
        warning: <WarningIcon className="size-4" />,
        error: <XCircleIcon className="size-4" />,
        loading: <SpinnerIcon className="size-4 animate-spin" />,
      }}
      style={
        {
          '--normal-bg': 'var(--popover)',
          '--normal-text': 'var(--popover-foreground)',
          '--normal-border':
            'color-mix(in oklab, var(--foreground) 10%, transparent)',
          '--border-radius': '0px',
        } as React.CSSProperties
      }
      toastOptions={{
        style: { fontSize: 'var(--text-xs)' },
      }}
      {...props}
    />
  );
};

export { Toaster };
