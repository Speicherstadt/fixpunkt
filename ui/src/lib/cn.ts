import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

/** Tailwind-Klassen zusammenführen, spätere gewinnen (shadcn/ui) */
export const cn = (...inputs: ClassValue[]): string => twMerge(clsx(inputs));
