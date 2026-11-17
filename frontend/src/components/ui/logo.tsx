import { cn } from "@/lib/utils";
import { Icon } from "@/components/ui/icon";

/**
 * Vigil brand mark — uses the custom `vigil-brand.svg` radar/awareness glyph on a
 * gradient tile. Reused in the sidebar and as the app/tray icon.
 */
export function VigilLogo({ size = 32, className }: { size?: number; className?: string }) {
  return (
    <div
      className={cn(
        "inline-grid place-items-center rounded-[30%] bg-gradient-to-br from-(--accent) to-(--accent-2) text-white shadow-sm",
        className,
      )}
      style={{ width: size, height: size }}
    >
      <Icon name="vigil-brand" size={size * 0.62} />
    </div>
  );
}
