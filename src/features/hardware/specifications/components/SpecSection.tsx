import type { JSX, ReactNode } from "react";

/**
 * One subject on the System Specifications sheet: a hue-coded heading over a
 * flat list, replacing the former card grid so section height follows content.
 */
export const SpecSection = ({
  icon,
  title,
  badge,
  children,
}: {
  icon: JSX.Element;
  title: string;
  badge?: ReactNode;
  children: ReactNode;
}) => (
  <section className="py-5 first:pt-2">
    <div className="mb-2 flex items-center gap-2.5">
      {icon}
      <h3 className="font-bold text-base">{title}</h3>
      {badge}
    </div>
    {children}
  </section>
);

export const SpecBadge = ({ children }: { children: ReactNode }) => (
  <span className="rounded-full bg-muted px-2 py-0.5 font-mono text-muted-foreground text-xs">
    {children}
  </span>
);
