/** Auswahl aus wenigen Möglichkeiten als Segmente (Radio-Gruppe, mit Tastatur bedienbar) */
import { cn } from "@/lib/cn";

export function Segmented<T extends string>({
  name,
  legend,
  options,
  value,
  onChange,
}: {
  name: string;
  legend: string;
  options: { id: T; text: string }[];
  value: T;
  onChange: (v: T) => void;
}) {
  return (
    <fieldset className="m-0 flex flex-wrap gap-1 rounded-lg border-0 bg-subtle p-1">
      <legend className="sr-only">{legend}</legend>
      {options.map((o) => (
        <label
          key={o.id}
          className={cn(
            "flex h-8 cursor-pointer items-center rounded-md px-3 text-[13px] text-muted has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-fg",
            o.id === value && "bg-bg font-medium text-fg shadow-sm",
          )}
        >
          <input
            type="radio"
            name={name}
            value={o.id}
            checked={o.id === value}
            onChange={() => onChange(o.id)}
            className="sr-only"
          />
          {o.text}
        </label>
      ))}
    </fieldset>
  );
}
