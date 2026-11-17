import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

const field =
  "h-10 w-full rounded-lg border border-field-border bg-bg px-3 text-[14px] text-fg outline-none transition-colors focus-visible:border-fg";

export const Input = ({ className, ...props }: ComponentProps<"input">) => (
  <input className={cn(field, className)} {...props} />
);

export const Textarea = ({ className, ...props }: ComponentProps<"textarea">) => (
  <textarea className={cn(field, "h-auto min-h-24 py-2", className)} {...props} />
);

export const Label = ({ className, ...props }: ComponentProps<"label">) => (
  // biome-ignore lint/a11y/noLabelWithoutControl: das Feld kommt als Kind oder über htmlFor
  <label className={cn("flex flex-col gap-1.5 text-[13px] text-muted", className)} {...props} />
);

export const Select = ({ className, ...props }: ComponentProps<"select">) => (
  <select className={cn(field, "px-2", className)} {...props} />
);
