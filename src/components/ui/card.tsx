import type * as React from "react";

import { cn } from "@/lib/utils";

const Card = ({
  className,
  ref,
  ...props
}: React.HTMLAttributes<HTMLDivElement> &
  React.RefAttributes<HTMLDivElement>) => (
  <div
    ref={ref}
    className={cn(
      "rounded-2xl border-neutral-200 bg-card text-neutral-950 shadow-xs dark:border-neutral-800 dark:text-neutral-50",
      className,
    )}
    {...props}
  />
);
Card.displayName = "Card";

const CardHeader = ({
  className,
  ref,
  ...props
}: React.HTMLAttributes<HTMLDivElement> &
  React.RefAttributes<HTMLDivElement>) => (
  <div
    ref={ref}
    className={cn("flex flex-col space-y-1.5 p-6", className)}
    {...props}
  />
);
CardHeader.displayName = "CardHeader";

const CardTitle = ({
  className,
  ref,
  ...props
}: React.HTMLAttributes<HTMLDivElement> &
  React.RefAttributes<HTMLDivElement>) => (
  <div
    ref={ref}
    className={cn(
      "font-semibold text-2xl leading-none tracking-tight",
      className,
    )}
    {...props}
  />
);
CardTitle.displayName = "CardTitle";

const CardDescription = ({
  className,
  ref,
  ...props
}: React.HTMLAttributes<HTMLDivElement> &
  React.RefAttributes<HTMLDivElement>) => (
  <div
    ref={ref}
    className={cn("text-neutral-500 text-sm dark:text-neutral-400", className)}
    {...props}
  />
);
CardDescription.displayName = "CardDescription";

const CardContent = ({
  className,
  ref,
  ...props
}: React.HTMLAttributes<HTMLDivElement> &
  React.RefAttributes<HTMLDivElement>) => (
  <div ref={ref} className={cn("p-6 pt-0", className)} {...props} />
);
CardContent.displayName = "CardContent";

const CardFooter = ({
  className,
  ref,
  ...props
}: React.HTMLAttributes<HTMLDivElement> &
  React.RefAttributes<HTMLDivElement>) => (
  <div
    ref={ref}
    className={cn("flex items-center p-6 pt-0", className)}
    {...props}
  />
);
CardFooter.displayName = "CardFooter";

export {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
};
