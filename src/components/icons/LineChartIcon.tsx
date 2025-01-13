import type { LineGraphType } from "@/rspc/bindings";

const MonotoneChartIcon = ({ className }: { className?: string }) => (
  <svg
    width="24"
    height="24"
    viewBox="0 0 24 24"
    fill="none"
    className={className}
  >
    <title>Default Chart Icon</title>
    <path
      d="M2 18 C6 6, 10 6, 12 12 C14 18, 18 18, 22 10"
      stroke="currentColor"
      strokeWidth="2"
    />
  </svg>
);

const StepChartIcon = ({ className }: { className?: string }) => (
  <svg
    width="24"
    height="24"
    viewBox="0 0 24 24"
    fill="none"
    className={className}
  >
    <title>Step Chart Icon</title>
    <path
      d="M2 20 L10 20 L10 10 L18 10 L18"
      stroke="currentColor"
      strokeWidth="2"
    />
  </svg>
);

const LinearChartIcon = ({ className }: { className?: string }) => (
  <svg
    width="24"
    height="24"
    viewBox="0 0 24 24"
    fill="none"
    className={className}
  >
    <title>Linear Chart Icon</title>
    <path d="M2 18 L8 8 L14 16 L20 6" stroke="currentColor" strokeWidth="2" />
  </svg>
);

const BasisChartIcon = ({ className }: { className?: string }) => (
  <svg
    width="24"
    height="24"
    viewBox="0 0 24 24"
    fill="none"
    className={className}
  >
    <title>Basis Chart Icon</title>
    <path
      d="M2 18 C6 12, 10 12, 12 14 C14 16, 18 16, 22 12"
      stroke="currentColor"
      strokeWidth="2"
    />
  </svg>
);

export const LineChartIcon = ({
  type,
  className,
}: { type: LineGraphType; className?: string }) => {
  return {
    default: <MonotoneChartIcon className={className} />,
    step: <StepChartIcon className={className} />,
    linear: <LinearChartIcon className={className} />,
    basis: <BasisChartIcon className={className} />,
  }[type];
};
