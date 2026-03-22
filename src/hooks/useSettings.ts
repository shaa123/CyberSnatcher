import { useState } from "react";

export function useSettings() {
  const [showSettings, setShowSettings] = useState(false);
  return { showSettings, setShowSettings };
}
