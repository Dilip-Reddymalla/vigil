import { Icon } from "@/components/ui/icon";

/** Generic designed empty state (every view must render gracefully with no data). */
export function EmptyView({ title }: { title: string }) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 py-24 text-center">
      <div className="flex size-14 items-center justify-center rounded-full bg-(--surface) text-(--text-tertiary) ring-1 ring-(--border-subdued)">
        <Icon name="empty-state" size={26} />
      </div>
      <h2 className="text-lg font-semibold">{title}</h2>
      <p className="max-w-sm text-sm text-(--text-secondary)">
        No data yet. This section is implemented in Phase 5 of the Vigil plan.
      </p>
    </div>
  );
}
