import { ApiError } from "@/lib/api";
import { m } from "@/paraglide/messages";

type Messages = Record<string, (() => string) | undefined>;

/** Fehlerschlüssel des Servers (`auth.wrongCredentials`) → Text in der Sprache der UI */
export const errorText = (error: unknown): string => {
  if (error instanceof ApiError) {
    if (error.status === 0) return m.error_offline();
    const text = (m as unknown as Messages)[`server_${error.code.replace(".", "_")}`];
    return text ? text() : m.error_unknown();
  }
  return m.error_unknown();
};
