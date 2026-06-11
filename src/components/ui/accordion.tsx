"use client";

import * as AccordionPrimitive from "@radix-ui/react-accordion";
import { ChevronDown } from "lucide-react";
import type * as React from "react";

import { cn } from "@/lib/utils";

const Accordion = AccordionPrimitive.Root;

const AccordionItem = ({
  className,
  ref,
  ...props
}: React.ComponentPropsWithoutRef<typeof AccordionPrimitive.Item> &
  React.RefAttributes<React.ElementRef<typeof AccordionPrimitive.Item>>) => (
  <AccordionPrimitive.Item
    ref={ref}
    className={cn("border-b", className)}
    {...props}
  />
);
AccordionItem.displayName = "AccordionItem";

const AccordionTrigger = ({
  className,
  children,
  ref,
  ...props
}: React.ComponentPropsWithoutRef<typeof AccordionPrimitive.Trigger> &
  React.RefAttributes<React.ElementRef<typeof AccordionPrimitive.Trigger>>) => (
  <AccordionPrimitive.Header className="flex">
    <AccordionPrimitive.Trigger
      ref={ref}
      className={cn(
        "flex flex-1 cursor-pointer items-center justify-between py-4 font-medium transition-all hover:opacity-60 [&[data-state=open]>svg]:rotate-180",
        className,
      )}
      {...props}
    >
      {children}
      <ChevronDown className="h-6 w-6 shrink-0 transition-transform duration-200" />
    </AccordionPrimitive.Trigger>
  </AccordionPrimitive.Header>
);
AccordionTrigger.displayName = AccordionPrimitive.Trigger.displayName;

const AccordionContent = ({
  className,
  children,
  ref,
  ...props
}: React.ComponentPropsWithoutRef<typeof AccordionPrimitive.Content> &
  React.RefAttributes<React.ElementRef<typeof AccordionPrimitive.Content>>) => (
  <AccordionPrimitive.Content
    ref={ref}
    className="overflow-hidden text-sm transition-all data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down"
    {...props}
  >
    <div className={cn("pt-0 pb-4", className)}>{children}</div>
  </AccordionPrimitive.Content>
);

AccordionContent.displayName = AccordionPrimitive.Content.displayName;

export { Accordion, AccordionContent, AccordionItem, AccordionTrigger };
