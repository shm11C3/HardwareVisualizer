import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical } from "lucide-react";
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
    <div ref={setNodeRef} style={style} className="relative">
      <div
        {...attributes}
        {...listeners}
        className="absolute top-9.5 left-8 z-10 cursor-grab"
      >
        <GripVertical size={16} />
      </div>

      <div>{children}</div>
    </div>
  );
};
