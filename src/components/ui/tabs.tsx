import * as TabsPrimitive from "@radix-ui/react-tabs";
import type * as React from "react";

import { cn } from "@/lib/utils";

function Tabs({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Root>) {
  return (
    <TabsPrimitive.Root
      data-slot="tabs"
      className={cn("flex flex-col gap-2", className)}
      {...props}
    />
  );
}

function TabsList({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.List>) {
  return (
    <TabsPrimitive.List
      data-slot="tabs-list"
      className={cn(
        "inline-flex h-9 w-fit cursor-pointer items-center justify-center rounded-lg bg-neutral-100 p-[3px] text-neutral-500 dark:bg-neutral-800 dark:text-neutral-400",
        className,
      )}
      {...props}
    />
  );
}

function TabsTrigger({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Trigger>) {
  return (
    <TabsPrimitive.Trigger
      data-slot="tabs-trigger"
      className={cn(
        "inline-flex h-[calc(100%-1px)] flex-1 cursor-pointer items-center justify-center gap-1.5 whitespace-nowrap rounded-md border border-neutral-200 px-2 py-1 font-medium text-sm transition-[color,box-shadow] focus-visible:border-neutral-950 focus-visible:outline-1 focus-visible:outline-ring focus-visible:ring-[3px] focus-visible:ring-neutral-950/50 disabled:pointer-events-none disabled:opacity-50 data-[state=active]:bg-white data-[state=active]:text-neutral-950 data-[state=active]:shadow-sm dark:border-neutral-800 dark:dark:data-[state=active]:border-neutral-800 dark:data-[state=active]:border-neutral-200 dark:dark:data-[state=active]:bg-neutral-800/50 dark:data-[state=active]:bg-neutral-950 dark:data-[state=active]:text-neutral-50 dark:focus-visible:border-neutral-300 dark:focus-visible:ring-neutral-300/50 [&_svg:not([class*='size-'])]:size-4 [&_svg]:pointer-events-none [&_svg]:shrink-0",
        className,
      )}
      {...props}
    />
  );
}

function TabsContent({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Content>) {
  return (
    <TabsPrimitive.Content
      data-slot="tabs-content"
      className={cn("flex-1 outline-none", className)}
      {...props}
    />
  );
}

export { Tabs, TabsList, TabsTrigger, TabsContent };
