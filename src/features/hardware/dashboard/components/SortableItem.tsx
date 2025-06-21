import { useDroppable } from "@dnd-kit/core";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { DashboardItemType } from "../types/dashboardItem";

export const SortableItem = ({
  id,
  children,
}: {
  id: DashboardItemType;
  children: React.ReactNode;
}) => {
  const { setNodeRef, attributes, listeners, transform, transition } =
    useSortable({ id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div ref={setNodeRef} style={style} {...attributes} {...listeners}>
      {children}
    </div>
  );
};

export const DroppableColumn = ({
  id,
  children,
}: {
  id: string;
  children: React.ReactNode;
}) => {
  const { setNodeRef } = useDroppable({ id });

  return (
    <div ref={setNodeRef} className="flex min-h-[20px] flex-1 flex-col gap-4">
      {children}
    </div>
  );
};
