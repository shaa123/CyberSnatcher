import { useDownloads } from "../../hooks/useDownloads";
import QueueItem from "./QueueItem";

export default function QueueList() {
  const { items, selectedId, selectItem } = useDownloads();

  if (items.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center p-4">
        <p className="text-xs text-cyber-text-tertiary text-center">
          No downloads yet.
          <br />
          Paste a URL above to get started.
        </p>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto px-2 py-1 space-y-0.5">
      {items.map((item) => (
        <QueueItem
          key={item.id}
          item={item}
          isSelected={selectedId === item.id}
          onSelect={selectItem}
        />
      ))}
    </div>
  );
}
